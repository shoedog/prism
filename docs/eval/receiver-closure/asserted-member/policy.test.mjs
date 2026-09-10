import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { validateFixtures } from './verify.mjs';

const fixtures = JSON.parse(readFileSync(new URL('./fixtures.json', import.meta.url)));
const fresh = () => structuredClone(fixtures);

test('fixed proof population contains positive, control and refusal obligations', () => {
  validateFixtures(fixtures);
  assert.equal(fixtures.cases.length, 27);
  assert.equal(fixtures.cases.filter(f => f.role === 'repair-candidate').length, 6);
  assert.equal(fixtures.cases.filter(f => f.role === 'control').length, 2);
  assert.equal(fixtures.cases.filter(f => f.role === 'identity-control').length, 4);
  assert.equal(fixtures.cases.filter(f => f.role === 'invalid').length, 2);
});

test('rejects unknown envelope, schema and case fields', () => {
  for (const bad of [null, [], {}, { ...fresh(), extra: true }, { ...fresh(), schema: 'other' }])
    assert.throws(() => validateFixtures(bad));
  const bad = fresh(); bad.cases[0].typo = true;
  assert.throws(() => validateFixtures(bad));
});

test('rejects missing, empty and oversized populations', () => {
  for (const cases of [undefined, [], new Array(33).fill(fixtures.cases[0])])
    assert.throws(() => validateFixtures({ schema: fixtures.schema, cases }));
});

test('rejects duplicate and empty identity', () => {
  const bad = fresh(); bad.cases[1].id = bad.cases[0].id;
  assert.throws(() => validateFixtures(bad));
  bad.cases[1].id = '';
  assert.throws(() => validateFixtures(bad));
});

test('rejects unsupported expressions and inconsistent policy expectations', () => {
  for (const patch of [{ expression: '' }, { expression: 'a\nb' }, { expression: 1 },
    { role: 'unknown' }, { verdict: 'optional-member' }, { role: 'refusal', verdict: 'candidate' },
    { role: 'invalid', verdict: 'compiler-invalid' }]) {
    const bad = fresh(); Object.assign(bad.cases[0], patch);
    assert.throws(() => validateFixtures(bad), JSON.stringify(patch));
  }
});

test('rejects malformed optional diagnostic and dialect obligations', () => {
  for (const patch of [{ diagnostic: 0 }, { diagnostic: '2304' }, { tsx_invalid: 'true' }]) {
    const bad = fresh(); Object.assign(bad.cases[0], patch);
    assert.throws(() => validateFixtures(bad));
  }
});

test('requires every obligation class so repair checking cannot pass vacuously', () => {
  for (const role of ['control', 'identity-control', 'repair-candidate', 'refusal', 'invalid']) {
    const bad = fresh(); bad.cases = bad.cases.filter(f => f.role !== role);
    assert.throws(() => validateFixtures(bad), role);
  }
});
