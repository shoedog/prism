import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawn, spawnSync } from "node:child_process";

const implementationPath = process.env.PRISM_PARAMETER_FREQUENCY_IMPLEMENTATION
  ?? fileURLToPath(new URL("./index.mjs", import.meta.url));
const implementation = await import(pathToFileURL(implementationPath));
const compiler = process.env.PRISM_TYPESCRIPT;
assert(compiler, "PRISM_TYPESCRIPT is required");
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
const COMPILER_SHA = "3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675";
const ORIGINAL = "function take(value){return sink(value)}\nfunction run(input){return take(input)}";
const INERT = `/*p2*/${ORIGINAL}`;
const MEANINGFUL = ORIGINAL.replace("take(value)", "take({value})");
assert.deepEqual([Buffer.byteLength(ORIGINAL), sha(ORIGINAL)], [80, "e9a638744cd81b7875d82519dccb5a44095d66330d97c4693325fd69d54cb86b"]);
assert.deepEqual([Buffer.byteLength(INERT), sha(INERT)], [86, "ea8d905bfd336dbe72ab21753defabb8e1df2395e6ae87ca585d7f418bc60407"]);
assert.deepEqual([Buffer.byteLength(MEANINGFUL), sha(MEANINGFUL)], [82, "244101631c79f4f2d5c4083a9ac69c9b846ae5a4149a98777c522f9f693cc6cd"]);

function extension(file) {
  if (file.endsWith(".d.ts")) return ".d.ts";
  return path.extname(file);
}

function makeManifest(sources, overrides = {}) {
  const root = mkdtempSync(path.join(tmpdir(), "prism-parameter-frequency-"));
  const members = [];
  for (const [relative, value] of Object.entries(sources).sort()) {
    const target = path.join(root, relative);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, value);
    const bytes = Buffer.from(value);
    members.push({
      path: relative,
      sha256: sha(bytes),
      bytes: bytes.length,
      extension: extension(relative),
      declaration_only: relative.endsWith(".d.ts"),
      test_path: /(^|\/)(test|tests|__tests__)(\/|$)|\.(test|spec)\.[cm]?[jt]sx?$/.test(relative),
    });
  }
  const manifest = {
    schema: "prism.p2-source-manifest.v1",
    repository: "synthetic://parameter-frequency",
    commit: "fixture-v1",
    tree: "fixture-v1",
    population_root: "src",
    selection: "recursive regular files with final extension in .js,.jsx,.ts,.tsx",
    source_archive_sha256: "0".repeat(64),
    complete_source_manifest_sha256: "1".repeat(64),
    test_path_rule: "fixture rule",
    members,
    member_count: members.length,
    source_bytes: members.reduce((sum, member) => sum + member.bytes, 0),
    declaration_only_count: members.filter(member => member.declaration_only).length,
    test_path_count: members.filter(member => member.test_path).length,
    ...overrides,
  };
  const manifestPath = path.join(root, "manifest.json");
  const bytes = `${JSON.stringify(manifest)}\n`;
  writeFileSync(manifestPath, bytes);
  return {
    root,
    manifest,
    manifestPath,
    manifestSha: sha(bytes),
    cleanup: () => rmSync(root, { recursive: true, force: true }),
  };
}

function observe(fixture, extra = {}) {
  return implementation.observe({
    root: fixture.root,
    manifest: fixture.manifestPath,
    manifestSha256: fixture.manifestSha,
    typescript: compiler,
    ...extra,
  });
}

function withFixture(sources, run, overrides) {
  const fixture = makeManifest(sources, overrides);
  try { return run(fixture); } finally { fixture.cleanup(); }
}

const COMPLETE_SOURCE = [
  "const bare = value => value;",
  "const paren = (value) => (value);",
  "const asyncBare = async value => value;",
  "const generic = <T>(value: T) => value;",
  "function plain(a, {b: {c = 1}}, [d], ...rest) { function nested(x) { return x; } return a; }",
  "function negative({x}, required) { return required; }",
  "class Box { constructor(value) {} method({x}, maybe?: string, later?: number) {} get item() { return 1; } set item(value) {} }",
  "const obj = { method(value) { return value; } };",
  "type T = (value: string) => number;",
  "interface I { method(value: string): void; (value: string): void; new(value: string): I; }",
].join("\n");

