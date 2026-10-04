// READ: S2-0 census using the caller's native TypeScript ProjectService checker.
// No Prism module/export resolution is used by the oracle. No source writes.
// Usage: node census.cjs TS_JS ROOT BASE_SITES FACTS OUT_DIRECTORY
'use strict';
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto'), zlib = require('node:zlib');
const [tsPath, rootArg, sitesPath, factsPath, out] = process.argv.slice(2);
const ts = require(tsPath), root = fs.realpathSync(rootArg);
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
if (ts.version !== '5.9.3' || sha(tsPath) !== '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675') throw Error('oracle drift');
const readLines = p => (p.endsWith('.gz') ? zlib.gunzipSync(fs.readFileSync(p)).toString('utf8') : fs.readFileSync(p, 'utf8')).trim().split('\n').filter(Boolean).map(JSON.parse);
const sites = readLines(sitesPath).filter(r => r.record_kind === 'call_site');
if (!sites.length) throw Error('zero-site probe is inadmissible');
const facts = readLines(factsPath);
const rel = p => path.relative(root, p).split(path.sep).join('/');
const inside = p => rel(p) !== '..' && !rel(p).startsWith('../') && !path.isAbsolute(rel(p));
const sourceHashes = {};
for (const f of facts) {
  const actual = sha(path.join(root, f.file));
  const expected = Array.isArray(f.hash) ? Buffer.from(f.hash).toString('hex') : f.hash;
  if (actual !== expected) throw Error('source/facts drift');
  sourceHashes[f.file] = actual;
}
const inputs = new Map();
const host = {...ts.sys, getCurrentDirectory: () => root,
  writeFile() { throw Error('oracle source write refused'); },
  watchFile: () => ({close(){}}), watchDirectory: () => ({close(){}}),
  setTimeout: () => 0, clearTimeout(){}, readFile(p) {
    const text = ts.sys.readFile(p);
    if (text !== undefined) inputs.set(p, crypto.createHash('sha256').update(text).digest('hex'));
    return text;
  }};
const logger = {hasLevel:()=>false, loggingEnabled:()=>false, info(){}, msg(){}, perftrc(){}, startGroup(){}, endGroup(){}, getLogFileName:()=>undefined};
const service = new ts.server.ProjectService({host, logger, cancellationToken:{isCancellationRequested:()=>false},
  useSingleInferredProject:false, useInferredProjectPerProjectRoot:true, typingsInstaller:ts.server.nullTypingsInstaller});
const key = r => [r.caller.file, r.caller.name, r.caller.start_line, r.source_span.file,
  r.source_span.start_byte, r.source_span.end_byte, r.callee_text];
const metaKey = s => JSON.stringify([s.caller.file, s.caller.name, s.caller.start_line,
  s.caller.file, s.start_byte, s.end_byte, s.callee_text]);
