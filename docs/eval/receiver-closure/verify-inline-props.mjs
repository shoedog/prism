// node verify-inline-props.mjs <evidence-directory>
// Assert the bounded archived-source gain and both served navigation directions.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import assert from "node:assert/strict";
const root = process.argv[2];
const read = (name) => JSON.parse(readFileSync(join(root, name), "utf8"));
const m = read("measurement.json");
assert.equal(m.sites, 2780);
assert.equal(m.exact_sites_before, 369);
assert.equal(m.exact_sites_after, 372);
assert.equal(m.changed_sites.length, 3);
assert.equal(m.relevant_unique_source_spans, 11);
const callers = ["LibraryMenuContent", "_onAddToLibrary", "addToLibrary"].sort();
assert.deepEqual(m.changed_sites.map((r) => r.caller.name).sort(), callers);
for (const r of m.changed_sites) {
  assert.equal(r.source_span.file, "packages/excalidraw/components/LibraryMenu.tsx");
  assert.equal(r.source_span.line, 114);
  assert.equal(r.callee_text, "setLibrary");
  assert.equal(r.resolved_targets.length, 1);
  assert.equal(r.resolved_targets[0].confidence, "exact");
  assert.equal(r.resolved_targets[0].kind, "typed_param");
  assert.deepEqual(r.resolved_targets[0].function_id, {
    file: "packages/excalidraw/data/library.ts", name: "setLibrary",
    start_line: 351, end_line: 400,
  });
}
const beforeCallers = read("callers-base.json"), afterCallers = read("callers-candidate.json");
const beforeCallees = read("callees-base.json"), afterCallees = read("callees-candidate.json");
for (const query of [beforeCallers, afterCallers, beforeCallees, afterCallees]) {
  assert.equal(query.truncated, false);
  assert.deepEqual(query.warnings, []);
}
const signature = (item) => JSON.stringify(item.symbol);
const prior = new Set(beforeCallers.items.map(signature));
assert(beforeCallers.items.every((i) => afterCallers.items.some((j) => signature(i) === signature(j))));
const gained = afterCallers.items.filter((i) => !prior.has(signature(i)));
assert.deepEqual(gained.map((i) => i.symbol.Function.name).sort(), callers);
for (const i of gained) {
  assert.equal(i.fallback, false);
  assert(i.why.some((w) => w.Resolution?.kind === "typed_param"));
  assert(i.why.some((w) => w.CalledBy?.call_site_line === 114));
}
assert.equal(beforeCallees.items.length, 0);
assert.equal(afterCallees.items.length, 1);
const target = afterCallees.items[0];
assert.equal(target.symbol.Function.file, "packages/excalidraw/data/library.ts");
assert.equal(target.symbol.Function.name, "setLibrary");
assert.equal(target.symbol.Function.start_line, 351);
assert.equal(target.fallback, false);
assert(target.why.some((w) => w.Resolution?.kind === "typed_param"));
console.log(JSON.stringify({
  sites: m.sites, exact_before: m.exact_sites_before, exact_after: m.exact_sites_after,
  gained_unique_source_spans: 1, gained_caller_records: 3,
  relevant_unique_source_spans: 11, still_unproven_unique_source_spans: 10,
  served_callers_added: callers, served_callee: target.symbol.Function,
  source_sha256: m.call_bearing_source_sha256,
}, null, 2));
