// node verify-callable-authority.mjs <typescript.js> <profiles-directory>
// Isolated compiler observations ONLY: no Prism runtime proof consumer exists.
import { createRequire } from "node:module";
import { readFileSync, readdirSync, lstatSync } from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const [compiler, profiles] = process.argv.slice(2);
const ts = createRequire(import.meta.url)(compiler);
assert.equal(ts.version, "5.9.3");
const corpus = JSON.parse(readFileSync(new URL("./callable-authority-fixtures.json", import.meta.url)));
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const libRoot = path.dirname(path.resolve(compiler)) + path.sep;
const results = [], failures = [], packages = [];
function readTree(root, prefix = "") {
  return readdirSync(root).sort().flatMap(name => {
    const file = path.join(root, name), relative = prefix + name;
    const stat = lstatSync(file);
    assert(!stat.isSymbolicLink(), "fixture dependency symlink is unsupported");
    return stat.isDirectory() ? readTree(file, relative + "/") : [[relative, readFileSync(file, "utf8")]];
  });
}
for (const [profile, version, css, treeHash] of [
  ["react19", "19.0.10", "3.1.3", "49c6c7a3cde29161a5af224dede5e4442295f9251ef4c694699341ed3682baad"],
  ["react18", "18.3.31", "3.2.3", "7b8bbdc844cd38cbf691987a229858e4006273c3d3b3c4d8c8fef70267859b34"],
]) {
  const dependencies = readTree(path.join(profiles, profile));
  assert.equal(hash(JSON.stringify(dependencies)), treeHash, "pinned declaration tree changed");
  const dependencyMap = new Map(dependencies);
  assert.equal(JSON.parse(dependencyMap.get("node_modules/@types/react/package.json")).version, version);
  assert.equal(JSON.parse(dependencyMap.get("node_modules/csstype/package.json")).version, css);
  packages.push({profile, react_types: version, csstype: css,
    declaration_tree_sha256: hash(JSON.stringify(dependencies))});
  for (const fixture of corpus.cases) {
    const root = `/__prism_callable__/${profile}/${fixture.id}/`;
    const files = new Map(dependencies.filter(([f]) => !fixture.omit_react || !f.startsWith("node_modules/@types/react/"))
      .map(([f,s]) => [root + f,s]));
    for (const [file, source] of Object.entries(fixture.files)) files.set(root + file, source);
    const options = {noEmit: true, strict: true, skipLibCheck: false, types: [], allowJs: true,
      checkJs: true, target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler, jsx: ts.JsxEmit.ReactJSX,
      allowSyntheticDefaultImports: true, baseUrl: root, ...fixture.options};
    const host = ts.createCompilerHost(options), original = {...host};
    // No ambient host node_modules or declaration fallback outside the pinned libs.
    host.readFile = f => files.get(f) ?? (f.startsWith(libRoot) ? original.readFile(f) : undefined);
    host.fileExists = f => files.has(f) || (f.startsWith(libRoot) && original.fileExists(f));
    host.directoryExists = d => [...files.keys()].some(f => f.startsWith(d.endsWith("/") ? d : d + "/"))
      || (d.startsWith(libRoot) && original.directoryExists(d));
    host.getSourceFile = (f, v, err, fresh) => files.has(f) ? ts.createSourceFile(f, files.get(f), v, true)
      : f.startsWith(libRoot) ? original.getSourceFile(f, v, err, fresh) : undefined;
    const program = ts.createProgram(Object.keys(fixture.files).map(f => root + f), options, host);
    const checker = program.getTypeChecker();
    const diagnostics = ts.getPreEmitDiagnostics(program);
    const codes = [...new Set(diagnostics.map(d => d.code))].sort((a,b) => a-b);
    const sf = program.getSourceFile(root + (fixture.call_file ?? "app.ts"));
    const calls = [];
    function visit(node) {
      if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)
          && node.expression.expression.getText(sf) === "client" && node.expression.name.text === "m") {
        const owners = [...new Set((checker.getSymbolAtLocation(node.expression.name)?.declarations ?? [])
          .map(d => d.getSourceFile().fileName.slice(root.length)))].sort();
        let implementation = node.parent;
        while (implementation && !ts.isFunctionLike(implementation)) implementation = implementation.parent;
        const context = implementation && (ts.isArrowFunction(implementation) || ts.isFunctionExpression(implementation))
          ? checker.getContextualType(implementation) : undefined;
        const signatures = context ? checker.getSignaturesOfType(context, ts.SignatureKind.Call) : [];
        const start = node.getStart(sf), end = node.end;
        const startByte = Buffer.byteLength(sf.text.slice(0,start));
        const endByte = Buffer.byteLength(sf.text.slice(0,end));
        assert.equal(Buffer.from(sf.text).subarray(startByte,endByte).toString(),node.getText(sf));
        calls.push({owners, source_span: {start_utf16: start,end_utf16: end,start_byte: startByte,end_byte: endByte},
          receiver_type: checker.typeToString(checker.getTypeAtLocation(node.expression.expression)),
          explicit_parameter: !!implementation?.parameters?.[0]?.type,
          contextual_signatures: signatures.map(s => ({
            declaration_file: s.declaration?.getSourceFile().fileName.slice(root.length),
            declaration_kind: s.declaration && ts.SyntaxKind[s.declaration.kind],
            first_parameter_type: s.parameters[0] && checker.typeToString(checker.getTypeOfSymbolAtLocation(s.parameters[0],implementation)),
          }))});
      }
      ts.forEachChild(node, visit);
    }
    visit(sf);
    const inputs = program.getSourceFiles().map(f => ({file: f.fileName.startsWith(root) ? f.fileName.slice(root.length) : "typescript-lib/" + path.basename(f.fileName), sha256: hash(f.text)})).sort((a,b) => a.file.localeCompare(b.file));
    const result = {id: fixture.id,profile,diagnostic_codes: codes,calls,
      authorizes_runtime_edge: false,program_input_sha256: hash(JSON.stringify({options,inputs})),inputs};
    results.push(result);
    try {
      assert.equal(calls.length,1,"fixture must contain one real receiver call");
      assert.deepEqual(codes,fixture.diagnostic_codes);
      assert.deepEqual(calls[0].owners,fixture.ts_owner_files);
      if (fixture.id === "augmentation_overload") assert.equal(calls[0].contextual_signatures.length,2);
      if (fixture.id === "explicit_any") {
        assert.equal(calls[0].explicit_parameter,true);
        assert.equal(calls[0].receiver_type,"any");
      }
      if (["assertion_not_context", "satisfies_not_retroactive", "imported_js_cast"].includes(fixture.id)) {
        assert.equal(calls[0].contextual_signatures.length,0);
      }
      if (fixture.policy === "context_candidate") {
        assert.equal(calls[0].explicit_parameter,false);
        assert.equal(calls[0].contextual_signatures.length,1);
        assert.equal(calls[0].contextual_signatures[0].declaration_file,"node_modules/@types/react/index.d.ts");
      }
    } catch (error) { failures.push({id:fixture.id,profile,message:error.message}); }
  }
}
console.log(JSON.stringify({compiler_version:ts.version,compiler_sha256:hash(readFileSync(compiler)),
  packages,results,failures,scope:"isolated fixture programs; observations are not runtime edge authority"},null,2));
assert.deepEqual(failures,[]);
