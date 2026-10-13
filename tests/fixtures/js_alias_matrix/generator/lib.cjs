// Line helpers shared by run.cjs and export.cjs.
'use strict';
function markerLine(src, marker) {
  const i = src.split('\n').findIndex((l) => l.includes(marker));
  return i < 0 ? null : i + 1;
}
// The declaration line of `name` in `src`: a line carrying `// TARGET:<name>` when the fixture marks
// one, otherwise the only line declaring it. Two unmarked declarations of one name are an authoring
// error (the truth would not say which callable is meant).
function declLine(src, name) {
  const lines = src.split('\n');
  const marked = lines.findIndex((l) => l.includes(`// TARGET:${name}`));
  if (marked >= 0) return marked + 1;
  const re = new RegExp(`\\b(function|class|const)\\s+${name}\\b`);
  const hits = lines.map((l, i) => (re.test(l) ? i + 1 : 0)).filter(Boolean);
  if (hits.length > 1) throw new Error(`ambiguous declaration of ${name} on lines ${hits.join(', ')}: add a // TARGET:${name} marker`);
  return hits.length ? hits[0] : null;
}
module.exports = { markerLine, declLine };
