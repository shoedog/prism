// Export the variant matrix for the in-repo table-driven test.
//   node export.cjs <matrix.jsonl> <out-dir> <binary-tag>
// Writes <out-dir>/fixtures.jsonl (inputs + authored truth, one fixture per line) and
// <out-dir>/expected.jsonl (what the named binary did on each fixture, one row per line).
'use strict';
const fs = require('fs');
const path = require('path');
const [matrixFile, outDir, tag] = process.argv.slice(2);
const { all } = require('./variants.cjs');
const results = new Map(fs.readFileSync(matrixFile, 'utf8').split('\n').filter(Boolean).map((l) => { const r = JSON.parse(l); return [r.id, r]; }));
const lineWith = (src, test) => { const i = src.split('\n').findIndex(test); return i < 0 ? null : i + 1; };
const declLine = (src, name) => lineWith(src, (l) => new RegExp(`\\b(function|class|const)\\s+${name}\\b`).test(l));
const target = (v, t) => { const [file, name] = t.split(':'); return { file, name, line: v.files[file] ? declLine(v.files[file], name) : null }; };
const byKey = (a, b) => (JSON.stringify(a) < JSON.stringify(b) ? -1 : 1);
const fixtures = [];
const expected = [];
for (const v of all()) {
  const T = v.truth;
  let truth;
  if (T.t === 'none') truth = { kind: 'none' };
  else if (T.t === 'anon') truth = { kind: 'anon', file: T.file, line: lineWith(v.files[T.file], (l) => l.includes('// ANON')) };
  else truth = { kind: T.t, targets: (T.t === 'one' ? [T.target] : T.targets).map((t) => target(v, t)) };
  fixtures.push({ id: v.id, step: v.step, shape: v.shape, scope: v.scope, form: v.form, naming: v.naming, lang: v.lang, contract: v.contract || null,
    site_file: v.siteFile, site_line: lineWith(v.files[v.siteFile], (l) => l.includes('// SITE')), truth, files: v.files });
  const r = results.get(v.id);
  if (!r) throw new Error(`no result for ${v.id}`);
  const p = r.results[tag];
  if (p.error) throw new Error(`${v.id}: ${p.error}`);
  const norm = (list) => list.map((t) => ({ file: t.file, name: t.name, line: t.line, kind: t.kind })).sort(byKey);
  expected.push({ id: v.id, verdict: p.verdict, record: p.record !== false, exact: norm(p.exact || []), nameonly: norm(p.nameonly || []), drop: p.drop || null, origins: (p.origins || []).slice().sort() });
}
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, 'fixtures.jsonl'), fixtures.map((x) => JSON.stringify(x)).join('\n') + '\n');
fs.writeFileSync(path.join(outDir, 'expected.jsonl'), expected.map((x) => JSON.stringify(x)).join('\n') + '\n');
console.log(`fixtures ${fixtures.length}, expected ${expected.length}`);
