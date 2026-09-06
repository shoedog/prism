// Run with PRISM_AUDIT_{TYPESCRIPT,UPSTREAM,SLICE,SITES,SOURCE_REPO} set.
// The temporary copies are test-owned; the pinned source is never edited.
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, cpSync, appendFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
const required = ["TYPESCRIPT", "UPSTREAM", "SLICE", "SITES", "SOURCE_REPO"];
const values = required.map(k => {
  const value = process.env[`PRISM_AUDIT_${k}`];
  assert(value, `missing PRISM_AUDIT_${k}; do not silently skip source custody tests`);
  return value;
});
const script = fileURLToPath(new URL("./audit-imported-props-source.mjs", import.meta.url));
const invoke = args => spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
test("accepts the pinned tree and classifies all six receiver spans", () => {
  const result = invoke(values);
  assert.equal(result.status, 0, result.stderr);
  const audit = JSON.parse(result.stdout);
  assert.equal(audit.git_tree_blobs_verified, true);
  assert.equal(audit.tracked_typescript_files, 605);
  assert.equal(audit.remaining_unique_spans.filter(s => s.producer === "React.FC").length, 4);
  assert.equal(audit.remaining_unique_spans.filter(s => s.producer === "useApp/useContext").length, 2);
  assert.equal(audit.compiler_program_checked, false);
  assert.equal(audit.dependency_closure_checked, false);
});
for (const [name, mutate, message] of [
  ["changed augmentation bytes", dir => appendFileSync(path.join(dir, "packages/excalidraw/css.d.ts"), "\n// altered audit input\n"), /archive blob differs from pinned commit/],
  ["added augmentation file", dir => writeFileSync(path.join(dir, "extra.d.ts"), "export {}; declare global { interface Extra {} }\n"), /archive file set differs from pinned tree/],
]) {
  test(`rejects ${name} even when all five measured files are unchanged`, () => {
    const dir = mkdtempSync(path.join(tmpdir(), "prism-audit-negative-"));
    cpSync(values[1], dir, { recursive: true });
    mutate(dir);
    const args = [...values];
    args[1] = dir;
    const result = invoke(args);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, message);
    assert.equal(result.stdout, "", "a rejected source must not emit an audit claim");
  });
}
