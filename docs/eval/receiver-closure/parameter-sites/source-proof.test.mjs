import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { mkdtempSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  classifyCensus, classifyMatchedParameter, hasNoninertSibling, isInertInitializer,
  loadTypeScript, resolveWithinRoot, run, utf8ByteOffset,
} from './source-proof.mjs';

const TS_MODULE_PATH = process.env.PRISM_TYPESCRIPT;
assert.ok(TS_MODULE_PATH, 'Set PRISM_TYPESCRIPT to an absolute TypeScript 5.9.3 module path');
const require = createRequire(import.meta.url);
const ts = require(TS_MODULE_PATH);
assert.equal(ts.version, '5.9.3');

function byteOffset(text, utf16Index) { return Buffer.byteLength(text.slice(0, utf16Index), 'utf8'); }
function hashOf(text) { return createHash('sha256').update(Buffer.from(text, 'utf8')).digest('hex'); }

function initializerNode(exprText) {
  const sf = ts.createSourceFile('x.ts', `const v = ${exprText};`, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  return sf.statements[0].declarationList.declarations[0].initializer;
}

function firstFunctionParams(text) {
  const sf = ts.createSourceFile('x.ts', text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  let params;
  ts.forEachChild(sf, node => { if (ts.isFunctionDeclaration(node)) params = node.parameters; });
  return params;
}

function census(files, functions) {
  return { schema: 'prism.parameter-site-census/1', authorizes_runtime_edge: false, files, skipped: [], functions, flows: [] };
}

function functionEntry(file, text, overrides = {}) {
  const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  let fnNode;
  ts.forEachChild(sf, node => { if (ts.isFunctionDeclaration(node)) fnNode = node; });
  const byte = i => byteOffset(text, i);
  const parameters = fnNode.parameters.map(p => ({
    start_byte: byte(p.getStart(sf)), end_byte: byte(p.end), kind: 'parameter',
    form: p.questionToken ? 'optional_identifier' : p.initializer ? 'default_identifier' : 'identifier',
  }));
  const slots = fnNode.parameters.filter(p => ts.isIdentifier(p.name))
    .map(p => ({ name: p.name.getText(sf), start_byte: byte(p.name.getStart(sf)), end_byte: byte(p.name.end) }));
  return {
    file, start_byte: byte(fnNode.getStart(sf)), end_byte: byte(fnNode.end), start_line: 1,
    owner_name: 'widget', kind: 'function', parameter_recovery: false,
    parameters, slots, occurrences: [], ...overrides,
  };
}

function fileManifest(file, text, overrides = {}) {
  return { file, sha256: hashOf(text), language: 'TypeScript', parse_errors: 0, ...overrides };
}

function resolverFor(map) {
  return file => {
    if (!(file in map)) throw new Error('unexpected file lookup');
    return Buffer.from(map[file], 'utf8');
  };
}

test('all initializer classes: inert literals, containers, and unary numerics', () => {
  for (const expr of ['0', '-1', '+2', '[]', '{}', '""', 'true', 'false', 'null', '9n', '`x`']) {
    assert.equal(isInertInitializer(ts, initializerNode(expr)), true, expr);
  }
});

test('all initializer classes: calls, identifiers, filled containers, side effects, and casts are not inert', () => {
  for (const expr of ['compute()', 'ref', '[1]', '{ a: 1 }', '(1)', '1 as number', 'a + b', '-a']) {
    assert.equal(isInertInitializer(ts, initializerNode(expr)), false, expr);
  }
});

test('optional identifier with no initializer classifies as optional_no_initializer', () => {
  const [label] = firstFunctionParams('function f(label?: string) {}');
  assert.equal(classifyMatchedParameter(ts, label, 'optional_identifier').optionalNoInitializer, true);
});

test('optional candidate requires an initializer-free entire signature', () => {
  for (const signature of ['a?: any, b = 0', 'a?: any, b = call()', 'a?: any, {b = 1}: any', 'a?: any, [b = 1]: any']) {
    const [a] = firstFunctionParams(`function f(${signature}) {}`);
    assert.equal(classifyMatchedParameter(ts, a, 'optional_identifier').optionalNoInitializer, false, signature);
  }
});

test('census form must agree with compiler question/default syntax', () => {
  for (const text of ['function widget(a: any) {}', 'function widget(a = 0) {}']) {
    const fn = functionEntry('a.ts', text);
    fn.parameters[0].form = 'optional_identifier';
    assert.throws(() => classifyCensus(ts, census([fileManifest('a.ts', text)], [fn]), resolverFor({'a.ts': text})), /form mismatch/);
  }
});

test('slot identity includes exact source binding spelling', () => {
  const text = 'function widget(a?: any) {}';
  const fn = functionEntry('a.ts', text);
  fn.slots[0].name = 'wrong';
  const summary = classifyCensus(ts, census([fileManifest('a.ts', text)], [fn]), resolverFor({'a.ts': text}));
  assert.equal(summary.optional_no_initializer_named_owner_slot_entries, 0);
});

test('sibling binding-pattern nested default disqualifies an inert default candidate', () => {
  const [a] = firstFunctionParams('function f(a = 1, { b = 2 }) {}');
  assert.equal(hasNoninertSibling(ts, a), true);
});

test('sibling binding pattern with an inert default and no nested default does not disqualify', () => {
  const [a] = firstFunctionParams('function f(a = 1, { b } = {}) {}');
  assert.equal(hasNoninertSibling(ts, a), false);
});

test('sibling with a non-inert default disqualifies an inert default candidate', () => {
  const [a] = firstFunctionParams('function f(a = 1, b = compute()) {}');
  assert.equal(hasNoninertSibling(ts, a), true);
});

test('classifies optional and inert-default entries and separates named-owner+slot subsets', () => {
  const text = 'function widget(size = 0, label?: string) {\n  return `${size}${label}`;\n}\n';
  const anonymousText = 'export default function (size = 0) {\n  return size;\n}\n';
  const files = [fileManifest('a.ts', text), fileManifest('b.ts', anonymousText)];
  const anonymousFn = functionEntry('b.ts', anonymousText, { owner_name: null });
  const c = census(files, [functionEntry('a.ts', text), anonymousFn]);
  const summary = classifyCensus(ts, c, resolverFor({ 'a.ts': text, 'b.ts': anonymousText }));

  assert.equal(summary.target_entries, 3);
  assert.equal(summary.matched_entries, 3);
  assert.equal(summary.optional_no_initializer_entries, 0);
  assert.equal(summary.optional_no_initializer_named_owner_slot_entries, 0);
  assert.equal(summary.inert_default_candidate_entries, 2);
  assert.equal(summary.inert_default_candidate_named_owner_slot_entries, 1);
});

test('slot mismatch excludes an entry from the named-owner+slot subset without affecting raw counts', () => {
  const text = 'function widget(size = 0) {\n  return size;\n}\n';
  const fn = functionEntry('a.ts', text);
  fn.slots = [{ name: 'slot', start_byte: 0, end_byte: 1 }];
  const c = census([fileManifest('a.ts', text)], [fn]);
  const summary = classifyCensus(ts, c, resolverFor({ 'a.ts': text }));
  assert.equal(summary.inert_default_candidate_entries, 1);
  assert.equal(summary.inert_default_candidate_named_owner_slot_entries, 0);
});

test('recovery-marked functions are skipped rather than counted or failed', () => {
  const text = 'function widget(label?: string) {\n  return label;\n}\n';
  const fn = functionEntry('a.ts', text, { parameter_recovery: true });
  const c = census([fileManifest('a.ts', text)], [fn]);
  const summary = classifyCensus(ts, c, resolverFor({ 'a.ts': text }));
  assert.equal(summary.recovery_skipped_entries, 1);
  assert.equal(summary.matched_entries, 0);
  assert.equal(summary.optional_no_initializer_entries, 0);
});

test('fails closed on a source hash mismatch', () => {
  const text = 'function widget(label?: string) {\n  return label;\n}\n';
  const c = census([fileManifest('a.ts', text, { sha256: 'a'.repeat(64) })], [functionEntry('a.ts', text)]);
  assert.throws(() => classifyCensus(ts, c, resolverFor({ 'a.ts': text })), /hash mismatch/);
});

test('fails closed on a missing/unmatched parameter token', () => {
  const text = 'function widget(label?: string) {\n  return label;\n}\n';
  const fn = functionEntry('a.ts', text);
  fn.parameters[0].start_byte += 100;
  fn.parameters[0].end_byte += 100;
  const c = census([fileManifest('a.ts', text)], [fn]);
  assert.throws(() => classifyCensus(ts, c, resolverFor({ 'a.ts': text })), /unmatched parameter token/);
});

test('fails closed on source parse drift', () => {
  const brokenText = 'function widget(label?: string {\n  return label;\n}\n';
  const fn = {
    file: 'broken.ts', start_byte: 0, end_byte: byteOffset(brokenText, brokenText.length), start_line: 1,
    owner_name: 'widget', kind: 'function', parameter_recovery: false,
    parameters: [{ start_byte: 16, end_byte: 31, kind: 'parameter', form: 'optional_identifier' }],
    slots: [{ name: 'label', start_byte: 17, end_byte: 22 }], occurrences: [],
  };
  const c = census([fileManifest('broken.ts', brokenText)], [fn]);
  assert.throws(() => classifyCensus(ts, c, resolverFor({ 'broken.ts': brokenText })), /parse diagnostics/);
});

test('fails closed when the pinned compiler version does not match', () => {
  const fakeModulePath = join(mkdtempSync(join(tmpdir(), 'source-proof-fake-ts-')), 'fake-ts.cjs');
  writeFileSync(fakeModulePath, 'module.exports = { version: "4.0.0" };\n');
  try {
    assert.throws(() => loadTypeScript(fakeModulePath), /unsupported TypeScript compiler version/);
  } finally {
    rmSync(fakeModulePath, { force: true });
    rmSync(join(fakeModulePath, '..'), { recursive: true, force: true });
  }
});

test('UTF-16 to UTF-8 byte offsets account for multi-byte prefixes', () => {
  const text = 'const \u{1F389} = 1;';
  assert.equal(utf8ByteOffset(text, 6), 6);
  assert.equal(utf8ByteOffset(text, 8), 10);
});

test('matches an optional parameter preceded by multi-byte Unicode source text', () => {
  const text = '// \u{1F389} unicode prefix\nfunction widget(label?: string) {\n  return label;\n}\n';
  const c = census([fileManifest('a.ts', text)], [functionEntry('a.ts', text)]);
  const summary = classifyCensus(ts, c, resolverFor({ 'a.ts': text }));
  assert.equal(summary.matched_entries, 1);
  assert.equal(summary.optional_no_initializer_entries, 1);
});

test('resolveWithinRoot rejects paths that escape the source root', () => {
  const root = mkdtempSync(join(tmpdir(), 'source-proof-root-'));
  try {
    assert.throws(() => resolveWithinRoot(root, '../outside.ts'), /escapes source root/);
    writeFileSync(join(root, 'inside.ts'), '');
    assert.doesNotThrow(() => resolveWithinRoot(root, 'inside.ts'));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('source resolver refuses an escaping symlink and accepts an in-root link', () => {
  const root = mkdtempSync(join(tmpdir(), 'source-proof-links-'));
  try {
    mkdirSync(join(root, 'source'));
    writeFileSync(join(root, 'outside.ts'), '');
    writeFileSync(join(root, 'source', 'inside.ts'), '');
    symlinkSync(join(root, 'outside.ts'), join(root, 'source', 'escape.ts'));
    symlinkSync(join(root, 'source', 'inside.ts'), join(root, 'source', 'safe.ts'));
    assert.throws(() => resolveWithinRoot(join(root, 'source'), 'escape.ts'), /escapes source root/);
    assert.doesNotThrow(() => resolveWithinRoot(join(root, 'source'), 'safe.ts'));
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
});

test('run() produces an aggregate-only report with no source identities, paths, or contents', () => {
  const root = mkdtempSync(join(tmpdir(), 'source-proof-privacy-'));
  const secretName = 'confidential-module.ts';
  const text = 'function widget(label?: string) {\n  return label; // top secret marker\n}\n';
  try {
    writeFileSync(join(root, secretName), text);
    const censusPath = join(root, 'census.json');
    writeFileSync(censusPath, JSON.stringify(census([fileManifest(secretName, text)], [functionEntry(secretName, text)])));
    return run({ tsModulePath: TS_MODULE_PATH, censusPath, sourceRoot: root }).then(report => {
      assert.equal(report.schema, 'prism.parameter-site-source-proof/1');
      assert.equal(report.authorizes_runtime_edge, false);
      assert.equal(report.optional_no_initializer_entries, 1);
      const serialized = JSON.stringify(report);
      assert.doesNotMatch(serialized, /confidential|top secret|widget/);
      assert.doesNotMatch(serialized, new RegExp(root.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('CLI failures are sanitized and never echo the source root or file paths', () => {
  const root = mkdtempSync(join(tmpdir(), 'source-proof-cli-'));
  const scriptPath = new URL('./source-proof.mjs', import.meta.url).pathname;
  try {
    const missingCensusPath = join(root, 'missing-census.json');
    assert.throws(() => execFileSync('node', [scriptPath, TS_MODULE_PATH, missingCensusPath, root], { stdio: 'pipe' }));
    try {
      execFileSync('node', [scriptPath, TS_MODULE_PATH, missingCensusPath, root], { stdio: 'pipe' });
    } catch (error) {
      const stderr = error.stderr.toString('utf8');
      assert.equal(stderr, 'source-proof failed\n');
      assert.doesNotMatch(stderr, new RegExp(root.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
