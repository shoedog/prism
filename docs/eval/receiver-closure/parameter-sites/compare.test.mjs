import assert from 'node:assert/strict';
import test from 'node:test';

import { compare, validateCensus } from './compare.mjs';

const endpoint = (overrides = {}) => ({
  file: 'fixture.ts', function: 'f', function_start_line: 1, line: 1,
  path: 'x', start_byte: 12, end_byte: 13, ...overrides,
});

const census = (overrides = {}) => ({
  schema: 'prism.parameter-site-census/1',
  authorizes_runtime_edge: false,
  files: [{ file: 'fixture.ts', sha256: 'a'.repeat(64), language: 'TypeScript', parse_errors: 0 }],
  skipped: [],
  functions: [{
    file: 'fixture.ts', start_byte: 0, end_byte: 30, start_line: 1,
    owner_name: 'f', kind: 'function', parameter_recovery: true,
    parameters: [{ start_byte: 11, end_byte: 13, kind: 'required_parameter', form: 'identifier' }],
    slots: [{ name: 'x', start_byte: 12, end_byte: 13 }],
    occurrences: [{ name: 'x', start_byte: 20, end_byte: 21, bare_reference: true, dfg_def: false, cpg_def: false }],
  }],
  flows: [{ from: endpoint({ start_byte: 20, end_byte: 21, path: 'use' }), to: endpoint(), confidence: { exact: true } }],
  ...overrides,
});

const clone = value => structuredClone(value);

test('validates and summarizes a non-empty census without exposing identifiers or paths', () => {
  const result = compare(census(), census());
  assert.equal(result.population.files, 1);
  assert.equal(result.candidate.parameter_forms.identifier, 1);
  assert.equal(result.candidate.slot_matched_flows, 1);
  assert.equal(result.edge_changes.added, 0);
  assert.doesNotMatch(JSON.stringify(result), /fixture|\"f\"|\"x\"/);
});

test('reports definition and boolean deltas by language', () => {
  const base = census();
  base.functions[0].occurrences = [];
  const candidate = census();
  candidate.functions[0].occurrences[0].dfg_def = true;
  candidate.functions[0].occurrences[0].cpg_def = true;
  const result = compare(base, candidate);
  assert.deepEqual(result.delta, { functions: 0, owner_null: 0, parameters: 0, occurrences: 1, dfg_defs: 1, cpg_defs: 1, slot_matched_flows: 0, non_parameter_definition_flows: 0, flows: 0 });
  assert.equal(result.by_language.TypeScript.delta.cpg_defs, 1);
});

test('classifies an in-function non-parameter definition target', () => {
  const value = census();
  value.flows[0].to = endpoint({ path: 'x', start_byte: 20, end_byte: 21 });
  const result = compare(value, value);
  assert.equal(result.base.slot_matched_flows, 0);
  assert.equal(result.base.non_parameter_definition_flows, 1);
});

test('computes duplicate flow changes as a multiset and groups opaque confidence canonically', () => {
  const base = census();
  const candidate = census();
  base.flows.push(clone(base.flows[0]));
  candidate.flows[0].confidence = { z: 1, a: 2 };
  const result = compare(base, candidate);
  assert.deepEqual(result.edge_changes, { added: 1, removed: 2 });
  assert.equal(Object.values(result.candidate.confidence_groups)[0], 1);
  assert.match(Object.keys(result.candidate.confidence_groups)[0], /^[a-f0-9]{64}$/);
  candidate.flows[0].confidence = 'opaque-low';
  assert.equal(compare(candidate, candidate).candidate.flows, 1);
});

test('refuses schema and runtime-authority mismatches', () => {
  assert.throws(() => validateCensus(census({ schema: 'wrong' })), /schema/);
  assert.throws(() => compare(census(), census({ authorizes_runtime_edge: true })), /authorizes_runtime_edge/);
});

test('refuses empty populations and non-JS-family files', () => {
  assert.throws(() => validateCensus(census({ files: [], functions: [], flows: [] })), /non-empty/);
  const value = census(); value.files[0].language = 'Python';
  assert.throws(() => validateCensus(value), /language/);
});

test('refuses duplicate file, function, and occurrence identities', () => {
  const file = census(); file.files.push(clone(file.files[0]));
  assert.throws(() => validateCensus(file), /duplicate file/);
  const fn = census(); fn.functions.push(clone(fn.functions[0]));
  assert.throws(() => validateCensus(fn), /duplicate function/);
  const token = census(); token.functions[0].occurrences.push(clone(token.functions[0].occurrences[0]));
  assert.throws(() => validateCensus(token), /duplicate token/);
});

test('refuses impossible booleans and invalid token bounds', () => {
  const impossible = census(); impossible.functions[0].occurrences[0].cpg_def = true;
  assert.throws(() => validateCensus(impossible), /cpg_def requires dfg_def/);
  const bounds = census(); bounds.functions[0].slots[0].end_byte = 31;
  assert.throws(() => validateCensus(bounds), /within function/);
});

test('refuses changed files, skips, function shapes, parameters, and slots', () => {
  for (const mutate of [
    v => { v.files[0].sha256 = 'b'.repeat(64); },
    v => { v.skipped.push({ path: 'bad.ts', reason: 'parse failed' }); },
    v => { v.functions[0].kind = 'method'; },
    v => { v.functions[0].slots[0].start_byte = 11; },
    v => { v.functions[0].parameters[0].form = 'default_identifier'; },
  ]) {
    const candidate = census(); mutate(candidate);
    assert.throws(() => compare(census(), candidate), /population mismatch/);
  }
  const taggedSkip = census(); taggedSkip.skipped.push({ path: 'large.ts', reason: { TooLarge: { bytes: 123 } } });
  assert.equal(validateCensus(taggedSkip), taggedSkip);
});

test('refuses malformed flow endpoints and unknown top-level fields', () => {
  const endpointBounds = census(); endpointBounds.flows[0].to.end_byte = 12;
  assert.throws(() => validateCensus(endpointBounds), /positive range/);
  assert.throws(() => validateCensus(census({ secret_source: 'nope' })), /unexpected field/);
});
