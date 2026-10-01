"""Apply each bounded S1b-4 mutant alone; restore exact bytes in finally.
Usage: mutate_s1b4.py PROTO_ROOT OUTPUT_DIR [VARIANT ...]
One attempt per mutant; compile/setup/zero-test failures are inadmissible.
Never run concurrently with source editing, review, or another cargo process.
"""
from pathlib import Path
import difflib,json,re,subprocess,sys
root=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True)

cases = [('D-M1',
  [('src/resolution_js_namespace.rs',
    '        let module_file =',
    '        return None;\n        let module_file =')],
  'd1_direct_and_directory_decoys'),
 ('D-M2',
  [('src/ast/js_binding_namespace.rs',
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);',
    '        return Some((JsBinding::Import, declarations.first().and_then(|(_, s)| s.clone())));\n'
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);')],
  'd4_scope_write_recovery_and_positions'),
 ('D-M3',
  [('src/resolution_js_namespace.rs',
    'confidence: ResolutionConfidence::NameOnly,',
    'confidence: ResolutionConfidence::Exact,')],
  'r1_rule7_'),
 ('D-M4',
  [('src/call_graph.rs',
    '            let Some((binding, module)) = parsed.js_ts_namespace_binding(q, cache) else {',
    '            if let Some(module_path) = '
    'parsed.extract_imports().get(parsed.node_text(&q)).cloned() { return '
    'JsLocalBinding::NamespaceImport { module_path }; }\n'
    '            let Some((binding, module)) = parsed.js_ts_namespace_binding(q, cache) else {')],
  'd6_non_namespace_imports_keep_base'),
 ('D-M5-import',
  [('src/ast/js_binding.rs',
    'if local_route && self.js_ts_scoped_written(scope, name, cache) {',
    'if false && local_route && self.js_ts_scoped_written(scope, name, cache) {')],
  'd9_written_import_and_export_keep_base'),
 ('D-M5-kind',
  [('src/ast/js_binding_namespace.rs',
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);',
    '        let mut binding = match self.js_ts_site_binding(ident, name, cache) { '
    'JsBinding::MayCall => JsBinding::Refused("not_callable"), b => b };')],
  'd9_written_import_and_export_keep_base'),
 ('D-M6-span',
  [('src/resolution_js_namespace.rs',
    '&& export.span == Some((target.start_line, target.end_line))',
    '&& true')],
  'd1_direct_and_directory_decoys'),
 ('D-M6-wrapped',
  [('src/resolution_js_namespace.rs',
    'if export.wrapped && !site.jsx_element {',
    'if false && export.wrapped && !site.jsx_element {')],
  'r1_wrapped_nonjsx_'),
 ('D-M8',
  [('src/ast/js_binding_namespace.rs',
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);',
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);\n'
    '        if cache.write_targets.as_ref().is_some_and(|w| w.contains_key(name)) { '
    'binding=JsBinding::MayCall; }')],
  'd4_scope_write_recovery_and_positions'),
 ('D-M9',
  [('src/ast/js_binding.rs',
    '        if !self.js_ts_recovery_sealed(scope, site, cache) {',
    '        if false && !self.js_ts_recovery_sealed(scope, site, cache) {')],
  'd4_scope_write_recovery_and_positions'),
 ('D-M14-site',
  [('src/ast/js_binding_namespace.rs',
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);',
    '        let ident = declarations[0].0;\n'
    '        let mut binding = self.js_ts_site_binding(ident, name, cache);')],
  'd8_serde_cache_and_incremental_epochs'),
 ('D-M14-cache',
  [('src/cpg_cache.rs', 'const CACHE_VERSION: u32 = 104;', 'const CACHE_VERSION: u32 = 103;'),
   ('src/navigation/call_edge_cache.rs',
    'const NAV_CALL_EDGE_CACHE_VERSION: u32 = 60;',
    'const NAV_CALL_EDGE_CACHE_VERSION: u32 = 59;')],
  'pinned'),
 ('R3-jsx-sibling',
  [('src/resolution_js_namespace.rs', '(p, &[".tsx"])', '(p, &[])')],
  'd14_jsx_specifier_tsx_sibling'),
 ('R1-namespace-refused-off',
  [('src/resolution.rs',
    '                let namespace_refused = match &site.local_binding {',
    '                let namespace_refused = false && match &site.local_binding {')],
  'd13_b0_and_nonproving_refusals_keep_base'),
 ('R2-recovered-type-admitted',
  [('src/ast/js_binding_namespace.rs',
    'if self.js_ts_import_statement_is_type_only(stmt) || recovered_type {',
    'if self.js_ts_import_statement_is_type_only(stmt) || (false && recovered_type) {')],
  'd6_non_namespace_imports_keep_base'),
 ('W3-all-unproven-refused',
  [('src/resolution.rs',
    'matches!(\n'
    '                            r.as_str(),\n'
    '                            "not_callable" | "unindexed" | "namespace_shadow"\n'
    '                        )',
    'true')],
  'd13_b0_and_nonproving_refusals_keep_base'),
 ('P2-incomplete-module',
  [('src/js_exports.rs', '.filter(|f| f.namespace_proof_complete)', '.filter(|_| true)')],
  'r1_rule2_'),
 ('P2-ignore-unknown-star',
  [('src/js_exports.rs',
    'namespace_identity(raw, resolve_module, &next, name, depth + 1, visiting)?',
    'namespace_identity(raw, resolve_module, &next, name, depth + 1, visiting).unwrap_or(None)')],
  'r1_rule2_'),
 ('P2-depth-unbounded',
  [('src/js_exports.rs',
    '    if depth > MAX_REEXPORT_DEPTH {\n        return Err(());',
    '    if depth > 99 {\n        return Err(());')],
  'r1_depth_budget_'),
 ('P2-cycle-is-absence',
  [('src/js_exports.rs',
    '    if !visiting.insert(key.clone()) {\n        return Err(());',
    '    if !visiting.insert(key.clone()) {\n        return Ok(None);')],
  'r1_rule2_'),
 ('P2-competing-star',
  [('src/js_exports.rs',
    'if unique.as_ref().is_some_and(|prior| prior != &target) {',
    'if false && unique.as_ref().is_some_and(|prior| prior != &target) {')],
  'r1_rule2_'),
 ('P4-add-nonbase-target',
  [('src/resolution_js_namespace.rs',
    'let ids = base\n                .iter()\n                .copied()',
    'let ids = self.functions.get(member)?.iter()')],
  'r1_rule4_'),
 ('P4-zero-match-drops',
  [('src/resolution_js_namespace.rs', 'if ids.len() != 1 {', 'if ids.len() > 1 {')],
  'r1_rule4_'),
 ('P6-noncallable-terminal',
  [('src/ast.rs',
    '            if let JsBinding::Callable(t) = self.js_ts_module_binding(local, root, &mut d4.1) '
    '{',
    '            if !matches!(self.js_ts_module_binding(local, root, &mut d4.1), '
    'JsBinding::Callable(_)) {\n'
    '                let mut cursor = root.walk();\n'
    '                for node in root.named_children(&mut cursor) {\n'
    '                    let decl = node.child_by_field_name("declaration").unwrap_or(node);\n'
    '                    if matches!(decl.kind(), "function_declaration" | '
    '"generator_function_declaration")\n'
    '                        && decl.child_by_field_name("name").is_some_and(|n| '
    'self.node_text(&n) == local) {\n'
    '                        let (start, end) = self.node_line_range(&decl);\n'
    '                        '
    'facts.namespace_callable_locals.entry(local.clone()).or_default().push(crate::js_exports::ResolvedJsExport '
    '{\n'
    '                            file: self.path.clone(), local_name: local.clone(), span: '
    'Some((start, end)), wrapped: false,\n'
    '                        });\n'
    '                    }\n'
    '                }\n'
    '            }\n'
    '            if let JsBinding::Callable(t) = self.js_ts_module_binding(local, root, &mut d4.1) '
    '{')],
  'r1_rule6_'),
 ('P7-filter-candidates',
  [('src/resolution_js_namespace.rs',
    '            base.iter()\n                .map(|target| ResolvedCallee {',
    '            base.iter().take(1)\n                .map(|target| ResolvedCallee {')],
  'r1_rule7_'),
 ('REV-local-arm-lookup',
  [('src/js_exports.rs',
    '                JsExportTarget::Local(_)\n'
    '                | JsExportTarget::UnprovenLocal(_)\n'
    '                | JsExportTarget::Class(_) => Err(()),',
    '                JsExportTarget::Local(local)\n'
    '                | JsExportTarget::UnprovenLocal(local)\n'
    '                | JsExportTarget::Class(local) => {\n'
    '                    let candidates = facts.namespace_callable_locals.get(local).ok_or(())?;\n'
    '                    match candidates.as_slice() {\n'
    '                        [only] => Ok(Some(only.clone())),\n'
    '                        _ => Err(()),\n'
    '                    }\n'
    '                }')],
  'r2_')]
report=[]
for name,edits,test in cases:
 if len(sys.argv)>3 and name not in sys.argv[3:]: continue
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
