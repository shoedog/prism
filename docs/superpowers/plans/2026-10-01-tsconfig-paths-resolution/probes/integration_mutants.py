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
 'I08-no-opaque-star-guard':('src/js_exports.rs','facts.skipped_expr_count > facts.skipped_decl_reasons.values().sum::<usize>()','false','js_paths_cap_test::structural_opaque_ts_hop'),
 'I09-ignore-skipped-declaration-name':('src/js_exports.rs','facts.module_value_bindings.contains(name)','false','js_paths_r1_test::js_paths_r1_skipped_star_preserves_base'),
 'I10-trim-alias-cache-key':('src/call_graph.rs','.get(&(caller.into(), module.into()))','.get(&(caller.into(), module.trim().into()))','js_paths_r1_test::js_paths_r1_membership_barriers_dot_and_relative'),
 'I11-taint-namespace-terminal':('src/ast.rs','                        via_unresolved_star: false,','                        via_unresolved_star: true,','js_paths_test::js_paths_s1b_namespace_star_proof_is_reused'),
}
mutations.update({
 'I12-legacy-alias-hop-table':('src/resolution.rs','let Some(exports) = self.js_ts_path_exports.get(allow_js) else {','let Some(exports) = Some(&self.js_ts_resolved_exports) else {','js_paths_repair_test::js_paths_repair_barrel_hops'),
 'I13-no-probe-occupancy':('src/js_paths_snapshot.rs','''(
                    p.clone(),
                    self.first_pass_facts
                        .lock()
                        .unwrap()
                        .get(p)
                        .copied()
                        .unwrap_or_else(|| {
                            self.type_entries
                                .get(p)
                                .or_else(|| self.entries.get(p))
                                .copied()
                        }),
                )''','(p.clone(), None::<u8>)','js_paths_repair_test::js_paths_repair_cache_occupancy'),
 'I14-last-config-wins':('src/js_paths_snapshot.rs','.and_modify(|prior| *prior = None)','.and_modify(|prior| *prior = bytes.clone())','js_paths_snapshot::tests::config_case_collision_is_order_independent_barrier'),
})
mutations.update({
 'I15-no-js-hop-cut':('src/js_paths.rs','(!js_family(&q)).then_some(q)','Some(q)','js_paths_cap_test::structural_js_module_and_hop_terminals'),
 'I16-no-config-types-cut':('src/js_paths.rs','get("types")','get("__absent_types")','js_paths_r2b_test::uncovered_config_type_inputs'),
 'I17-no-project-reference-cut':('src/js_paths_snapshot.rs','let referenced = !references.is_empty();','let referenced = false;','js_paths_cap_test::structural_project_triple_references'),
 'I18-no-ambient-content-hash':('src/js_paths_snapshot.rs','if !patterns.is_empty() || referenced {','if false {','js_paths_cap_test::structural_scan_dependency_add_remove_edit'),
 'I19-no-ambient-read-occupancy':('src/js_paths_snapshot.rs','format!("scan:{rel}"),','"scan:one-file".into(),','js_paths_cap_test::structural_scan_dependency_add_remove_edit'),
 'I20-no-js-module-cut':('src/js_paths.rs','if js_family(&q)\n','if false && js_family(&q)\n','js_paths_r2d_test::node_modules_file_directory_and_types_keep_base'),
 'I21-no-typeRoots-coverage':('src/js_paths.rs','.map(|root| self.snapshot.covered_root(dir(p), root))','.map(|root| Some(root.clone()))','js_paths_r2b_test::uncovered_config_type_inputs'),
 'I22-no-reference-path-arm':('src/js_paths_snapshot.rs','if matches!(key, "path" | "types") {','if key == "types" {','js_paths_r2b_test::uncovered_triple_reference_inputs'),
 'I23-no-reference-types-arm':('src/js_paths_snapshot.rs','if matches!(key, "path" | "types") {','if key == "path" {','js_paths_r2b_test::uncovered_triple_reference_inputs'),
 'I24-no-reference-closure':('src/js_paths_snapshot.rs','self.references.iter().filter','std::collections::BTreeMap::<String, Vec<(String, String)>>::new().iter().filter','js_paths_r2b_test::transitive_references_and_cycles'),
 'I27-global-export-priming':('src/repo_loader.rs','let mut pending = BTreeSet::new();','let mut pending: BTreeSet<String> = files.keys().cloned().collect();','repo_loader::paths_projection_tests::primes_alias_export_closure_without_unrelated_exports'),
 'I28-no-reachable-export-priming':('src/repo_loader.rs','if let Some(target) = resolver.relative(&file, module, &indexed, true) {\n                        pending.insert(target);\n                    }','let _ = resolver.relative(&file, module, &indexed, true);','repo_loader::paths_projection_tests::primes_alias_export_closure_without_unrelated_exports'),
 'I29-no-package-types':('src/js_paths_snapshot.rs','["types", "typings", "typesVersions"]','["__types", "typings", "typesVersions"]','js_paths_r2b_test::type_package_redirects_must_reach_scanned_inputs'),
 'I30-no-package-typings':('src/js_paths_snapshot.rs','["types", "typings", "typesVersions"]','["types", "__typings", "typesVersions"]','js_paths_r2b_test::type_package_redirects_must_reach_scanned_inputs'),
 'I31-no-package-typesVersions':('src/js_paths_snapshot.rs','["types", "typings", "typesVersions"]','["types", "typings", "__typesVersions"]','js_paths_r2b_test::type_package_redirects_must_reach_scanned_inputs'),
 'I32-broad-package-exports-cut':('src/js_paths_snapshot.rs','Some(out)','if object.contains_key("exports") { out.push("/__outside.d.ts".into()); } Some(out)','js_paths_r2b_test::type_package_redirects_must_reach_scanned_inputs'),
 'I33-no-opaque-type-metadata':('src/js_paths_snapshot.rs','return false;\n                };\n                for target in redirects','continue;\n                };\n                for target in redirects','js_paths_r2b_test::opaque_type_metadata_declines'),
})
mutations.update({
 'I34-absent-declines':('src/js_paths_snapshot.rs','TypeInput::Absent => true,','TypeInput::Absent => false,','js_paths_r2c_test::absent_type_inputs_bind'),
 'I35-no-installed-ambient':('src/js_paths_snapshot.rs','let patterns = if ts {','let patterns = if false && ts {','js_paths_r2c_test::installed_matching_ambient_keeps_base'),
 'I36-all-wildcards-match':('src/js_paths.rs',"fn ambient_matches(pattern: &str, spec: &str) -> bool {","fn ambient_matches(pattern: &str, spec: &str) -> bool { if pattern.contains('*') { return true; }",'js_paths_r2c_test::installed_nonmatching_wildcard_binds'),
 'I37-outside-is-absent':('src/js_paths_snapshot.rs','return TypeInput::Unsafe;\n        };\n        self.probes', 'return TypeInput::Absent;\n        };\n        self.probes','js_paths_r2c_test::outside_type_inputs_decline'),
 'I38-no-secondary-type-lookup':('src/js_paths_snapshot.rs','let mut directory = base;','if roots.is_some() { return TypeInput::Absent; } let mut directory = base;','js_paths_r2c_test::custom_roots_miss_uses_secondary_package_lookup'),
})
mutations.update({
 'I39-no-first-pass-dependencies':('src/js_paths_snapshot.rs','''        self.probes.lock().unwrap().insert(p.into());
        self.complete''','''        self.complete''','js_paths_first_pass::tests::every_location_is_an_occupancy_dependency'),
 'I40-restore-blanket-JS-cut':('src/js_paths.rs','if js_family(&q)\n','if js_family(&q) || js_family(&q)\n','js_paths_r2d_test::js_only_first_pass_absent_binds_exact'),
 'I41-no-node-file':('src/js_paths_first_pass.rs','let suffixes: &[&str] = match original {','if stem.contains("node_modules/") && !stem.contains("/@types/") && !stem.ends_with("/index") { return true; } let suffixes: &[&str] = match original {','js_paths_r2d_test::node_modules_file_directory_and_types_keep_base'),
 'I42-no-node-index':('src/js_paths_first_pass.rs','let suffixes: &[&str] = match original {','if stem.contains("node_modules/") && !stem.contains("/@types/") && stem.ends_with("/index") { return true; } let suffixes: &[&str] = match original {','js_paths_r2d_test::node_modules_file_directory_and_types_keep_base'),
 'I43-no-at-types':('src/js_paths_first_pass.rs','absent &= pass.module(&format!("{modules}/@types"), &mangle(spec), false)?;','/* omit @types */','js_paths_r2d_test::node_modules_file_directory_and_types_keep_base'),
 'I44-no-module-types':('src/js_paths_first_pass.rs','types: Value,','#[serde(rename="__types")] types: Value,','js_paths_r2d_test::package_fields_and_versions_keep_base'),
 'I45-no-module-typings':('src/js_paths_first_pass.rs','typings: Value,','#[serde(rename="__typings")] typings: Value,','js_paths_r2d_test::package_fields_and_versions_keep_base'),
 'I46-no-module-versions':('src/js_paths_first_pass.rs','range_matches(range) == Some(true)','false','js_paths_r2d_test::package_fields_and_versions_keep_base'),
 'I47-no-module-typeRoots':('src/js_paths_first_pass.rs','for root in roots.unwrap_or_default() {','for root in roots.unwrap_or_default().iter().take(0) {','js_paths_r2d_test::custom_type_roots_file_directory_and_scoped_names_keep_base'),
 'I48-no-module-main':('src/js_paths_first_pass.rs','main: Value,','#[serde(rename="__main")] main: Value,','js_paths_r2d_test::package_fields_and_versions_keep_base'),
})
mutations.update({
 'I49-no-JS-object-order':('src/js_paths_first_pass.rs','.filter(|n| *n != u32::MAX && n.to_string() == *key)','.filter(|_| false)','js_paths_r2d_test::types_versions_javascript_object_order_and_large_versions'),
 'I50-no-last-duplicate-value':('src/js_paths_first_pass.rs','.find(|(name, _)| name == &key)','.find(|_| false)','js_paths_r2d_test::types_versions_javascript_object_order_and_large_versions'),
 'I51-range-u64-overflow':('src/js_paths_first_pass.rs','let n = part.parse().ok()?;','let n = part.parse::<u64>().ok()? as f64;','js_paths_r2d_test::types_versions_javascript_object_order_and_large_versions'),
 'I52-relative-only-package-field':('src/js_paths_first_pass.rs','self.snapshot.input_path(p, s)','crate::js_paths_syntax::norm(p, s)','js_paths_r2d_test::package_entries_normalize_inside_the_captured_root'),
 'I53-relative-only-version-substitution':('src/js_paths_first_pass.rs','self.snapshot.input_path(base, &target)','crate::js_paths_syntax::norm(base, &target)','js_paths_r2d_test::package_entries_normalize_inside_the_captured_root'),
 'I54-remap-out-of-package-entry':('src/js_paths_first_pass.rs','entry.strip_prefix(&format!("{p}/"))','entry.strip_prefix(&format!("{p}/")).or(Some("index"))','js_paths_r2d_test::package_entries_normalize_inside_the_captured_root'),
 'I55-truthy-empty-exact-key':('src/js_paths_first_pass.rs','if key.is_empty() {','if false && key.is_empty() {','js_paths_r2d_test::package_entries_normalize_inside_the_captured_root'),
 'I56-no-paths-first-pass':('src/js_paths_first_pass.rs','let mut absent = pass.relative(target, true, 0)?;','let mut absent = true;','js_paths_r2d_test::first_pass_paths_keep_base'),
 'I57-Rust-version-whitespace':('src/js_paths_first_pass.rs','fn version_whitespace(c: char) -> bool {','fn version_whitespace(c: char) -> bool { return c.is_whitespace();','js_paths_r2d_test::types_versions_ecmascript_whitespace'),
 'I58-no-root-file-boundary':('src/js_paths_first_pass.rs','if p.is_empty() {','if false && p.is_empty() {','js_paths_r2d_test::package_root_file_candidates_are_outside_capture'),
 'I59-no-physical-absence':('src/js_paths_snapshot.rs','fn first_pass_occupancy(&self, p: &str) -> Option<u8> {','fn first_pass_occupancy(&self, p: &str) -> Option<u8> { return self.type_entries.get(p).or_else(|| self.entries.get(p)).copied();','js_paths_r2d_test::first_pass_filesystem_aliases_are_occupied'),
})
# Enumerate every selector and mutation before launching a build.
for label,(file,old,new,test) in mutations.items():
 assert (repo/file).read_text().count(old)==(2 if label in ['I16-no-config-types-cut','I34-absent-declines','I46-no-module-versions'] else 1),(label,old)
