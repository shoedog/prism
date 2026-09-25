import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdtempSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import * as builder from "./request.mjs";

const native = process.env.PRISM_NATIVE_PARAMETER_EXAMPLE;
assert(native, "PRISM_NATIVE_PARAMETER_EXAMPLE is required");
const OUTPUT = "prism.native-parameter-characterization/1";
const sha = value => createHash("sha256").update(value).digest("hex");
const nativeSha256 = sha(readFileSync(native));
const bytes = value => Buffer.byteLength(value);
const kind = file => file.endsWith(".tsx") ? "Tsx" : file.endsWith(".ts") ? "TypeScript" :
  file.endsWith(".jsx") ? "Jsx" : "JavaScript";
const language = file => kind(file) === "Jsx" ? "JavaScript" : kind(file);
const p = (ordinal, kind, start_byte, end_byte, pattern_kind = kind,
  ordinary_required_identifier = kind === "identifier") => (
  { ordinal, kind, start_byte, end_byte, pattern_kind, ordinary_required_identifier }
);
const o = (name, start_byte, end_byte, source_ordinal) => (
  { name, start_byte, end_byte, source_ordinal }
);
const c = (parameters, slots, bindings, name = "take", kind = "function_declaration",
  start_line = 1, end_line = 1) => (
  { kind, start_line, end_line, name, parameters, slots, bindings }
);
const select = (path, source, compiler_kind = "FunctionDeclaration", object_ordinals = [0],
  later_required_ordinals = [1], start_byte = 0, end_byte = bytes(source)) => (
  { path, start_byte, end_byte, compiler_kind, object_ordinals, later_required_ordinals }
);
const site = (selector, status, candidates, next_proof_eligibility) => ({
  path: selector.path, start_byte: selector.start_byte, end_byte: selector.end_byte, status,
  candidates, next_proof_eligibility
});

function fixture(sources, sites, mutate = {}) {
  const root = mkdtempSync(path.join(realpathSync(tmpdir()), "prism-slot-"));
  const members = Object.entries(sources).map(([file, source]) => {
    const target = path.join(root, file);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, source);
    return { path: file, sha256: sha(source), bytes: bytes(source), script_kind: kind(file) };
  });
  const manifest = {
    upstream_repository: "synthetic://native-slot", members, sites, ...mutate
  };
  const manifestPath = path.join(root, "manifest.json");
  const save = () => {
    const text = `${JSON.stringify(manifest)}\n`;
    writeFileSync(manifestPath, text);
    return sha(text);
  };
  return {
    root, members, sites, manifest, manifestPath, manifestSha256: save(),
    options() {
      return { root, manifest: manifestPath, manifestSha256: this.manifestSha256, nativeSha256 };
    },
    save() { this.manifestSha256 = save(); },
    cleanup() { rmSync(root, { recursive: true, force: true }); }
  };
}

const using = (value, body) => {
  try { return body(value); } finally { value.cleanup(); }
};

const worker = input => spawnSync(native, [], { input, encoding: "utf8" });

function run(options) {
  const request = builder.buildRequest(options);
  const result = worker(`${JSON.stringify(request)}\n`);
  if (result.status !== 0) throw Error(result.stderr);
  return JSON.parse(result.stdout);
}

function expected(fixture, sites, errors = {}) {
  const counts = Object.fromEntries(
    ["unique_named", "unique_unnamed", "missing", "ambiguous", "recovery_quarantined"].map(
      status => [status, sites.filter(value => value.status === status).length]
    )
  );
  const eligible = sites.some(value => value.next_proof_eligibility === "eligible");
  return {
    schema: OUTPUT,
    measurement: "native_parameter_api_characterization",
    authorizes_runtime_edge: false, native_entry_measured: false, callee_resolution_measured: false,
    input_manifest_sha256: fixture.manifestSha256,
    native_binary_sha256: nativeSha256,
    files: [...fixture.members].sort((a, b) => a.path.localeCompare(b.path)).map(member => ({
      path: member.path,
      sha256: member.sha256,
      bytes: member.bytes,
      language: language(member.path),
      parse_error_count: errors[member.path] ?? 0
    })),
    sites,
    totals: { files: fixture.members.length, sites: sites.length, ...counts },
    next_action: eligible ? "bounded_entry_and_call_proof" : "defer",
    reason: eligible
      ? "named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof"
      : "no_eligible_native_slot_gap"
  };
}

