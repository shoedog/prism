// Compare same-environment base/candidate receipts, never execute fixture JS.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const [basePath, repairedPath, ...extra] = process.argv.slice(2);
assert(basePath && repairedPath && !extra.length, 'expected base and repaired receipts');
const base = JSON.parse(readFileSync(basePath));
const repaired = JSON.parse(readFileSync(repairedPath));
assert.equal(base.schema, 'prism.asserted-member-observations/1');
assert.equal(repaired.schema, base.schema);
assert.equal(base.requireRepaired, true);
assert.equal(repaired.requireRepaired, true);
assert.equal(base.fixtureSha256, repaired.fixtureSha256);
assert.equal(base.compiler.sha256, repaired.compiler.sha256);
assert.equal(base.verifierSha256, repaired.verifierSha256);
assert.equal(base.summary.observations, 54);
assert.equal(repaired.summary.observations, 54);
assert.equal(base.results.length, 54);
assert.equal(repaired.results.length, 54);
assert.equal(base.failures.length, 12);
assert(base.failures.every(f => f.reason === 'UNFIXED: missing field-write-to-read edge'));
assert.deepEqual(repaired.failures, []);
assert.equal(base.emittedCodeExecuted, false);
assert.equal(repaired.emittedCodeExecuted, false);
let refusals = 0, repairs = 0;
for (const [i, before] of base.results.entries()) {
  const after = repaired.results[i];
  assert.equal(before.id, after.id);
  assert.equal(before.role, after.role);
  assert.deepEqual(before.compiler, after.compiler);
  for (const key of ['id', 'language', 'source', 'parse_errors', 'tree', 'calls', 'names', 'returns'])
    assert.deepEqual(before.native[key], after.native[key], `${before.id}: raw ${key} changed`);
  assert.deepEqual(after.native.full, after.native.subset, `${before.id}: subset mismatch`);
  if (before.role === 'refusal') {
    refusals++;
    assert.deepEqual(before.native, after.native, `${before.id}: refused native observation changed`);
  }
  if (before.role === 'repair-candidate') {
    repairs++;
    assert.equal(before.matchingFieldEdges, 0);
    assert.equal(after.matchingFieldEdges, 1);
    const member = after.compiler.proposed;
    for (const [path, [start, end]] of [
      [member.path, member.member.utf8],
      [{ base: member.path.base, fields: [] }, member.receiver.utf8],
    ]) assert(after.native.spans.some(s => JSON.stringify(s.path) === JSON.stringify(path)
      && s.start_byte === start && s.end_byte === end), `${before.id}: exact occurrence missing`);
  }
}
assert.equal(refusals, 26);
assert.equal(repairs, 12);
console.log(JSON.stringify({ observations: 54, repaired: repairs, unchangedRefusals: refusals,
  rawSourceTreeCallsNamesReturnsUnchanged: true, exactCompilerSpans: true,
  fullSubsetParity: true, failures: [] }, null, 2));