assert (repo/'src/js_paths.rs').read_text().count('if self.snapshot.kind(&q).is_some() {')==1
if len(sys.argv)>4:
 selected=sys.argv[4];assert selected in mutations,selected;mutations={selected:mutations[selected]}
results=[]
for label,(file,old,new,test) in mutations.items():
 d=out/'integration-mutants'/label/'repo'

 for name in ['src','tests','vendor','scripts/callable-observations','docs/eval/receiver-closure']:
  shutil.copytree(repo/name,d/name,dirs_exist_ok=True)
 for name in ['Cargo.toml','Cargo.lock','build.rs']:shutil.copy2(repo/name,d/name)
 p=d/file;s=p.read_text();p.write_text(s.replace(old,new))
 if label=='I56-no-paths-first-pass':
  q=d/'src/js_paths.rs';s=q.read_text();q.write_text(s.replace('if self.snapshot.kind(&q).is_some() {','if (q.ends_with(".js") || q.ends_with(".jsx")) && self.snapshot.kind(&q).is_some() {'))
 if label=='I19-no-ambient-read-occupancy':
  # Coverage now provides a second occupancy witness; remove both witnesses.
  p.write_text(p.read_text().replace('self.covered.insert(rel.into(), 0);','/* omit read coverage */'))
 # Seed immutable dependency outputs, excluding Prism and test artifacts.
 # Every mutation builds in its own target, so Cargo cannot adopt another
 # mutant's library or test executable.
 isolated=d.parent/'cargo';seed=target/'debug'
 for component in ['deps','build','.fingerprint']:
  shutil.copytree(seed/component,isolated/'debug'/component,dirs_exist_ok=True,
    ignore=shutil.ignore_patterns('*prism*','integration-*') if component!='build' else None)
 env=__import__('os').environ.copy();env['CARGO_TARGET_DIR']=str(isolated)
 cmd=['cargo','test','--offline','--manifest-path',str(d/'Cargo.toml'),*(['--lib'] if test.startswith(('js_paths_snapshot::','js_paths_first_pass::','repo_loader::')) else ['--test','integration']),test,'--','--exact']
 proc=subprocess.run(cmd,capture_output=True,text=True,env=env)
 log=proc.stdout+proc.stderr;(d.parent/'test.log').write_text(log)
 # Read the behavioral failure; compile/setup/zero-test failures are inadmissible.
 admissible='running 1 test' in log and 'panicked at' in log and 'test result: FAILED.' in log
 results.append({'mutant':label,'test':test,'killed':admissible,'status':proc.returncode,'isolated_target':str(isolated)})
 (out/'integration-mutants-summary.json').write_text(json.dumps(results,indent=2))
 print(label, 'KILLED' if admissible else 'SURVIVED',flush=True)
 if not admissible and not ('running 1 test' in log and 'test result: ok.' in log):raise RuntimeError(label+' inadmissible; inspect test.log')
 shutil.copy2(p,d.parent/Path(file).name)
 shutil.rmtree(isolated);shutil.rmtree(d)
(out/'integration-mutants-summary.json').write_text(json.dumps(results,indent=2));print(json.dumps(results))
