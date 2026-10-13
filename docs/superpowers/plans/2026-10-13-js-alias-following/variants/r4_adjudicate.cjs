// For locally bound callees whose prism edge today is a same-file `local_def` to a function declared
// outside the binding's own declaration: is the target plausibly the value anyway, because the same
// file passes it under the same name (`<C name={name} />`, `{ name }`, `name: name`)?
//   node r4_adjudicate.cjs <typescript.js> <corpus-root> <sites.jsonl> <callresult-rows.jsonl>
// Prints aggregate counts only.
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, rootArg, sitesFile, crFile] = process.argv.slice(2);
const ts = require(tsPath);
const root = fs.realpathSync(rootArg);
const rd = (f) => fs.readFileSync(f, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
const idx = new Map();
for (const r of rd(sitesFile)) if (r.group === 'local' && r.joined) idx.set(`${r.file}|${r.line}|${r.name}|${(r.join_spans && r.join_spans[0] ? r.join_spans[0] : r.span).join(',')}`, r);
const parsed = new Map();
function load(file) {
  if (!parsed.has(file)) {
    const text = fs.readFileSync(path.join(root, file), 'utf8');
    const kind = /\.tsx$/.test(file) ? ts.ScriptKind.TSX : /\.ts$/.test(file) ? ts.ScriptKind.TS : ts.ScriptKind.JSX;
    parsed.set(file, ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, kind));
  }
  return parsed.get(file);
}
const tally = {};
for (const c of rd(crFile)) {
  if (c.test || (c.cls.startsWith('fn') && c.hops === 0)) continue;
  const r = idx.get(`${c.file}|${c.line}|${c.name}|${c.span.join(',')}`);
  if (!r) continue;
  const t = (r.prism_targets || []).find((x) => x.kind === 'local_def' && x.confidence === 'exact');
  if (!t) continue;
  const d = r.declarations[0];
  const sf = load(c.file);
  const lineOf = (n) => sf.getLineAndCharacterOfPosition(n.getStart(sf)).line + 1;
  const endLineOf = (n) => sf.getLineAndCharacterOfPosition(n.getEnd()).line + 1;
  // the binding's whole declaration statement
  let declStmt = null;
  const findDecl = (n) => {
    if (declStmt) return;
    if ((ts.isBindingElement(n) || ts.isVariableDeclaration(n)) && ts.isIdentifier(n.name) && n.name.text === c.name && lineOf(n) === d.line) {
      let p = n;
      while (p && !ts.isVariableDeclaration(p)) p = p.parent;
      declStmt = p || n;
      return;
    }
    ts.forEachChild(n, findDecl);
  };
  findDecl(sf);
  let rel = 'declaration_not_located';
  if (declStmt) {
    const inside = t.target.start_line >= lineOf(declStmt) && t.target.end_line <= endLineOf(declStmt);
    rel = inside ? 'inside_declaration' : 'outside_declaration';
  }
  let passed = 'n/a';
  if (rel === 'outside_declaration') {
    // is the target passed under the same name somewhere in this file?
    let hit = false;
    const visit = (n) => {
      if (hit) return;
      if (ts.isJsxAttribute(n) && n.name.getText(sf) === c.name && n.initializer && ts.isJsxExpression(n.initializer) && n.initializer.expression && ts.isIdentifier(n.initializer.expression) && n.initializer.expression.text === t.target.name) hit = true;
      else if (ts.isShorthandPropertyAssignment(n) && n.name.text === c.name && c.name === t.target.name) hit = true;
      else if (ts.isPropertyAssignment(n) && n.name.getText(sf) === c.name && ts.isIdentifier(n.initializer) && n.initializer.text === t.target.name) hit = true;
      ts.forEachChild(n, visit);
    };
    visit(sf);
    passed = hit ? 'passed_under_same_name_in_file' : 'not_passed_in_file';
  }
  const k = `${rel}|${passed}|${c.cls}`;
  tally[k] = (tally[k] || 0) + 1;
}
console.log(JSON.stringify(tally, null, 1));