test("complete body-bearing syntax records and frequency tuples", () => withFixture({
  "src/main.ts": COMPLETE_SOURCE,
  "src/empty.ts": "",
}, fixture => {
  const packet = observe(fixture);
  assert.equal(packet.schema, "prism.parameter-syntax-frequency/1");
  assert.equal(packet.authorizes_runtime_edge, false);
  assert.equal(packet.measurement, "compiler_syntax_only");
  assert.deepEqual(packet.compiler, { version: "5.9.3", sha256: COMPILER_SHA });
  assert.deepEqual(packet.files.map(file => file.path), ["src/empty.ts", "src/main.ts"]);
  const main = packet.files[1];
  assert.equal(main.parse_state, "clean");
  assert.equal(main.excluded_bodyless_signature_count, 4);
  assert.deepEqual(main.callables.map(row => [row.syntax_kind, row.arrow_parenthesized, row.parameters.length]), [
    ["ArrowFunction", false, 1], ["ArrowFunction", true, 1], ["ArrowFunction", false, 1],
    ["ArrowFunction", true, 1], ["FunctionDeclaration", null, 4], ["FunctionDeclaration", null, 1],
    ["FunctionDeclaration", null, 2], ["Constructor", null, 1], ["MethodDeclaration", null, 3],
    ["GetAccessor", null, 0], ["SetAccessor", null, 1], ["MethodDeclaration", null, 1],
  ]);
  const plain = main.callables[4];
  assert.deepEqual(plain.parameters.map(row => [row.ordinal, row.pattern, row.optional, row.rest, row.outer_initializer, row.nested_pattern, row.nested_default]), [
    [0, "identifier", false, false, false, false, false],
    [1, "object", false, false, false, true, true],
    [2, "array", false, false, false, false, false],
    [3, "identifier", false, true, false, false, false],
  ]);
  const method = main.callables[8];
  assert.deepEqual(method.parameters.map(row => [row.pattern, row.optional]), [["object", false], ["identifier", true], ["identifier", true]]);
  const frequency = packet.frequency_tables.find(row => row.script_kind === "TypeScript" && !row.declaration_only && !row.test_path && row.parse_state === "clean");
  assert.deepEqual([
    frequency.files, frequency.body_bearing_callables, frequency.parameters,
    frequency.object_pattern_parameters, frequency.array_pattern_parameters,
    frequency.nested_pattern_parameters, frequency.nested_default_parameters,
    frequency.rest_parameters, frequency.parenthesized_arrows, frequency.unparenthesized_arrows,
    frequency.destructured_before_later_optional_parameters,
    frequency.callables_with_destructured_before_later_optional,
  ], [2, 12, 17, 3, 1, 1, 1, 1, 2, 2, 1, 1]);
}));

test("bodyless type fixture is independently excluded", () => withFixture({
  "src/only.d.ts": "type T = (value: string) => number;",
}, fixture => {
  const file = observe(fixture).files[0];
  assert.equal(file.excluded_bodyless_signature_count, 1);
  assert.deepEqual(file.callables, []);
}));

test("initializer expressions and types do not forge nested binding flags", () => withFixture({
  "src/flags.ts": "function flags(value: Type<{x:number}> = make({x:{y:1}}), {a = make({b:1})}) { return value; }",
}, fixture => {
  const parameters = observe(fixture).files[0].callables[0].parameters;
  assert.deepEqual(parameters.map(row => [row.pattern, row.outer_initializer, row.nested_pattern, row.nested_default]), [
    ["identifier", true, false, false], ["object", false, false, true],
  ]);
}));

