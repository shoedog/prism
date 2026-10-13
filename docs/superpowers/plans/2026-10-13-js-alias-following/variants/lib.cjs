// Line helpers shared by run.cjs and export.cjs.
'use strict';
function markerLine(src, marker) {
  const i = src.split('\n').findIndex((l) => l.includes(marker));
  return i < 0 ? null : i + 1;
}
// The declaration line of `name` in `src`: the line carrying `// TARGET:<name>` when the fixture marks
// one, otherwise the only declaration of that name. Two unmarked declarations of one name, or two
// declarations of it on one line (which a line cannot tell apart), are authoring errors: the truth
// would not say which callable is meant.
function declLine(src, name) {
  const lines = src.split('\n');
  const esc = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const decl = new RegExp(`\\b(function|class|const)\\s+${esc}(?![\\w$])`, 'g');
  const count = (l) => (l.match(decl) || []).length;
  const marker = new RegExp(`// TARGET:${esc}(?![\\w$])`);
  const marked = lines.findIndex((l) => marker.test(l));
  if (marked >= 0) {
    if (count(lines[marked]) > 1) throw new Error(`line ${marked + 1} declares ${name} more than once: a line cannot identify one of them`);
    return marked + 1;
  }
  const hits = lines.map((l, i) => (count(l) ? i + 1 : 0)).filter(Boolean);
  if (hits.length > 1) throw new Error(`ambiguous declaration of ${name} on lines ${hits.join(', ')}: add a // TARGET:${name} marker`);
  if (hits.length === 1 && count(lines[hits[0] - 1]) > 1) throw new Error(`line ${hits[0]} declares ${name} more than once: a line cannot identify one of them`);
  return hits.length ? hits[0] : null;
}
module.exports = { markerLine, declLine };
