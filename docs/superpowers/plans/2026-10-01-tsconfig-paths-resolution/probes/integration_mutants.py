"""Eleven legacy integration mutants plus bounded alias/cache/config repairs."""
import subprocess,sys,shutil,json
from pathlib import Path
repo=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();target=Path(sys.argv[3]).resolve()
mutations={
 'I01-no-config-fingerprint':('src/js_paths_snapshot.rs','for (p, b) in &self.configs {','for (p, b) in self.configs.iter().take(0) {','js_paths_repair_test::js_paths_repair_config_swap_cache_key'),
 'I02-no-post-merge-map':('src/cpg/build.rs','            cached_cg.apply_js_paths(scope_inputs);','            // mutant omitted post-merge map','js_paths_test::js_paths_config_change_and_incremental_rebuild'),
 'I03-no-span-requirement':('src/resolution.rs',"if !binding.module_path.trim().starts_with('.') && resolved.span.is_none() {","if false && !binding.module_path.trim().starts_with('.') && resolved.span.is_none() {",'js_paths_test::js_paths_cjs_terminal_without_span_preserves_base'),
 'I04-no-binding-position-guard':('src/resolution.rs',"if !binding.module_path.trim().starts_with('.')\n            && (", "if false && !binding.module_path.trim().starts_with('.')\n            && (",'js_paths_r1_test::js_paths_require_alias_preserves_base_even_with_esm_same_module'),
 'I05-no-skipped-star-guard':('src/resolution.rs',"if !binding.module_path.trim().starts_with('.') && resolved.via_unresolved_star {", "if false && !binding.module_path.trim().starts_with('.') && resolved.via_unresolved_star {",'js_paths_r1_test::js_paths_r1_skipped_star_preserves_base'),
 'I06-all-file-occupancy':('src/js_paths_snapshot.rs','self.opaque_directories.contains(*p)', 'true || self.opaque_directories.contains(*p)','js_paths_r1_test::js_paths_unrelated_text_keeps_topology'),
 'I07-no-proven-empty-facts':('src/call_graph.rs','if !exports.is_empty() || !parsed.tree.root_node().has_error() {','if !exports.is_empty() {','js_paths_r1_test::js_paths_r1_membership_barriers_dot_and_relative'),
 'I08-no-opaque-star-guard':('src/js_exports.rs','facts.skipped_expr_count > facts.skipped_decl_reasons.values().sum::<usize>()','false','js_paths_r1_test::js_paths_r1_skipped_star_preserves_base'),
 'I09-ignore-skipped-declaration-name':('src/js_exports.rs','facts.module_value_bindings.contains(name)','false','js_paths_r1_test::js_paths_r1_skipped_star_preserves_base'),
 'I10-trim-alias-cache-key':('src/call_graph.rs','.get(&(caller.into(), module.into()))','.get(&(caller.into(), module.trim().into()))','js_paths_r1_test::js_paths_r1_membership_barriers_dot_and_relative'),
 'I11-taint-namespace-terminal':('src/ast.rs','                        via_unresolved_star: false,','                        via_unresolved_star: true,','js_paths_test::js_paths_s1b_namespace_star_proof_is_reused'),
}
mutations.update({
 'I12-legacy-alias-hop-table':('src/resolution.rs','let Some(exports) = self.js_ts_path_exports.get(allow_js) else {','let Some(exports) = Some(&self.js_ts_resolved_exports) else {','js_paths_repair_test::js_paths_repair_barrel_hops'),
 'I13-no-probe-occupancy':('src/js_paths_snapshot.rs','(p.clone(), self.entries.get(p).copied())','(p.clone(), None::<u8>)','js_paths_repair_test::js_paths_repair_cache_occupancy'),
 'I14-last-config-wins':('src/js_paths_snapshot.rs','.and_modify(|prior| *prior = None)','.and_modify(|prior| *prior = bytes.clone())','js_paths_snapshot::tests::config_case_collision_is_order_independent_barrier'),
})
# Enumerate every selector and mutation before launching a build.
for label,(file,old,new,test) in mutations.items():
 assert (repo/file).read_text().count(old)==1,(label,old)
results=[]
for label,(file,old,new,test) in mutations.items():
 d=out/'integration-mutants'/label/'repo'
 shutil.copytree(repo,d,dirs_exist_ok=True,ignore=shutil.ignore_patterns('target','__pycache__','.git'))
 p=d/file;s=p.read_text();assert s.count(old)==1,(label,s.count(old));p.write_text(s.replace(old,new))
 env=__import__('os').environ.copy();env['CARGO_TARGET_DIR']=str(target)
 cmd=['cargo','test','--offline','--manifest-path',str(d/'Cargo.toml'),*(['--lib'] if test.startswith('js_paths_snapshot::') else ['--test','integration']),test,'--','--exact']
 proc=subprocess.run(cmd,capture_output=True,text=True,env=env)
 log=proc.stdout+proc.stderr;(d.parent/'test.log').write_text(log)
 # Read the behavioral failure; compile/setup/zero-test failures are inadmissible.
 admissible='running 1 test' in log and 'panicked at' in log and 'test result: FAILED.' in log
 results.append({'mutant':label,'test':test,'killed':admissible,'status':proc.returncode})
 if not admissible:raise RuntimeError(label+' inadmissible or surviving; inspect test.log')
(out/'integration-mutants-summary.json').write_text(json.dumps(results,indent=2));print(json.dumps(results))
