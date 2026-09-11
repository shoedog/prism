import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const SCHEMA = 'prism.parameter-site-census/1';
const LANGUAGES = new Set(['JavaScript', 'TypeScript', 'Tsx', 'TSX']);
const TOP_KEYS = ['schema', 'authorizes_runtime_edge', 'files', 'skipped', 'functions', 'flows'];
const FILE_KEYS = ['file', 'sha256', 'language', 'parse_errors'];
const SKIP_KEYS = ['path', 'reason'];
const FUNCTION_KEYS = ['file', 'start_byte', 'end_byte', 'start_line', 'owner_name', 'kind', 'parameter_recovery', 'parameters', 'slots', 'occurrences'];
const PARAMETER_KEYS = ['start_byte', 'end_byte', 'kind', 'form'];
const SLOT_KEYS = ['name', 'start_byte', 'end_byte'];
const OCCURRENCE_KEYS = ['name', 'start_byte', 'end_byte', 'bare_reference', 'dfg_def', 'cpg_def'];
const FLOW_KEYS = ['from', 'to', 'confidence'];
const ENDPOINT_KEYS = ['file', 'function', 'function_start_line', 'line', 'path', 'start_byte', 'end_byte'];
const METRICS = ['functions', 'owner_null', 'parameters', 'occurrences', 'dfg_defs', 'cpg_defs', 'slot_matched_flows', 'unmatched_slot_flows', 'flows'];

function fail(message) { throw new TypeError(`invalid parameter-site census: ${message}`); }
function object(value, label) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail(`${label} must be an object`);
}
function exactKeys(value, keys, label) {
  object(value, label);
  const actual = Object.keys(value);
  const expected = new Set(keys);
  const extra = actual.find(key => !expected.has(key));
  if (extra !== undefined) fail(`${label} has unexpected field`);
  if (keys.some(key => !Object.hasOwn(value, key))) fail(`${label} is missing required field`);
}
function array(value, label) { if (!Array.isArray(value)) fail(`${label} must be an array`); }
function string(value, label, nullable = false) {
  if ((nullable && value === null) || (typeof value === 'string' && value.length > 0)) return;
  fail(`${label} must be ${nullable ? 'a non-empty string or null' : 'a non-empty string'}`);
}
function integer(value, label, minimum = 0) {
  if (!Number.isSafeInteger(value) || value < minimum) fail(`${label} must be an integer >= ${minimum}`);
}
function bool(value, label) { if (typeof value !== 'boolean') fail(`${label} must be boolean`); }
function json(value, label, seen = new Set()) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return;
  if (typeof value === 'number' && Number.isFinite(value)) return;
  if (typeof value !== 'object' || seen.has(value)) fail(`${label} must be finite, acyclic JSON`);
  seen.add(value);
  if (Array.isArray(value)) value.forEach(item => json(item, label, seen));
  else for (const [key, item] of Object.entries(value)) {
    if (typeof key !== 'string' || item === undefined) fail(`${label} must be JSON`);
    json(item, label, seen);
  }
  seen.delete(value);
}
function range(value, label, fn) {
  integer(value.start_byte, `${label}.start_byte`);
  integer(value.end_byte, `${label}.end_byte`);
  if (value.end_byte <= value.start_byte) fail(`${label} must have a positive range`);
  if (fn && (value.start_byte < fn.start_byte || value.end_byte > fn.end_byte)) fail(`${label} range must be within function`);
}
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  return JSON.stringify(value);
}
function digest(value) { return createHash('sha256').update(canonical(value)).digest('hex'); }
function unique(values, label) {
  const seen = new Set();
  for (const value of values) {
    if (seen.has(value)) fail(`duplicate ${label} identity`);
    seen.add(value);
  }
}

