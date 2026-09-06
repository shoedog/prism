// node verify-imported-props-types.mjs /absolute/path/to/typescript.js
// In-memory compiler fixtures: no package installs, emitted files or source rewrites.
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
const require = createRequire(import.meta.url);
const ts = require(process.argv[2]);
const corpus = JSON.parse(readFileSync(new URL("./imported-props-identity-fixtures.json", import.meta.url), "utf8"));
assert.equal(corpus.schema, 1);
assert.equal(corpus.cases.length, 24);
assert.equal(new Set(corpus.cases.map(c => c.id)).size, corpus.cases.length);
const results = [], failures = [];
for (const fixture of corpus.cases) {
  const root = `/__prism_type_audit__/${fixture.id}/`;
  const files = new Map(Object.entries(fixture.files).map(([f, s]) => [root + f, s]));
  const options = { noEmit: true, strict: true, types: [], target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler };
  const host = ts.createCompilerHost(options);
  const original = { readFile: host.readFile, fileExists: host.fileExists,
    directoryExists: host.directoryExists, getSourceFile: host.getSourceFile };
  host.readFile = f => files.get(f) ?? original.readFile(f);
  host.fileExists = f => files.has(f) || original.fileExists(f);
  host.directoryExists = d => [...files.keys()].some(f => f.startsWith(d + "/")) || original.directoryExists(d);
  host.getSourceFile = (f, version, onError, fresh) => files.has(f)
    ? ts.createSourceFile(f, files.get(f), version, true)
    : original.getSourceFile(f, version, onError, fresh);
  const program = ts.createProgram([...files.keys()], options, host);
  const diagnostics = ts.getPreEmitDiagnostics(program);
  const codes = [...new Set(diagnostics.map(d => d.code))].sort((a, b) => a - b);
  const checker = program.getTypeChecker(), owners = new Set();
  let calls = 0;
  const visit = node => {
    if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)
        && node.expression.expression.getText() === "client" && node.expression.name.text === "m") {
      calls++;
      for (const declaration of checker.getSymbolAtLocation(node.expression.name)?.declarations ?? []) {
        owners.add(path.relative(root, declaration.getSourceFile().fileName));
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(program.getSourceFile(root + "app.ts"));
  const ownerFiles = [...owners].sort();
  const result = { id: fixture.id, diagnostic_codes: codes, method_calls: calls,
    method_declaration_files: ownerFiles,
    diagnostics: diagnostics.map(d => ({ code: d.code, file: d.file && path.relative(root, d.file.fileName),
      message: ts.flattenDiagnosticMessageText(d.messageText, " ") })) };
  results.push(result);
  try {
    assert.equal(calls, 1);
    assert.deepEqual(codes, fixture.diagnostic_codes);
    if (fixture.ts_owner_files !== null) assert.deepEqual(ownerFiles, fixture.ts_owner_files);
  } catch (error) { failures.push({ id: fixture.id, message: error.message }); }
}
console.log(JSON.stringify({ compiler_version: ts.version, fixtures: results, failures }, null, 2));
assert.deepEqual(failures, [], "compiler fixtures must match diagnostics and declaration identities");
