// node verify-contextual-imported-types.mjs /absolute/path/to/typescript.js
// Isolated compiler programs, not whole-application augmentation closure.
import { createRequire } from "node:module";
import assert from "node:assert/strict";
const ts = createRequire(import.meta.url)(process.argv[2]);
assert.equal(ts.version, "5.9.3");
const cases = [
  ["direct", "const run: (p: Props) => void = ({client}) => client.m();", "client.ts"],
  ["function", "const run: (p: Props) => void = function ({client}) { client.m(); };", "client.ts"],
  ["alias", "type F = (p: Props) => void; const run: F = ({client}) => client.m();", "client.ts"],
  ["generic", "type F<P> = (p: P) => void; const run: F<Props> = ({client}) => client.m();", "client.ts"],
  ["callable", "type F = {(p: Props): void}; const run: F = ({client}) => client.m();", "client.ts"],
  ["generic_callable", "type F<P> = {(p: P): void}; const run: F<Props> = ({client}) => client.m();", "client.ts"],
  ["interface", "interface F {(p: Props): void} const run: F = ({client}) => client.m();", "client.ts"],
  ["generic_interface", "interface F<P> {(p: P): void} const run: F<Props> = ({client}) => client.m();", "client.ts"],
  ["original_scope", "type F = (p: Props) => void; function outer<Props>() { const run: F = ({client}) => client.m(); }", "client.ts"],
  ["explicit_any_terminal", "const run: (p: Props) => void = ({client}: any) => client.m();", null],
  ["explicit_decoy_terminal", "const run: (p: Props) => void = ({client}: {client: DeclaredClient}) => client.m();", "app.ts"],
];
const results = [], failures = [];
for (const [id, declaration, expected] of cases) {
  const root = `/__prism_contextual_import__/${id}/`;
  const files = new Map([
    [root + "client.ts", "export default class Client { m() {} }"],
    [root + "props.ts", "import type DeclaredClient from './client'; export type Props = {client: DeclaredClient};"],
    [root + "app.ts", "import type {Props} from './props'; class DeclaredClient { m() {} } " + declaration],
  ]);
  const options = {strict: true, noEmit: true, types: [], target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext, moduleResolution: ts.ModuleResolutionKind.Bundler};
  const host = ts.createCompilerHost(options);
  const original = {readFile: host.readFile, fileExists: host.fileExists,
    directoryExists: host.directoryExists, getSourceFile: host.getSourceFile};
  host.readFile = f => files.get(f) ?? original.readFile(f);
  host.fileExists = f => files.has(f) || original.fileExists(f);
  host.directoryExists = d => [...files.keys()].some(f => f.startsWith(d + "/")) || original.directoryExists(d);
  host.getSourceFile = (f, version, onError, fresh) => files.has(f)
    ? ts.createSourceFile(f, files.get(f), version, true)
    : original.getSourceFile(f, version, onError, fresh);
  const program = ts.createProgram([...files.keys()], options, host);
  const checker = program.getTypeChecker();
  const codes = ts.getPreEmitDiagnostics(program).map(d => d.code);
  const owners = new Set();
  let calls = 0;
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)
        && node.expression.expression.getText() === "client" && node.expression.name.text === "m") {
      calls++;
      for (const d of checker.getSymbolAtLocation(node.expression.name)?.declarations ?? []) {
        owners.add(d.getSourceFile().fileName.slice(root.length));
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(program.getSourceFile(root + "app.ts"));
  const result = {id, calls, diagnostic_codes: codes, owners: [...owners].sort()};
  results.push(result);
  try {
    assert.equal(calls, 1);
    assert.deepEqual(codes, []);
    assert.deepEqual(result.owners, expected === null ? [] : [expected]);
  } catch (error) { failures.push({id, message: error.message}); }
}
console.log(JSON.stringify({compiler_version: ts.version, results, failures}, null, 2));
assert.deepEqual(failures, []);
