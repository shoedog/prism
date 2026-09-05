// node verify-contextual-props.mjs <evidence-directory>
// Source-backed fixture gain plus unchanged archived real-source controls.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
const root = process.argv[2];
const read = (name) => JSON.parse(readFileSync(join(root, name), "utf8"));
const m = read("measurement.json");
assert.equal(m.sites, 2780);
assert.equal(m.exact_sites_before, 372);
assert.equal(m.exact_sites_after, 372);
assert.deepEqual(m.changed_sites, []);
assert.equal(m.relevant_unique_source_spans, 11);
for (const [direction, file, name] of [["callers", "app.ts", "visible"], ["callees", "client.ts", "m"]]) {
  const before = read(`fixture-${direction}-base.json`);
  const after = read(`fixture-${direction}-candidate.json`);
  assert.equal(before.items.length, 0);
  assert.equal(after.items.length, 1);
  for (const result of [before, after]) {
    assert.equal(result.truncated, false);
    assert.deepEqual(result.warnings, []);
  }
  const item = after.items[0];
  assert.equal(item.symbol.Function.file, file);
  assert.equal(item.symbol.Function.name, name);
  assert.equal(item.symbol.Function.start_line, 2);
  assert.equal(item.fallback, false);
  assert(item.why.some((w) => w.Resolution?.kind === "typed_param"));
}
const controls = {};
for (const name of ["black", "excalidraw", "javascript"]) {
  const base = readFileSync(join(root, `${name}-control-base.jsonl`));
  assert(base.equals(readFileSync(join(root, `${name}-control-candidate.jsonl`))));
  controls[name] = createHash("sha256").update(base).digest("hex");
}
console.log(JSON.stringify({
  real_sites: m.sites, exact_before: 372, exact_after: 372, changed_records: 0,
  fixture_served_callers: ["visible"], fixture_served_callees: ["client.ts:2:m"],
  fixture_excluded_callers: ["optional", "written"],
  source_sha256: m.call_bearing_source_sha256, control_jsonl_sha256: controls,
}, null, 2));
