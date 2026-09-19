// Opt-in syntax-frequency research tool. It does not authorize Prism behavior.
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import {
  existsSync,
  linkSync,
  lstatSync,
  readFileSync,
  readdirSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const SCHEMA = "prism.parameter-syntax-frequency/1";
export const COMPILER_SHA = "3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675";
export const LIMITS = Object.freeze({
  files: 512,
  sourceBytes: 8 * 1024 * 1024,
  rows: 200_000,
  outputBytes: 128 * 1024 * 1024,
  wallMs: 15 * 60 * 1000,
});
const SOURCE_SUFFIXES = [".js", ".jsx", ".ts", ".tsx"];
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
const ownPath = fileURLToPath(import.meta.url);

function mergedLimits(overrides = {}) {
  if (!overrides || typeof overrides !== "object" || Array.isArray(overrides)) throw Error("invalid limits");
  if (Object.keys(overrides).some(key => !Object.hasOwn(LIMITS, key))) throw Error("invalid limits");
  const limits = { ...LIMITS, ...overrides };
  for (const [key, value] of Object.entries(limits)) {
    if (!Number.isSafeInteger(value) || value < 0 || value > LIMITS[key]) throw Error("invalid limit");
  }
  return limits;
}

function relative(value) {
  return typeof value === "string" && value.length > 0 && value.length <= 4096
    && !value.startsWith("/") && !value.includes("\\") && !/[\0-\x1f]/.test(value)
    && value.split("/").every(part => part && part !== "." && part !== "..");
}

function sourceExtension(file) {
  if (file.endsWith(".d.ts")) return ".d.ts";
  return path.posix.extname(file);
}

function decode(bytes, file) {
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch {
    throw Error(`invalid UTF-8: ${file}`);
  }
}

function checkedPath(root, relativePath) {
  let current = root;
  const rootStat = lstatSync(current);
  if (!rootStat.isDirectory() || rootStat.isSymbolicLink()) throw Error("source root is not a regular directory");
  for (const part of relativePath.split("/")) {
    current = path.join(current, part);
    let stat;
    try { stat = lstatSync(current); } catch { throw Error(`missing member: ${relativePath}`); }
    if (stat.isSymbolicLink()) throw Error(`symlink in population: ${relativePath}`);
  }
  return current;
}

function discover(root, relativeRoot, output = []) {
  const directory = checkedPath(root, relativeRoot);
  for (const entry of readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
    const relativePath = `${relativeRoot}/${entry.name}`;
    const absolute = path.join(root, ...relativePath.split("/"));
    const stat = lstatSync(absolute);
    if (stat.isSymbolicLink()) throw Error(`symlink in population: ${relativePath}`);
    if (stat.isDirectory()) discover(root, relativePath, output);
    else if (!stat.isFile()) throw Error(`unsupported population entry: ${relativePath}`);
    else if (SOURCE_SUFFIXES.some(suffix => relativePath.endsWith(suffix))) output.push(relativePath);
  }
  return output;
}

function validateManifest(options, limits) {
  const root = path.resolve(options.root);
  const manifestBytes = readFileSync(options.manifest);
  const manifestSha = sha(manifestBytes);
  if (manifestSha !== options.manifestSha256) throw Error(`manifest SHA mismatch: ${manifestSha}`);
  const manifest = JSON.parse(decode(manifestBytes, "manifest"));
  for (const field of ["repository", "commit", "tree", "population_root", "selection", "test_path_rule"]) {
    if (typeof manifest[field] !== "string" || !manifest[field]) throw Error(`invalid manifest ${field}`);
  }
  if (manifest.schema !== "prism.p2-source-manifest.v1") throw Error("unsupported manifest schema");
  if (!relative(manifest.population_root)) throw Error("unsafe population_root");
  if (!Array.isArray(manifest.members) || manifest.members.length !== manifest.member_count) throw Error("member count mismatch");
  if (manifest.members.length > limits.files) throw Error("file limit exceeded");
  const expectedPaths = new Set();
  const files = [];
  let sourceBytes = 0;
  let declarations = 0;
  let tests = 0;
  for (const member of manifest.members) {
    if (!member || !relative(member.path) || !member.path.startsWith(`${manifest.population_root}/`)) throw Error("unsafe member path");
    if (expectedPaths.has(member.path)) throw Error(`duplicate member: ${member.path}`);
    expectedPaths.add(member.path);
    if (![...SOURCE_SUFFIXES, ".d.ts"].includes(sourceExtension(member.path))) throw Error("unsupported member extension");
    if (member.extension !== sourceExtension(member.path)) throw Error("member extension mismatch");
    if (!Number.isSafeInteger(member.bytes) || member.bytes < 0 || !/^[a-f0-9]{64}$/.test(member.sha256)) throw Error("invalid member identity");
    if (typeof member.declaration_only !== "boolean" || typeof member.test_path !== "boolean") throw Error("invalid member stratum");
    const absolute = checkedPath(root, member.path);
    const stat = lstatSync(absolute);
    if (!stat.isFile() || stat.isSymbolicLink()) throw Error(`member is not a regular file: ${member.path}`);
    const bytes = readFileSync(absolute);
    if (bytes.length !== member.bytes || sha(bytes) !== member.sha256) throw Error(`member identity mismatch: ${member.path}`);
    const text = decode(bytes, member.path);
    sourceBytes += bytes.length;
    if (sourceBytes > limits.sourceBytes) throw Error("source byte limit exceeded");
    declarations += Number(member.declaration_only);
    tests += Number(member.test_path);
    files.push({ member, text });
  }
  if (sourceBytes !== manifest.source_bytes || declarations !== manifest.declaration_only_count || tests !== manifest.test_path_count) throw Error("manifest aggregate mismatch");
  const discovered = discover(root, manifest.population_root);
  if (discovered.length !== expectedPaths.size || discovered.some(file => !expectedPaths.has(file))) throw Error("manifest population differs");
  files.sort((a, b) => a.member.path < b.member.path ? -1 : a.member.path > b.member.path ? 1 : 0);
  return { manifest, manifestSha, files, sourceBytes };
}

export function utf8Map(text) {
  const map = Array(text.length + 1).fill(null);
  let bytes = 0;
  map[0] = 0;
  for (let index = 0; index < text.length;) {
    const codePoint = text.codePointAt(index);
    const units = codePoint > 0xffff ? 2 : 1;
    bytes += Buffer.byteLength(String.fromCodePoint(codePoint));
    index += units;
    map[index] = bytes;
  }
  return map;
}

function byte(map, offset, label) {
  if (!Number.isSafeInteger(offset) || offset < 0 || offset >= map.length || map[offset] === null) throw Error(`unmappable UTF-16 boundary: ${label}`);
  return map[offset];
}

function scriptKind(ts, file) {
  if (file.endsWith(".tsx")) return [ts.ScriptKind.TSX, "Tsx"];
  if (file.endsWith(".jsx")) return [ts.ScriptKind.JSX, "Jsx"];
  if (file.endsWith(".js")) return [ts.ScriptKind.JS, "JavaScript"];
  return [ts.ScriptKind.TS, "TypeScript"];
}

function bodyCandidate(ts, node) {
  return ts.isFunctionDeclaration(node) || ts.isFunctionExpression(node) || ts.isArrowFunction(node)
    || ts.isMethodDeclaration(node) || ts.isConstructorDeclaration(node)
    || ts.isGetAccessorDeclaration(node) || ts.isSetAccessorDeclaration(node);
}

function explicitBodyless(ts, node) {
  return ts.isMethodSignature(node) || ts.isCallSignatureDeclaration(node)
    || ts.isConstructSignatureDeclaration(node) || ts.isFunctionTypeNode(node)
    || ts.isConstructorTypeNode(node);
}

function nestedBindingFlags(ts, root) {
  let nestedPattern = false;
  let nestedDefault = false;
  function visit(node) {
    if (node !== root && (ts.isObjectBindingPattern(node) || ts.isArrayBindingPattern(node))) nestedPattern = true;
    if (node !== root && ts.isBindingElement(node) && node.initializer) nestedDefault = true;
    ts.forEachChild(node, visit);
  }
  visit(root);
  return [nestedPattern, nestedDefault];
}

function parameterRow(ts, sourceFile, map, parameter, ordinal) {
  const name = parameter.name;
  const pattern = ts.isIdentifier(name) ? "identifier"
    : ts.isObjectBindingPattern(name) ? "object"
      : ts.isArrayBindingPattern(name) ? "array" : "other";
  const [nestedPattern, nestedDefault] = nestedBindingFlags(ts, name);
  return {
    ordinal,
    token_start: byte(map, parameter.getStart(sourceFile, false), "parameter start"),
    end: byte(map, parameter.end, "parameter end"),
    binding_start: byte(map, name.getStart(sourceFile, false), "binding start"),
    binding_end: byte(map, name.end, "binding end"),
    pattern,
    optional: Boolean(parameter.questionToken),
    rest: Boolean(parameter.dotDotDotToken),
    outer_initializer: Boolean(parameter.initializer),
    nested_pattern: nestedPattern,
    nested_default: nestedDefault,
  };
}

function arrowParenthesized(ts, sourceFile, arrow) {
  return arrow.getChildren(sourceFile).some(child => child.kind === ts.SyntaxKind.OpenParenToken && child.pos < arrow.equalsGreaterThanToken.pos);
}

function observeFile(ts, entry) {
  const [kind, script_kind] = scriptKind(ts, entry.member.path);
  const sourceFile = ts.createSourceFile(entry.member.path, entry.text, ts.ScriptTarget.Latest, true, kind);
  const map = utf8Map(entry.text);
  const diagnostics = sourceFile.parseDiagnostics.map(diagnostic => {
    const positioned = Number.isSafeInteger(diagnostic.start) && Number.isSafeInteger(diagnostic.length);
    return {
      code: diagnostic.code,
      start: positioned ? diagnostic.start : null,
      length: positioned ? diagnostic.length : null,
      start_byte: positioned ? byte(map, diagnostic.start, "diagnostic start") : null,
      end_byte: positioned ? byte(map, diagnostic.start + diagnostic.length, "diagnostic end") : null,
    };
  }).sort((a, b) => (a.start ?? -1) - (b.start ?? -1) || a.code - b.code);
  const callables = [];
  let excluded = 0;
  function visit(node) {
    if (bodyCandidate(ts, node)) {
      if (!node.body) excluded++;
      else {
        const parameters = node.parameters.map((parameter, ordinal) => parameterRow(ts, sourceFile, map, parameter, ordinal));
        callables.push({
          syntax_kind: ts.SyntaxKind[node.kind],
          token_start: byte(map, node.getStart(sourceFile, false), "callable start"),
          end: byte(map, node.end, "callable end"),
          arrow_parenthesized: ts.isArrowFunction(node) ? arrowParenthesized(ts, sourceFile, node) : null,
          parameters,
        });
      }
    } else if (explicitBodyless(ts, node)) excluded++;
    ts.forEachChild(node, visit);
  }
  visit(sourceFile);
  callables.sort((a, b) => a.token_start - b.token_start || a.end - b.end);
  const tuples = new Set();
  for (const callable of callables) for (const parameter of callable.parameters) {
    const tuple = `${entry.member.path}\0${callable.token_start}\0${callable.end}\0${parameter.ordinal}`;
    if (tuples.has(tuple)) throw Error("duplicate parameter tuple");
    tuples.add(tuple);
  }
  return {
    path: entry.member.path,
    sha256: entry.member.sha256,
    bytes: entry.member.bytes,
    script_kind,
    declaration_only: entry.member.declaration_only,
    test_path: entry.member.test_path,
    parse_state: diagnostics.length ? "syntax_diagnostics" : "clean",
    diagnostics,
    excluded_bodyless_signature_count: excluded,
    callables,
  };
}

function frequencies(files) {
  const groups = new Map();
  for (const file of files) {
    const key = JSON.stringify([file.script_kind, file.declaration_only, file.test_path, file.parse_state]);
    if (!groups.has(key)) groups.set(key, {
      script_kind: file.script_kind,
      declaration_only: file.declaration_only,
      test_path: file.test_path,
      parse_state: file.parse_state,
      files: 0,
      diagnostics: 0,
      excluded_bodyless_signatures: 0,
      body_bearing_callables: 0,
      parameters: 0,
      object_pattern_parameters: 0,
      array_pattern_parameters: 0,
      nested_pattern_parameters: 0,
      nested_default_parameters: 0,
      rest_parameters: 0,
      parenthesized_arrows: 0,
      unparenthesized_arrows: 0,
      destructured_before_later_optional_parameters: 0,
      callables_with_destructured_before_later_optional: 0,
    });
    const group = groups.get(key);
    group.files++;
    group.diagnostics += file.diagnostics.length;
    group.excluded_bodyless_signatures += file.excluded_bodyless_signature_count;
    group.body_bearing_callables += file.callables.length;
    for (const callable of file.callables) {
      if (callable.arrow_parenthesized === true) group.parenthesized_arrows++;
      if (callable.arrow_parenthesized === false) group.unparenthesized_arrows++;
      let callableRelationship = false;
      for (const parameter of callable.parameters) {
        group.parameters++;
        if (parameter.pattern === "object") group.object_pattern_parameters++;
        if (parameter.pattern === "array") group.array_pattern_parameters++;
        if (parameter.nested_pattern) group.nested_pattern_parameters++;
        if (parameter.nested_default) group.nested_default_parameters++;
        if (parameter.rest) group.rest_parameters++;
        if (["object", "array"].includes(parameter.pattern)
          && callable.parameters.some(later => later.ordinal > parameter.ordinal && later.pattern === "identifier" && later.optional)) {
          group.destructured_before_later_optional_parameters++;
          callableRelationship = true;
        }
      }
      if (callableRelationship) group.callables_with_destructured_before_later_optional++;
    }
  }
  return [...groups.values()].sort((a, b) => {
    const left = JSON.stringify([a.script_kind, a.declaration_only, a.test_path, a.parse_state]);
    const right = JSON.stringify([b.script_kind, b.declaration_only, b.test_path, b.parse_state]);
    return left < right ? -1 : left > right ? 1 : 0;
  });
}

export function observe(options) {
  const limits = mergedLimits(options.limits);
  const captured = validateManifest(options, limits);
  const compilerBytes = readFileSync(options.typescript);
  if (sha(compilerBytes) !== COMPILER_SHA) throw Error("compiler SHA mismatch");
  const ts = createRequire(import.meta.url)(path.resolve(options.typescript));
  if (ts.version !== "5.9.3") throw Error("compiler version mismatch");
  const files = captured.files.map(entry => observeFile(ts, entry));
  const rowCount = files.length + files.reduce((sum, file) => sum + file.diagnostics.length + file.callables.length
    + file.callables.reduce((count, callable) => count + callable.parameters.length, 0), 0);
  if (rowCount > limits.rows) throw Error("row limit exceeded");
  const packet = {
    schema: SCHEMA,
    authorizes_runtime_edge: false,
    measurement: "compiler_syntax_only",
    compiler: { version: ts.version, sha256: COMPILER_SHA },
    manifest: {
      sha256: captured.manifestSha,
      repository: captured.manifest.repository,
      commit: captured.manifest.commit,
      tree: captured.manifest.tree,
      member_count: captured.manifest.member_count,
      source_bytes: captured.sourceBytes,
    },
    files,
    frequency_tables: frequencies(files),
  };
  if (Buffer.byteLength(`${JSON.stringify(packet)}\n`) > limits.outputBytes) throw Error("output byte limit exceeded");
  return packet;
}

function parseFlags(argv) {
  if (argv.length % 2) throw Error("arguments must be --key value pairs");
  const allowed = new Set(["root", "manifest", "manifest-sha256", "typescript", "out"]);
  const flags = {};
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index].startsWith("--") ? argv[index].slice(2) : "";
    if (!allowed.has(name) || Object.hasOwn(flags, name)) throw Error(`unknown or duplicate flag: ${argv[index]}`);
    flags[name] = argv[index + 1];
  }
  if ([...allowed].some(name => !flags[name])) throw Error("missing required flag");
  return flags;
}

