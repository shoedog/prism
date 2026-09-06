// node audit-imported-props-source.mjs <typescript.js> <source-archive> <fixed-slice> <sites.jsonl> <source-git-repo>
// Literal source census, NOT a configured TypeScript Program or dependency closure.
import { createRequire } from "node:module";
import { readFileSync, readdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import path from "node:path";
import assert from "node:assert/strict";
const require = createRequire(import.meta.url);
const [compilerPath, upstream, slice, sitesPath, sourceRepo] = process.argv.slice(2);
const upstreamSha = "0642e72cfa2d9a71198200e52f37399384610ee3";
const tree = execFileSync("git", ["-C", sourceRepo, "ls-tree", "-rz", upstreamSha], { encoding: "utf8" });
const blobs = new Map(tree.split("\0").filter(Boolean).map(row => {
  const [meta, file] = row.split("\t");
  const [, kind, oid] = meta.split(" ");
  assert.equal(kind, "blob", `unexpected non-blob at ${file}`);
  return [file, oid];
}));
const ts = require(compilerPath);
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const source = f => readFileSync(path.join(upstream, f), "utf8");
const prefix = "packages/excalidraw/";
const sourceFiles = ["components/App.tsx", "components/LibraryMenu.tsx",
  "components/LibraryMenuHeaderContent.tsx", "data/library.ts", "types.ts"].map(f => prefix + f);
const hashes = {};
for (const f of sourceFiles) {
  const original = readFileSync(path.join(upstream, f));
  assert(original.equals(readFileSync(path.join(slice, f))), `pinned source mismatch: ${f}`);
  hashes[f] = hash(original);
}
const records = readFileSync(sitesPath, "utf8").trim().split("\n").map(JSON.parse)
  .filter(r => r.record_kind === "call_site");
assert.equal(records.length, 2780);
assert.equal(records.filter(r => r.resolved_targets.some(t => t.confidence === "exact")).length, 376);
const remaining = new Map(), allRelevant = new Set();
for (const r of records) {
  const span = r.source_span;
  const expression = Buffer.from(source(span.file)).subarray(span.start_byte, span.end_byte).toString();
  const match = /^(?:this\.)?library\s*(?:\?\.|\.)\s*(destroy|resetLibrary|getLatestLibrary|updateLibrary|setLibrary)\s*\(/.exec(expression);
  if (!match || r.callee_text !== match[1]) continue;
  const key = `${span.file}:${span.start_byte}:${span.end_byte}`;
  allRelevant.add(key);
  if (r.resolved_targets.some(t => t.confidence === "exact")) continue;
  const item = remaining.get(key) ?? { file: span.file, line: span.line,
    start_byte: span.start_byte, end_byte: span.end_byte, expression,
    caller_records: [], producer: [155, 160, 184, 265].includes(span.line) ? "React.FC" : "useApp/useContext" };
  item.caller_records.push(r.caller.name);
  remaining.set(key, item);
}
const sites = [...remaining.values()].sort((a, b) => a.line - b.line);
assert.equal(allRelevant.size, 11);
assert.equal(sites.length, 6);
assert.deepEqual(sites.map(s => s.line), [155, 160, 184, 265, 297, 307]);
assert(sites.every(s => s.file === prefix + "components/LibraryMenuHeaderContent.tsx"));
// Pin every edge in the source trace; a line move or changed declaration fails the audit.
const anchors = [
  ["components/LibraryMenuHeaderContent.tsx", 14, "import { useApp, useExcalidrawSetAppState } from \"./App\";"],
  ["components/LibraryMenuHeaderContent.tsx", 30, "import type Library from \"../data/library\";"],
  ["components/LibraryMenuHeaderContent.tsx", 38, "export const LibraryDropdownMenuButton: React.FC<{"],
  ["components/LibraryMenuHeaderContent.tsx", 41, "  library: Library;"],
  ["components/LibraryMenuHeaderContent.tsx", 286, "  const { library } = useApp();"],
  ["components/App.tsx", 474, "  AppClassProperties,"],
  ["components/App.tsx", 499, "} from \"../types\";"],
  ["components/App.tsx", 503, "const AppContext = React.createContext<AppClassProperties>(null!);"],
  ["components/App.tsx", 568, "export const useApp = () => useContext(AppContext);"],
  ["types.ts", 56, "import type Library from \"./data/library\";"],
  ["types.ts", 801, "export type AppClassProperties = {"],
  ["types.ts", 809, "  focusContainer(): void;"],
  ["types.ts", 810, "  library: Library;"],
  ["data/library.ts", 197, "class Library {"],
  ["data/library.ts", 403, "export default Library;"],
];
for (const [f, line, expected] of anchors) assert.equal(source(prefix + f).split("\n")[line - 1], expected);
const tsFiles = [];
function walk(dir) {
  for (const entry of readdirSync(path.join(upstream, dir), { withFileTypes: true })) {
    const f = path.posix.join(dir, entry.name);
    if (entry.isDirectory()) walk(f);
    else if (/\.(?:[cm]?ts|tsx)$/.test(f)) tsFiles.push(f);
  }
}
walk("");
tsFiles.sort();
assert.deepEqual(tsFiles, [...blobs.keys()].filter(f => /\.(?:[cm]?ts|tsx)$/.test(f)).sort(), "archive file set differs from pinned tree");
for (const f of [...tsFiles, "package.json"]) {
  const bytes = readFileSync(path.join(upstream, f));
  const oid = createHash("sha1").update(`blob ${bytes.length}\0`).update(bytes).digest("hex");
  assert.equal(oid, blobs.get(f), `archive blob differs from pinned commit: ${f}`);
}
const packageVersions = JSON.parse(source("package.json")).devDependencies;
const declarations = [], scriptInterfaces = [], errors = [];
for (const f of tsFiles) {
  const text = source(f);
  const file = ts.createSourceFile(f, text, ts.ScriptTarget.Latest, true);
  for (const error of file.parseDiagnostics) errors.push({ file: f, code: error.code });
  const module = ts.isExternalModule(file);
  if (!module) for (const n of file.statements) {
    if (ts.isInterfaceDeclaration(n)) scriptInterfaces.push({ file: f, name: n.name.text,
      line: file.getLineAndCharacterOfPosition(n.getStart()).line + 1 });
  }
  const visit = n => {
    if (ts.isModuleDeclaration(n) && (ts.isStringLiteral(n.name) || (n.flags & ts.NodeFlags.GlobalAugmentation))) {
      declarations.push({ file: f, line: file.getLineAndCharacterOfPosition(n.getStart()).line + 1,
        name: n.name.text, kind: n.flags & ts.NodeFlags.GlobalAugmentation ? "global_augmentation"
          : module ? "module_augmentation" : "ambient_module",
        file_sha256: hash(text) });
    }
    ts.forEachChild(n, visit);
  };
  visit(file);
}
assert.equal(errors.length, 0, JSON.stringify(errors));
const treeHash = createHash("sha256");
for (const f of tsFiles) treeHash.update(f + "\0" + hash(source(f)) + "\n");
console.log(JSON.stringify({ schema: 1, upstream_sha: upstreamSha, git_tree_blobs_verified: true,
  upstream_typescript: packageVersions.typescript, upstream_react_types: packageVersions["@types/react"],
  compiler_version: ts.version, compiler_program_checked: false, dependency_closure_checked: false,
  source_sha256: hashes, measured_call_records: records.length, exact_records: 376,
  relevant_unique_spans: allRelevant.size, remaining_unique_spans: sites,
  proof_anchors: anchors.map(([file, line, text]) => ({ file: prefix + file, line, text })),
  tracked_typescript_files: tsFiles.length, typescript_path_content_sha256: treeHash.digest("hex"),
  declarations, script_interfaces: scriptInterfaces, parse_errors: errors }, null, 2));
