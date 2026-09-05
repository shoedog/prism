// node verify-constructor-fields.mjs <evidence-directory>
// Paired fixed-source evidence, served navigation, and earlier control samples.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
const root = process.argv[2];
const read = (name) => JSON.parse(readFileSync(join(root, name), "utf8"));
const m = read("measurement.json");
const app = "packages/excalidraw/components/App.tsx";
const library = "packages/excalidraw/data/library.ts";
const expected = [
  [2929, "initializeScene", "updateLibrary", 287],
  [3224, "componentWillUnmount", "destroy", 249],
  [12113, "handleAppOnDrop", "getLatestLibrary", 268],
  [12239, "loadFileToCanvas", "updateLibrary", 287],
];
assert.equal(m.sites, 2780);
assert.equal(m.exact_sites_before, 372);
assert.equal(m.exact_sites_after, 376);
assert.equal(m.changed_sites.length, 4);
assert.equal(m.relevant_unique_source_spans, 11);
assert.deepEqual(m.changed_sites.map((s) => [s.source_span.file, s.source_span.line])
  .sort((a,b) => a[1]-b[1]), expected.map(([line]) => [app,line]));
for (const [line, caller, method, targetLine] of expected) {
  const site = m.relevant_sites.filter((s) => s.file === app && s.line === line);
  assert.equal(site.length, 1);
  assert.equal(site[0].caller, caller);
  assert.equal(site[0].before.length, 1);
  assert.equal(site[0].before[0].confidence, "name_only");
  assert.equal(site[0].after.length, 1);
  assert.equal(site[0].after[0].confidence, "exact");
  assert.equal(site[0].after[0].kind, "field_typed");
  assert.equal(site[0].after[0].function_id.file, library);
  assert.equal(site[0].after[0].function_id.name, method);
  assert.equal(site[0].after[0].function_id.start_line, targetLine);
}
for (const s of m.relevant_sites.filter((s) => s.file !== app)) {
  assert.deepEqual(s.after, s.before);
}
const key = (i) => JSON.stringify(i.symbol.Function);
const served = (stem) => {
  const before = read(`${stem}-base.json`), after = read(`${stem}-candidate.json`);
  for (const result of [before, after]) {
    assert.equal(result.truncated, false);
    assert.deepEqual(result.warnings, []);
    assert(result.items.every((i) => !i.fallback));
  }
  const old = new Set(before.items.map(key)), current = new Set(after.items.map(key));
  assert([...old].every((k) => current.has(k)), `removed served item: ${stem}`);
  const added = after.items.filter((i) => !old.has(key(i)));
  assert(added.every((i) => i.why.some((w) => w.Resolution?.kind === "field_typed")));
  return added;
};
const callerGains = [];
for (const targetLine of [249,268,287]) {
  const added = served(`callers-${targetLine}`);
  const expectedNames = expected.filter((s) => s[3] === targetLine).map((s) => s[1]).sort();
  assert.deepEqual(added.map((i) => i.symbol.Function.name).sort(), expectedNames);
  assert(added.every((i) => i.symbol.Function.file === app));
  callerGains.push(...expectedNames);
}
for (const [line,,method,targetLine] of expected) {
  const added = served(`callees-${line}`);
  assert.equal(added.length, 1);
  assert.equal(added[0].symbol.Function.file, library);
  assert.equal(added[0].symbol.Function.name, method);
  assert.equal(added[0].symbol.Function.start_line, targetLine);
}
const controls = {};
for (const name of ["black", "excalidraw", "javascript"]) {
  const base = readFileSync(join(root, `${name}-control-base.jsonl`));
  assert(base.equals(readFileSync(join(root, `${name}-control-candidate.jsonl`))));
  controls[name] = createHash("sha256").update(base).digest("hex");
}
console.log(JSON.stringify({
  sites:m.sites, exact_before:372, exact_after:376, changed_records:4,
  changed_unique_source_spans:4, tracked_unique_source_spans:11,
  gains:expected.map(([line,caller,method,target_line]) => ({file:app,line,caller,method,target_file:library,target_line})),
  served_caller_gains:callerGains.sort(), served_callee_gains:4,
  call_bearing_source_sha256:m.call_bearing_source_sha256, control_jsonl_sha256:controls,
}, null, 2));
