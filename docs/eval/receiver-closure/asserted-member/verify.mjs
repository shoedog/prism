// Synthetic source/compiler/native observations only. No production authority.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const compilerSha = '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675';
const prelude = 'declare function factory(): any;\ndeclare function sideEffect(): void;\ndeclare const key: string, flag: boolean;\n';
const moduleSource = 'export class X {}';
const literalIdentifier = /^[A-Za-z_$][A-Za-z0-9_$]*$/;

// Detached proposed syntax allowlist, NOT a binding, receiver, or edge proof.
export function candidate(ts, expression, sourceFile, body, offset) {
  if (!expression || !ts.isPropertyAccessExpression(expression)) return { verdict: 'not-dot-member' };
  if (expression.questionDotToken) return { verdict: 'optional-member' };
  if (!ts.isIdentifier(expression.name) || !literalIdentifier.test(expression.name.getText(sourceFile)))
    return { verdict: 'nonliteral-field' };
  let receiver = expression.expression;
  const wrappers = [];
  while (ts.isParenthesizedExpression(receiver) || ts.isAsExpression(receiver) || ts.isSatisfiesExpression(receiver)) {
    wrappers.push(ts.SyntaxKind[receiver.kind]);
    if (wrappers.length > 8) return { verdict: 'wrapper-budget' };
    receiver = receiver.expression;
  }
  if (!ts.isIdentifier(receiver)) return { verdict: 'receiver-not-identifier' };
  if (!literalIdentifier.test(receiver.getText(sourceFile))) return { verdict: 'nonliteral-receiver' };
  const location = node => {
    const start = node.getStart(sourceFile) - offset, end = node.end - offset;
    assert(start >= 0 && end <= body.length);
    return { utf16: [start, end], utf8: [Buffer.byteLength(body.slice(0, start)), Buffer.byteLength(body.slice(0, end))] };
  };
  return { verdict: 'candidate', path: { base: receiver.text, fields: [expression.name.text] },
    wrappers, member: location(expression), receiver: location(receiver), field: location(expression.name) };
}

function returnExpression(ts, file) {
  const owner = file.statements.find(n => ts.isFunctionDeclaration(n) && n.name?.text === 'owner');
  return owner?.body?.statements.find(n => ts.isReturnStatement(n))?.expression;
}

function emittedPath(ts, emitted) {
  const file = ts.createSourceFile('emitted.js', emitted, ts.ScriptTarget.ES2022, true, ts.ScriptKind.JS);
  const expression = returnExpression(ts, file);
  if (!expression || !ts.isPropertyAccessExpression(expression) || expression.questionDotToken) return null;
  let receiver = expression.expression;
  while (ts.isParenthesizedExpression(receiver)) receiver = receiver.expression;
  return ts.isIdentifier(receiver) && ts.isIdentifier(expression.name)
    ? { base: receiver.text, fields: [expression.name.text] } : null;
}

function compile(ts, fixture, language, body) {
  const filename = `/virtual/fixture.${language === 'tsx' ? 'tsx' : 'ts'}`;
  const options = { strict: true, types: [], target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.Preserve };
  const virtual = { [filename]: prelude + body, '/virtual/m.ts': moduleSource };
  const host = ts.createCompilerHost(options);
  const originalGet = host.getSourceFile.bind(host), originalRead = host.readFile.bind(host), originalExists = host.fileExists.bind(host);
  host.readFile = p => Object.hasOwn(virtual, p) ? virtual[p] : originalRead(p);
  host.fileExists = p => Object.hasOwn(virtual, p) || originalExists(p);
  host.directoryExists = p => p === '/virtual' || ts.sys.directoryExists(p);
  host.getSourceFile = (p, version, ...rest) => Object.hasOwn(virtual, p)
    ? ts.createSourceFile(p, virtual[p], version, true, p.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS)
    : originalGet(p, version, ...rest);
  const emitted = {};
  host.writeFile = (p, text) => { emitted[p] = text; };
  const program = ts.createProgram([filename], options, host);
  const file = program.getSourceFile(filename);
  const diagnostics = ts.getPreEmitDiagnostics(program).map(d => ({ code: d.code, file: d.file?.fileName,
    start: d.start, length: d.length, message: ts.flattenDiagnosticMessageText(d.messageText, '\n') }));
  const syntacticDiagnostics = program.getSyntacticDiagnostics(file).map(d => d.code);
  const emit = program.emit();
  const output = emitted[filename.replace(/\.tsx?$/, language === 'tsx' ? '.jsx' : '.js')];
  const expression = returnExpression(ts, file);
  return { diagnostics, syntacticDiagnostics, emitDiagnostics: emit.diagnostics.map(d => d.code),
    fullSourceSha256: sha(prelude + body), emitted: output, emittedSha256: output === undefined ? null : sha(output),
    emittedPath: output === undefined ? null : emittedPath(ts, output),
    proposed: diagnostics.length ? { verdict: 'compiler-invalid' } : candidate(ts, expression, file, body, prelude.length) };
}

