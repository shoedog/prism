// Adjudicate changed Exact targets against the pinned TypeScript checker (typed corpora only).
//   node adjudicate_edges.cjs <typescript.js> <corpus-root> <edge-diff-rows.jsonl>
// For each changed site, asks the checker for the resolved signature's declaration and compares it
// with every head Exact target (and every removed base Exact target). Prints aggregate counts only.
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, rootArg, rowsFile, detailOut] = process.argv.slice(2);
const detail = [];
const ts = require(tsPath);
const root = fs.realpathSync(rootArg);
const rows = fs.readFileSync(rowsFile, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)).filter((r) => r.change !== 'record');
let options = { allowJs: true, checkJs: true, noEmit: true, skipLibCheck: true, types: [], jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler };
for (const name of ['tsconfig.json', 'tsconfig.base.json', 'jsconfig.json']) {
  const p = path.join(root, name);
  if (!fs.existsSync(p)) continue;
  const cfg = ts.readConfigFile(p, ts.sys.readFile);
  if (cfg.error) continue;
  const pc = ts.parseJsonConfigFileContent(cfg.config, ts.sys, root);
  if (pc.options.paths || pc.options.baseUrl) { options = { ...options, baseUrl: pc.options.baseUrl || root, paths: pc.options.paths }; break; }
}
const files = [...new Set(rows.map((r) => path.join(root, r.key[0])))];
const program = ts.createProgram(files, options);
const checker = program.getTypeChecker();
const cache = new Map();
function nodes(abs, sf) {
  if (cache.has(abs)) return cache.get(abs);
  const m = new Map();
  const visit = (n) => {
    let c = null;
    if (ts.isCallExpression(n) || ts.isNewExpression(n)) c = n.expression;
    else if (ts.isJsxSelfClosingElement(n) || ts.isJsxOpeningElement(n)) c = n.tagName;
    else if (ts.isTaggedTemplateExpression(n)) c = n.tag;
    if (c) {
      // chained calls share a start offset (`a.b().c()`): key by the callee's own name as well
      while (c && (ts.isParenthesizedExpression(c) || ts.isNonNullExpression(c) || ts.isAsExpression(c))) c = c.expression;
      const name = c && ts.isIdentifier(c) ? c.text : c && ts.isPropertyAccessExpression(c) ? c.name.text : null;
      const k = `${n.getStart(sf)}:${name}`;
      if (!m.has(k)) m.set(k, n);
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  const rec = { m, buf: fs.readFileSync(abs) };
  cache.set(abs, rec);
  return rec;
}
const isFn = (n) => ts.isFunctionDeclaration(n) || ts.isFunctionExpression(n) || ts.isArrowFunction(n) || ts.isMethodDeclaration(n) || ts.isConstructorDeclaration(n);
const tally = {};
const bump = (k) => { tally[k] = (tally[k] || 0) + 1; };
for (const r of rows) {
  const [file, startByte] = r.key;
  const abs = path.join(root, file);
  const sf = program.getSourceFile(abs);
  let decl = null;
  let state = 'site_not_located';
  if (sf) {
    const rec = nodes(abs, sf);
    const base = rec.buf.subarray(0, startByte).toString('utf8').length;
    const node = rec.m.get(`${base}:${r.key[3]}`) || rec.m.get(`${base + 1}:${r.key[3]}`);
    if (node) {
      let sig = null;
      try { sig = checker.getResolvedSignature(node); } catch (e) { sig = null; }
      decl = sig && sig.declaration;
      state = !decl ? 'checker_no_declaration' : 'located';
    }
  }
  let dfile = null; let dline = null; let hasBody = false; let impls = [];
  const pos = (d) => { const dsf = d.getSourceFile(); return [path.relative(root, dsf.fileName), dsf.getLineAndCharacterOfPosition(d.getStart(dsf)).line + 1]; };
  if (decl) {
    [dfile, dline] = pos(decl);
    hasBody = isFn(decl) && !!decl.body;
    // an overload signature or an abstract/interface member: look for bodies among the symbol's declarations
    const sym = decl.symbol;
    if (!hasBody && sym && sym.declarations) impls = sym.declarations.filter((d) => isFn(d) && !!d.body).map(pos);
  }
  const verdictFor = (t) => {
    if (state !== 'located') return state;
    if (t[2] === dfile && t[4] === dline) return 'agree';
    if (!hasBody && impls.some(([f, l]) => f === t[2] && l === t[4])) return 'agree_overload_implementation';
    if (!hasBody) return impls.length ? 'disagree_with_overload_implementation' : 'checker_names_a_signature_without_body';
    if (dfile.startsWith('..') || dfile.includes('node_modules')) return 'checker_names_external';
    return 'disagree';
  };
  const headExact = r.head_targets.filter((t) => t[0] === 'exact');
  const baseExact = r.base_targets.filter((t) => t[0] === 'exact');
  const hk = new Set(headExact.map((t) => t.slice(2).join('|')));
  const bk = new Set(baseExact.map((t) => t.slice(2).join('|')));
  for (const t of headExact) if (!bk.has(t.slice(2).join('|'))) {
    const v = verdictFor(t);
    bump(`${r.part}|ADDED exact:${t[1]}|${v}`);
    if (detailOut && !v.startsWith('agree')) detail.push({ file, line: r.line, callee: r.key[3], kind: t[1], target: t.slice(2), verdict: v, checker: decl ? [dfile, dline, ts.SyntaxKind[decl.kind]] : null });
  }
  for (const t of baseExact) if (!hk.has(t.slice(2).join('|'))) bump(`${r.part}|REMOVED exact:${t[1]}|${verdictFor(t)}`);
  if (!headExact.length && !baseExact.length) bump(`${r.part}|non-exact change|${r.change}`);
}
for (const k of Object.keys(tally).sort()) console.log(String(tally[k]).padStart(8), k);
if (detailOut) fs.writeFileSync(detailOut, detail.map((x) => JSON.stringify(x)).join('\n') + '\n', { mode: 0o600 });