function complete(fixture, sites, errors) {
  assert.deepEqual(run(fixture.options()), expected(fixture, sites, errors));
}

function adapter(compressOrdinal) {
  return {
    source_ordinal: compressOrdinal ? 1 : 2,
    path: "alias.js",
    parameter_count: 3,
    next_action: "bounded_entry_and_call_proof"
  };
}

test("RED adapter exposes compressed alias binding ordinal", {
  skip: process.env.PRISM_CAPTURE_RED !== "1"
}, () => assert.deepEqual(adapter(true), adapter(false)));

test("1: raw positions, prefix refusal, and shape controls are complete", () => {
  const object = "function take({x}, later){return later;}";
  const alias = "function take(first, {key: value}, later){return later;}";
  const aliasSite = select("alias.js", alias, "FunctionDeclaration", [1], [2]);
  using(fixture({ "object.js": object, "alias.js": alias }, [
    select("object.js", object), aliasSite
  ]), f => complete(f, [
    site(aliasSite, "unique_named", [c([
      p(0, "identifier", 14, 19), p(1, "object_pattern", 21, 33),
      p(2, "identifier", 35, 40)
    ], [o("first", 14, 19, 0)], [
      o("first", 14, 19, 0), o("later", 35, 40, 2)
    ])], "eligible"),
    site(f.sites[0], "unique_named", [c([
      p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)
    ], [], [o("later", 19, 24, 1)])], "eligible")
  ]));
  const cases = [
    ["plain.js", "function take(first, later){return later;}", [0], [1],
      c([p(0, "identifier", 14, 19), p(1, "identifier", 21, 26)],
        [o("first", 14, 19, 0), o("later", 21, 26, 1)],
        [o("first", 14, 19, 0), o("later", 21, 26, 1)]), "selection_native_shape_mismatch"],
    ["array.js", "function take([x], later){return later;}", [0], [1],
      c([p(0, "array_pattern", 14, 17), p(1, "identifier", 19, 24)], [],
        [o("later", 19, 24, 1)]), "selection_native_shape_mismatch"],
    ["default.js", "function take({x}, later = 1){return later;}", [0], [1],
      c([p(0, "object_pattern", 14, 17),
        p(1, "assignment_pattern", 19, 28, "assignment_pattern", false)], [], []),
      "selection_native_shape_mismatch"],
    ["range.js", object, [9], [1],
      c([p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [],
        [o("later", 19, 24, 1)]), "selection_native_shape_mismatch"],
    ["gapless.js", "function take({x}, later){return 0;}", [0], [1],
      c([p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [],
        [o("later", 19, 24, 1)]), "eligible"]
  ];
  for (const [file, source, objects, later, candidate, eligibility] of cases) {
    const selector = select(file, source, "FunctionDeclaration", objects, later);
    using(fixture({ [file]: source }, [selector]), f => complete(f, [
      site(selector, "unique_named", [candidate], eligibility)
    ]));
  }
});

test("2: arrow identity, names, nesting, and exact span selection are complete", () => {
  const assigned = "const take = ({x}, later) => later;";
  const wrapped = "const take = memo(({x}, later) => later);";
  const unnamed = "consume(({x}, later) => later);";
  const cases = [
    ["assigned.js", assigned, 13, 34, "take", [p(0, "object_pattern", 14, 17),
      p(1, "identifier", 19, 24)], [o("later", 19, 24, 1)], "unique_named", "eligible"],
    ["wrapped.js", wrapped, 18, 39, "take", [p(0, "object_pattern", 19, 22),
      p(1, "identifier", 24, 29)], [o("later", 24, 29, 1)], "unique_named", "eligible"],
    ["unnamed.js", unnamed, 8, 29, null, [p(0, "object_pattern", 9, 12),
      p(1, "identifier", 14, 19)], [o("later", 14, 19, 1)], "unique_unnamed", "not_clean_named"]
  ];
  for (const [file, source, start, end, name, parameters, bindings, status, eligibility] of cases) {
    const selector = select(file, source, "ArrowFunction", [0], [1], start, end);
    using(fixture({ [file]: source }, [selector]), f => complete(f, [
      site(selector, status, [c(parameters, [], bindings, name, "arrow_function")], eligibility)
    ]));
  }
  const nested = "function take({x}, later){function inner(later){return later;}return later;}";
  const outer = select("nested.js", nested);
  const inner = select("nested.js", nested, "FunctionDeclaration", [0], [1], 26, 62);
  using(fixture({ "nested.js": nested }, [outer, inner]), f => complete(f, [
    site(outer, "unique_named", [c([p(0, "object_pattern", 14, 17),
      p(1, "identifier", 19, 24)], [], [o("later", 19, 24, 1)])], "eligible"),
    site(inner, "unique_named", [c([p(0, "identifier", 41, 46)],
      [o("later", 41, 46, 0)], [o("later", 41, 46, 0)], "inner")],
      "selection_native_shape_mismatch")
  ]));
  using(fixture({ "near.js": assigned }, [
    select("near.js", assigned, "ArrowFunction", [0], [1], 14, 34)
  ]), f => complete(f, [site(f.sites[0], "missing", [], "not_clean_named")]));
});

test("3 and 3b: JS, TS, TSX, recovery, and required-wrapper records are complete", () => {
  const duplicate = "function take({x}, x){return x;}";
  const duplicates = [
    ["duplicate.js", [p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 20)],
      [o("x", 19, 20, 1)]],
    ["duplicate.ts", [p(0, "required_parameter", 14, 17, "object_pattern", false),
      p(1, "required_parameter", 19, 20, "identifier", true)], []],
    ["duplicate.tsx", [p(0, "required_parameter", 14, 17, "object_pattern", false),
      p(1, "required_parameter", 19, 20, "identifier", true)], []]
  ];
  for (const [file, parameters, bindings] of duplicates) {
    const selector = select(file, duplicate);
    using(fixture({ [file]: duplicate }, [selector]), f => complete(f, [
      site(selector, "unique_named", [c(parameters, null, bindings)],
        "native_slot_authority_unavailable")
    ]));
  }
  const typed = [
    ["comment.tsx", "function take({x}: {x:number}, /*a*/ later /*ok*/: string){return later;}",
      [p(0, "required_parameter", 14, 29, "object_pattern", false),
        p(1, "required_parameter", 37, 57, "identifier", true)],
        [o("later", 37, 42, 1)], "eligible"],
    ["modifier.ts", "function take({x}: {x:number}, public later: string){return later;}",
      [p(0, "required_parameter", 14, 29, "object_pattern", false),
        p(1, "required_parameter", 31, 51, "identifier", false)],
        [], "selection_native_shape_mismatch"],
    ["optional-object.ts", "function take({x}?: {x:number}, later: string){return later;}",
      [p(0, "optional_parameter", 14, 30, "object_pattern", false),
        p(1, "required_parameter", 32, 45, "identifier", true)], [o("later", 32, 37, 1)],
      "selection_native_shape_mismatch"]
  ];
  for (const [file, source, parameters, bindings, eligibility] of typed) {
    const selector = select(file, source);
    using(fixture({ [file]: source }, [selector]), f => complete(f, [
      site(selector, "unique_named", [c(parameters, [], bindings)], eligibility)
    ]));
  }
  const recovery = "function take({x}, later { return later; }";
  const recovered = select("recovery.js", recovery);
  using(fixture({ "recovery.js": recovery }, [recovered]), f => complete(f, [
    site(recovered, "recovery_quarantined", [c([
      p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)
    ], null, [o("later", 19, 24, 1)])], "not_clean_named")
  ], { "recovery.js": 1 }));
});