test("expression-local arrow bindings do not leak into outer binding flags", () => {
  const cases = [
    {
      source: "function take({ x = (({y}) => y)({y:1}) }) { return x; }",
      callables: [
        { syntax_kind: "FunctionDeclaration", token_start: 0, end: 56, arrow_parenthesized: null, parameters: [
          { ordinal: 0, token_start: 14, end: 41, binding_start: 14, binding_end: 41, pattern: "object", optional: false, rest: false, outer_initializer: false, nested_pattern: false, nested_default: true },
        ] },
        { syntax_kind: "ArrowFunction", token_start: 21, end: 31, arrow_parenthesized: true, parameters: [
          { ordinal: 0, token_start: 22, end: 25, binding_start: 22, binding_end: 25, pattern: "object", optional: false, rest: false, outer_initializer: false, nested_pattern: false, nested_default: false },
        ] },
      ],
    },
    {
      source: "function take({ [(({y = 1}) => \"x\")({})]: x }) { return x; }",
      callables: [
        { syntax_kind: "FunctionDeclaration", token_start: 0, end: 60, arrow_parenthesized: null, parameters: [
          { ordinal: 0, token_start: 14, end: 45, binding_start: 14, binding_end: 45, pattern: "object", optional: false, rest: false, outer_initializer: false, nested_pattern: false, nested_default: false },
        ] },
        { syntax_kind: "ArrowFunction", token_start: 18, end: 34, arrow_parenthesized: true, parameters: [
          { ordinal: 0, token_start: 19, end: 26, binding_start: 19, binding_end: 26, pattern: "object", optional: false, rest: false, outer_initializer: false, nested_pattern: false, nested_default: true },
        ] },
      ],
    },
  ];
  for (const current of cases) withFixture({ "src/main.ts": current.source }, fixture => {
    const packet = observe(fixture);
    assert.deepEqual(packet.files[0].callables, current.callables);
    assert.deepEqual(packet.frequency_tables[0], {
      script_kind: "TypeScript", declaration_only: false, test_path: false, parse_state: "clean",
      files: 1, diagnostics: 0, excluded_bodyless_signatures: 0, body_bearing_callables: 2,
      parameters: 2, object_pattern_parameters: 2, array_pattern_parameters: 0,
      nested_pattern_parameters: 0, nested_default_parameters: 1, rest_parameters: 0,
      parenthesized_arrows: 1, unparenthesized_arrows: 0,
      destructured_before_later_optional_parameters: 0,
      callables_with_destructured_before_later_optional: 0,
    });
  });
});

test("destructuring before optional is per parameter and keeps a negative stratum", () => withFixture({
  "src/one.ts": "function one({x}, later?: string) {}",
  "src/tests/multiple.test.ts": "function multiple({x}, first?: string, second?: number) {}",
  "src/negative.js": "function negative({x}, required) {}",
}, fixture => {
  const rows = observe(fixture).frequency_tables;
  const counts = Object.fromEntries(rows.map(row => [`${row.script_kind}:${row.test_path}`, [
    row.destructured_before_later_optional_parameters,
    row.callables_with_destructured_before_later_optional,
  ]]));
  assert.deepEqual(counts, { "JavaScript:false": [0, 0], "TypeScript:false": [1, 1], "TypeScript:true": [1, 1] });
}));

test("UTF8 mapping and emitted spans retain BOM astral CRLF and trivia", () => withFixture({
  "src/unicode.ts": "\uFEFFconst fox = \"🦊\";\r\nfunction f(/*π*/value: string) { return value; }",
}, fixture => {
  const file = observe(fixture).files[0];
  const callable = file.callables[0];
  const parameter = callable.parameters[0];
  assert.deepEqual([callable.token_start, callable.end], [24, 73]);
  assert.deepEqual([parameter.token_start, parameter.end, parameter.binding_start, parameter.binding_end], [41, 54, 41, 46]);
  const map = implementation.utf8Map("A🦊π");
  assert.deepEqual([map.at(0), map.at(1), map.at(2), map.at(3), map.at(4)], [0, 1, null, 5, 7]);
}));