export function validateCensus(value) {
  exactKeys(value, TOP_KEYS, 'root');
  if (value.schema !== SCHEMA) fail('schema mismatch');
  if (value.authorizes_runtime_edge !== false) fail('authorizes_runtime_edge must be false');
  array(value.files, 'files'); array(value.skipped, 'skipped'); array(value.functions, 'functions'); array(value.flows, 'flows');
  if (value.files.length === 0 || value.functions.length === 0) fail('files and functions must be non-empty');

  for (const file of value.files) {
    exactKeys(file, FILE_KEYS, 'file'); string(file.file, 'file.file');
    if (!/^[a-f0-9]{64}$/i.test(file.sha256)) fail('file.sha256 must be a SHA-256 hex digest');
    if (!LANGUAGES.has(file.language)) fail('file.language must be JavaScript, TypeScript, or TSX');
    integer(file.parse_errors, 'file.parse_errors');
  }
  unique(value.files.map(file => file.file), 'file');
  const files = new Set(value.files.map(file => file.file));
  for (const skip of value.skipped) {
    exactKeys(skip, SKIP_KEYS, 'skipped entry'); string(skip.path, 'skipped.path'); json(skip.reason, 'skipped.reason');
  }
  unique(value.skipped.map(skip => skip.path), 'skipped path');

  for (const fn of value.functions) {
    exactKeys(fn, FUNCTION_KEYS, 'function'); string(fn.file, 'function.file');
    if (!files.has(fn.file)) fail('function references an unloaded file');
    range(fn, 'function'); integer(fn.start_line, 'function.start_line', 1);
    string(fn.owner_name, 'function.owner_name', true); string(fn.kind, 'function.kind'); bool(fn.parameter_recovery, 'function.parameter_recovery');
    array(fn.parameters, 'function.parameters'); array(fn.occurrences, 'function.occurrences');
    if (fn.slots !== null) array(fn.slots, 'function.slots');
    for (const parameter of fn.parameters) {
      exactKeys(parameter, PARAMETER_KEYS, 'parameter'); range(parameter, 'parameter', fn);
      string(parameter.kind, 'parameter.kind'); string(parameter.form, 'parameter.form');
    }
    unique(fn.parameters.map(p => `${p.start_byte}:${p.end_byte}`), 'parameter');
    for (const slot of fn.slots ?? []) {
      exactKeys(slot, SLOT_KEYS, 'slot'); string(slot.name, 'slot.name'); range(slot, 'slot', fn);
      if (!fn.parameters.some(parameter => slot.start_byte >= parameter.start_byte && slot.end_byte <= parameter.end_byte)) fail('slot range must be within parameter syntax');
    }
    unique((fn.slots ?? []).map(slot => `${slot.start_byte}:${slot.end_byte}:${slot.name}`), 'slot');
    for (const occurrence of fn.occurrences) {
      exactKeys(occurrence, OCCURRENCE_KEYS, 'occurrence'); string(occurrence.name, 'occurrence.name'); range(occurrence, 'occurrence', fn);
      bool(occurrence.bare_reference, 'occurrence.bare_reference'); bool(occurrence.dfg_def, 'occurrence.dfg_def'); bool(occurrence.cpg_def, 'occurrence.cpg_def');
      if (!fn.parameters.some(parameter => occurrence.start_byte >= parameter.start_byte && occurrence.end_byte <= parameter.end_byte)) fail('occurrence range must be within parameter syntax');
      if (occurrence.dfg_def && !occurrence.bare_reference) fail('occurrence.dfg_def requires bare_reference');
      if (occurrence.dfg_def && fn.owner_name === null) fail('occurrence.dfg_def requires a non-null owner');
      if (occurrence.cpg_def && !occurrence.dfg_def) fail('occurrence.cpg_def requires dfg_def');
    }
    unique(fn.occurrences.map(o => `${o.start_byte}:${o.end_byte}:${o.name}`), 'token');
  }
  unique(value.functions.map(fn => `${fn.file}:${fn.start_byte}:${fn.end_byte}`), 'function');
  if (value.functions.reduce((count, fn) => count + fn.parameters.length, 0) === 0) fail('parameter population must be non-empty');

  for (const flow of value.flows) {
    exactKeys(flow, FLOW_KEYS, 'flow'); json(flow.confidence, 'flow.confidence');
    for (const side of ['from', 'to']) {
      const endpoint = flow[side]; exactKeys(endpoint, ENDPOINT_KEYS, `flow.${side}`);
      string(endpoint.file, `flow.${side}.file`); string(endpoint.function, `flow.${side}.function`, true);
      integer(endpoint.function_start_line, `flow.${side}.function_start_line`, 1); integer(endpoint.line, `flow.${side}.line`, 1);
      string(endpoint.path, `flow.${side}.path`); range(endpoint, `flow.${side}`);
      if (!files.has(endpoint.file)) fail(`flow.${side} references an unloaded file`);
    }
  }
  return value;
}