function worker(argv) {
  const flags = parseFlags(argv);
  if (existsSync(flags.out)) throw Error("output already exists");
  const delay = Number(process.env.PRISM_PARAMETER_FREQUENCY_TEST_DELAY_MS ?? 0);
  if (delay > 0) {
    const until = Date.now() + delay;
    while (Date.now() < until) {}
  }
  const packet = observe({
    root: path.resolve(flags.root),
    manifest: path.resolve(flags.manifest),
    manifestSha256: flags["manifest-sha256"],
    typescript: path.resolve(flags.typescript),
  });
  const temporary = `${flags.out}.partial-${process.pid}`;
  try {
    writeFileSync(temporary, `${JSON.stringify(packet)}\n`, { flag: "wx" });
    linkSync(temporary, flags.out);
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary);
  }
}

export function supervise(argv, { wallMs = LIMITS.wallMs, delayMs = 0 } = {}) {
  const flags = parseFlags(argv);
  if (existsSync(flags.out)) return { ok: false, reason: "output already exists" };
  const result = spawnSync(process.execPath, [ownPath, "--internal-worker", ...argv], {
    encoding: "utf8",
    timeout: wallMs,
    maxBuffer: 1024 * 1024,
    env: { ...process.env, PRISM_PARAMETER_FREQUENCY_TEST_DELAY_MS: String(delayMs) },
  });
  if (result.error?.code === "ETIMEDOUT") {
    if (existsSync(flags.out)) unlinkSync(flags.out);
    return { ok: false, reason: "timeout" };
  }
  return result.status === 0
    ? { ok: true }
    : { ok: false, reason: "worker failed", stderr: result.stderr };
}

if (process.argv[1] && path.resolve(process.argv[1]) === ownPath) {
  try {
    if (process.argv[2] === "--internal-worker") worker(process.argv.slice(3));
    else {
      const result = supervise(process.argv.slice(2));
      if (!result.ok) throw Error(result.reason === "worker failed" ? result.stderr.trim() : result.reason);
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
