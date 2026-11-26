'use strict';

const fs = require('node:fs');
const path = require('node:path');
const Module = require('node:module');
const { buildContext, readBound, sourceExtension, utf8 } = require('./bytes.cjs');
const { loadQuestion } = require('./question_loader.cjs');
const argument = require('./questions/argument_binding.cjs');
const lexical = require('./questions/lexical_binding.cjs');
const calls = require('./questions/calls.cjs');
const { bodyFor, identity } = require('./questions/callable.cjs');

const fallback = 'relation-not-graded';
const reserved = 'input-admission';
const builtins = { argument_binding: argument, lexical_binding: lexical, call_target: calls,
  [fallback]: { property: fallback, grade: () => ({ verdict: 'UNDECIDED', property: fallback, why: fallback }) } };
const forms = ['named-function', 'named-method', 'decorated-method'];
const verdictVocabulary = ['CORRECT', 'WRONG', 'UNDECIDED', 'INADMISSIBLE'];
const zeros = () => Object.fromEntries(verdictVocabulary.map(key => [key, 0]));
const blank = value => typeof value !== 'string' || !value.trim();
const pair = (relation, property) => JSON.stringify([relation, property]);
const order = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const object = () => Object.create(null);

function canonical(value) {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  return '{' + Object.keys(value).filter(key => value[key] !== undefined).sort(order)
    .map(key => JSON.stringify(key) + ':' + canonical(value[key])).join(',') + '}';
}

function keys(value, required, optional = []) {
  return value && typeof value === 'object' && !Array.isArray(value)
    && required.every(key => Object.hasOwn(value, key))
    && Object.keys(value).every(key => required.includes(key) || optional.includes(key));
}

function checker(pins) {
  // Compile precisely the bytes read and verified in this process. A Node
  // module-cache hit or a second pathname read cannot substitute the checker.
  const file = path.join(pins.root, pins.entry);
  const tsModule = new Module(file);
  tsModule.filename = file;
  tsModule.paths = [];
  tsModule._compile(utf8.decode(readBound(pins.root, pins.entry, pins.files[pins.entry])), file);
  if (tsModule.exports.version !== '5.9.3') throw Error('checker-version: expected 5.9.3');
  return tsModule.exports;
}

function projectKey(row, index) {
  if (index !== null) {
    const site = row.source_span;
    const target = row.resolved_targets[index].function_id;
    return { site: { file: site.file, line: site.line, start_byte: site.start_byte, end_byte: site.end_byte },
      target: { file: target.file, name: target.name, start_line: target.start_line, end_line: target.end_line } };
  }
  const endpoint = ({ file, line, path, start_byte, end_byte }) => ({
    file, line, base: path.base, fields: path.fields, start_byte, end_byte,
  });
  return { from: endpoint(row.from), to: endpoint(row.to) };
}

function infer(row, index) {
  if (index !== null) return row.resolved_targets[index].confidence === 'exact' ? 'call_target' : fallback;
  if (row.from.access === 'use' && row.to.access === 'def') return 'argument_binding';
  if (row.from.access === 'def' && row.to.access === 'use') return 'lexical_binding';
  return fallback;
}

function choose(row, index, explicit, question) {
  const inferred = infer(row, index);
  const declared = question?.relations ?? [];
  const relation = explicit ?? (declared.includes(row.relation) ? row.relation
    : declared.includes(inferred) ? inferred
      : inferred === fallback && declared.length === 1 ? declared[0] : inferred);
  return { relation, custom: declared.includes(relation) };
}

function admitSite(record, context, qualified) {
  return context.coordinate(record.source_span) ||
    (calls.callerContradicts(record, context, qualified) ? 'caller-identity-mismatch' : null);
}

function grade(claim, source, context, question, explicit, labels, qualified, censusEdges) {
  const { row, arm, index, change } = claim;
  const { relation, custom } = choose(row, index, explicit, question);
  const builtin = Object.hasOwn(builtins, relation) ? builtins[relation] : builtins[fallback];
  const properties = labels.permitted.get(relation);
  // Inadmissible lane claims carry the one graded property, else the reserved label.
  const property = custom ? (properties?.size === 1 && !labels.admission.has(relation)
    ? [...properties][0] : reserved) : builtin.property;
  const target = index !== null;
  let why = null;
  if (!source.matches || !source.pair) why = 'source-digest-mismatch';
  else if (target) why = admitSite(row, context, qualified);
  else {
    why = context.coordinate(row.from, row.from.access === 'def')
      || context.coordinate(row.to, row.to.access === 'def');
    if (!why && censusEdges && (!sourceExtension.test(row.from.file) || !sourceExtension.test(row.to.file))) {
      return null;
    }
  }
  let graded;
  if (why) graded = { verdict: 'INADMISSIBLE', property, why };
  else if (custom) {
    graded = question.grade(structuredClone(row), { ...context.public, relation, arm,
      target_index: index, change: change === null ? null : structuredClone(change) });
    if (!keys(graded, ['property', 'verdict', 'why']) || blank(graded.property) || blank(graded.why)
        || !verdictVocabulary.includes(graded.verdict)
        || !properties?.has(graded.property)) throw Error('question-result: invalid or unchecked verdict/property/why');
  } else {
    const typeMatches = target === (relation === 'call_target');
    graded = typeMatches ? builtin.grade(row, context, index, qualified)
      : { verdict: 'UNDECIDED', property, why: fallback };
  }
  const verdict = { key: projectKey(row, index), arm, relation, ...graded, row };
  if (target) verdict.target_index = index;
  return verdict;
}

