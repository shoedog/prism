// Adjudicate changed Exact targets against the pinned TypeScript checker (typed corpora only).
//   node adjudicate_edges.cjs <typescript.js> <corpus-root> <edge-diff-rows.jsonl>
// For each changed site, asks the checker for the resolved signature's declaration and compares it
// with every head Exact target (and every removed base Exact target). Prints aggregate counts only.
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, rootArg, rowsFile] = process.argv.slice(2);
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
    if (c) { const k = String(n.getStart(sf)); if (!m.has(k)) m.set(k, n); }
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
    const node = rec.m.get(String(base));
    if (node) {
      let sig = null;
      try { sig = checker.getResolvedSignature(node); } catch (e) { sig = null; }
      decl = sig && sig.declaration;
      state = !decl ? 'checker_no_declaration' : 'located';
    }
  }
  let dfile = null; let dline = null; let hasBody = false;
  if (decl) {
    const dsf = decl.getSourceFile();
    dfile = path.relative(root, dsf.fileName);
    dline = dsf.getLineAndCharacterOfPosition(decl.getStart(dsf)).line + 1;
    hasBody = isFn(decl) && !!decl.body;
    // a const-bound arrow: prism's span starts at the declaration line as well
  }
  const verdictFor = (t) => {
    if (state !== 'located') return state;
    if (t[2] === dfile && t[4] === dline) return 'agree';
    if (!hasBody) return 'checker_names_a_signature_without_body';
    if (dfile.startsWith('..') || dfile.includes('node_modules')) return 'checker_names_external';
    return 'disagree';
  };
  const headExact = r.head_targets.filter((t) => t[0] === 'exact');
  const baseExact = r.base_targets.filter((t) => t[0] === 'exact');
  const hk = new Set(headExact.map((t) => t.slice(2).join('|')));
  const bk = new Set(baseExact.map((t) => t.slice(2).join('|')));
  for (const t of headExact) if (!bk.has(t.slice(2).join('|'))) bump(`${r.part}|ADDED exact:${t[1]}|${verdictFor(t)}`);
  for (const t of baseExact) if (!hk.has(t.slice(2).join('|'))) bump(`${r.part}|REMOVED exact:${t[1]}|${verdictFor(t)}`);
  if (!headExact.length && !baseExact.length) bump(`${r.part}|non-exact change|${r.change}`);
}
for (const k of Object.keys(tally).sort()) console.log(String(tally[k]).padStart(8), k);
