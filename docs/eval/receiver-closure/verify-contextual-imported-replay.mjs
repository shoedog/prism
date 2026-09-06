// node verify-contextual-imported-replay.mjs <evidence-directory>
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
const root = process.argv[2];
const bytes = name => readFileSync(join(root, name));
const rows = name => bytes(name).toString().trim().split("\n").map(JSON.parse);
const exact = r => r.resolved_targets.filter(t => t.confidence === "exact");
assert(bytes("real-base.jsonl").equals(bytes("real-candidate.jsonl")), "all raw real-site records must be unchanged");
const real = rows("real-candidate.jsonl");
assert.equal(real.length, 2780);
assert(real.every(r => r.record_kind === "call_site"));
assert.equal(real.filter(r => exact(r).length).length, 376);
const base = rows("served-base.jsonl"), candidate = rows("served-candidate.jsonl");
assert.equal(base.length, 6);
assert.equal(candidate.length, 6);
assert(base.every(r => exact(r).length === 0));
const positives = ["callable", "direct", "generic"];
assert.deepEqual(candidate.map(r => r.caller.name).sort(), [...positives, "explicit", "framework", "written"].sort());
for (const r of candidate) {
  const edges = exact(r);
  assert.equal(edges.length, Number(positives.includes(r.caller.name)), r.caller.name);
  if (edges.length) {
    assert.equal(edges[0].kind, "typed_param");
    assert.equal(edges[0].function_id.file, "client.ts");
    assert.equal(edges[0].function_id.start_line, 2);
  }
}
console.log(JSON.stringify({real_sites: real.length, real_exact: 376, changed_real_records: 0,
  served_exact_before: 0, served_exact_after: positives.length,
  excluded: ["explicit", "framework", "written"]}, null, 2));
