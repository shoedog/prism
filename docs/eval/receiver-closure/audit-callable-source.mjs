// node audit-callable-source.mjs <typescript.js> <git-repo>
// Read-only tracked-source census. Output may contain private paths: keep local.
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { readFileSync, lstatSync } from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";
const [compiler, root] = process.argv.slice(2);
const ts = createRequire(import.meta.url)(compiler);
const git = (...args) => execFileSync("git", ["-C", root, ...args], {encoding: "utf8"});
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const tracked = git("ls-files", "-z").split("\0").filter(Boolean).sort();
const inventory = [], references = [], extensions = {}, refused = [];
for (const file of tracked.filter(f => /\.(?:[cm]?js|jsx|ts|tsx)$/.test(f))) {
  const absolute = path.join(root, file);
  const stat = lstatSync(absolute);
  if (!stat.isFile() || stat.isSymbolicLink()) { refused.push(file); continue; }
  const bytes = readFileSync(absolute), source = bytes.toString("utf8");
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true);
  const ext = path.extname(file);
  extensions[ext] = (extensions[ext] ?? 0) + 1;
  inventory.push({file, sha256: hash(bytes), parse_errors: sf.parseDiagnostics.map(d => d.code)});
  const namespace = new Set(), named = new Set();
  for (const statement of sf.statements) {
    if (!ts.isImportDeclaration(statement) || statement.moduleSpecifier.text !== "react") continue;
    const clause = statement.importClause;
    if (clause?.name) namespace.add(clause.name.text);
    const bindings = clause?.namedBindings;
    if (bindings && ts.isNamespaceImport(bindings)) namespace.add(bindings.name.text);
    if (bindings && ts.isNamedImports(bindings)) {
      for (const e of bindings.elements) {
        if (["FC", "FunctionComponent"].includes((e.propertyName ?? e.name).text)) named.add(e.name.text);
      }
    }
  }
  function visit(node) {
    if (ts.isTypeReferenceNode(node)) {
      const name = node.typeName;
      const matched = ts.isIdentifier(name) ? named.has(name.text)
        : ts.isQualifiedName(name) && ts.isIdentifier(name.left)
          && namespace.has(name.left.text) && ["FC", "FunctionComponent"].includes(name.right.text);
      if (matched) {
        const parent = node.parent;
        let producer = "other_type_use";
        if ((ts.isAsExpression(parent) || ts.isTypeAssertionExpression(parent)) && parent.type === node) producer = "assertion";
        else if (ts.isVariableDeclaration(parent) && parent.type === node) {
          producer = parent.initializer && (ts.isArrowFunction(parent.initializer) || ts.isFunctionExpression(parent.initializer))
            ? "direct_function_annotation" : "other_variable_annotation";
        }
        const start = node.getStart(sf);
        references.push({file, line: sf.getLineAndCharacterOfPosition(start).line + 1,
          start_utf16: start, end_utf16: node.end, spelling: name.getText(sf), producer});
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(sf);
}
const configs = tracked.filter(f => /(?:^|\/)(?:tsconfig[^/]*\.json|package\.json|package-lock\.json|yarn\.lock|pnpm-lock\.yaml)$/.test(f))
  .flatMap(file => {
    const absolute = path.join(root, file), stat = lstatSync(absolute);
    if (!stat.isFile() || stat.isSymbolicLink()) { refused.push(file); return []; }
    return [{file, sha256: hash(readFileSync(absolute))}];
  });
console.log(JSON.stringify({schema: 1, compiler_version: ts.version,
  head: git("rev-parse", "HEAD").trim(), status: git("status", "--porcelain"),
  source_files: inventory.length, extensions, references, refused,
  snapshot_sha256: hash(JSON.stringify({inventory, configs})), inventory, configs,
  evidence_scope: "tracked-source syntax census only; import spelling is not symbol authority",
  compiler_program_checked: false}, null, 2));