test("same-name rows keep distinct sorted tuples and ordinals", () => withFixture({
  "src/names.ts": "function same(x){} function same(x,y){}",
}, fixture => {
  const rows = observe(fixture).files[0].callables;
  assert.equal(rows.length, 2);
  assert(rows[0].token_start < rows[1].token_start);
  assert.deepEqual(rows.flatMap(row => row.parameters.map(parameter => parameter.ordinal)), [0, 0, 1]);
  assert.equal(new Set(rows.map(row => `${row.token_start}:${row.end}:0`)).size, 2);
}));

test("canonical file order uses Unicode code points and repeats byte-for-byte", () => withFixture({
  "src/\u{10000}.ts": "function later(value) { return value; }",
  "src/\uE000.ts": "function earlier(value) { return value; }",
}, fixture => {
  assert.deepEqual(fixture.manifest.members.map(member => member.path), ["src/\u{10000}.ts", "src/\uE000.ts"]);
  const first = observe(fixture);
  assert.deepEqual(first.files.map(file => file.path), ["src/\uE000.ts", "src/\u{10000}.ts"]);
  assert.deepEqual(observe(fixture), first);
}));

test("diagnostics continue to later files and preserve disjoint strata", () => withFixture({
  "src/broken.ts": "function broken({",
  "src/valid.ts": "function valid(value) { return value; }",
  "src/types.d.ts": "declare function declared(value: string): void;",
  "src/tests/valid.test.tsx": "const view = (value: string) => <div>{value}</div>;",
}, fixture => {
  const packet = observe(fixture);
  assert.deepEqual(packet.files.map(row => [row.path, row.parse_state, row.declaration_only, row.test_path]), [
    ["src/broken.ts", "syntax_diagnostics", false, false], ["src/tests/valid.test.tsx", "clean", false, true],
    ["src/types.d.ts", "clean", true, false], ["src/valid.ts", "clean", false, false],
  ]);
  assert.deepEqual(packet.files[0].diagnostics, [{ code: 1005, start: 17, length: 0, start_byte: 17, end_byte: 17 }]);
  assert.equal(packet.files[3].callables.length, 1);
  assert.equal(packet.frequency_tables.reduce((sum, row) => sum + row.files, 0), 4);
}));