function staticFunction(fn) {
  return { file: fn.file, start_byte: fn.start_byte, end_byte: fn.end_byte, start_line: fn.start_line, owner_name: fn.owner_name, kind: fn.kind,
    parameter_recovery: fn.parameter_recovery, parameters: fn.parameters, slots: fn.slots };
}
function assertSamePopulation(base, candidate) {
  const left = { files: base.files, skipped: base.skipped, functions: base.functions.map(staticFunction) };
  const right = { files: candidate.files, skipped: candidate.skipped, functions: candidate.functions.map(staticFunction) };
  if (canonical(left) !== canonical(right)) throw new Error('parameter-site population mismatch');
}
function blank() {
  return { parameter_forms: {}, functions: 0, owner_null: 0, parameters: 0, occurrences: 0, dfg_defs: 0, cpg_defs: 0,
    slot_matched_flows: 0, unmatched_slot_flows: 0, flows: 0, confidence_groups: {} };
}
function summarize(census, onlyLanguage = null) {
  const result = blank();
  const fileLanguages = new Map(census.files.map(file => [file.file, file.language]));
  const functions = census.functions.filter(fn => onlyLanguage === null || fileLanguages.get(fn.file) === onlyLanguage);
  result.functions = functions.length;
  for (const fn of functions) {
    result.owner_null += Number(fn.owner_name === null); result.parameters += fn.parameters.length; result.occurrences += fn.occurrences.length;
    result.dfg_defs += fn.occurrences.filter(o => o.dfg_def).length; result.cpg_defs += fn.occurrences.filter(o => o.cpg_def).length;
    for (const parameter of fn.parameters) result.parameter_forms[parameter.form] = (result.parameter_forms[parameter.form] ?? 0) + 1;
  }
  const includedFiles = new Set(functions.map(fn => fn.file));
  const flows = census.flows.filter(flow => onlyLanguage === null || includedFiles.has(flow.to.file));
  result.flows = flows.length;
  for (const flow of flows) {
    const matched = functions.some(fn => fn.file === flow.to.file && fn.start_line === flow.to.function_start_line && fn.owner_name === flow.to.function
      && (fn.slots ?? []).some(slot => slot.name === flow.to.path && slot.start_byte === flow.to.start_byte && slot.end_byte === flow.to.end_byte));
    result.slot_matched_flows += Number(matched); result.unmatched_slot_flows += Number(!matched);
    const group = digest(flow.confidence); result.confidence_groups[group] = (result.confidence_groups[group] ?? 0) + 1;
  }
  return result;
}
function delta(base, candidate) { return Object.fromEntries(METRICS.map(key => [key, candidate[key] - base[key]])); }
function multisetDifference(base, candidate) {
  const counts = values => values.reduce((map, value) => map.set(canonical(value), (map.get(canonical(value)) ?? 0) + 1), new Map());
  const left = counts(base.flows), right = counts(candidate.flows); let added = 0, removed = 0;
  for (const [key, count] of right) added += Math.max(0, count - (left.get(key) ?? 0));
  for (const [key, count] of left) removed += Math.max(0, count - (right.get(key) ?? 0));
  return { added, removed };
}

export function compare(baseValue, candidateValue) {
  const base = validateCensus(baseValue), candidate = validateCensus(candidateValue);
  assertSamePopulation(base, candidate);
  const baseSummary = summarize(base), candidateSummary = summarize(candidate);
  const languages = [...new Set(base.files.map(file => file.language))].sort();
  const by_language = Object.fromEntries(languages.map(language => {
    const before = summarize(base, language), after = summarize(candidate, language);
    return [language, { base: before, candidate: after, delta: delta(before, after) }];
  }));
  return { schema: 'prism.parameter-site-comparison/1', authorizes_runtime_edge: false,
    population: { files: base.files.length, skipped: base.skipped.length, functions: base.functions.length },
    base: baseSummary, candidate: candidateSummary, delta: delta(baseSummary, candidateSummary), by_language,
    edge_changes: multisetDifference(base, candidate) };
}

async function main() {
  if (process.argv.length !== 4) throw new Error('usage: node compare.mjs base.json candidate.json');
  const [base, candidate] = await Promise.all(process.argv.slice(2).map(async path => JSON.parse(await readFile(path, 'utf8'))));
  process.stdout.write(`${JSON.stringify(compare(base, candidate), null, 2)}\n`);
}
if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) main().catch(error => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
