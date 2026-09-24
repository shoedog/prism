import assert from "node:assert/strict";
import test from "node:test";
import{
  createHash
} from "node:crypto";
import{
  chmodSync, copyFileSync, existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync,
  realpathSync, rmSync, symlinkSync, writeFileSync
} from "node:fs";
import{
  spawnSync
} from "node:child_process";
import path from "node:path";
import { tmpdir } from "node:os";
const native = process.env.PRISM_NATIVE_PARAMETER_EXAMPLE;
assert(native, "PRISM_NATIVE_PARAMETER_EXAMPLE is required; a missing binary is a hard failure");
const shell = Buffer.from("#!/bin/sh\nexec /bin/sh \"$@\"\n");
const implementation = await import(new URL("./index.mjs", import.meta.url));
const sha = value => createHash("sha256").update(value).digest("hex");
const bytes = value => Buffer.byteLength(value);
const kind = file => file.endsWith(".tsx") ? "Tsx" : file.endsWith(".ts") ? "TypeScript" :
  file.endsWith(".jsx") ? "Jsx" : "JavaScript";
const language = file => kind(file) === "Jsx" ? "JavaScript" : kind(file);
const p = (ordinal, kind, start_byte, end_byte, pattern_kind = kind,
  ordinary_required_identifier = kind === "identifier") => ({
  ordinal, kind, start_byte, end_byte, pattern_kind, ordinary_required_identifier
});
const o = (name, start_byte, end_byte, source_ordinal) => ({
  name, start_byte, end_byte, source_ordinal
});
const c = (parameters, slots, bindings, name = "take", kind = "function_declaration",
  start_line = 1, end_line = 1) => ({
  kind, start_line, end_line, name, parameters, slots, bindings
});
const s = (selector, status, candidates, next_proof_eligibility) => ({
  path: selector.path, start_byte: selector.start_byte, end_byte: selector.end_byte, status,
    candidates, next_proof_eligibility
});
function selector(path, source, compiler_kind = "FunctionDeclaration", object_ordinals = [0],
  later_required_ordinals = [1], start_byte = 0, end_byte = bytes(source)){
  return{
    path, start_byte, end_byte, compiler_kind, object_ordinals, later_required_ordinals
  };
} function fixture(sources, sites, mutate ={
}){
  const root = mkdtempSync(path.join(realpathSync(tmpdir()), "prism-slot-"));
  const members = Object.entries(sources).map(([file, source]) =>{
    const target = path.join(root, file); mkdirSync(path.dirname(target),{
      recursive: true
    }); writeFileSync(target, source); return{
      path: file, sha256: sha(source), bytes: bytes(source), script_kind: kind(file),
        declaration_only: false, test_path: false
    };
  });
  const manifest ={
    schema: "prism.native-parameter-sites/1", upstream_repository: "synthetic://native-slot",
    upstream_commit: "fixture", upstream_tree: "fixture", parent_packet_sha256: "a".repeat(64),
    parent_manifest_sha256: "b".repeat(64), selection: "synthetic exact selectors",
      member_count: members.length,
    source_bytes: members.reduce((sum, member) => sum + member.bytes, 0), site_count: sites.length,
    members, sites, ...mutate
  };
  const manifestPath = path.join(root, "manifest.json");
  const save = () =>{
    const text = `${JSON.stringify(manifest)}\n`;
    writeFileSync(manifestPath, text);
    return sha(text);
  };
  const manifestSha256 = save();
  return{
    root, sources, members, sites, manifest, manifestPath, manifestSha256, save,
      options(out = path.join(root,
      "out.json")){
      return{
        root, manifest: manifestPath, manifestSha256: this.manifestSha256, native,
          nativeSha256: sha(readFileSync(native)),
        out
      };
    }, cleanup(){
      rmSync(root,{
        recursive: true, force: true
      });
    }
  };
} function expected(fixture, sites){
  const counts = Object.fromEntries(["unique_named", "unique_unnamed", "missing", "ambiguous",
    "recovery_quarantined"].map(status => [status,
      sites.filter(site => site.status === status).length]));
  const eligible = sites.some(site => site.next_proof_eligibility === "eligible");
  return{
    schema: implementation.OUTPUT, measurement: "native_parameter_api_characterization",
    authorizes_runtime_edge: false, native_entry_measured: false, callee_resolution_measured: false,
    input_manifest_sha256: fixture.manifestSha256, native_binary_sha256: sha(readFileSync(native)),
    files: [...fixture.members].sort((left, right) => left.path < right.path ? -1 :
      left.path > right.path ? 1 : 0).map(member => ({
      path: member.path, sha256: member.sha256, bytes: member.bytes, language:
        language(member.path),
        parse_error_count: 0
    })), sites, totals:{
      files: fixture.members.length, sites: sites.length, ...counts
    }, next_action: eligible ? "bounded_entry_and_call_proof" : "defer", reason: eligible ?
      "named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof" :
        "no_eligible_native_slot_gap"
  };
} function complete(fixture, sites){
  const actual = implementation.launch(fixture.options());
  assert.deepEqual(actual, expected(fixture, sites));
  assert.equal(readFileSync(fixture.options().out, "utf8"), `${JSON.stringify(actual)}\n`);
} function using(fixture, body){
  try{
    return body(fixture);
  } finally{
    fixture.cleanup();
  }
} function rewrite(fixture){
  fixture.manifestSha256 = fixture.save();
} function adapter(compressOrdinal){
  return{
    schema: implementation.OUTPUT, measurement: "native_parameter_api_characterization",
    authorizes_runtime_edge: false, native_entry_measured: false, callee_resolution_measured: false,
    input_manifest_sha256: "a".repeat(64), native_binary_sha256: "b".repeat(64), files: [{
      path: "alias.js", sha256: "c".repeat(64), bytes: 56, language: "JavaScript",
        parse_error_count: 0
    }], sites: [{
      path: "alias.js", start_byte: 0, end_byte: 56, status: "unique_named", candidates: [c([p(0,
        "identifier", 14, 19), p(1, "object_pattern", 21, 33), p(2, "identifier", 35,
        40)], [o("first", 14, 19, 0)], [o("first", 14, 19, 0), o("later", 35, 40,
          compressOrdinal ? 1 : 2)])],
        next_proof_eligibility: "eligible"
    }], totals:{
      files: 1, sites: 1, unique_named: 1, unique_unnamed: 0, missing: 0, ambiguous: 0,
      recovery_quarantined: 0
    }, next_action: "bounded_entry_and_call_proof", reason:
      "named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof"
  };
} test("RED adapter exposes compressed alias binding ordinal",{
  skip: process.env.PRISM_CAPTURE_RED !== "1"
}, () => assert.deepEqual(adapter(true), adapter(false)));
test("1: raw positions, prefix refusal, and selection-shape controls are complete", () =>{
  const object = "function take({x}, later){return later;}";
    const alias = "function take(first, {key: value}, later){return later;}"; using(fixture({
    "object.js": object, "alias.js": alias
  }, [selector("object.js", object), selector("alias.js", alias, "FunctionDeclaration",
    [1], [2])]), f => complete(f, [s(f.sites[1], "unique_named", [c([p(0, "identifier",
    14, 19), p(1, "object_pattern", 21, 33), p(2, "identifier", 35, 40)], [o("first",
    14, 19, 0)], [o("first", 14, 19, 0), o("later", 35, 40, 2)])], "eligible"), s(f.sites[0],
    "unique_named", [c([p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [],
    [o("later", 19, 24, 1)])], "eligible")])); for (const [file, source, labels, candidate,
    eligibility] of [ ["plain.js", "function take(first, later){return later;}", [[0],
    [1]], c([p(0, "identifier", 14, 19), p(1, "identifier", 21, 26)], [o("first", 14,
    19, 0), o("later", 21, 26, 1)], [o("first", 14, 19, 0), o("later", 21, 26, 1)]),
      "selection_native_shape_mismatch"],
    ["array.js", "function take([x], later){return later;}", [[0], [1]], c([p(0, "array_pattern",
    14, 17), p(1, "identifier", 19, 24)], [], [o("later", 19, 24, 1)]),
      "selection_native_shape_mismatch"],
    ["default.js", "function take({x}, later = 1){return later;}", [[0], [1]], c([p(0,
    "object_pattern", 14, 17), p(1, "assignment_pattern", 19, 28, "assignment_pattern",
    false)], [], []), "selection_native_shape_mismatch"], ["range.js", object, [[9], [1]],
    c([p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [], [o("later", 19,
    24, 1)]), "selection_native_shape_mismatch"], ["gapless.js",
      "function take({x}, later){return 0;}",
    [[0], [1]], c([p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [], [o("later",
    19, 24, 1)]), "eligible"],]) using(fixture({
    [file]: source
  }, [selector(file, source, "FunctionDeclaration", ...labels)]), f => complete(f, [s(f.sites[0],
    "unique_named", [candidate], eligibility)]));
});
test("2: exact arrow identity, inferred names, nested scope, and near spans", () =>{
  const assigned = "const take = ({x}, later) => later;",
    wrapped = "const take = memo(({x}, later) => later);",
    unnamed = "consume(({x}, later) => later);"; for (const [file, source, start, end,
    name, parameters, bindings, status, eligibility] of [["assigned.js", assigned, 13,
    34, "take", [p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [o("later",
    19, 24, 1)], "unique_named", "eligible"], ["wrapped.js", wrapped, 18, 39, "take",
    [p(0, "object_pattern", 19, 22), p(1, "identifier", 24, 29)], [o("later", 24, 29,
    1)], "unique_named", "eligible"], ["unnamed.js", unnamed, 8, 29, null, [p(0, "object_pattern",
    9, 12), p(1, "identifier", 14, 19)], [o("later", 14, 19, 1)], "unique_unnamed",
      "not_clean_named"]]) using(fixture({
    [file]: source
  }, [selector(file, source, "ArrowFunction", [0], [1], start, end)]), f => complete(f,
    [s(f.sites[0], status, [c(parameters, [], bindings, name, "arrow_function")],
      eligibility)]));
        const shadow =
          "function take({x}, later){function nested(later){return later;}return later;}";
          const outer = selector("shadow.js",
    shadow), inner = selector("shadow.js", shadow, "FunctionDeclaration", [0], [1], 26,
    63); using(fixture({
    "shadow.js": shadow
  }, [outer, inner]), f => complete(f, [s(outer, "unique_named", [c([p(0, "object_pattern",
    14, 17), p(1, "identifier", 19, 24)], [], [o("later", 19, 24, 1)])], "eligible"),
    s(inner, "unique_named", [c([p(0, "identifier", 42, 47)], [o("later", 42, 47, 0)],
    [o("later", 42, 47, 0)], "nested")], "selection_native_shape_mismatch")])); using(fixture({
    "near.js": assigned
  }, [selector("near.js", assigned, "ArrowFunction", [0], [1], 14, 34)]), f => complete(f,
    [s(f.sites[0], "missing", [], "not_clean_named")]));
});
test("3: refusal variants retain complete native JS/TS/TSX records", () =>{
  const duplicate = "function take({x}, x){return x;}"; for (const [file, params,
    bindings] of [["duplicate.js",
    [p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 20)], [o("x", 19, 20, 1)]],
    ["duplicate.ts", [p(0, "required_parameter", 14, 17, "object_pattern", false), p(1,
    "required_parameter", 19, 20, "identifier", true)], []], ["duplicate.tsx", [p(0,
      "required_parameter",
    14, 17, "object_pattern", false), p(1, "required_parameter", 19, 20, "identifier",
    true)], []]]) using(fixture({
    [file]: duplicate
  }, [selector(file, duplicate)]), f => complete(f, [s(f.sites[0], "unique_named", [c(params,
    null, bindings)], "native_slot_authority_unavailable")])); const variants = [ ["rest.js",
    "function take(...items){return items;}", [p(0, "rest_pattern", 14, 22, "rest_pattern",
    false)], []], ["nested.js", "function take({x = init()}, later){return later;}", [p(0,
    "object_pattern", 14, 26), p(1, "identifier", 28, 33)], [o("later", 28, 33, 1)]],
    ["escaped.js", "function take({\\u0078}, later){return later;}", [p(0, "object_pattern",
    14, 22), p(1, "identifier", 24, 29)], [o("later", 24, 29, 1)]], ["recovery.js",
      "function take({x}, later { return later; }",
    [p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [o("later", 19, 24,
    1)]],]; for (const [file, source, params, bindings] of variants){
    const recovered = file === "recovery.js"; using(fixture({
      [file]: source
    }, [selector(file, source)]), f =>{
      const record = expected(f, [s(f.sites[0], recovered ? "recovery_quarantined" : "unique_named",
        [c(params, recovered ? null : [], bindings)], recovered ? "not_clean_named" :
          file === "rest.js" ? "selection_native_shape_mismatch" : "eligible")]);
            if (recovered) record.files[0].parse_error_count = 1;
              const actual = implementation.launch(f.options()); assert.deepEqual(actual,
        record);
    });
  }
});
test("3b: typed required parameter shapes retain complete native records", () =>{
  const source = parameters => `function take(${parameters}){return later;}`;
  const required = "required_parameter", optional = "optional_parameter";
  const cases = [
    ["typed.ts", "{x}: {x:number}, later: string", 29, 31, 44, true, true, false, true],
    ["typed.tsx", "{x}: {x:number}, later: string", 29, 31, 44, true, true, false, true],
    ["initialized.ts", "{x}, later: string = 'x'", 17, 19, 38, false, false, false, false],
    ["modifier.ts", "{x}: {x:number}, public later: string", 29, 31, 51, false, false,
      false, false],
    ["optional-object.ts", "{x}?: {x:number}, later: string", 30, 32, 45, true, true, true, false],
    ["comment.tsx", "{x}: {x:number}, /*a*/ later /*ok*/: string", 29, 37, 57, true, true,
      false, true]];
  for (const [file, raw, firstEnd, start, end, ordinary, bound, isOptional, eligible] of cases){
    const text = source(raw), first = p(0, isOptional ? optional : required, 14, firstEnd,
      "object_pattern", false), later = p(1, required, start, end, "identifier", ordinary);
    const bindings = bound ? [o("later", start, start + 5, 1)] : [];
    const eligibility = eligible ? "eligible" : "selection_native_shape_mismatch";
    using(fixture({[file]: text}, [selector(file, text)]), f => complete(f,
      [s(f.sites[0], "unique_named", [c([first, later], [], bindings)], eligibility)]));
  }
});
test("4: BOM, astral, CRLF, and inside-code-point selectors preserve bytes", () =>{
  const source = "\ufeffconst fox=\"🦊\";\r\nfunction take({x}, later){return later;}";
    const start = bytes(source.slice(0,
    source.indexOf("function"))); const select = selector("unicode.ts", source,
      "FunctionDeclaration",
    [0], [1], start, bytes(source)); using(fixture({
    "unicode.ts": source
  }, [select]), f => complete(f, [s(select, "unique_named", [c([p(0, "required_parameter",
    36, 39, "object_pattern", false), p(1, "required_parameter", 41, 46, "identifier",
    true)], [], [o("later", 41, 46, 1)], "take", "function_declaration", 2, 2)],
      "eligible")])); using(fixture({
    "bad.ts": source
  }, [selector("bad.ts", source, "FunctionDeclaration", [0], [1], 1, 2)]),
    f => assert.throws(() => implementation.prepare(f.options()),
    /selector boundary/));
});
test("5: manifest, request, child, cap, and publication custody fail closed", () =>{
  const source = "function take({x}, later){return later;}"; using(fixture({
    "a.js": source
  }, [selector("a.js", source)]), f =>{
    assert.throws(() => implementation.prepare({
      ...f.options(), manifestSha256: "0".repeat(64)
    }), /SHA/); f.manifest.members.push(f.manifest.members[0]); f.manifest.member_count += 1;
      rewrite(f); assert.throws(() => implementation.prepare(f.options()),
      /duplicate member/); f.manifest.members.pop(); f.manifest.member_count -= 1;
        f.manifest.sites.push({
      ...f.sites[0]
    }); f.manifest.site_count += 1; rewrite(f);
      assert.throws(() => implementation.prepare(f.options()),
      /duplicate selector/); f.manifest.sites.pop(); f.manifest.site_count -= 1; rewrite(f);
        assert.throws(() => implementation.prepare(f.options(),
{
      files: 0
    }), /limit/); assert.throws(() => implementation.prepare(f.options(),{
      sites: 0
    }), /limit/); assert.throws(() => implementation.prepare(f.options(),{
      sourceBytes: 1
    }), /limit/); assert.throws(() => implementation.prepare(f.options(),{
      inputBytes: 1
    }), /input/); for (const member of ["../outside.js", "/outside.js", "a\\b.js"]){
      f.manifest.members[0].path = f.manifest.sites[0].path = member; rewrite(f);
      assert.throws(() => implementation.prepare(f.options()), /invalid or duplicate member/);
    }
    f.manifest.members[0].path = f.manifest.sites[0].path = "a.js"; rewrite(f);
    f.manifest.extra = true; rewrite(f);
    assert.throws(() => implementation.prepare(f.options()), /invalid manifest/);
    delete f.manifest.extra;
    f.manifest.members.push({ ...f.manifest.members[0], path: "unused.js" });
    f.manifest.member_count++; f.manifest.source_bytes += f.manifest.members[0].bytes; rewrite(f);
    assert.throws(() => implementation.prepare(f.options()), /unreferenced member/);
    f.manifest.sites[0].path = "missing.js"; rewrite(f);
    assert.throws(() => implementation.prepare(f.options()), /invalid or duplicate selector/);
    f.manifest.sites[0].path = "a.js"; f.manifest.members.pop(); f.manifest.member_count--;
    f.manifest.source_bytes -= f.manifest.members[0].bytes; rewrite(f);
    assert.throws(() => implementation.prepare({
      ...f.options(), nativeSha256: "0".repeat(64)
    }), /native binary SHA mismatch/);
    const ready = implementation.prepare(f.options());
    writeFileSync(path.join(f.root,
      "a.js"), "function changed(){}"); assert.equal(ready.request.files[0].source, source);
        assert.doesNotThrow(() => implementation.runNative(ready.native,
      ready.request, ready.cap)); assert.throws(() => implementation.launch(f.options()),
      /identity/); const bad = spawnSync(native, [],{
      input: "{\n", encoding: "utf8"
    }); assert.notEqual(bad.status, 0); const extra = structuredClone(ready.request);
      extra.extra = true; assert.notEqual(spawnSync(native,
      [],{
      input: `${JSON.stringify(extra)}\n`, encoding: "utf8"
    }).status, 0); ready.request.native_binary_sha256 = sha(shell);
    assert.throws(() => implementation.runNative(shell, ready.request,
{
      ...implementation.LIMITS, wallMs: 5
    }, ["-c", "sleep 1"]), /timeout/); assert.throws(() => implementation.runNative(shell,
      ready.request,{
      ...implementation.LIMITS, outputBytes: 1
    }, ["-c", "printf xx"]), /worker failed|output|timeout/);
      assert.throws(() => implementation.runNative(shell,
      ready.request, implementation.LIMITS, ["-c", "printf '{\"schema\":\"bad\"}\\n'"]),
      /output/); writeFileSync(path.join(f.root, "a.js"), source); const out = path.join(f.root,
      "foreign.json"); writeFileSync(out, "foreign"); assert.throws(() => implementation.launch({
      ...f.options(out), nativeSha256: sha(readFileSync(native))
    }), /already exists/); assert.equal(readFileSync(out, "utf8"), "foreign");
  }); using(fixture({
    "a.tsx": source
  }, [selector("a.tsx", source)]), f =>{
    f.manifest.members[0].script_kind = "TypeScript"; rewrite(f);
      assert.throws(() => implementation.prepare(f.options()),
      /mismatch/);
  }); using(fixture({
    "link/a.js": source
  }, [selector("link/a.js", source)]), f =>{
    const link = path.join(f.root, "link"), real = path.join(f.root, "real"); rmSync(link,
{
      recursive: true, force: true
    }); mkdirSync(real); writeFileSync(path.join(real, "a.js"), source); symlinkSync(real,
      link); assert.throws(() => implementation.prepare(f.options()), /symlink/);
    f.manifest.members[0].path = f.manifest.sites[0].path = "a.js";
    rewrite(f);
    const parent = path.join(f.root, "real-parent"), nested = path.join(parent, "nested-root");
    mkdirSync(nested, {recursive: true}); writeFileSync(path.join(nested, "a.js"), source);
    const linked = path.join(f.root, "linked-parent"); symlinkSync(parent, linked);
    assert.throws(() => implementation.prepare({
      ...f.options(), root: path.join(linked, "nested-root")
    }), /symlink/);
  });
});
test("6: cold/repeat, inert shifting, and meaningful alias mutation are complete", () =>{
  const source = "function take({key: value}, later){return later;}";
    const inert = `/*p*/${source}`,
    plain = "function take(value, later){return later;}"; const original = selector("a.js",
    source); using(fixture({
    "a.js": source
  }, [original]), f =>{
    const ready = implementation.prepare(f.options()), cold = implementation.runNative(ready.native,
      ready.request, ready.cap); assert.equal(implementation.runNative(ready.native, ready.request,
      ready.cap), cold); writeFileSync(path.join(f.root, "a.js"), inert);
        assert.throws(() => implementation.prepare(f.options()),
      /identity/); writeFileSync(path.join(f.root, "a.js"), source);
        assert.equal(implementation.runNative(ready.native,
      ready.request, ready.cap), cold); complete(f, [s(original, "unique_named", [c([p(0,
      "object_pattern", 14, 26), p(1, "identifier", 28, 33)], [], [o("later", 28, 33,
      1)])], "eligible")]);
  }); const shifted = selector("a.js", inert, "FunctionDeclaration", [0], [1], 5,
    bytes(inert)); using(fixture({
    "a.js": inert
  }, [shifted]), f => complete(f, [s(shifted, "unique_named", [c([p(0, "object_pattern",
    19, 31), p(1, "identifier", 33, 38)], [], [o("later", 33, 38, 1)])], "eligible")]));
      const changed = selector("a.js",
    plain); using(fixture({
    "a.js": plain
  }, [changed]), f => complete(f, [s(changed, "unique_named", [c([p(0, "identifier", 14,
    19), p(1, "identifier", 21, 26)], [o("value", 14, 19, 0), o("later", 21, 26, 1)],
    [o("value", 14, 19, 0), o("later", 21, 26, 1)])], "selection_native_shape_mismatch")]));
});
test("7: complete terminal packets include zero-gap, unnamed, missing, ambiguous, and recovered",
  () =>{
  const zero = "function take({x}, later){return later;}"; using(fixture({
    "zero.js": zero
  }, [selector("zero.js", zero)]), f =>{
    const ready = implementation.prepare(f.options()), row = s(f.sites[0], "unique_named", [c([
      p(0, "object_pattern", 14, 17), p(1, "identifier", 19, 24)], [o("later", 19, 24, 1)],
      [o("later", 19, 24, 1)])], "no_selected_suffix_binding_gap"), packet = expected(f, [row]);
    ready.request.native_binary_sha256 = packet.native_binary_sha256 = sha(shell);
    const command = (value, pretty = false) => ["-c",
      `printf '%s\\n' '${JSON.stringify(value, null, pretty ? 2 : undefined)}'`];
    const badKind = structuredClone(packet);
    badKind.sites[0].candidates[0].kind = null;
    const stages = () => readdirSync(tmpdir()).filter(name => name.startsWith("prism-native-"));
    const before = stages();
    assert.throws(() => implementation.runNative(shell, ready.request, ready.cap, command(badKind)),
      /candidate/);
    assert.deepEqual(stages(), before);
    assert.deepEqual(JSON.parse(implementation.runNative(shell, ready.request, ready.cap,
      command(packet))), packet);
    assert.throws(() => implementation.runNative(shell, ready.request, ready.cap,
      command(packet, true)), /noncanonical worker output/);
  }); const unnamed = "consume(({x}, later) => later);"; using(fixture({
    "unnamed.js": unnamed
  }, [selector("unnamed.js", unnamed, "ArrowFunction", [0], [1], 8, 29)]), f => complete(f,
    [s(f.sites[0], "unique_unnamed", [c([p(0, "object_pattern", 9, 12), p(1, "identifier",
    14, 19)], [], [o("later", 14, 19, 1)], null, "arrow_function")], "not_clean_named")]));
      const missing = "function take({x}, later){return later;}"; using(fixture({
    "missing.js": missing
  }, [selector("missing.js", missing, "ArrowFunction")]), f => complete(f, [s(f.sites[0],
    "missing", [], "not_clean_named")]));
});

test("8: authenticated binary bytes, path controls, limits, and wire order stay bounded", () =>{
  const source = "function take({x}, later){return later;}";
  using(fixture({
    "a.js": source
  }, [selector("a.js", source)]), f =>{
    const copied = path.join(f.root, "worker-copy"), marker = path.join(f.root, "replacement-ran");
    copyFileSync(native, copied); chmodSync(copied, 0o700);
    const ready = implementation.prepare({
      ...f.options(), native: copied, nativeSha256: sha(readFileSync(copied))
    });
    writeFileSync(copied, `#!/bin/sh\nprintf swapped > ${JSON.stringify(marker)}\nexit 23\n`);
    chmodSync(copied, 0o700);
    assert.doesNotThrow(() => implementation.runNative(ready.native, ready.request, ready.cap));
    assert.equal(existsSync(marker), false, "the replacement original must not execute");
    readFileSync(copied).copy(ready.native);
    assert.throws(() => implementation.runNative(ready.native, ready.request, ready.cap), /SHA/);
    assert.equal(existsSync(marker), false, "mutated authenticated bytes must not execute");
    assert.throws(() => implementation.prepare(f.options(), {wallMs: 0}), /limits/);
    for (const wallMs of [0, implementation.LIMITS.wallMs + 1])
      assert.throws(() => implementation.runNative(ready.native, ready.request,
        { ...implementation.LIMITS, wallMs }), /invalid limits/);
    for (const [limit, error] of [["files", /count/], ["sites", /count/],
      ["sourceBytes", /source/], ["inputBytes", /input/]]){
      assert.throws(() => implementation.prepare(f.options(), {[limit]: 0}), error);
    }
    const outputZero = implementation.prepare(f.options(), {outputBytes: 0});
    assert.throws(() => implementation.runNative(outputZero.native, outputZero.request,
      outputZero.cap), /timeout|output/);
    for (const control of ["\u007f", "\u0085"]){
      f.manifest.members[0].path = `bad${control}.js`;
      f.manifest.sites[0].path = `bad${control}.js`;
      rewrite(f);
      assert.throws(() => implementation.prepare(f.options()), /invalid or duplicate member/);
    }
    ready.request.native_binary_sha256 = sha(shell);
    assert.throws(() => implementation.runNative(shell, ready.request, ready.cap,
      ["-c", "kill -SEGV $$"]), /worker signal SIGSEGV/);
  }); using(fixture({
    "wire.js": source
  }, [selector("wire.js", source)]), f =>{
    const site = f.manifest.sites[0];
    f.manifest.sites[0] = {
      compiler_kind: site.compiler_kind, later_required_ordinals: site.later_required_ordinals,
      path: site.path, end_byte: site.end_byte, object_ordinals: site.object_ordinals,
      start_byte: site.start_byte
    };
    rewrite(f);
    const ready = implementation.prepare(f.options()), nativeSha256 = f.options().nativeSha256;
    const expectedRequest = {
      schema: implementation.REQUEST, input_manifest_sha256: f.manifestSha256,
      native_binary_sha256: nativeSha256,
      files: [{
        path: "wire.js", sha256: sha(source), bytes: bytes(source), script_kind: "JavaScript",
        source, sites: [selector("wire.js", source)]
      }]
    };
    assert.equal(JSON.stringify(ready.request), JSON.stringify(expectedRequest));
  });
});

test("9: raw worker refusal matrix, canonical reordering, and JSX mapping are complete", () =>{
  const first = "function one({x}, later){return later;}function two({y}, next){return next;}",
    second = "function jsx({z}, final){return final;}",
    secondStart = first.indexOf("function two");
  const f = fixture({
    "a.js": first, "b.jsx": second
  }, [selector("a.js", first, "FunctionDeclaration", [0], [1], 0, secondStart),
    selector("a.js", first, "FunctionDeclaration", [0], [1], secondStart, bytes(first)),
    selector("b.jsx", second)]);
  using(f, fixture =>{
    const ready = implementation.prepare(fixture.options()), valid = ready.request;
    const text = value => `${JSON.stringify(value)}\n`, bad = change =>{
      const value = structuredClone(valid); change(value); return text(value);
    };
    const unsafe = (path, value) => bad(request => path(request, value));
    const refusals = [
      ["missing source", bad(request => delete request.files[0].source), /object keys/],
      ["duplicate file", bad(request => request.files[1].path = request.files[0].path),
        /duplicate file/],
      ["mismatched selector", bad(request => request.files[0].sites[0].path = "other.js"),
        /selector/],
      ["nested extra", bad(request => request.files[0].sites[0].extra = true), /object keys/],
      ["dotfile", bad(request => request.files[0].path = ".js"), /mismatch/],
      ["no extension", bad(request => request.files[0].path = "no-extension"), /mismatch/],
      ["trailing value", `${text(valid)}{}`, /invalid request JSON/],
      ["oversized", Buffer.alloc(implementation.LIMITS.inputBytes + 1, 0x20), /input limit/],
      ["unsafe bytes", unsafe((request, value) => request.files[0].bytes = value,
        9_007_199_254_740_992), /integer too large/],
      ["unsafe start", unsafe((request, value) => request.files[0].sites[0].start_byte = value,
        9_007_199_254_740_992), /integer too large/],
      ["unsafe end", unsafe((request, value) => request.files[0].sites[0].end_byte = value,
        9_007_199_254_740_992), /integer too large/],
      ["unsafe object ordinal", unsafe((request, value) =>
        request.files[0].sites[0].object_ordinals[0] = value, 9_007_199_254_740_992),
      /integer too large/],
      ["unsafe later ordinal", unsafe((request, value) =>
        request.files[0].sites[0].later_required_ordinals[0] = value, 9_007_199_254_740_992),
      /integer too large/]
    ];
    for (const [label, input, expected] of refusals){
      const result = spawnSync(native, [], {input, encoding: "utf8"});
      assert.notEqual(result.status, 0, label);
      assert.equal(result.stdout, "", `${label} must not emit a packet`);
      assert.match(result.stderr, expected, label);
    }
    const canonical = spawnSync(native, [], {input: text(valid), encoding: "utf8"});
    const reordered = structuredClone(valid);
    reordered.files.reverse(); reordered.files.forEach(file => file.sites.reverse());
    const again = spawnSync(native, [], {input: text(reordered), encoding: "utf8"});
    assert.equal(canonical.status, 0, canonical.stderr);
    assert.equal(again.status, 0, again.stderr);
    assert.equal(again.stdout, canonical.stdout);
    assert.equal(JSON.parse(canonical.stdout).files.find(file => file.path === "b.jsx").language,
      "JavaScript");
  });
});

test("10: frozen 30-case baseline vectors replay from the planning packet", () =>{
  const plan = path.join(process.cwd(), "docs", "superpowers", "plans",
    "2026-09-19-post319-native-positional-gap");
  const fixtures = JSON.parse(readFileSync(path.join(plan, "BASELINE-FIXTURES.json"), "utf8"));
  const rows = new Map(), tuplePattern = /\("([^"\n]+)", (\d+), (\d+)\)/g;
  const tuples = value => [...value.matchAll(tuplePattern)].map(match =>
    [match[1], Number(match[2]), Number(match[3])]);
  let current;
  const baseline = readFileSync(path.join(plan, "baseline-output.log"), "utf8");
  const functionPattern = new RegExp(
    "^FN (Some\\(\\\"([^\"\\n]+)\\\"\\)|None) (\\d+) (\\d+) " +
    "slots=(None|Some\\((.*)\\)) occurrences=(\\[.*\\])$"
  );
  for (const line of baseline.trim().split("\n")){
    const header = line.match(/^CASE (\S+) (\S+) errors=(\d+)$/);
    if (header){
      current = {id: header[1], script_kind: header[2], errors: Number(header[3]), functions: []};
      rows.set(`${current.id}:${current.script_kind}`, current);
      continue;
    }
    const functionRow = line.match(functionPattern);
    assert(functionRow, `unreadable frozen vector: ${line}`);
    current.functions.push({
      name: functionRow[2] ?? null, start: Number(functionRow[3]), end: Number(functionRow[4]),
      slots: functionRow[5] === "None" ? null : tuples(functionRow[6]),
      bindings: tuples(functionRow[7])
    });
  }
  const dialects = [["JavaScript", "js"], ["TypeScript", "ts"], ["Tsx", "tsx"]];
  for (const [script_kind, extension] of dialects){
    const request = {
      schema: implementation.REQUEST, input_manifest_sha256: "a".repeat(64),
      native_binary_sha256: sha(readFileSync(native)), files: fixtures.map(fixture => {
        const vector = rows.get(`${fixture.id}:${script_kind}`);
        return {
          path: `${fixture.id}.${extension}`, sha256: fixture.sha256, bytes: fixture.bytes,
          script_kind, source: fixture.source, sites: vector.functions.map(row => ({
            path: `${fixture.id}.${extension}`, start_byte: row.start, end_byte: row.end,
            compiler_kind: fixture.source.slice(row.start, row.end).includes("=>") ?
              "ArrowFunction" : "FunctionDeclaration", object_ordinals: [0],
            later_required_ordinals: [1]
          }))
        };
      })
    };
    const result = spawnSync(native, [], {input: `${JSON.stringify(request)}\n`, encoding: "utf8"});
    assert.equal(result.status, 0, result.stderr);
    const packet = JSON.parse(result.stdout);
    for (const fixture of fixtures){
      const vector = rows.get(`${fixture.id}:${script_kind}`), actual = packet.sites.filter(site =>
        site.path === `${fixture.id}.${extension}`);
      const file = packet.files.find(file => file.path === `${fixture.id}.${extension}`);
      assert.equal(file.parse_error_count, vector.errors);
      assert.equal(actual.length, vector.functions.length);
      actual.forEach((site, index) =>{
        const expected = vector.functions[index], candidate = site.candidates[0];
        assert.equal(site.status, expected.name === null ? "unique_unnamed" : "unique_named");
        assert.deepEqual([site.start_byte, site.end_byte, candidate.name],
          [expected.start, expected.end, expected.name]);
        const slots = candidate.slots?.map(row => [row.name, row.start_byte, row.end_byte]) ?? null;
        assert.deepEqual(slots, expected.slots);
        assert.deepEqual(candidate.bindings.map(row => [row.name, row.start_byte, row.end_byte]),
          expected.bindings);
      });
    }
  }
});