test("manifest identity, population, path, UTF8, compiler and caps fail closed", async t => {
  await t.test("stale manifest hash", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    assert.throws(() => observe(fixture, { manifestSha256: "0".repeat(64) }), /manifest SHA mismatch/);
  }));
  await t.test("member mutation and missing", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    writeFileSync(path.join(fixture.root, "src/a.ts"), "function b(x){}");
    assert.throws(() => observe(fixture), /member identity mismatch/);
    rmSync(path.join(fixture.root, "src/a.ts"));
    assert.throws(() => observe(fixture), /member/);
  }));
  await t.test("extra source and duplicate member", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    writeFileSync(path.join(fixture.root, "src/extra.ts"), "export{};");
    assert.throws(() => observe(fixture), /population differs/);
    rmSync(path.join(fixture.root, "src/extra.ts"));
    fixture.manifest.members.push(fixture.manifest.members[0]);
    fixture.manifest.member_count++;
    const bytes = `${JSON.stringify(fixture.manifest)}\n`;
    writeFileSync(fixture.manifestPath, bytes); fixture.manifestSha = sha(bytes);
    assert.throws(() => observe(fixture), /duplicate member/);
  }));
  await t.test("path escape and symlink", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    fixture.manifest.members[0].path = "../a.ts";
    const bytes = `${JSON.stringify(fixture.manifest)}\n`;
    writeFileSync(fixture.manifestPath, bytes); fixture.manifestSha = sha(bytes);
    assert.throws(() => observe(fixture), /unsafe member path/);
  }));
  await t.test("symlinked population ancestor", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    const real = path.join(fixture.root, "real-src"); mkdirSync(real); writeFileSync(path.join(real, "a.ts"), "function a(x){}");
    rmSync(path.join(fixture.root, "src"), { recursive: true }); symlinkSync(real, path.join(fixture.root, "src"));
    assert.throws(() => observe(fixture), /symlink in population/);
  }));
  await t.test("invalid UTF8", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    const target = path.join(fixture.root, "src/a.ts"); const bytes = Buffer.from([0xff]); writeFileSync(target, bytes);
    fixture.manifest.members[0].bytes = 1; fixture.manifest.members[0].sha256 = sha(bytes); fixture.manifest.source_bytes = 1;
    const json = `${JSON.stringify(fixture.manifest)}\n`; writeFileSync(fixture.manifestPath, json); fixture.manifestSha = sha(json);
    assert.throws(() => observe(fixture), /UTF-8/);
  }));
  await t.test("declaration stratum matches the file extension in both directions", () => withFixture({
    "src/code.ts": "function code(value) { return value; }",
    "src/types.d.ts": "declare function types(value: string): void;",
  }, fixture => {
    assert.deepEqual(observe(fixture).files.map(file => [file.path, file.declaration_only]), [
      ["src/code.ts", false], ["src/types.d.ts", true],
    ]);
    fixture.manifest.members.find(member => member.path.endsWith(".d.ts")).declaration_only = false;
    fixture.manifest.declaration_only_count = 0;
    let bytes = `${JSON.stringify(fixture.manifest)}\n`;
    writeFileSync(fixture.manifestPath, bytes); fixture.manifestSha = sha(bytes);
    assert.throws(() => observe(fixture), /declaration stratum mismatch/);
    fixture.manifest.members.find(member => member.path.endsWith(".d.ts")).declaration_only = true;
    fixture.manifest.members.find(member => member.path.endsWith("code.ts")).declaration_only = true;
    fixture.manifest.declaration_only_count = 2;
    bytes = `${JSON.stringify(fixture.manifest)}\n`;
    writeFileSync(fixture.manifestPath, bytes); fixture.manifestSha = sha(bytes);
    assert.throws(() => observe(fixture), /declaration stratum mismatch/);
  }));
  await t.test("all resource caps and compiler identity", () => withFixture({ "src/a.ts": "function a(x){}" }, fixture => {
    for (const limits of [{ files: 0 }, { sourceBytes: 1 }, { rows: 1 }, { outputBytes: 1 }]) {
      assert.throws(() => observe(fixture, { limits }), /limit|cap/i);
    }
    const fake = path.join(fixture.root, "typescript.js"); writeFileSync(fake, "module.exports={version:'5.9.3'}");
    assert.throws(() => observe(fixture, { typescript: fake }), /compiler SHA mismatch/);
  }));
});

test("cold repeat and exact mutation restoration are deterministic", () => withFixture({ "src/mutation.ts": ORIGINAL }, fixture => {
  const original = observe(fixture);
  assert.equal(`${JSON.stringify(original)}\n`, `${JSON.stringify(observe(fixture))}\n`);
  const source = path.join(fixture.root, "src/mutation.ts");
  writeFileSync(source, INERT);
  assert.throws(() => observe(fixture), /member identity mismatch/);
  const inertFixture = makeManifest({ "src/mutation.ts": INERT });
  const meaningfulFixture = makeManifest({ "src/mutation.ts": MEANINGFUL });
  try {
    const inert = observe(inertFixture); const meaningful = observe(meaningfulFixture);
    const inertExpected = structuredClone(original);
    inertExpected.manifest.sha256 = inertFixture.manifestSha;
    inertExpected.manifest.source_bytes = Buffer.byteLength(INERT);
    inertExpected.files[0].sha256 = sha(INERT);
    inertExpected.files[0].bytes = Buffer.byteLength(INERT);
    for (const callable of inertExpected.files[0].callables) {
      callable.token_start += 6; callable.end += 6;
      for (const parameter of callable.parameters) {
        parameter.token_start += 6; parameter.end += 6;
        parameter.binding_start += 6; parameter.binding_end += 6;
      }
    }
    assert.deepEqual(inert, inertExpected);
    const meaningfulExpected = structuredClone(original);
    meaningfulExpected.manifest.sha256 = meaningfulFixture.manifestSha;
    meaningfulExpected.manifest.source_bytes = Buffer.byteLength(MEANINGFUL);
    meaningfulExpected.files[0].sha256 = sha(MEANINGFUL);
    meaningfulExpected.files[0].bytes = Buffer.byteLength(MEANINGFUL);
    meaningfulExpected.files[0].callables[0].end += 2;
    meaningfulExpected.files[0].callables[0].parameters[0].end += 2;
    meaningfulExpected.files[0].callables[0].parameters[0].binding_end += 2;
    meaningfulExpected.files[0].callables[0].parameters[0].pattern = "object";
    meaningfulExpected.files[0].callables[1].token_start += 2;
    meaningfulExpected.files[0].callables[1].end += 2;
    for (const field of ["token_start", "end", "binding_start", "binding_end"]) meaningfulExpected.files[0].callables[1].parameters[0][field] += 2;
    meaningfulExpected.frequency_tables[0].object_pattern_parameters = 1;
    assert.deepEqual(meaningful, meaningfulExpected);
    const restored = makeManifest({ "src/mutation.ts": ORIGINAL });
    try { assert.deepEqual(observe(restored), original); } finally { restored.cleanup(); }
  } finally { inertFixture.cleanup(); meaningfulFixture.cleanup(); }
}));