export function validateFixtures(data) {
  assert(data && typeof data === 'object' && !Array.isArray(data));
  assert.deepEqual(Object.keys(data).sort(), ['cases', 'schema']);
  assert.equal(data.schema, 'prism.asserted-member-fixtures/1');
  assert(Array.isArray(data.cases) && data.cases.length > 0 && data.cases.length <= 32);
  const seen = new Set();
  for (const f of data.cases) {
    assert(f && typeof f === 'object' && !Array.isArray(f));
    assert(Object.keys(f).every(k => ['id', 'expression', 'role', 'verdict', 'diagnostic', 'tsx_invalid'].includes(k)));
    assert(typeof f.id === 'string' && f.id.length > 0 && !seen.has(f.id), 'missing/duplicate fixture id');
    seen.add(f.id);
    assert(typeof f.expression === 'string' && f.expression.length > 0 && !f.expression.includes('\n'));
    assert(['control', 'identity-control', 'repair-candidate', 'refusal', 'invalid'].includes(f.role));
    const refusals = ['not-dot-member', 'optional-member', 'nonliteral-field', 'wrapper-budget', 'receiver-not-identifier', 'nonliteral-receiver'];
    assert(f.role === 'invalid' ? f.verdict === 'compiler-invalid' : f.role === 'refusal'
      ? refusals.includes(f.verdict) : f.verdict === 'candidate');
    if (f.diagnostic !== undefined) assert(Number.isInteger(f.diagnostic) && f.diagnostic > 0);
    if (f.role === 'invalid') assert(f.diagnostic !== undefined);
    if (f.tsx_invalid !== undefined) assert(typeof f.tsx_invalid === 'boolean');
  }
  for (const role of ['control', 'identity-control', 'repair-candidate', 'refusal', 'invalid'])
    assert(data.cases.some(f => f.role === role), `missing ${role} obligations`);
}