test("4: BOM, astral, CRLF, and UTF-8 boundaries preserve bytes", () => {
  const source = "\ufeffconst fox=\"🦊\";\r\nfunction take({x}, later){return later;}";
  const start = bytes(source.slice(0, source.indexOf("function")));
  const selector = select("unicode.ts", source, "FunctionDeclaration",
    [0], [1], start, bytes(source));
  using(fixture({ "unicode.ts": source }, [selector]), f => complete(f, [
    site(selector, "unique_named", [c([
      p(0, "required_parameter", 36, 39, "object_pattern", false),
      p(1, "required_parameter", 41, 46, "identifier", true)
    ], [], [o("later", 41, 46, 1)], "take", "function_declaration", 2, 2)], "eligible")
  ]));
  using(fixture({ "bad.ts": source }, [
    select("bad.ts", source, "FunctionDeclaration", [0], [1], 1, 2)
  ]), f => assert.throws(() => run(f.options()), /invalid selector/));
});

test("5: builder controls and worker-held refusals are exact", () => {
  const source = "\ufefffunction take({x}, later){return later;}";
  const selector = select("a.js", source);
  using(fixture({ "a.js": source }, [selector]), f => {
    const request = builder.buildRequest(f.options());
    const expectedRequest = {
      schema: builder.REQUEST,
      input_manifest_sha256: f.manifestSha256,
      native_binary_sha256: f.options().nativeSha256,
      files: [{ path: "a.js", sha256: f.members[0].sha256, bytes: f.members[0].bytes,
        script_kind: "JavaScript", source, sites: [selector] }]
    };
    assert.equal(JSON.stringify(request), JSON.stringify(expectedRequest));
    assert.doesNotThrow(() => builder.buildRequest(f.options()));
    assert.throws(() => builder.buildRequest({ ...f.options(), manifestSha256: "0".repeat(64) }),
      /manifest SHA mismatch/);
    for (const unsafe of ["/a.js", "../a.js", "a\\b.js"]) {
      f.members[0].path = f.sites[0].path = unsafe;
      f.save();
      assert.throws(() => builder.buildRequest(f.options()), /invalid or duplicate member path/);
    }
    selector.path = "a.js";
  });
  using(fixture({ "a.js": source }, [selector]), f => {
    f.manifest.upstream_repository = "https://example.invalid/public";
    f.save();
    assert.throws(() => builder.buildRequest(f.options()), /frozen manifest SHA mismatch/);
    f.sites[0].compiler_kind = "FunctionDeclaration";
    f.save();
    assert.throws(() => builder.buildRequest(f.options()), /frozen manifest SHA mismatch/);
  });
  using(fixture({ "a.js": source }, [selector]), f => {
    f.members.push({ ...f.members[0] });
    f.save();
    assert.throws(() => builder.buildRequest(f.options()), /invalid or duplicate member path/);
  });
  using(fixture({ "a.js": source }, [selector]), f => {
    writeFileSync(path.join(f.root, "a.js"), "function changed(){}");
    assert.throws(() => builder.buildRequest(f.options()), /pre-read size mismatch/);
  });
  using(fixture({ "a.js": source }, [selector]), f => {
    writeFileSync(path.join(f.root, "a.js"), Buffer.from([0xff]));
    f.members[0].bytes = 1;
    f.members[0].sha256 = sha(Buffer.from([0xff]));
    f.save();
    assert.throws(() => builder.buildRequest(f.options()), /invalid UTF-8/);
  });
  using(fixture({ "dir": "" }, [select("dir", "")]), f => {
    rmSync(path.join(f.root, "dir"));
    mkdirSync(path.join(f.root, "dir"));
    f.members[0].bytes = 0;
    f.members[0].sha256 = sha("");
    f.save();
    assert.throws(() => builder.buildRequest(f.options()), /not a regular file/);
  });
  using(fixture({ "a.js": source }, [selector]), f => {
    f.members[0].bytes += 1;
    f.save();
    assert.throws(() => builder.buildRequest(f.options()), /pre-read size mismatch/);
  });
});