test("CLI success, refusal, output custody and owned-child timeout", () => withFixture({ "src/a.ts": "function a(value){return value}" }, fixture => {
  const out = path.join(fixture.root, "out.json");
  const args = ["--root", fixture.root, "--manifest", fixture.manifestPath, "--manifest-sha256", fixture.manifestSha, "--typescript", compiler, "--out", out];
  const first = spawnSync(process.execPath, [implementationPath, ...args], { encoding: "utf8", timeout: 30_000 });
  assert.equal(first.status, 0, first.stderr);
  const output = readFileSync(out, "utf8");
  assert.equal(JSON.parse(output).schema, "prism.parameter-syntax-frequency/1");
  assert(output.endsWith("\n")); assert(!output.endsWith("\n\n"));
  const existing = spawnSync(process.execPath, [implementationPath, ...args], { encoding: "utf8" });
  assert.notEqual(existing.status, 0); assert.match(existing.stderr, /output already exists/);
  const bad = spawnSync(process.execPath, [implementationPath, ...args, "--unknown", "x"], { encoding: "utf8" });
  assert.notEqual(bad.status, 0); assert.equal(readFileSync(out, "utf8").startsWith("{"), true);
  const duplicate = spawnSync(process.execPath, [implementationPath, ...args, "--root", fixture.root], { encoding: "utf8" });
  assert.notEqual(duplicate.status, 0); assert.match(duplicate.stderr, /duplicate flag/);
  const timed = implementation.supervise(args.map((value, index) => index === args.length - 1 ? path.join(fixture.root, "timeout.json") : value), { wallMs: 10, delayMs: 100 });
  assert.equal(timed.ok, false); assert.equal(timed.reason, "timeout");
  assert.throws(() => readFileSync(path.join(fixture.root, "timeout.json")), /ENOENT/);
}));

test("timeout cleanup preserves a concurrently-created foreign output", () => withFixture({ "src/a.ts": "function a(value){return value}" }, fixture => {
  const out = path.join(fixture.root, "foreign.json");
  const marker = path.join(fixture.root, "writer-ran");
  const args = ["--root", fixture.root, "--manifest", fixture.manifestPath, "--manifest-sha256", fixture.manifestSha, "--typescript", compiler, "--out", out];
  const writer = spawn(process.execPath, ["-e", `setTimeout(()=>{require('fs').writeFileSync(${JSON.stringify(out)},'foreign sentinel');require('fs').writeFileSync(${JSON.stringify(marker)},'yes')},75)`], { stdio: "ignore" });
  try {
    const timed = implementation.supervise(args, { wallMs: 500, delayMs: 2_000 });
    assert.deepEqual(timed, { ok: false, reason: "timeout" });
    assert.equal(readFileSync(marker, "utf8"), "yes");
    assert.equal(readFileSync(out, "utf8"), "foreign sentinel");
    assert.deepEqual(readdirSync(fixture.root).filter(name => name.includes(".partial-")), []);
  } finally {
    writer.kill();
  }
}));
