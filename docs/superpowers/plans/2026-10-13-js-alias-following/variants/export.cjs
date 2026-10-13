// Export the variant matrix inputs for the in-repo table-driven test.
//   node export.cjs <out-dir>
// Writes <out-dir>/fixtures.jsonl (sources + authored truth, one fixture per line). It never writes
// expected.jsonl: pins come only from the Rust test's bless mode, which guards against regressions.
'use strict';
const fs = require('fs');
const path = require('path');
const [outDir] = process.argv.slice(2);
const { all } = require('./variants.cjs');
const { markerLine, declLine } = require('./lib.cjs');
const target = (v, t) => { const [file, name] = t.split(':'); return { file, name, line: v.files[file] ? declLine(v.files[file], name) : null }; };
const fixtures = [];
const ids = new Set();
for (const v of all()) {
  if (ids.has(v.id)) throw new Error(`duplicate id ${v.id}`);
  ids.add(v.id);
  const T = v.truth;
  let truth;
  if (T.t === 'none') truth = { kind: 'none' };
  else if (T.t === 'anon') truth = { kind: 'anon', file: T.file, line: markerLine(v.files[T.file], '// ANON') };
  else truth = { kind: T.t, targets: (T.t === 'one' ? [T.target] : T.targets).map((t) => target(v, t)) };
  const siteLine = markerLine(v.files[v.siteFile], '// SITE');
  if (siteLine === null) throw new Error(`${v.id}: no // SITE line`);
  if (truth.kind === 'anon' && truth.line === null) throw new Error(`${v.id}: no // ANON line`);
  fixtures.push({ id: v.id, step: v.step, shape: v.shape, scope: v.scope, form: v.form, naming: v.naming, lang: v.lang, contract: v.contract || null,
    site_file: v.siteFile, site_line: siteLine, truth, files: v.files });
}
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, 'fixtures.jsonl'), fixtures.map((x) => JSON.stringify(x)).join('\n') + '\n');
console.log(`fixtures ${fixtures.length}`);