test("5b: CLI rejects missing, duplicate, and unknown flags", () => {
  const source = "function take({x}, later){return later;}";
  using(fixture({ "a.js": source }, [select("a.js", source)]), f => {
    const script = path.join(process.cwd(), "scripts",
      "parameter-slot-characterization", "request.mjs");
    const flags = ["--root", f.root, "--manifest", f.manifestPath, "--manifest-sha256",
      f.manifestSha256, "--native-sha256", f.options().nativeSha256];
    const cases = [
      [flags.slice(2), /missing flag: --root/],
      [[...flags, "--root", f.root], /duplicate flag: --root/],
      [[...flags, "--unknown", "value"], /unknown flag: --unknown/]
    ];
    for (const [args, expected] of cases) {
      const result = spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, expected);
    }
  });
});

test("6 and 7: repeat, shifted, and terminal dispositions are complete", () => {
  const source = "function take({key: value}, later){return later;}";
  const selector = select("a.js", source);
  using(fixture({ "a.js": source }, [selector]), f => {
    const first = JSON.stringify(run(f.options()));
    assert.equal(JSON.stringify(run(f.options())), first);
    writeFileSync(path.join(f.root, "a.js"), `/*p*/${source}`);
    assert.throws(() => builder.buildRequest(f.options()), /pre-read size mismatch/);
  });
  const cases = [
    ["zero.js", "function take({x}, later){return later;}", "unique_named",
      "eligible"],
    ["unnamed.js", "consume(({x}, later) => later);", "unique_unnamed", "not_clean_named",
      "ArrowFunction", 8, 29],
    ["missing.js", "function take({x}, later){return later;}", "missing", "not_clean_named",
      "ArrowFunction"],
    ["recovery.js", "function take({x}, later { return later; }", "recovery_quarantined",
      "not_clean_named"]
  ];
  for (const [file, source, status, eligibility, kind, start, end] of cases) {
    const selector = select(file, source, kind ?? "FunctionDeclaration", [0], [1],
      start ?? 0, end ?? bytes(source));
    using(fixture({ [file]: source }, [selector]), f => {
      const packet = run(f.options());
      const eligible = eligibility === "eligible";
      assert.deepEqual([
        packet.sites[0].status, packet.sites[0].next_proof_eligibility,
        packet.next_action, packet.reason
      ], [status, eligibility, eligible ? "bounded_entry_and_call_proof" : "defer", eligible
        ? "named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof"
        : "no_eligible_native_slot_gap"]);
    });
  }
});