function records(bytes) {
  return utf8.decode(bytes).split('\n').filter(line => /[^ \t\r]/.test(line)).map(line => JSON.parse(line));
}

const namedBoundary = new Set(['structured-path', 'type-only-declaration', 'slot-unproved',
  'ambiguous-dispatch', 'external-callable', 'target-id-unproved', 'endpoint-coordinate-off', 'endpoint-spelling-mismatch', fallback]);

function selfcheck(job, ts, question) {
  const permitted = new Map();
  const admission = new Set();
  const expectedPairs = new Map();
  const coverage = new Map();
  const fixtures = [];
  let expectedCount = 0;
  for (const test of job.cases) {
    const read = key => records(readBound(test.root, key, test.files[key]));
    const expected = read('expected.jsonl');
    const inputs = read('input.jsonl');
    if (!expected.length || inputs.length !== expected.length) throw Error('selfcheck-cardinality: ' + test.id);
    const source = { root: path.join(test.root, 'repo'), matches: true, pair: true, files: object() };
    for (const [key, hash] of Object.entries(test.files)) {
      if (key.startsWith('repo/')) source.files[key.slice(5)] = hash;
    }
    const caseModule = test.prefix === 'lane' ? question : null;
    for (const golden of expected) {
      if (!keys(golden, ['key', 'relation', 'property', 'verdict'], ['why'])
          || blank(golden.relation) || blank(golden.property) || !verdictVocabulary.includes(golden.verdict)
          || (golden.why !== undefined && blank(golden.why))) throw Error('selfcheck-schema: expected row');
      // The label belongs to inadmissible lane claims; a decided golden cannot borrow it.
      if (golden.property === reserved && golden.verdict !== 'INADMISSIBLE') {
        throw Error('selfcheck-schema: reserved input-admission label');
      }
      let properties = expectedPairs.get(golden.relation);
      if (!properties) expectedPairs.set(golden.relation, properties = new Set());
      properties.add(golden.property);
      const k = pair(golden.relation, golden.property);
      // Coverage is judged per question set: a lane module cannot borrow the built-in labels.
      const scope = test.prefix + '\0' + k;
      let kinds = coverage.get(scope);
      if (!kinds) coverage.set(scope, kinds = { relation: golden.relation, property: golden.property, verdicts: new Set() });
      kinds.verdicts.add(golden.verdict);
      if (caseModule) {
        if (!caseModule.relations.includes(golden.relation)) throw Error('selfcheck-schema: undeclared relation');
        let allowed = permitted.get(golden.relation);
        if (!allowed) permitted.set(golden.relation, allowed = new Set());
        if (golden.property === reserved) admission.add(golden.relation);
        else allowed.add(golden.property);
      }
    }
    fixtures.push({ test, inputs, expected, source, caseModule });
    expectedCount += expected.length;
  }
  for (const rel of question?.relations ?? []) {
    if (!permitted.get(rel)?.size) throw Error('selfcheck-coverage: untested relation');
  }
  for (const [scope, { relation, property, verdicts }] of coverage) {
    // `input-admission` is the reserved label of inadmissible lane claims, never a graded
    // property. A lane relation with several properties, or any reserved golden, labels
    // them with it, so its real properties need the three graded verdicts; otherwise all four.
    const lane = scope.startsWith('lane\0');
    const shared = lane && (permitted.get(relation)?.size > 1 || admission.has(relation));
    const needed = shared ? verdictVocabulary.filter(verdict => verdict !== 'INADMISSIBLE') : verdictVocabulary;
    // Only the built-in fallback relation is exempt; a lane that declares the name is not.
    if ((lane || relation !== fallback) && property !== reserved && !needed.every(verdict => verdicts.has(verdict))) {
      throw Error('selfcheck-coverage: need all verdicts for ' + relation + '/' + property);
    }
  }
  for (const { test, inputs, expected, source, caseModule } of fixtures) {
    const context = buildContext(ts, source, job.checker);
    const observed = inputs.map((wrapper, ordinal) => {
      const wrapped = !Object.hasOwn(wrapper, 'from') && !Object.hasOwn(wrapper, 'record_kind');
      if (wrapped && (!keys(wrapper, ['relation', 'arm', 'row'], ['target_index', 'change'])
          || !['census', 'base', 'head'].includes(wrapper.arm) || blank(wrapper.relation))) {
        throw Error('selfcheck-schema: invalid input wrapper');
      }
      const row = wrapped ? wrapper.row : wrapper;
      const index = (wrapped ? wrapper.target_index : undefined) ?? (row.record_kind === 'call_site' ? 0 : null);
      if (index !== null && (!Number.isSafeInteger(index) || index < 0 || !row.resolved_targets?.[index])) {
        throw Error('selfcheck-schema: target index');
      }
      const arm = wrapped ? wrapper.arm : 'census';
      const change = wrapped ? wrapper.change ?? null : null;
      if (change !== null && (arm === 'census' || !keys(change,
        ['id', 'key', 'kind', 'count', 'base_bag', 'head_bag', 'records', 'class', 'error'])
        || !['added', 'removed', 'label_moved', 'ambiguous'].includes(change.kind))) {
        throw Error('selfcheck-schema: change context');
      }
      const occurrence = { row, arm, index, change, ordinal };
      return grade(occurrence, source, context, caseModule, wrapped ? wrapper.relation : null,
        { permitted, admission }, test.prefix === 'kit' ? forms : [], false);
    }).sort((a, b) => order(canonical(a.key), canonical(b.key)) || order(a.arm, b.arm));
    for (let i = 0; i < observed.length; i++) {
      const { key, relation, property, verdict, why } = observed[i];
      const golden = expected[i];
      const actual = { key, relation, property, verdict };
      if (canonical(actual) !== canonical({ key: golden.key, relation: golden.relation,
        property: golden.property, verdict: golden.verdict }) || blank(why) ||
        (!caseModule && (namedBoundary.has(why) || namedBoundary.has(golden.why)) && why !== golden.why)) {
        throw Error('selfcheck-disagreement: ' + test.id + '/' + (i + 1) + ' ' + canonical(actual) + ' ' + why);
      }
    }
  }
  return { permitted, admission, expectedPairs, count: expectedCount };
}

