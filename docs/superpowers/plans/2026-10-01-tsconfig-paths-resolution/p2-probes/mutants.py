"""Bounded P2 mutants in an isolated Gitless source copy. Never edit live source."""
import hashlib, io, json, os, re, shutil, subprocess, tarfile
from pathlib import Path
repo=Path.cwd();out=repo/'target/p2-plan';work=out/'base-control'
if not (work/'Cargo.toml').exists():
    work.mkdir(parents=True,exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(subprocess.check_output(['git','archive','HEAD']))) as archive:
        archive.extractall(work)
env=os.environ.copy();env['CARGO_TARGET_DIR']=str(repo/'target/p2-base-control')
files=['src/js_paths.rs','src/js_paths_first_pass.rs','src/cpg_cache.rs','src/navigation/call_edge_cache.rs']
for name in files+['tests/integration/main.rs','tests/integration/js_paths_common.rs','tests/integration/js_paths_p2_test.rs','tests/integration/js_paths_cap_test.rs','tests/integration/js_paths_repair_test.rs']:
    shutil.copy2(repo/name,work/name)
text={name:(repo/name).read_text() for name in files}
gate='|| !crate::js_paths_first_pass::relative_absent(self.snapshot, &p)'
mutants=[
 ('P2-M01-refuse-js','src/js_paths.rs','Some(q)\n    }\n    fn prove_path','(!js_family(&q)).then_some(q)\n    }\n    fn prove_path','integration'),
 ('P2-M02-ignore-first-pass','src/js_paths.rs',gate,'|| false','integration'),
 ('P2-M03-ignore-allowjs','src/js_paths.rs','!allow_js','false','integration'),
 ('P2-M04-no-explicit-js','src/js_paths.rs','if js_family(&p) {','if false {','integration'),
 ('P2-M05-nonrelative-search','src/js_paths_first_pass.rs','Pass { snapshot }.relative(target, true, 0).unwrap_or(false)','absent(snapshot, "barrel.ts", "@lib", target, Some(&["types".into()]))','integration'),
 ('P2-M06-no-suffix-replacement','src/js_paths_first_pass.rs','absent &= self.extensions(stem, extension, ts);','absent &= true;','integration'),
 ('P2-M07-no-implicit-extension','src/js_paths_first_pass.rs','absent & self.extensions(p, "", ts)','absent','integration'),
 ('P2-M08-no-declarations','src/js_paths_first_pass.rs','&[".ts", ".tsx", ".d.ts"]','&[".ts", ".tsx"]','integration'),
 ('P2-M09-no-dmts','src/js_paths_first_pass.rs','&[".mts", ".d.mts"]','&[".mts"]','integration'),
 ('P2-M10-no-dcts','src/js_paths_first_pass.rs','&[".cts", ".d.cts"]','&[".cts"]','integration'),
 ('P2-M11-cpg-cache-version','src/cpg_cache.rs','const CACHE_VERSION: u32 = 106;','const CACHE_VERSION: u32 = 105;','lib'),
 ('P2-M12-nav-cache-version','src/navigation/call_edge_cache.rs','const NAV_CALL_EDGE_CACHE_VERSION: u32 = 62;','const NAV_CALL_EDGE_CACHE_VERSION: u32 = 61;','lib'),
]
defects=[(label,text[file].count(old)) for label,file,old,new,suite in mutants if text[file].count(old)!=1]
assert not defects,defects
(out/'p2-mutant-manifest.json').write_text(json.dumps(mutants,indent=2)+'\n')
records=[]
for label,mutation in [('reference',None)]+[(m[0],m) for m in mutants]:
    for file,s in text.items():
        if mutation and file==mutation[1]:s=s.replace(mutation[2],mutation[3])
        (work/file).write_text(s)
    suite=mutation[4] if mutation else 'integration'
    command=['cargo','test','--offline','--manifest-path',str(work/'Cargo.toml')]
    command+=['--test','integration','js_paths_p2_test'] if suite=='integration' else ['--lib','pinned']
    with (out/(label+'.log')).open('w') as f:
        p=subprocess.run(command,stdout=f,stderr=subprocess.STDOUT,env=env)
    log=(out/(label+'.log')).read_text()
    groups=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed;',log)
    selected=sum(sum(map(int,g)) for g in groups)
    failed=sum(int(g[1]) for g in groups)
    admissible=selected>0 and not re.search(r'error\[E\d+\]',log)
    if mutation:
        record={'mutant':label,'admissible':admissible,'killed':admissible and failed>0,'selected':selected,'failed':failed,
                'source_sha256':hashlib.sha256((work/mutation[1]).read_bytes()).hexdigest()}
        records.append(record);print(json.dumps(record),flush=True)
    else:
        assert p.returncode==0 and selected==6 and failed==0, 'inadmissible/failed reference'
    (out/'p2-mutants.json').write_text(json.dumps(records,indent=2)+'\n')
for file,s in text.items():(work/file).write_text(s)
assert len(records)==12 and all(r['killed'] for r in records),records
