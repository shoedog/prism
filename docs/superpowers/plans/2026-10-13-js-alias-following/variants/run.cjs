// Runs the variant matrix: writes each fixture repo, asks the pinned TypeScript checker what the
// marked site calls (cross-check of the authored truth), runs the frozen prism binaries, and
// tabulates a verdict per variant and binary.
//   node run.cjs <typescript.js> <out-dir> name=binary [name=binary ...]
'use strict';
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const [tsPath, outDir, ...binArgs] = process.argv.slice(2);
const ts = require(tsPath);
const { all } = require('./variants.cjs');
const bins = binArgs.map((a) => a.split('='));

function writeRepo(v, dir) {
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(path.join(dir, 'repo'), { recursive: true });
  for (const [f, src] of Object.entries(v.files)) {
    const p = path.join(dir, 'repo', f);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, src);
  }
}
const { markerLine, declLine } = require('./lib.cjs');

function oracle(v, repo, siteLine) {
  const files = Object.keys(v.files).map((f) => path.join(repo, f));
  const program = ts.createProgram(files, {
    allowJs: true, checkJs: true, noEmit: true, skipLibCheck: true, types: [],
    jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler,
  });
  const checker = program.getTypeChecker();
  const sf = program.getSourceFile(path.join(repo, v.siteFile));
  let site = null;
  const visit = (n) => {
    if (site) return;
    const line = sf.getLineAndCharacterOfPosition(n.getStart(sf)).line + 1;
    if (line === siteLine && (ts.isCallExpression(n) || ts.isNewExpression(n) || ts.isJsxSelfClosingElement(n) || ts.isJsxOpeningElement(n))) {
      site = n;
      return;
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  if (!site) return 'no-site';
  const nameOf = (d) => {
    if (!d) return null;
    const file = path.relative(repo, d.getSourceFile().fileName);
    if (file.startsWith('..')) return 'lib';
    if (ts.isConstructorDeclaration(d)) return `${file}:${d.parent.name ? d.parent.name.text : '<class>'}`;
    if ((ts.isFunctionDeclaration(d) || ts.isClassDeclaration(d) || ts.isMethodDeclaration(d)) && d.name) return `${file}:${d.name.getText()}`;
    if (ts.isArrowFunction(d) || ts.isFunctionExpression(d)) {
      const p = d.parent;
      if (ts.isVariableDeclaration(p) && ts.isIdentifier(p.name)) return `${file}:${p.name.text}`;
      return `${file}:<anon@${sf.getLineAndCharacterOfPosition(d.getStart()).line + 1}>`;
    }
    return `(${ts.SyntaxKind[d.kind]})`;
  };
  let sig;
  try { sig = checker.getResolvedSignature(site); } catch (e) { return 'checker-error'; }
  const viaSig = sig && sig.declaration ? nameOf(sig.declaration) : null;
  if (viaSig && !viaSig.startsWith('(')) return viaSig;
  return viaSig || 'untyped';
}

function prismSite(bin, dir, tag, v, siteLine) {
  const cache = path.join(dir, `cache-${tag}`);
  fs.rmSync(cache, { recursive: true, force: true });
  const r = spawnSync(bin, ['nav', '--cache-dir', cache, 'call-stats', '--repo', path.join(dir, 'repo'), '--dump-sites'], { encoding: 'utf8', maxBuffer: 1 << 26 });
  fs.rmSync(cache, { recursive: true, force: true });
  if (r.status !== 0) return { error: `exit ${r.status}: ${(r.stderr || '').slice(0, 200)}` };
  const recs = [];
  for (const line of r.stdout.split('\n')) {
    if (!line.startsWith('{')) continue;
    let o;
    try { o = JSON.parse(line); } catch { continue; }
    if (o.record_kind !== 'call_site' || !o.source_span) continue;
    if (o.source_span.file === v.siteFile && o.source_span.line === siteLine) recs.push(o);
  }
  if (!recs.length) return { record: false, exact: [], nameonly: [], drop: null };
  const exact = [];
  const nameonly = [];
  const drops = new Set();
  const origins = new Set();
  for (const o of recs) {
    origins.add(o.origin);
    if (o.drop) drops.add(o.drop);
    for (const t of o.resolved_targets || []) {
      const e = { file: t.function_id.file, name: t.function_id.name, line: t.function_id.start_line, kind: t.kind };
      (t.confidence === 'exact' ? exact : nameonly).push(e);
    }
  }
  return { record: true, exact, nameonly, drop: [...drops].join('+') || null, origins: [...origins] };
}

function matcher(v) {
  const lineOf = (target) => {
    const [file, name] = target.split(':');
    return { file, name, line: v.files[file] ? declLine(v.files[file], name) : null };
  };
  // Identity: same file, same registered name, and the same declaration line when the truth has one.
  const hit = (t, target) => {
    const d = lineOf(target);
    return t.file === d.file && t.name === d.name && (d.line === null || t.line === d.line);
  };
  return hit;
}

function verdict(v, p) {
  if (p.error) return 'ERROR';
  const hit = matcher(v);
  const T = v.truth;
  const ex = p.exact;
  const no = p.nameonly;
  if (T.t === 'anon') {
    const line = markerLine(v.files[T.file], '// ANON');
    if (ex.length && ex.every((t) => t.file === T.file && t.line === line)) return 'CORRECT';
    if (ex.length) return 'WRONG';
    return no.some((t) => t.file === T.file && t.line === line) ? 'DEMOTED' : 'MISSING';
  }
  if (T.t === 'none') return ex.length ? 'WRONG' : no.length ? 'NONE+NAMEONLY' : 'NONE-OK';
  const targets = T.t === 'one' ? [T.target] : T.targets;
  const inTruth = (t) => targets.some((g) => hit(t, g));
  if (T.t === 'may') {
    if (ex.some((t) => !inTruth(t))) return 'WRONG';
    if (ex.length) return 'OVERCLAIM';
    return no.some(inTruth) ? 'MAY-NAMEONLY' : 'MAY-NONE';
  }
  if (ex.some((t) => !inTruth(t))) return 'WRONG';
  if (ex.length) {
    const covered = targets.filter((g) => ex.some((t) => hit(t, g))).length;
    return covered === targets.length ? 'CORRECT' : 'PARTIAL';
  }
  if (no.some(inTruth)) return 'DEMOTED';
  return 'MISSING';
}

const variants = all();
const ids = new Set();
for (const v of variants) { if (ids.has(v.id)) throw new Error(`duplicate id ${v.id}`); ids.add(v.id); }
fs.mkdirSync(outDir, { recursive: true });
const rows = [];
for (const v of variants) {
  const dir = path.join(outDir, 'work', v.id);
  writeRepo(v, dir);
  const siteLine = markerLine(v.files[v.siteFile], '// SITE');
  const row = { id: v.id, step: v.step, shape: v.shape, scope: v.scope, form: v.form, naming: v.naming, lang: v.lang, truth: v.truth, contract: v.contract, siteFile: v.siteFile, siteLine };
  row.oracle = oracle(v, path.join(dir, 'repo'), siteLine);
  row.results = {};
  for (const [tag, bin] of bins) {
    const p = prismSite(bin, dir, tag, v, siteLine);
    row.results[tag] = { ...p, verdict: p.record === false ? 'NO-RECORD' : verdict(v, p) };
  }
  rows.push(row);
}
fs.writeFileSync(path.join(outDir, 'matrix.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n');
// Cross-check of the authored truth: where the checker can name the callee it must agree, except for
// the shapes listed here, where it answers a different question than "which callable runs".
const ORACLE_NOTES = {
  'destructured-props-default-overridden': 'JavaScript: the checker types a defaulted destructured parameter by its default; the caller passes another function',
  'named-js-suffix-dts-only': 'the checker names the .d.ts signature, which has no body',
  'hook-result-two-slots': 'JavaScript: the returned array literal is typed as an array of a union; the checker names its first member',
  'named-js-suffix-literal-and-ts': 'policy row: TypeScript binds x.ts; prism keeps the literal x.js as main does',
};
const oracleOk = (r) => {
  const o = r.oracle;
  const T = r.truth;
  if (T.t === 'one') return o === T.target;
  if (T.t === 'set') return T.targets.includes(o);
  if (T.t === 'may') return T.targets.length === 0 || T.targets.includes(o);
  if (T.t === 'anon') return o.startsWith(`${T.file}:`);
  return false;
};
const typed = rows.filter((r) => r.oracle.includes(':'));
const unexplained = typed.filter((r) => !oracleOk(r) && !(r.shape in ORACLE_NOTES));
console.log(`checker: typed ${typed.length} of ${rows.length}, differing ${typed.filter((r) => !oracleOk(r)).length}, unexplained ${unexplained.length}`);
for (const r of unexplained) console.log(`  UNEXPLAINED ${r.id}: checker ${r.oracle}, truth ${JSON.stringify(r.truth)}`);
if (unexplained.length) process.exitCode = 1;
const tally = {};
for (const r of rows) for (const [tag] of bins) {
  const k = `${tag}\t${r.step}\t${r.results[tag].verdict}`;
  tally[k] = (tally[k] || 0) + 1;
}
console.log(`variants ${rows.length}`);
for (const k of Object.keys(tally).sort()) console.log(`${k}\t${tally[k]}`);
