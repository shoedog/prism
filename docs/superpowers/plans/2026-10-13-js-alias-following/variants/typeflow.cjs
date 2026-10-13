// Type-flow ceiling for callees bound to a local or a parameter.
// For every recorded census site in those groups, ask the pinned checker for the resolved
// signature's declaration and classify it: an implementation in the repo (a callable with a
// body), a type-only signature, something outside the repo, or nothing.
// This bounds what value-flow following (returns, object literals, wrappers) could ever resolve.
//   node typeflow.cjs <typescript.js> <corpus-root> <sites.jsonl> <functions.json> <out-dir>
// Prints aggregate counts only. Per-site rows go to <out-dir>/typeflow-rows.jsonl (mode 600).
'use strict';
const fs = require('fs');
const path = require('path');
const [tsPath, rootArg, sitesFile, functionsFile, outDir] = process.argv.slice(2);
const ts = require(tsPath);
const root = fs.realpathSync(rootArg);

const rows = fs.readFileSync(sitesFile, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l))
  .filter((r) => r.joined && (r.group === 'local' || r.group === 'parameter'));
const inventory = new Map();
for (const f of JSON.parse(fs.readFileSync(functionsFile, 'utf8'))) {
  const k = `${f.file}:${f.start_line}`;
  if (!inventory.has(k) || f.name) inventory.set(k, f.name ? 'named' : 'anonymous');
}

let options = {
  allowJs: true, checkJs: true, noEmit: true, skipLibCheck: true, types: [],
  jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
};
let pathsFrom = 'none';
for (const name of ['tsconfig.json', 'tsconfig.base.json', 'jsconfig.json']) {
  const p = path.join(root, name);
  if (!fs.existsSync(p)) continue;
  const cfg = ts.readConfigFile(p, ts.sys.readFile);
  if (cfg.error) continue;
  const parsed = ts.parseJsonConfigFileContent(cfg.config, ts.sys, root);
  if (parsed.options.paths || parsed.options.baseUrl) {
    options = { ...options, baseUrl: parsed.options.baseUrl || root, paths: parsed.options.paths };
    pathsFrom = name;
    break;
  }
}
const files = [...new Set(rows.map((r) => path.join(root, r.file)))];
const program = ts.createProgram(files, options);
const checker = program.getTypeChecker();

const perFile = new Map();
function candidates(sf) {
  const out = new Map();
  const visit = (n) => {
    let callee = null;
    if (ts.isCallExpression(n)) callee = n.expression;
    else if (ts.isJsxSelfClosingElement(n) || ts.isJsxOpeningElement(n)) callee = n.tagName;
    if (callee && ts.isIdentifier(callee)) {
      const k = `${callee.getStart(sf)}:${callee.text}`;
      if (!out.has(k)) out.set(k, n);
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return out;
}
function charOffset(buf, byte) {
  const bom = buf.length >= 3 && buf[0] === 0xef && buf[1] === 0xbb && buf[2] === 0xbf ? 3 : 0;
  return buf.subarray(bom, Math.max(bom, byte)).toString('utf8').length;
}
function classify(decl) {
  if (!decl) return 'no_declaration';
  const file = decl.getSourceFile().fileName;
  const rel = path.relative(root, file);
  if (rel.startsWith('..') || rel.includes('node_modules') || program.isSourceFileDefaultLibrary(decl.getSourceFile())) return 'outside_repo';
  const hasBody = (ts.isFunctionDeclaration(decl) || ts.isMethodDeclaration(decl) || ts.isConstructorDeclaration(decl) || ts.isFunctionExpression(decl) || ts.isArrowFunction(decl) || ts.isGetAccessorDeclaration(decl)) && !!decl.body;
  if (!hasBody) return 'type_only_signature';
  const sf = decl.getSourceFile();
  const line = sf.getLineAndCharacterOfPosition(decl.getStart(sf)).line + 1;
  const reg = inventory.get(`${rel}:${line}`);
  if (reg === 'named') return 'repo_impl_named';
  if (reg === 'anonymous') return 'repo_impl_anonymous';
  return 'repo_impl_not_in_inventory';
}

const tally = {};
const out = [];
let unlocated = 0;
for (const r of rows) {
  const abs = path.join(root, r.file);
  const sf = program.getSourceFile(abs);
  let cls = 'unlocated';
  if (sf) {
    if (!perFile.has(abs)) perFile.set(abs, { buf: fs.readFileSync(abs), nodes: candidates(sf) });
    const pf = perFile.get(abs);
    let node = null;
    for (const [start] of r.join_spans || [r.span]) {
      const base = charOffset(pf.buf, start);
      // a JSX span starts at "<": the tag name follows it
      node = pf.nodes.get(`${base}:${r.name}`) || pf.nodes.get(`${base + 1}:${r.name}`);
      if (node) break;
    }
    if (node) {
      let sig = null;
      try { sig = checker.getResolvedSignature(node); } catch (e) { sig = null; }
      cls = classify(sig && sig.declaration);
    }
  }
  if (cls === 'unlocated') unlocated += 1;
  const part = r.test ? 'test' : 'product';
  const key = `${r.binding}|${part}|${r.verdict}|${cls}`;
  tally[key] = (tally[key] || 0) + 1;
  out.push({ file: r.file, line: r.line, name: r.name, binding: r.binding, verdict: r.verdict, typeflow: cls });
}
fs.mkdirSync(outDir, { recursive: true, mode: 0o700 });
fs.writeFileSync(path.join(outDir, 'typeflow-rows.jsonl'), out.map((x) => JSON.stringify(x)).join('\n') + '\n', { mode: 0o600 });
const summary = { sites: rows.length, unlocated, paths_from: pathsFrom, program_files: program.getSourceFiles().length, tally };
fs.writeFileSync(path.join(outDir, 'typeflow-aggregate.json'), JSON.stringify(summary, null, 2) + '\n', { mode: 0o600 });
console.log(JSON.stringify(summary));