const metadata = new Map(facts.flatMap(f => f.sites.map(s => [metaKey(s), s])));
const unwrap = n => {
  while (n && (ts.isParenthesizedExpression(n) || ts.isAsExpression(n) || ts.isSatisfiesExpression(n) || ts.isNonNullExpression(n) || ts.isTypeAssertionExpression(n))) n = n.expression;
  return n;
};
const unalias = (checker, s) => {
  const seen = new Set();
  while (s && (s.flags & ts.SymbolFlags.Alias) && !seen.has(s)) { seen.add(s); s = checker.getAliasedSymbol(s); }
  return s;
};
function requireSpec(checker, n) {
  n = unwrap(n);
  if (!n || !ts.isCallExpression(n) || !ts.isIdentifier(n.expression) || n.expression.text !== 'require' || n.arguments.length !== 1 || !ts.isStringLiteral(n.arguments[0])) return null;
  // READ: ambient shadowing outside the repo is owner-accepted; a local shadow isn't.
  const ds = checker.getSymbolAtLocation(n.expression)?.declarations || [];
  if (ds.some(d => inside(d.getSourceFile().fileName) && !d.getSourceFile().isDeclarationFile)) return null;
  return n.arguments[0].text;
}
function binding(checker, ident) {
  const ds = checker.getSymbolAtLocation(ident)?.declarations || [];
  if (ds.length !== 1) return null;
  const d = ds[0];
  if (ts.isImportClause(d) && d.name === ident || ts.isImportClause(d) && d.name?.text === ident.text) {
    return {kind:'default', specifier:d.parent.moduleSpecifier.text, imported:'default', type_only:d.isTypeOnly};
  }
  if (ts.isImportSpecifier(d)) {
    const imp = d.parent.parent.parent;
    return {kind:'named', specifier:imp.moduleSpecifier.text, imported:(d.propertyName || d.name).text,
      type_only:d.isTypeOnly || imp.importClause.isTypeOnly};
  }
  if (ts.isImportEqualsDeclaration(d) && ts.isExternalModuleReference(d.moduleReference)) {
    return {kind:'require_object', specifier:d.moduleReference.expression.text, imported:'export=', type_only:d.isTypeOnly};
  }
  if (ts.isVariableDeclaration(d)) {
    const spec = requireSpec(checker, d.initializer);
    if (spec !== null) return {kind:'require_object', specifier:spec, imported:'module', type_only:false};
  }
  if (ts.isBindingElement(d)) {
    let pattern = d.parent;
    if (ts.isObjectBindingPattern(pattern) && ts.isVariableDeclaration(pattern.parent)) {
      const spec = requireSpec(checker, pattern.parent.initializer);
      if (spec !== null) return {kind:'require_destructure', specifier:spec, imported:(d.propertyName || d.name).text,
        type_only:false, defaulted:!!d.initializer, rest:!!d.dotDotDotToken};
    }
  }
  return null;
}
function terminal(checker, symbol, seen = new Set()) {
  symbol = unalias(checker, symbol);
  if (!symbol || seen.has(symbol)) return {class:'unavailable_or_cycle'};
  seen = new Set(seen); seen.add(symbol);
  const ds = (symbol.declarations || []).filter(d => !ts.isFunctionDeclaration(d) || d.body);
  if (ds.length !== 1) return {class:'non_unique_or_unavailable', count:ds.length,
    declarations:ds.map(declaration)};
  let d = ds[0], n = d, cls = 'noncallable_or_type';
  if (ts.isShorthandPropertyAssignment(d)) return terminal(checker, checker.getShorthandAssignmentValueSymbol(d), seen);
  // READ: TS 53608-53612 follows assignment-backed access declarations.
  const assignment = (ts.isPropertyAccessExpression(d) || ts.isElementAccessExpression(d)) &&
    ts.isBinaryExpression(d.parent) && d.parent.left === d && d.parent.operatorToken.kind === ts.SyntaxKind.EqualsToken ? d.parent : d;
  const init = unwrap(ts.isVariableDeclaration(d) || ts.isPropertyAssignment(d) || ts.isPropertyDeclaration(d) ? d.initializer : ts.isExportAssignment(d) ? d.expression :
    ts.isBinaryExpression(assignment) && assignment.operatorToken.kind === ts.SyntaxKind.EqualsToken ? assignment.right : null);
  if (init && ts.isIdentifier(init)) return terminal(checker, checker.getSymbolAtLocation(init), seen);
  if (ts.isFunctionDeclaration(d) && d.body) cls = 'function';
  else if (ts.isMethodDeclaration(d) && d.body) {
    cls = ts.isObjectLiteralExpression(d.parent) ? 'object_method' :
      ts.getCombinedModifierFlags(d) & ts.ModifierFlags.Static ? 'class_static_method' : 'class_instance_method';
  } else if (init && (ts.isArrowFunction(init) || ts.isFunctionExpression(init))) {
    cls = ts.isPropertyDeclaration(d) ? ts.getCombinedModifierFlags(d) & ts.ModifierFlags.Static ?
      'class_static_field' : 'class_instance_field' : 'function_value'; n = init;
  }
  else if (d.getSourceFile().isDeclarationFile || ts.isMethodSignature(d) || ts.isPropertySignature(d)) cls = 'declaration_only';
  const sf = n.getSourceFile();
  const result = {class:cls, file:rel(sf.fileName), name:d.name?.text || symbol.name,
    start_line:sf.getLineAndCharacterOfPosition(n.getStart(sf)).line + 1,
    end_line:sf.getLineAndCharacterOfPosition(n.end - 1).line + 1,
    syntax:ts.SyntaxKind[d.kind], in_repo:inside(sf.fileName), declaration_file:sf.isDeclarationFile};
  if (ts.isFunctionExpression(n) && n.name) result.name = n.name.text;
  return result;
}
function declaration(d) {
  const sf = d.getSourceFile();
  return {file:rel(sf.fileName), syntax:ts.SyntaxKind[d.kind], name:d.name?.text || null,
    start_line:sf.getLineAndCharacterOfPosition(d.getStart(sf)).line+1,
    end_line:sf.getLineAndCharacterOfPosition(d.end-1).line+1, declaration_file:sf.isDeclarationFile};
}
function shape(checker, symbol, seen = new Set()) {
  symbol = unalias(checker, symbol);
  if (!symbol || seen.has(symbol)) return 'unavailable';
  seen = new Set(seen); seen.add(symbol);
  for (const d of symbol.declarations || []) {
    if (ts.isSourceFile(d)) return 'module_namespace';
    if (ts.isClassDeclaration(d) || ts.isClassExpression(d)) return 'class';
    let n = unwrap(ts.isVariableDeclaration(d) ? d.initializer : ts.isExportAssignment(d) ? d.expression : null);
    if (n && ts.isIdentifier(n)) return shape(checker, checker.getSymbolAtLocation(n), seen);
    if (n && ts.isObjectLiteralExpression(n)) return 'object';
    if (n && ts.isClassExpression(n)) return 'class';
    if (n && ts.isNewExpression(n)) return 'class_instance';
    if (n && ts.isCallExpression(n)) return 'call_result';
    if (n && (ts.isArrowFunction(n) || ts.isFunctionExpression(n)) || ts.isFunctionDeclaration(d)) return 'function_members';
    if (ts.isNamespaceExport(d) || ts.isNamespaceImport(d)) return 'module_namespace';
    if (ts.isModuleDeclaration(d)) return 'declared_namespace';
  }
  return 'other';
}
function mechanism(checker, q, b, t, program, project, caller) {
  if (b.kind === 'require_destructure') return 'require_destructure';
  const resolved = ts.resolveModuleName(b.specifier, caller, project.getCompilerOptions(), host).resolvedModule;
  const sf = resolved && program.getSourceFile(resolved.resolvedFileName);
  if (sf?.statements.some(s => ts.isExportAssignment(s) && s.isExportEquals)) return 'export_equals';
  if (sf?.commonJsModuleIndicator) {
    let moduleExports = false;
    function visit(n) {
      if (ts.isBinaryExpression(n) && n.operatorToken.kind === ts.SyntaxKind.EqualsToken &&
          ts.isPropertyAccessExpression(n.left) && ts.isIdentifier(n.left.expression) &&
          n.left.expression.text === 'module' && n.left.name.text === 'exports') moduleExports = true;
      ts.forEachChild(n, visit);
    }
    visit(sf);
    return moduleExports ? 'commonjs_module_exports' : 'commonjs_other_exports';
  }
  if (b.kind === 'require_object') return 'require_object_other';
  const sh = shape(checker, checker.getSymbolAtLocation(q));
  if (sh === 'module_namespace') return 'reexported_namespace';
  if (sh === 'class_instance') return b.kind + '_class_instance';
  if (sh === 'class' || t.class.startsWith('class_')) return b.kind + '_class';
  if (sh === 'object') return b.kind + '_object';
  if (sh !== 'other' && sh !== 'unavailable') return b.kind + '_' + sh;
  return b.kind + '_other';
}
const candidates = [], exclusions = {}, unjoinable = [];
const increment = (obj, k) => obj[k] = (obj[k] || 0) + 1;
const refuseJoin = (r, reason) => {
  increment(exclusions, 'UNJOINABLE');
  unjoinable.push({key:key(r), bucket:'UNJOINABLE', reason});
};
const grouped = new Map();
for (const r of sites) {
  const m = metadata.get(JSON.stringify(key(r)));
  if (!m) { refuseJoin(r, 'site_fact_join'); continue; }
  if (r.origin !== 'Source' || !m.qualifier) continue;
  const f = r.caller.file;
  if (!grouped.has(f)) grouped.set(f, []);
  grouped.get(f).push([r,m]);
}
for (const [file, rows] of [...grouped.entries()].sort()) {
  const absolute = path.join(root, file);
  service.openClientFile(absolute, undefined, undefined, root);
  const project = service.getDefaultProjectForFile(ts.server.toNormalizedPath(absolute), true);
  const program = project?.getLanguageService().getProgram(), checker = program?.getTypeChecker(), sf = program?.getSourceFile(absolute);
  if (!sf || !checker) {
    for (const [r] of rows) refuseJoin(r, 'caller_program');
    service.closeClientFile(absolute);
    continue;
  }
  // Prism offsets index source bytes; TypeScript's host removes a UTF-8 BOM.
  const raw = fs.readFileSync(absolute, 'utf8');
  const bom = raw.startsWith('\uFEFF') && raw.slice(1) === sf.text ? 1 : 0;
  for (const [r,m] of rows) {
    if (raw.slice(bom) !== sf.text || r.source_span.start_byte > Buffer.byteLength(raw)) {
      refuseJoin(r, 'source_position'); continue;
    }
    const pos = Buffer.from(raw, 'utf8').subarray(0,r.source_span.start_byte).toString('utf8').length - bom;
    let n = ts.getTokenAtPosition(sf, pos);
    while (n && !ts.isCallExpression(n) && !ts.isNewExpression(n) && !ts.isJsxOpeningElement(n) && !ts.isJsxSelfClosingElement(n)) n = n.parent;
    const expr = n && (n.expression || n.tagName);
    if (!expr || !ts.isPropertyAccessExpression(expr) || !ts.isIdentifier(expr.expression) || expr.expression.text !== m.qualifier || expr.name.text !== r.callee_text) { refuseJoin(r, 'non_direct_or_unmatched_syntax'); continue; }
    const q = expr.expression, b = binding(checker, q);
    if (!b) { increment(exclusions,'not_s2_binding'); continue; }
    const symbol = checker.getSymbolAtLocation(expr.name), t = terminal(checker, symbol);
    const importDecl = checker.getSymbolAtLocation(q)?.declarations?.[0];
    const moduleNode = importDecl && (ts.isImportSpecifier(importDecl) ? importDecl.parent.parent.parent.moduleSpecifier :
      ts.isImportClause(importDecl) ? importDecl.parent.moduleSpecifier : null);
    const moduleDeclarations = moduleNode ? (checker.getSymbolAtLocation(moduleNode)?.declarations || []).filter(ts.isSourceFile) : [];
    const nativeModule = moduleDeclarations.length === 1 ? rel(moduleDeclarations[0].fileName) : null;
    const prismProof = facts.find(f => f.file === file)?.module_proofs?.find(p => p.specifier === b.specifier) || null;
    const targets = r.resolved_targets || [];
    const low = targets.length === 0 || targets.length > 1 || !targets.some(t => t.confidence === 'exact');
    const callable = ['function','function_value','object_method','class_static_method','class_instance_method','class_static_field','class_instance_field'].includes(t.class) && t.in_repo && !t.declaration_file;
    const equal = target => ['file','name','start_line','end_line'].every(k => t[k] === target.function_id[k]);
    const record = {key:key(r), qualifier:m.qualifier, member:r.callee_text, binding:b, call_kind:r.call_kind,
      owner:project.projectKind === ts.server.ProjectKind.Configured ? rel(project.getProjectName()) : null,
      native_module:nativeModule, prism_module_proof:prismProof,
      compiler_mode:project.getCompilerOptions().moduleResolution,
      mechanism:mechanism(checker,q,b,t,program,project,absolute), terminal:t, low,
      qualifier_declarations:(unalias(checker,checker.getSymbolAtLocation(q))?.declarations || []).map(declaration),
      member_declarations:(unalias(checker,symbol)?.declarations || []).map(declaration),
      base_drop:r.drop, base_targets:targets, local_binding:m.local_binding,
      callable, base_contains_terminal:callable && targets.some(equal),
      base_exact_terminal:callable && targets.some(x => equal(x) && x.confidence === 'exact'),
      base_multiple:targets.length > 1, base_r3:targets.some(x => x.kind === 'import_qualified')};
    candidates.push(record);
  }
  service.closeClientFile(absolute);
}
const summary = {claim:'MEASURED', total_sites:sites.length, source_files:facts.length, s2_sites:candidates.length,
  low_sites:candidates.filter(r=>r.low).length, callable_low:candidates.filter(r=>r.low&&r.callable).length,
  positive_filter_ceiling:candidates.filter(r=>r.low&&r.base_multiple&&r.base_r3&&r.base_contains_terminal&&!r.binding.type_only).length,
  unjoinable:unjoinable.length, unjoinable_reasons:{},
  exclusions, mechanisms:{}, binding_kinds:{}, terminal_classes:{}, call_kinds:{}};