function grading(job, stage, ts, question, checks) {
  const rel = job.relation;
  if (rel !== null && !(question ? question.relations.includes(rel) : Object.hasOwn(builtins, rel))) {
    return { refusal: 'selector: unknown relation' };
  }
  const contexts = new Map();
  const programs = new Map();
  function forArm(arm) {
    const source = job.sources[arm];
    if (!source.matches) return null;
    if (!contexts.has(arm)) {
      const key = canonical(source.files);
      if (!programs.has(key)) programs.set(key, buildContext(ts, source, job.checker));
      contexts.set(arm, programs.get(key));
    }
    return contexts.get(arm);
  }

  const selectedSites = new Set(job.claims.filter(claim => claim.index !== null)
    .map(claim => pair(claim.arm, claim.ordinal)));
  if (job.denominators.mode === 'call-diff') {
    for (const [arm, places] of Object.entries(job.places)) {
      const context = forArm(arm);
      const qualified = job.builds[job.sources[arm].binary] ?? [];
      for (const site of places) {
        if (selectedSites.has(pair(arm, site.ordinal))) continue;
        // An unselected site of an unauthenticated snapshot cannot be shown valid or cancelled.
        if (!context) return { refusal: 'cancelled-site-coordinate: unauthenticated ungraded raw site' };
        if (admitSite(site, context, qualified)) {
          return { refusal: 'cancelled-site-coordinate: invalid ungraded raw site' };
        }
      }
    }
  }

  const rows = [];
  const tuples = object();
  const scopes = new Map();
  const totals = zeros();
  const denom = job.denominators;
  function bucket(relation, property) {
    const properties = tuples[relation] ??= object();
    return properties[property] ??= zeros();
  }
  const builtinRelations = denom.mode.startsWith('edge-')
    ? ['argument_binding', 'lexical_binding'] : ['call_target'];
  for (const relation of rel === null
    ? new Set([...builtinRelations, ...(question?.relations ?? [])]) : [rel]) {
    const properties = checks.permitted.get(relation) ?? checks.expectedPairs.get(relation);
    for (const property of properties ?? []) bucket(relation, property);
  }
  for (const claim of job.claims) {
    const source = job.sources[claim.arm];
    const verdict = grade(claim, source, forArm(claim.arm), question, rel, checks,
      job.builds[source.binary] ?? [], denom.mode === 'edge-census');
    if (!verdict) {
      denom.out_of_scope++;
      continue;
    }
    const target = claim.index === null ? null : claim.row.resolved_targets[claim.index];
    const confidence = target?.confidence ?? claim.row.confidence;
    const kind = target?.kind ?? null;
    const { relation, property, verdict: label } = verdict;
    bucket(relation, property)[label]++;
    totals[label]++;
    const identity = { relation, property, confidence, kind };
    const id = canonical(identity);
    if (!scopes.has(id)) scopes.set(id, { ...identity, graded: 0, ...zeros() });
    const scope = scopes.get(id);
    scope[label]++;
    scope.graded++;
    if (target) {
      const population = denom.populations[confidence];
      const count = kind === null ? population.missing_kind : population.kinds[kind];
      count.graded++;
      count[label]++;
    }
    if (!job.selection.aggregate_only) rows.push({ sort: canonical(verdict.key), verdict, ordinal: claim.ordinal });
  }
  denom.graded = Object.values(totals).reduce((a, b) => a + b, 0);
  denom.ungraded = denom.selected - denom.graded;
  denom.correct = totals.CORRECT;
  denom.wrong = totals.WRONG;
  denom.inadmissible = totals.INADMISSIBLE;
  denom.decided = denom.correct + denom.wrong;
  denom.scopes = [...scopes.keys()].sort(order).map(key => scopes.get(key));
  fs.writeFileSync(path.join(stage, 'tuples.json'), canonical(tuples) + '\n');
  fs.writeFileSync(path.join(stage, 'denominators.json'), canonical(denom) + '\n');
  if (!job.selection.aggregate_only) {
    rows.sort((a, b) => order(a.sort, b.sort) || order(a.verdict.arm, b.verdict.arm) || a.ordinal - b.ordinal
      || (a.verdict.target_index ?? -1) - (b.verdict.target_index ?? -1));
    const fd = fs.openSync(path.join(stage, 'verdicts.jsonl'), 'w');
    try {
      for (const row of rows) fs.writeSync(fd, canonical(row.verdict) + '\n');
    } finally {
      fs.closeSync(fd);
    }
  }
  return { node: process.versions.node };
}

