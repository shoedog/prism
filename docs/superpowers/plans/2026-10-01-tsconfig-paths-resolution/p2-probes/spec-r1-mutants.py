"""Three scoped W1 mutants, one shared incremental build cache, serial execution."""
import hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path
source,out=Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve()
out.mkdir(parents=True,exist_ok=False);work=out/'work'
shutil.copytree(source,work,ignore=shutil.ignore_patterns('target','.git'))
file='src/js_exports.rs';original=(source/file).read_text()
arm="""                    if matches!(target, JsExportTarget::ImportForward { .. })
                        && !module_path.starts_with('.')
                    {
                        return ExportLookup::BlockedClaim;
                    }
"""
assert original.count(arm)==1
mutants=[
 ('W01-drop-blocked-forward',arm,''),
 ('W02-block-relative-forward',arm,arm.replace("\n                        && !module_path.starts_with('.')",'')),
 ('W03-block-unresolved-reexport',arm,arm.replace('matches!(target, JsExportTarget::ImportForward { .. })','true')),
]
(out/'manifest.json').write_text(json.dumps(mutants,indent=2)+'\n')
env=dict(os.environ,CARGO_TARGET_DIR=str(source/'target'))
records=[]
for name,mutation in [('reference',None)]+[(m[0],m) for m in mutants]:
    text=original if not mutation else original.replace(mutation[1],mutation[2])
    (work/file).write_text(text)
    with (out/(name+'.log')).open('w') as log:
        p=subprocess.run(['cargo','test','--offline','--manifest-path',str(work/'Cargo.toml'),'--test','integration','js_paths_p2_test'],stdout=log,stderr=subprocess.STDOUT,env=env)
    log=(out/(name+'.log')).read_text()
    groups=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed;',log)
    selected=sum(sum(map(int,g)) for g in groups);failed=sum(int(g[1]) for g in groups)
    admissible=selected==12 and not re.search(r'error\[E\d+\]',log)
    if not mutation: assert p.returncode==0 and admissible and failed==0,log[-5000:]
    else:
        expected='legacy_star_nonrelative_forwards_preserve_complete_main_rows' if name.startswith('W01') else 'legacy_star_other_unresolved_claims_keep_existing_behavior'
        record={'mutant':name,'admissible':admissible,'selected':selected,'failed':failed,'killed':admissible and failed>0 and expected+' ... FAILED' in log,'expected_regression':expected,'mutated_sha256':hashlib.sha256(text.encode()).hexdigest()}
        records.append(record);print(json.dumps(record),flush=True)
        (out/'results.json').write_text(json.dumps(records,indent=2)+'\n')
(work/file).write_text(original)
assert all(r['killed'] for r in records),records
# Remove only this runner's reproducible source copy; keep manifest/logs/results.
shutil.rmtree(work)