test("9: raw worker refusal matrix, reordering, and JSX mapping stay in the worker", () => {
  const first = "function one({x}, later){return later;}function two({y}, next){return next;}";
  const second = "function jsx({z}, final){return final;}";
  const middle = first.indexOf("function two");
  using(fixture({ "a.js": first, "b.jsx": second }, [
    select("a.js", first, "FunctionDeclaration", [0], [1], 0, middle),
    select("a.js", first, "FunctionDeclaration", [0], [1], middle, bytes(first)),
    select("b.jsx", second)
  ]), f => {
    const valid = builder.buildRequest(f.options());
    const text = value => `${JSON.stringify(value)}\n`;
    const bad = change => {
      const value = structuredClone(valid);
      change(value);
      return text(value);
    };
    const unsafe = (path, value) => bad(request => path(request, value));
    const refusals = [
      ["missing source", bad(request => delete request.files[0].source), /object keys/],
      ["duplicate file", bad(request => request.files[1].path = request.files[0].path),
        /duplicate file/],
      ["mismatched selector", bad(request => request.files[0].sites[0].path = "other.js"),
        /selector/],
      ["nested extra", bad(request => request.files[0].sites[0].extra = true), /object keys/],
      ["trailing value", `${text(valid)}{}`, /invalid request JSON/],
      ["oversized", Buffer.alloc(2 * 1024 * 1024 + 1, 0x20), /input limit/],
      ["unsafe bytes", unsafe((request, value) => request.files[0].bytes = value,
        9_007_199_254_740_992), /integer too large/],
      ["unsafe start", unsafe((request, value) => request.files[0].sites[0].start_byte = value,
        9_007_199_254_740_992), /integer too large/],
      ["unsafe later", unsafe((request, value) => request.files[0].sites[0]
        .later_required_ordinals[0] = value, 9_007_199_254_740_992), /integer too large/]
    ];
    for (const [label, input, expected] of refusals) {
      const result = worker(input);
      assert.notEqual(result.status, 0, label);
      assert.equal(result.stdout, "", label);
      assert.match(result.stderr, expected, label);
    }
    const canonical = worker(text(valid));
    const reordered = structuredClone(valid);
    reordered.files.reverse();
    reordered.files.forEach(file => file.sites.reverse());
    const again = worker(text(reordered));
    assert.equal(canonical.status, 0, canonical.stderr);
    assert.equal(again.stdout, canonical.stdout);
    assert.equal(JSON.parse(canonical.stdout).files.find(file => file.path === "b.jsx").language,
      "JavaScript");
  });
});