function qualify(job, ts) {
  const contexts = new Map();
  for (const candidate of job.candidates) {
    const { label, raw, source } = candidate;
    if (!contexts.has(source.root)) contexts.set(source.root, buildContext(ts, source, job.checker));
    const context = contexts.get(source.root);
    const call = !admitSite(raw, context, [label.form]) && calls.exactCall(raw, context);
    const body = call && bodyFor(call, context, { decorated: 'identity' }).body;
    const proved = body && identity(ts, body, context);
    const point = label.declaration;
    const span = proved && context.span(body.name);
    if (!proved || proved.form !== label.form || proved.name !== point.name
        || proved.start_line !== point.line || span.file !== point.file
        || span.start_byte !== point.start_byte || span.end_byte !== point.end_byte) {
      throw Error('qualification-control: source label disagrees with resolved body');
    }
    const index = label.target_index;
    const contradicted = structuredClone(raw);
    contradicted.resolved_targets[index].function_id.start_line = label.contradiction_line;
    if (calls.grade(raw, context, index, [label.form]).verdict !== 'CORRECT' ||
        calls.grade(contradicted, context, index, [label.form]).verdict !== 'WRONG') {
      throw Error('qualification-control: serializer claim or changed-line control disagrees');
    }
  }
  return { qualified: job.candidates.length };
}

function main(job, stage) {
  if (job.operation === 'grade' && job.node !== process.versions.node) {
    throw Error('selfcheck-runtime: use the self-check Node runtime');
  }
  const ts = checker(job.checker);
  if (job.operation === "qualify") return qualify(job, ts);
  const makeQuestion = () => loadQuestion(job.module, ts, path.join(job.checker.root, job.checker.entry));
  const checks = selfcheck(job, ts, makeQuestion());
  if (job.operation === 'selfcheck') return { node: process.versions.node, cases: job.cases.length, expected: checks.count };
  if (job.prior !== checks.count) throw Error('selfcheck-receipt: row count disagrees with the current goldens');
  return grading(job, stage, ts, makeQuestion(), checks);
}

try {
  const job = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  process.stdout.write(canonical(main(job, process.argv[3])) + '\n');
} catch (error) {
  process.stderr.write(String(error.message) + '\n');
  process.exitCode = 1;
}