for (const r of unjoinable) increment(summary.unjoinable_reasons, r.reason);
for (const r of candidates) {
  increment(summary.binding_kinds,r.binding.kind); increment(summary.terminal_classes,r.terminal.class);
  increment(summary.call_kinds,r.call_kind);
  const m = summary.mechanisms[r.mechanism] ||= {sites:0,low:0,dropped:0,name_only:0,multiple:0,callable_low:0,positive_filter_ceiling:0,static_low:0,instance_low:0};
  m.sites++; if (r.low) m.low++;
  if (!r.base_targets.length) m.dropped++;
  if (r.base_targets.length && !r.base_targets.some(t=>t.confidence==='exact')) m.name_only++;
  if (r.base_multiple) m.multiple++;
  if (r.low && r.callable) m.callable_low++;
  if (r.low && r.base_multiple && r.base_r3 && r.base_contains_terminal && !r.binding.type_only) m.positive_filter_ceiling++;
  if (r.low && r.terminal.class.startsWith('class_static_')) m.static_low++;
  if (r.low && r.terminal.class.startsWith('class_instance_')) m.instance_low++;
}
fs.writeFileSync(path.join(out,'candidates.json'),JSON.stringify(candidates,null,2)+'\n');
fs.writeFileSync(path.join(out,'unjoinable.json'),JSON.stringify(unjoinable,null,2)+'\n');
fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');
fs.writeFileSync(path.join(out,'oracle-inputs.json'),JSON.stringify(Object.fromEntries([...inputs].map(([p,h])=>[rel(p),h])),null,2)+'\n');
fs.writeFileSync(path.join(out,'receipt.json'),JSON.stringify({claim:'MEASURED',oracle_version:ts.version,oracle_sha256:sha(tsPath),
  probe_sha256:sha(__filename),sites_sha256:sha(sitesPath),facts_sha256:sha(factsPath),source_hashes:sourceHashes,
  oracle_reads:inputs.size},null,2)+'\n');
console.log(JSON.stringify(summary));