test("10: frozen 30-case baseline vectors replay through the worker", () => {
  const plan = path.join(process.cwd(), "docs", "superpowers", "plans",
    "2026-09-19-post319-native-positional-gap");
  const fixtures = JSON.parse(readFileSync(path.join(plan, "BASELINE-FIXTURES.json"), "utf8"));
  const rows = new Map();
  const tuple = /\("([^"\n]+)", (\d+), (\d+)\)/g;
  const tuples = value => [...value.matchAll(tuple)].map(match => [
    match[1], Number(match[2]), Number(match[3])
  ]);
  let current;
  const baseline = readFileSync(path.join(plan, "baseline-output.log"), "utf8");
  const functionRow = /^FN (Some\("([^"\n]+)"\)|None) (\d+) (\d+) slots=(None|Some\((.*)\)) occurrences=(\[.*\])$/; /*
    "^FN (Some\\(\\\\"([^\\"\\n]+)\\\\"\\)|None) (\\d+) (\\d+) " +
    "slots=(None|Some\\((.*)\\)) occurrences=(\\[.*\\])$"
  );
  */
  for (const line of baseline.trim().split("\n")) {
    const header = line.match(/^CASE (\S+) (\S+) errors=(\d+)$/);
    if (header) {
      current = { id: header[1], script_kind: header[2], errors: Number(header[3]), functions: [] };
      rows.set(`${current.id}:${current.script_kind}`, current);
    } else {
      const match = line.match(functionRow);
      assert(match, `unreadable frozen vector: ${line}`);
      current.functions.push({
        name: match[2] ?? null,
        start: Number(match[3]),
        end: Number(match[4]),
        slots: match[5] === "None" ? null : tuples(match[6]),
        bindings: tuples(match[7])
      });
    }
  }
  for (const [script_kind, extension] of [
    ["JavaScript", "js"], ["TypeScript", "ts"], ["Tsx", "tsx"]
  ]) {
    const request = {
      schema: builder.REQUEST,
      input_manifest_sha256: "a".repeat(64),
      native_binary_sha256: nativeSha256,
      files: fixtures.map(fixture => {
        const vector = rows.get(`${fixture.id}:${script_kind}`);
        return {
          path: `${fixture.id}.${extension}`,
          sha256: fixture.sha256,
          bytes: fixture.bytes,
          script_kind,
          source: fixture.source,
          sites: vector.functions.map(row => ({
            path: `${fixture.id}.${extension}`,
            start_byte: row.start,
            end_byte: row.end,
            compiler_kind: fixture.source.slice(row.start, row.end).includes("=>")
              ? "ArrowFunction" : "FunctionDeclaration",
            object_ordinals: [0],
            later_required_ordinals: [1]
          }))
        };
      })
    };
    const result = worker(`${JSON.stringify(request)}\n`);
    assert.equal(result.status, 0, result.stderr);
    const packet = JSON.parse(result.stdout);
    for (const fixture of fixtures) {
      const vector = rows.get(`${fixture.id}:${script_kind}`);
      const sites = packet.sites.filter(site => site.path === `${fixture.id}.${extension}`);
      const file = packet.files.find(file => file.path === `${fixture.id}.${extension}`);
      assert.equal(file.parse_error_count, vector.errors);
      assert.equal(sites.length, vector.functions.length);
      sites.forEach((site, index) => {
        const expected = vector.functions[index];
        const candidate = site.candidates[0];
        assert.equal(site.status, expected.name === null ? "unique_unnamed" : "unique_named");
        assert.deepEqual([site.start_byte, site.end_byte, candidate.name], [
          expected.start, expected.end, expected.name
        ]);
        assert.deepEqual(
          candidate.slots?.map(row => [row.name, row.start_byte, row.end_byte]) ?? null,
          expected.slots);
        assert.deepEqual(candidate.bindings.map(row => [row.name, row.start_byte, row.end_byte]),
          expected.bindings);
      });
    }
  }
});
