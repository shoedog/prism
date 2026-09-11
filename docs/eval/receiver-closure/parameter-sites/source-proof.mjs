import { readFileSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

import { validateCensus } from './compare.mjs';

const PINNED_TS_VERSION = '5.9.3';
const TARGET_FORMS = new Set(['optional_identifier', 'default_identifier']);

function fail(message) { throw new Error(message); }
function count(obj, key) { obj[key] = (obj[key] ?? 0) + 1; }

export function loadTypeScript(modulePath, expectedVersion = PINNED_TS_VERSION) {
  const require = createRequire(import.meta.url);
  const ts = require(modulePath);
  if (ts.version !== expectedVersion) fail('unsupported TypeScript compiler version');
  return ts;
}

export function hashBytes(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
export function utf8ByteOffset(text, utf16Index) { return Buffer.byteLength(text.slice(0, utf16Index), 'utf8'); }

export function isInertInitializer(ts, node) {
  if (!node) return false;
  const literalKinds = [
    ts.SyntaxKind.NumericLiteral, ts.SyntaxKind.BigIntLiteral, ts.SyntaxKind.StringLiteral,
    ts.SyntaxKind.TrueKeyword, ts.SyntaxKind.FalseKeyword, ts.SyntaxKind.NullKeyword,
    ts.SyntaxKind.NoSubstitutionTemplateLiteral,
  ];
  if (literalKinds.includes(node.kind)) return true;
  if (ts.isArrayLiteralExpression(node)) return node.elements.length === 0;
  if (ts.isObjectLiteralExpression(node)) return node.properties.length === 0;
  return ts.isPrefixUnaryExpression(node)
    && (node.operator === ts.SyntaxKind.PlusToken || node.operator === ts.SyntaxKind.MinusToken)
    && ts.isNumericLiteral(node.operand);
}

export function hasNestedInitializer(ts, node) {
  if (!node) return false;
  let found = false;
  const walk = child => {
    if (found) return;
    if (ts.isBindingElement(child) && child.initializer) { found = true; return; }
    ts.forEachChild(child, walk);
  };
  ts.forEachChild(node, walk);
  return found;
}

export function parameterHasBindingPatternDefault(ts, parameter) {
  const name = parameter.name;
  if (!(ts.isObjectBindingPattern(name) || ts.isArrayBindingPattern(name))) return false;
  return hasNestedInitializer(ts, name);
}

export function hasNoninertSibling(ts, parameter) {
  const siblings = parameter.parent?.parameters ?? [];
  return siblings.some(sibling => sibling !== parameter
    && ((sibling.initializer && !isInertInitializer(ts, sibling.initializer))
      || parameterHasBindingPatternDefault(ts, sibling)));
}

export function classifyMatchedParameter(ts, node, form) {
  const hasInitializer = Boolean(node.initializer);
  const observedForm = node.questionToken ? 'optional_identifier' : hasInitializer ? 'default_identifier' : 'required_identifier';
  if (observedForm !== form) fail('parameter form mismatch');
  const signatureHasInitializer = (node.parent?.parameters ?? [node])
    .some(parameter => parameter.initializer || parameterHasBindingPatternDefault(ts, parameter));
  const optionalNoInitializer = form === 'optional_identifier' && !signatureHasInitializer;
  const inertDefaultCandidate = form === 'default_identifier' && hasInitializer
    && isInertInitializer(ts, node.initializer) && !hasNoninertSibling(ts, node);
  return { optionalNoInitializer, inertDefaultCandidate };
}

export function indexParameters(ts, sourceFile, text) {
  const byte = index => utf8ByteOffset(text, index);
  const params = new Map();
  const walk = node => {
    if (ts.isParameter(node)) params.set(`${byte(node.getStart(sourceFile))}:${byte(node.end)}`, node);
    ts.forEachChild(node, walk);
  };
  walk(sourceFile);
  return { params, byte };
}

function scriptKindFor(fileName, ts) {
  if (fileName.endsWith('.tsx')) return ts.ScriptKind.TSX;
  if (fileName.endsWith('.ts')) return ts.ScriptKind.TS;
  return ts.ScriptKind.JS;
}

export function classifyCensus(ts, census, resolveSource) {
  const wanted = census.functions.flatMap(fn => fn.parameters
    .filter(p => TARGET_FORMS.has(p.form))
    .map(p => ({ fn, p })));

  const summary = {
    target_entries: wanted.length,
    files: 0,
    forms: {},
    recovery_skipped_entries: 0,
    matched_entries: 0,
    optional_no_initializer_entries: 0,
    optional_no_initializer_named_owner_slot_entries: 0,
    inert_default_candidate_entries: 0,
    inert_default_candidate_named_owner_slot_entries: 0,
  };

  const fileState = new Map();
  for (const { fn, p } of wanted) {
    count(summary.forms, p.form);
    if (fn.parameter_recovery) { summary.recovery_skipped_entries++; continue; }
    if (!fileState.has(fn.file)) {
      const manifest = census.files.find(file => file.file === fn.file);
      if (!manifest) fail('function references an unloaded file');
      const bytes = resolveSource(fn.file);
      if (hashBytes(bytes) !== manifest.sha256) fail('source hash mismatch');
      const text = bytes.toString('utf8');
      const sourceFile = ts.createSourceFile(fn.file, text, ts.ScriptTarget.Latest, true, scriptKindFor(fn.file, ts));
      if (sourceFile.parseDiagnostics.length > 0) fail('source parse diagnostics present');
      fileState.set(fn.file, { sourceFile, text, ...indexParameters(ts, sourceFile, text) });
    }
    const { sourceFile, params, byte } = fileState.get(fn.file);
    const node = params.get(`${p.start_byte}:${p.end_byte}`);
    if (!node) fail('unmatched parameter token');
    if (!ts.isIdentifier(node.name)) fail('parameter binding is not a simple identifier');

    const nameStart = byte(node.name.getStart(sourceFile));
    const nameEnd = byte(node.name.end);
    const slot = (fn.slots ?? []).some(s => s.start_byte === nameStart && s.end_byte === nameEnd
      && s.name === node.name.getText(sourceFile));
    const namedOwner = fn.owner_name !== null;
    const { optionalNoInitializer, inertDefaultCandidate } = classifyMatchedParameter(ts, node, p.form);

    summary.matched_entries++;
    if (optionalNoInitializer) {
      summary.optional_no_initializer_entries++;
      if (namedOwner && slot) summary.optional_no_initializer_named_owner_slot_entries++;
    }
    if (inertDefaultCandidate) {
      summary.inert_default_candidate_entries++;
      if (namedOwner && slot) summary.inert_default_candidate_named_owner_slot_entries++;
    }
  }
  summary.files = fileState.size;
  return summary;
}

export function buildReport(summary) {
  return { schema: 'prism.parameter-site-source-proof/1', authorizes_runtime_edge: false, ...summary };
}

export function resolveWithinRoot(sourceRoot, fileRelPath) {
  const root = realpathSync(sourceRoot);
  const inside = path => path !== root && path.startsWith(root + sep);
  const lexical = resolve(root, fileRelPath);
  if (!inside(lexical)) fail('source path escapes source root');
  const resolved = realpathSync(lexical);
  if (!inside(resolved)) fail('source path escapes source root');
  return resolved;
}

export async function run({ tsModulePath, censusPath, sourceRoot }) {
  const ts = loadTypeScript(tsModulePath);
  const census = validateCensus(JSON.parse(readFileSync(censusPath, 'utf8')));
  const resolveSource = fileRelPath => readFileSync(resolveWithinRoot(sourceRoot, fileRelPath));
  const summary = classifyCensus(ts, census, resolveSource);
  return buildReport(summary);
}

async function main() {
  if (process.argv.length !== 5) fail('usage: node source-proof.mjs <ts-module-path> <census.json> <source-root>');
  const [, , tsModulePath, censusPath, sourceRoot] = process.argv;
  const report = await run({ tsModulePath, censusPath, sourceRoot });
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
}
if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  main().catch(() => {
    process.stderr.write('source-proof failed\n');
    process.exitCode = 1;
  });
}