export function run(compilerPath, nativePath, requireRepaired = false) {
  const compilerBytes = readFileSync(compilerPath), nativeBytes = readFileSync(nativePath);
  assert.equal(sha(compilerBytes), compilerSha, 'compiler pin mismatch');
  const ts = createRequire(import.meta.url)(compilerPath);
  assert.equal(ts.version, '5.9.3');
  const fixtureUrl = new URL('./fixtures.json', import.meta.url);
  const fixtureBytes = readFileSync(fixtureUrl), fixtures = JSON.parse(fixtureBytes);
  validateFixtures(fixtures);
  const requests = fixtures.cases.flatMap(f => ['typescript', 'tsx'].map(language => ({
    id: `${f.id}/${language}`, language,
    source: `function owner(runtime: any, value: any) {\n runtime.X = value;\n return ${f.expression};\n}`,
    lines: [3],
  })));
  const native = JSON.parse(execFileSync(nativePath, [], { input: JSON.stringify(requests), encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024, timeout: 60_000 }));
  assert(Array.isArray(native) && native.length === requests.length, 'native population mismatch');
  const failures = [], results = [];
  for (let i = 0; i < requests.length; i++) {
    const request = requests[i], observed = native[i], fixture = fixtures.cases[Math.floor(i / 2)];
    assert.equal(observed.id, request.id);
    assert.equal(observed.language, request.language);
    assert.equal(observed.source, request.source);
    const compiler = compile(ts, fixture, request.language, request.source);
    const expectedInvalid = fixture.role === 'invalid' || (request.language === 'tsx' && fixture.tsx_invalid === true);
    const expectedVerdict = expectedInvalid ? 'compiler-invalid' : fixture.verdict;
    const check = (ok, reason) => { if (!ok) failures.push({ id: request.id, reason }); };
    check(compiler.proposed.verdict === expectedVerdict, `verdict ${compiler.proposed.verdict}, expected ${expectedVerdict}`);
    if (expectedInvalid) {
      check(compiler.diagnostics.length > 0, 'expected compiler rejection missing');
      if (fixture.diagnostic) check(compiler.diagnostics.some(d => d.code === fixture.diagnostic), 'expected diagnostic absent');
      if (fixture.tsx_invalid) check(compiler.syntacticDiagnostics.length > 0, 'expected TSX syntax rejection absent');
    } else {
      check(compiler.diagnostics.length === 0 && compiler.emitDiagnostics.length === 0 && compiler.emitted !== undefined, 'compiler validity/emit failed');
      check(observed.parse_errors === 0, 'native parse error in compiler-valid fixture');
    }
    const pathEqual = p => p.base === 'runtime' && JSON.stringify(p.fields) === '["X"]';
    const matchingEdges = observed.full.edges.filter(e => pathEqual(e.from.path) && pathEqual(e.to.path) && e.from.line === 2 && e.to.line === 3);
    check(JSON.stringify(observed.full) === JSON.stringify(observed.subset), 'full/subset mismatch');
    if (expectedVerdict === 'candidate' && compiler.proposed.verdict === 'candidate') {
      check(JSON.stringify(compiler.emittedPath) === JSON.stringify(compiler.proposed.path), 'candidate emit path mismatch');
      const [start, end] = compiler.proposed.receiver.utf8;
      check(Buffer.from(request.source).subarray(start, end).toString() === compiler.proposed.path.base, 'receiver byte span mismatch');
      const [fieldStart, fieldEnd] = compiler.proposed.field.utf8;
      check(Buffer.from(request.source).subarray(fieldStart, fieldEnd).toString() === compiler.proposed.path.fields[0], 'field byte span mismatch');
      if (fixture.role === 'control') check(matchingEdges.length > 0, 'plain control lacks field edge');
      if (fixture.role === 'identity-control') {
        const toRead = observed.full.edges.filter(e => pathEqual(e.from.path) && e.from.line === 2 && e.to.line === 3);
        check(toRead.length === 0, 'different receiver/field gained flow from runtime.X');
      }
      if (requireRepaired && fixture.role === 'repair-candidate') check(matchingEdges.length > 0, 'UNFIXED: missing field-write-to-read edge');
    }
    results.push({ id: request.id, role: fixture.role, expectedInvalid, sourceSha256: sha(request.source),
      compiler, native: observed, matchingFieldEdges: matchingEdges.length });
  }
  assert.equal(sha(readFileSync(compilerPath)), compilerSha);
  assert.equal(sha(readFileSync(nativePath)), sha(nativeBytes));
  assert.equal(sha(readFileSync(fixtureUrl)), sha(fixtureBytes));
  return { schema: 'prism.asserted-member-observations/1', authorizesRuntimeEdge: false,
    productionNormalization: false, requireRepaired, compiler: { version: ts.version, sha256: compilerSha },
    nativeSha256: sha(nativeBytes), fixtureSha256: sha(fixtureBytes), verifierSha256: sha(readFileSync(fileURLToPath(import.meta.url))),
    prelude, moduleSource, emittedCodeExecuted: false, results, failures,
    summary: { fixtureDefinitions: fixtures.cases.length, observations: results.length,
      compilerValid: results.filter(r => !r.expectedInvalid && r.compiler.diagnostics.length === 0).length,
      expectedInvalid: results.filter(r => r.expectedInvalid).length,
      candidates: results.filter(r => r.compiler.proposed.verdict === 'candidate').length,
      missingCandidateEdges: results.filter(r => r.role === 'repair-candidate' && r.matchingFieldEdges === 0).length,
      failures: failures.length, skipped: 0 } };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [compiler, native, mode, ...extra] = process.argv.slice(2);
  if (!compiler || !native || extra.length || (mode && mode !== '--require-repaired'))
    throw new Error('usage: node verify.mjs <pinned-typescript.js> <native-observer> [--require-repaired]');
  const receipt = run(compiler, native, mode === '--require-repaired');
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
  process.exitCode = receipt.failures.length ? 1 : 0;
}
