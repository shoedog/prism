"""Apply each bounded S1b-4 mutant alone; restore exact bytes in finally.
Usage: mutate_s1b4.py PROTO_ROOT OUTPUT_DIR
One attempt per mutant; compile/setup/zero-test failures are inadmissible.
Never run concurrently with source editing, review, or another cargo process.
"""
from pathlib import Path
import difflib,json,re,subprocess,sys
root=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True)

def edit(path,old,new):
 return (path,old,new)
ns='src/resolution_js_namespace.rs';proof='src/ast/js_binding_namespace.rs';cg='src/call_graph.rs';res='src/resolution.rs';core='src/ast/js_binding.rs';exports='src/js_exports.rs'
start='        let missing = || {'
cases=[
('D-M1', [edit(ns,start,'        return None;\n'+start)], 'd1_direct_and_directory_decoys'),
('D-M2', [edit(proof,'        let binding = self.js_ts_site_binding(ident, name, cache);','        return Some((JsBinding::Import, declarations.first().and_then(|(_, s)| s.clone())));\n        let binding = self.js_ts_site_binding(ident, name, cache);')], 'd4_scope_write_recovery_and_positions'),
('D-M3', [edit(ns,'                        confidence: ResolutionConfidence::NameOnly,','                        confidence: ResolutionConfidence::Exact,')], 'd5_authoritative_missing_member_and_fallback'),
('D-M4', [edit(cg,'            let Some((binding, module)) = parsed.js_ts_namespace_binding(q, cache) else {','            if let Some(module_path) = parsed.extract_imports().get(parsed.node_text(&q)).cloned() { return JsLocalBinding::NamespaceImport { module_path }; }\n            let Some((binding, module)) = parsed.js_ts_namespace_binding(q, cache) else {')], 'd6_non_namespace_imports_keep_base'),
('D-M5-import', [edit(core,'if local_route && self.js_ts_scoped_written(scope, name, cache) {','if false && local_route && self.js_ts_scoped_written(scope, name, cache) {')], 'd9_written_import_and_export_keep_base'),
('D-M5-kind', [edit(proof,'        let binding = self.js_ts_site_binding(ident, name, cache);','        let binding = match self.js_ts_site_binding(ident, name, cache) { JsBinding::MayCall => JsBinding::Refused("not_callable"), b => b };')], 'd9_written_import_and_export_keep_base'),
('D-M6-span', [edit(res,'                        .is_none_or(|(s, e)| fid.start_line == s && fid.end_line == e)','                        .is_none_or(|(_s, _e)| true)')], 'd1_direct_and_directory_decoys'),
('D-M6-wrapped', [edit(res,'        if resolved.wrapped && !site.jsx_element {','        if false && resolved.wrapped && !site.jsx_element {')], 'd2_wrapped_call_and_jsx'),
('D-M7', [edit(ns,start,'        if !self.functions.contains_key(member) { return Some(ResolutionOutcome::dropped(DropReason::UnknownName)); }\n'+start)], 'd3_rename_named_and_star_barrels'),
('D-M8', [edit(proof,'        let binding = self.js_ts_site_binding(ident, name, cache);','        let mut binding = self.js_ts_site_binding(ident, name, cache);\n        if cache.write_targets.as_ref().is_some_and(|w| w.contains_key(name)) { binding=JsBinding::MayCall; }')], 'd4_scope_write_recovery_and_positions'),
('D-M9', [edit(core,'        if !self.js_ts_recovery_sealed(scope, site, cache) {','        if false && !self.js_ts_recovery_sealed(scope, site, cache) {')], 'd4_scope_write_recovery_and_positions'),
('D-M10', [edit(ns,start,'        if site.receiver_lexically_bound { return None; }\n'+start)], 'd4_scope_write_recovery_and_positions'),
('D-M11', [edit(ns,'                return Some(missing());','                return None;')], 'd5_authoritative_missing_member_and_fallback'),
('D-M12-named', [edit(exports,'    resolve_js_exports(&namespace_raw, resolve_module).resolved','    resolve_js_exports(&namespace_raw, resolve_module).resolved.into_iter().filter(|(file, _)| !raw[file].named.values().any(|t| matches!(t, JsExportTarget::ReExport{..} | JsExportTarget::ImportForward{..}))).collect()')], 'd10_alias_opacity_direct_named_star_forwarded_and_d4_pin'),
('D-M12-star', [edit(exports,'    resolve_js_exports(&namespace_raw, resolve_module).resolved','    resolve_js_exports(&namespace_raw, resolve_module).resolved.into_iter().filter(|(file, _)| raw[file].star_reexports.is_empty()).collect()')], 'd10_alias_opacity_direct_named_star_forwarded_and_d4_pin'),
('D-M13', [edit(cg,'        self.js_ts_resolved_exports = resolution.resolved;','        self.js_ts_resolved_exports = self.js_ts_namespace_exports.clone();')], 'd10_alias_opacity_direct_named_star_forwarded_and_d4_pin'),
('D-M14-site', [edit(proof,'        let binding = self.js_ts_site_binding(ident, name, cache);','        let ident = declarations[0].0;\n        let binding = self.js_ts_site_binding(ident, name, cache);')], 'd8_serde_cache_and_incremental_epochs'),
('D-M14-cache', [edit('src/cpg_cache.rs','const CACHE_VERSION: u32 = 104;','const CACHE_VERSION: u32 = 103;'),edit('src/navigation/call_edge_cache.rs','const NAV_CALL_EDGE_CACHE_VERSION: u32 = 60;','const NAV_CALL_EDGE_CACHE_VERSION: u32 = 59;')], 'pinned'),
]
report=[]
for name,edits,test in cases:
 originals={p:(root/p).read_bytes() for p,_,_ in edits};patch=[]
 try:
  for p,old,new in edits:
   s=(root/p).read_text();assert s.count(old)==1,(name,p,s.count(old))
   changed=s.replace(old,new);patch.extend(difflib.unified_diff(s.splitlines(True),changed.splitlines(True),fromfile=p,tofile=p));(root/p).write_text(changed)
  (out/(name+'.patch')).write_text(''.join(patch))
  command=['cargo','test','--offline']+(['--lib'] if name=='D-M14-cache' else ['--test','integration'])+[test,'--','--nocapture']
  with (out/(name+'.log')).open('w') as log:
   result=subprocess.run(command,cwd=root,stdout=log,stderr=subprocess.STDOUT,timeout=180)
  log=(out/(name+'.log')).read_text();match=re.search(r'test result: FAILED\. (\d+) passed; (\d+) failed',log)
  killed=bool(match and int(match[2])>0 and 'could not compile' not in log)
  status='KILLED' if killed else ('SURVIVED' if result.returncode==0 and re.search(r'test result: ok\. [1-9]\d* passed',log) else 'INADMISSIBLE')
  failures=re.findall(r'^test (\S+) \.\.\. FAILED',log,re.M)
  report.append(dict(mutant=name,status=status,test=test,failures=failures,command=command))
 except Exception as err:
  report.append(dict(mutant=name,status='INADMISSIBLE',error=str(err),test=test))
 finally:
  for p,bytes_ in originals.items(): (root/p).write_bytes(bytes_)
  (out/'results.json').write_text(json.dumps(report,indent=1))
 print(name,report[-1]['status'],flush=True)
print('Restored every mutated source file byte for byte.',flush=True)
