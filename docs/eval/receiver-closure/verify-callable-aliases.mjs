// node verify-callable-aliases.mjs <evidence-directory>
// Assert synthetic served gains separately from unchanged real-source controls.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
const root = process.argv[2];
const read = (name) => JSON.parse(readFileSync(join(root, name), "utf8"));
const m = read("measurement.json");
assert.equal(m.sites, 2780);
assert.equal(m.exact_sites_before, 376);
assert.equal(m.exact_sites_after, 376);
assert.deepEqual(m.changed_sites, []);
assert.equal(m.relevant_unique_source_spans, 11);
const positive = [["plain", 4], ["named", 6], ["generic", 8], ["composed", 9]];
for (const [query, expected] of [
  ["callers", positive.map(([name, line]) => ["app.ts", name, line])],
  ...positive.map(([name]) => [`callees-${name}`, [["client.ts", "m", 2]]]),
]) {
  const before = read(`fixture-${query}-base.json`);
  const after = read(`fixture-${query}-candidate.json`);
  assert.equal(before.items.length, 0);
  assert.equal(after.items.length, expected.length);
  for (const result of [before, after]) {
    assert.equal(result.truncated, false);
    assert.deepEqual(result.warnings, []);
  }
  const actual = after.items.map((item) => {
    assert.equal(item.fallback, false);
    assert(item.why.some((w) => w.Resolution?.kind === "typed_param"));
    const f = item.symbol.Function;
    return [f.file, f.name, f.start_line];
  });
  assert.deepEqual(actual.sort(), expected.sort());
}
const controls = {}, fixture = {};
const hash = (b) => createHash("sha256").update(b).digest("hex");
for (const name of ["black", "excalidraw", "javascript"]) {
  const base = readFileSync(join(root, `${name}-control-base.jsonl`));
  assert(base.equals(readFileSync(join(root, `${name}-control-candidate.jsonl`))));
  controls[name] = hash(base);
}
for (const file of ["app.ts", "client.ts", "decoy.ts"]) {
  fixture[file] = hash(readFileSync(join(root, "fixture", file)));
}
console.log(JSON.stringify({
  real_sites:m.sites, exact_before:376, exact_after:376, changed_records:0,
  fixture_served_callers:positive.map(([name, line]) => `app.ts:${line}:${name}`),
  fixture_served_callees:["client.ts:2:m"],
  fixture_excluded_callers:["overloaded", "extra", "optional", "written", "explicitUnknown", "inner", "inline", "interfaceCaller"],
  fixture_source_sha256:fixture, source_sha256:m.call_bearing_source_sha256,
  control_jsonl_sha256:controls,
}, null, 2));
