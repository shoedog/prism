"""Scoped changed-seam mutants; isolated copy and one incremental build directory."""
import hashlib,json,os,re,shutil,subprocess,sys
from pathlib import Path
source=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=False);work=out/'work'
shutil.copytree(source,work,ignore=shutil.ignore_patterns('target','.git'))
files=['src/js_paths.rs','src/ast/js_module_forwarding.rs','src/call_graph.rs','src/repo_loader.rs']
texts={f:(source/f).read_text() for f in files}
mutants=[
 ('N01-refuse-alias-hop','src/js_paths.rs','self.resolve_in(c, from, spec, indexed)','None','integration'),
 ('N02-share-project-options','src/js_paths.rs','let c = self.configs.get(project)?.as_ref()?;','let c = self.configs.values().filter_map(Option::as_ref).next()?;','integration'),
 ('N03-ignore-alias-first-pass','src/js_paths.rs','|| !crate::js_paths_first_pass::absent(','|| false && !crate::js_paths_first_pass::absent(','integration'),
 ('N04-drop-ambient-fence','src/js_paths.rs','if self.snapshot.ambient.values().any(|patterns| {','if false && self.snapshot.ambient.values().any(|patterns| {','integration'),
 ('N05-treat-alias-as-relative','src/js_paths.rs','self.resolve_in(c, from, spec, indexed)','self.relative(from, &format!("./{spec}"), indexed, self.allow_js(project))','integration'),
 ('N06-restore-forward-literal-gate','src/ast/js_module_forwarding.rs','|| binding.kind != ImportBindingKind::MemberImport','|| binding.kind != ImportBindingKind::MemberImport\n            || !binding.module_path.starts_with(".")','integration'),
 ('N07-skip-alias-priming','src/repo_loader.rs','resolver.hop(&project, &file, module, &indexed)','resolver.relative(&file, module, &indexed, true)','lib'),
]
assert all(texts[f].count(a)==1 for _,f,a,_,_ in mutants),[(n,texts[f].count(a)) for n,f,a,_,_ in mutants]
(out/'manifest.json').write_text(json.dumps(mutants,indent=2)+'\n')
env=dict(os.environ,CARGO_TARGET_DIR=str(source.parents[1]),PRISM_TYPESCRIPT=str(Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'))
records=[]
for name,m in [('reference',None)]+[(m[0],m) for m in mutants]:
 for f,t in texts.items():(work/f).write_text(t.replace(m[2],m[3]) if m and f==m[1] else t)
 suite=m[4] if m else 'integration';command=['cargo','test','--offline','--manifest-path',str(work/'Cargo.toml')]
 command+=['--test','integration','js_paths_p2_test'] if suite=='integration' else ['--lib','paths_projection_tests']
 with (out/(name+'.log')).open('w') as f:p=subprocess.run(command,stdout=f,stderr=subprocess.STDOUT,env=env)
 log=(out/(name+'.log')).read_text();groups=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed;',log)
 selected=sum(sum(map(int,g)) for g in groups);failed=sum(int(g[1]) for g in groups);admissible=selected>0 and not re.search(r'error\[E\d+\]',log)
 if not m:assert p.returncode==0 and selected==10 and failed==0,'reference failed'
 else:
  r={'mutant':name,'admissible':admissible,'killed':admissible and failed>0,'selected':selected,'failed':failed,'source_sha256':hashlib.sha256((work/m[1]).read_bytes()).hexdigest()};records.append(r);print(json.dumps(r),flush=True)
 (out/'results.json').write_text(json.dumps(records,indent=2)+'\n')
for f,t in texts.items():(work/f).write_text(t)
assert all(r['killed'] for r in records),records
