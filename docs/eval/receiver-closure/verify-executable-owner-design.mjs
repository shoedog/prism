// Design characterization ONLY; no proof constructor and no runtime consumer.
// node <this-file> <typescript.js> <fresh-prism-binary> <evidence-parent> [--expect-future-positive]
import assert from "node:assert/strict";
import {createHash} from "node:crypto";
import {mkdirSync, mkdtempSync, readFileSync, writeFileSync} from "node:fs";
import {spawnSync} from "node:child_process";
import path from "node:path";
import {produce, validate, producerHash} from "../../../scripts/callable-observations/index.mjs";

const [compiler, binary, parent, mode] = process.argv.slice(2);
assert(compiler && binary && parent && (!mode || mode === "--expect-future-positive"));
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const corpusBytes = readFileSync(new URL("./executable-owner-fixtures.json", import.meta.url));
const corpus = JSON.parse(corpusBytes);
const evidence = mkdtempSync(path.join(path.resolve(parent), "owner-design-"));
const results = [], failures = [];
const execute = args => {
  const run = spawnSync(path.resolve(binary), args, {encoding: "utf8", timeout: 30000, maxBuffer: 8 * 1024 * 1024});
  assert.equal(run.status, 0, run.error?.message ?? run.stderr);
  const output = JSON.parse(run.stdout);
  assert.equal(output.truncated, false, "truncated navigation is not admissible");
  assert.deepEqual(output.warnings, [], "navigation warning is not admissible");
  return output;
};
const checked = (id, fn) => {try {fn();} catch (error) {failures.push({id, message: error.message});}};
assert.equal(new Set(corpus.cases.map(c => c.id)).size, corpus.cases.length);
for (const extension of ["ts", "tsx"]) for (const fixture of corpus.cases) {
  const id = `${fixture.id}/${extension}`;
  const root = path.join(evidence, fixture.id, extension);
  mkdirSync(root, {recursive: true});
  const files = {...corpus.files, ...fixture.files};
  for (const name of fixture.remove ?? []) delete files[name];
  if (fixture.replace_app) {
    const [before, after] = fixture.replace_app;
    assert.equal(files["src/app.ts"].split(before).length, 2, `${id}: replacement must match once`);
    files["src/app.ts"] = files["src/app.ts"].replace(before, after);
  }
  const app = `src/app.${extension}`;
  if (extension === "tsx") {files[app] = files["src/app.ts"]; delete files["src/app.ts"];}
  const options = {...corpus.compiler_options, ...fixture.options};
  for (const name of fixture.delete_options ?? []) delete options[name];
  files["tsconfig.json"] = JSON.stringify({compilerOptions: options, include: ["src"]});
  files["package.json"] = JSON.stringify({type: "module"});
  for (const [file, text] of Object.entries(files)) {
    assert(!path.isAbsolute(file) && !file.split("/").includes(".."));
    mkdirSync(path.dirname(path.join(root, file)), {recursive: true});
    writeFileSync(path.join(root, file), text);
  }
  const observerOptions = {compiler, root, config: "tsconfig.json"};
  const packet = produce(observerOptions);
  const validation = validate(JSON.stringify(packet), observerOptions);
  // Keep packets OUTSIDE the indexed fixture roots so validation sees identical inputs.
  writeFileSync(path.join(evidence, `${fixture.id}-${extension}.packet.json`), JSON.stringify(packet, null, 2));
  const nav = execute(["nav", "--no-cache", "callees", "--repo", root, "--symbol", fixture.caller ?? "run",
    "--file", app, "--confidence", "exact", "--format", "json"]);
  const methods = nav.items.filter(item => item.symbol?.Function?.name === "m");
  const observation = packet.observations.find(o => o.implementation.file === `project/${app}`);
  const result = {id, design_requirement: fixture.design, status: packet.status, reasons: packet.reasons,
    diagnostics: packet.diagnostics, semantic_closure: packet.semantic_closure,
    validation, observation: observation ?? null, program_files: packet.snapshot.program_files,
    input_files: Object.entries(files).sort().map(([file, source]) => ({file, sha256: hash(source)})),
    snapshot_sha256: packet.snapshot.sha256, nav, exact_method_edges: methods,
    authorizes_runtime_edge: false};
  results.push(result);
  checked(id, () => {
    assert(!packet.reasons.some(r => ["worker_failed", "budget_exceeded"].includes(r)),
      "worker/setup failure is not a refusal observation");
    assert.equal(packet.compiler.verified, true);
    assert.equal(packet.closure.stable_snapshot, true);
    assert.equal(validation.valid, true);
    assert.equal(packet.authorizes_runtime_edge, false);
    assert.equal(validation.authorizes_runtime_edge, false);
    assert.equal(packet.semantic_closure.complete, fixture.semantic_complete, "semantic completeness");
    assert.equal(packet.diagnostics.length === 0, fixture.diagnostics_empty, "diagnostics empty");
    const future = mode && ["candidate", "different_genuine_input"].includes(fixture.id);
    const owner = future ? "src/client.ts" : fixture.baseline_exact?.replace("src/app.ts", app);
    assert.equal(methods.length, owner ? 1 : 0, future
      ? "FUTURE POSITIVE: direct contextual Client.m must resolve Exact" : "baseline Exact method edge count");
    if (owner) assert.equal(methods[0].symbol.Function.file, owner);
    if (fixture.id === "candidate" || fixture.id === "different_genuine_input") {
      assert(observation && !observation.explicit_parameter);
      assert.equal(observation.signatures.length, 1);
      assert.equal(observation.signatures[0].file, "project/src/contract.ts");
      assert.equal(observation.calls.length, 1);
      const call = observation.calls[0];
      assert.equal(call.receiver_type, "Client");
      assert.equal(call.declarations.length, 1);
      assert.equal(call.declarations[0].kind, "MethodDeclaration");
      assert.equal(call.declarations[0].file, "project/src/client.ts");
      assert.equal(call.declarations[0].start_byte, 24);
      assert.equal(call.declarations[0].end_byte, 49);
      assert.equal(Buffer.from(files[app]).subarray(call.call.start_byte, call.call.end_byte).toString(), "client.m()");
      assert.equal(Object.hasOwn(call, "props_class"), false);
      assert.deepEqual(observation.nested.calls, []);
    }
  });
  process.stderr.write(`${id}: ${failures.some(f => f.id === id) ? "FAIL" : "ok"}\n`);
}
for (const extension of ["ts", "tsx"]) checked(`two-genuine-inputs/${extension}`, () => {
  const a = results.find(r => r.id === `candidate/${extension}`);
  const b = results.find(r => r.id === `different_genuine_input/${extension}`);
  assert.notEqual(a.snapshot_sha256, b.snapshot_sha256);
  assert.deepEqual(a.observation.calls[0].call, b.observation.calls[0].call);
  assert.notEqual(a.observation.calls[0].declarations[0].sha256, b.observation.calls[0].declarations[0].sha256);
});
const report = {schema: "prism.executable-owner-design-report/1", evidence_directory: evidence,
  corpus_sha256: hash(corpusBytes), verifier_sha256: hash(readFileSync(new URL(import.meta.url))),
  compiler_sha256: hash(readFileSync(compiler)), producer_sha256: producerHash(),
  prism_binary_sha256: hash(readFileSync(binary)), mode: mode ?? "baseline-characterization",
  results, failures, authorizes_runtime_edge: false};
writeFileSync(path.join(evidence, "report.json"), JSON.stringify(report, null, 2) + "\n");
console.log(JSON.stringify({evidence_directory: evidence, cases: results.length, failures}, null, 2));
assert.deepEqual(failures, [], "complete fixture failure population (not first-error-only)");
