"""Four bounded integration mutants, each in its own copied source tree."""
import subprocess,sys,shutil,json
from pathlib import Path
repo=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();target=Path(sys.argv[3]).resolve()
mutations={
 'I01-no-config-fingerprint':('src/js_paths_snapshot.rs','for (p, b) in &self.configs {','for (p, b) in self.configs.iter().take(0) {','js_paths_test::js_paths_config_change_and_incremental_rebuild'),
 'I02-no-post-merge-map':('src/cpg/build.rs','            cached_cg.apply_js_paths(scope_inputs);','            // mutant omitted post-merge map','js_paths_test::js_paths_config_change_and_incremental_rebuild'),
 'I03-no-span-requirement':('src/resolution.rs',"if !binding.module_path.starts_with('.') && resolved.span.is_none() {","if false && !binding.module_path.starts_with('.') && resolved.span.is_none() {",'js_paths_test::js_paths_cjs_terminal_without_span_preserves_base'),
 'I04-no-binding-position-guard':('src/resolution.rs',"if !binding.module_path.starts_with('.')\n            && (", "if false && !binding.module_path.starts_with('.')\n            && (",'js_paths_test::js_paths_require_alias_preserves_base_even_with_esm_same_module'),
}
results=[]
for label,(file,old,new,test) in mutations.items():
 d=out/'integration-mutants'/label/'repo'
 shutil.copytree(repo,d,dirs_exist_ok=True,ignore=shutil.ignore_patterns('target','__pycache__'))
 p=d/file;s=p.read_text();assert s.count(old)==1,(label,s.count(old));p.write_text(s.replace(old,new))
 env=__import__('os').environ.copy();env['CARGO_TARGET_DIR']=str(target)
 cmd=['cargo','test','--offline','--manifest-path',str(d/'Cargo.toml'),'--test','integration',test,'--','--exact']
 proc=subprocess.run(cmd,capture_output=True,text=True,env=env)
 log=proc.stdout+proc.stderr;(d.parent/'test.log').write_text(log)
 # Read the behavioral failure; compile/setup/zero-test failures are inadmissible.
 admissible='running 1 test' in log and 'panicked at' in log and 'test result: FAILED.' in log
 results.append({'mutant':label,'test':test,'killed':admissible,'status':proc.returncode})
 if not admissible:raise RuntimeError(label+' inadmissible or surviving; inspect test.log')
(out/'integration-mutants-summary.json').write_text(json.dumps(results,indent=2));print(json.dumps(results))
