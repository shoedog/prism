// Syntactic adjudication of new Exact targets on locally bound callees, for corpora the type checker
// cannot type. For each changed site whose callee is a `const` alias of an import or of a namespace
// member (possibly through a chain of const identifier aliases), compute the module file and export
// name the alias denotes from syntax, and compare with prism's new target.
//   node adjudicate_alias_syntactic.cjs <typescript.js> <corpus-root> <edge-diff-rows.jsonl>
// Prints aggregate counts only.
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, rootArg, rowsFile] = process.argv.slice(2);
const ts = require(tsPath);
const root = fs.realpathSync(rootArg);
const rows = fs.readFileSync(rowsFile, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)).filter((r) => r.change !== 'record');
const parsed = new Map();
function load(file) {
  if (!parsed.has(file)) {
    let rec = null;
    try {
      const text = fs.readFileSync(path.join(root, file), 'utf8');
      const kind = /\.tsx$/.test(file) ? ts.ScriptKind.TSX : /\.ts$/.test(file) ? ts.ScriptKind.TS : ts.ScriptKind.JSX;
      rec = { text, sf: ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, kind) };
    } catch (e) { rec = null; }
    parsed.set(file, rec);
  }
  return parsed.get(file);
}
const isFn = (n) => ts.isFunctionDeclaration(n) || ts.isFunctionExpression(n) || ts.isArrowFunction(n) || ts.isMethodDeclaration(n);
function declIn(list, name) {
  for (const d of list) {
    let hit = null;
    const each = (n) => { if (ts.isIdentifier(n)) { if (!hit && n.text === name) hit = n; } else if (ts.isObjectBindingPattern(n) || ts.isArrayBindingPattern(n)) for (const el of n.elements) if (ts.isBindingElement(el)) each(el.name); };
    each(d.name);
    if (hit) return hit.parent;
  }
  return null;
}
function lookup(name, from) {
  for (let n = from.parent; n; n = n.parent) {
    if (isFn(n)) { const d = declIn(n.parameters, name); if (d) return d; }
    else if (ts.isBlock(n) || ts.isSourceFile(n) || ts.isModuleBlock(n)) {
      for (const s of n.statements) {
        if (ts.isVariableStatement(s)) { const d = declIn(s.declarationList.declarations, name); if (d) return d; }
        else if ((ts.isFunctionDeclaration(s) || ts.isClassDeclaration(s)) && s.name && s.name.text === name) return s;
        else if (ts.isImportDeclaration(s) && s.importClause) {
          const c = s.importClause;
          if (c.name && c.name.text === name) return c;
          if (c.namedBindings) {
            if (ts.isNamespaceImport(c.namedBindings) && c.namedBindings.name.text === name) return c.namedBindings;
            if (ts.isNamedImports(c.namedBindings)) for (const el of c.namedBindings.elements) if (el.name.text === name) return el;
          }
        }
      }
    }
  }
  return null;
}
const unwrap = (e) => { while (e && (ts.isParenthesizedExpression(e) || ts.isAsExpression(e) || ts.isNonNullExpression(e))) e = e.expression; return e; };
const specOf = (d) => { let c = d; while (c && !ts.isImportDeclaration(c)) c = c.parent; return c.moduleSpecifier.text; };
// what an identifier denotes: {kind:'import', spec, name} | {kind:'ns', spec} | {kind:'fn', line} | null
function denote(id, depth) {
  if (depth > 8) return null;
  const d = lookup(id.text, id);
  if (!d) return null;
  if (ts.isImportSpecifier(d)) return { kind: 'import', spec: specOf(d), name: (d.propertyName || d.name).text };
  if (ts.isImportClause(d)) return { kind: 'import', spec: specOf(d), name: 'default' };
  if (ts.isNamespaceImport(d)) return { kind: 'ns', spec: specOf(d) };
  if (ts.isFunctionDeclaration(d)) return { kind: 'fn', node: d };
  if (ts.isVariableDeclaration(d) && ts.isIdentifier(d.name) && d.initializer) {
    const v = unwrap(d.initializer);
    if (isFn(v)) return { kind: 'fn', node: d };
    if (ts.isIdentifier(v)) return denote(v, depth + 1);
    if (ts.isPropertyAccessExpression(v) && ts.isIdentifier(v.expression)) {
      const q = denote(v.expression, depth + 1);
      return q && q.kind === 'ns' ? { kind: 'import', spec: q.spec, name: v.name.text } : null;
    }
    return null;
  }
  if (ts.isBindingElement(d) && ts.isObjectBindingPattern(d.parent) && ts.isVariableDeclaration(d.parent.parent) && d.parent.parent.initializer && !d.initializer && !d.dotDotDotToken) {
    const v = unwrap(d.parent.parent.initializer);
    if (ts.isIdentifier(v)) {
      const q = denote(v, depth + 1);
      const key = d.propertyName || d.name;
      return q && q.kind === 'ns' && ts.isIdentifier(key) ? { kind: 'import', spec: q.spec, name: key.text } : null;
    }
  }
  return null;
}
let options = { allowJs: true, jsx: ts.JsxEmit.Preserve, module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler };
let configUsed = 'none';
for (const name of ['tsconfig.json', 'tsconfig.base.json', 'jsconfig.json']) {
  const p = path.join(root, name);
  if (!fs.existsSync(p)) continue;
  const cfg = ts.readConfigFile(p, ts.sys.readFile);
  if (cfg.error) continue;
  const pc = ts.parseJsonConfigFileContent(cfg.config, ts.sys, root);
  if (pc.options.paths || pc.options.baseUrl) { options = { ...options, baseUrl: pc.options.baseUrl || root, paths: pc.options.paths }; configUsed = name; break; }
}
function moduleFiles(spec, fromFile) {
  if (!spec.startsWith('.')) {
    // a path alias or baseUrl import: ask TypeScript's resolver with the corpus's own config
    const r = ts.resolveModuleName(spec, path.join(root, fromFile), options, ts.sys);
    if (!r.resolvedModule) return null;
    const rel = path.relative(root, r.resolvedModule.resolvedFileName);
    return rel.startsWith('..') || rel.includes('node_modules') ? null : [rel];
  }
  const base = path.posix.normalize(path.posix.join(path.posix.dirname(fromFile), spec));
  const out = [];
  for (const c of [base, ...['.js', '.jsx', '.ts', '.tsx', '.mjs', '.cjs'].map((e) => base + e), ...['index.js', 'index.jsx', 'index.ts', 'index.tsx'].map((i) => `${base}/${i}`)]) {
    try { if (fs.statSync(path.join(root, c)).isFile()) out.push(c); } catch (e) { /* absent */ }
  }
  return out;
}
const tally = {};
const bump = (k) => { tally[k] = (tally[k] || 0) + 1; };
for (const r of rows) {
  const head = r.head_targets.filter((t) => t[0] === 'exact');
  const baseK = new Set(r.base_targets.filter((t) => t[0] === 'exact').map((t) => t.slice(2).join('|')));
  const added = head.filter((t) => !baseK.has(t.slice(2).join('|')));
  if (!added.length) { bump(`${r.part}|no new exact target|${r.change}`); continue; }
  const [file, startByte, , callee] = r.key;
  const rec = load(file);
  let verdict = 'site_not_located';
  if (rec) {
    const pos = Buffer.from(rec.text).subarray(0, startByte).toString('utf8').length;
    let site = null;
    const visit = (n) => {
      if (site) return;
      let c = null;
      if (ts.isCallExpression(n)) c = n.expression; else if (ts.isJsxSelfClosingElement(n) || ts.isJsxOpeningElement(n)) c = n.tagName;
      if (c && ts.isIdentifier(c) && c.text === callee && (n.getStart(rec.sf) === pos || n.getStart(rec.sf) === pos + 1 || c.getStart(rec.sf) === pos)) { site = c; return; }
      ts.forEachChild(n, visit);
    };
    visit(rec.sf);
    if (site) {
      const d = denote(site, 0);
      if (!d) verdict = 'binding_not_an_alias_of_import_or_function';
      else if (d.kind === 'fn') {
        const line = rec.sf.getLineAndCharacterOfPosition(d.node.getStart(rec.sf)).line + 1;
        verdict = added.every((t) => t[2] === file && Math.abs(t[4] - line) <= 0 ) ? 'agree_same_file_function' : 'disagree_same_file_function';
      } else if (d.kind === 'import') {
        const files = moduleFiles(d.spec, file);
        if (files === null) verdict = 'non_relative_module';
        else if (!files.length) verdict = 'module_not_found_by_syntax';
        else if (added.every((t) => files.includes(t[2]) && (t[3] === d.name || d.name === 'default'))) verdict = 'agree_module_file_and_export_name';
        else if (added.every((t) => t[3] === d.name || d.name === 'default')) verdict = 'export_name_agrees_target_in_another_file_(re-export?)';
        else verdict = 'disagree';
      } else verdict = 'namespace_called_directly';
    }
  }
  bump(`${r.part}|ADDED ${added.map((t) => t[1]).sort().join('+')}|${verdict}`);
}
for (const k of Object.keys(tally).sort()) console.log(String(tally[k]).padStart(6), k);
console.log(`      (path config used: ${configUsed})`);
