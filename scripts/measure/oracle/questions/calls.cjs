'use strict';

const { bodyFor, identity } = require('./callable.cjs');

const property = 'callee-implementation';
const result = (verdict, why = property) => ({ verdict, property, why });
const tripleCache = new WeakMap();

function exactCall(row, context) {
  const { file, start_byte: start, end_byte: end } = row.source_span;
  const call = context.public.enclosing_call(file, start, end);
  const span = call && context.span(call);
  return span && span.start_byte === start && span.end_byte === end ? call : null;
}

function callerContradicts(row, context) {
  const { ts } = context.public;
  const { caller, source_span: span } = row;
  if (caller.file !== span.file) return true;
  const call = exactCall(row, context);
  for (let node = call?.parent; node; node = node.parent) {
    if (ts.isFunctionLike(node)) {
      const owner = identity(ts, node, context);
      return owner && (owner.name !== caller.name || owner.start_line !== caller.start_line);
    }
  }
  return false;
}

function countTriples(ts, sf, context) {
  if (tripleCache.has(sf)) return tripleCache.get(sf);
  const counts = new Map();
  function count(node) {
    if (ts.isFunctionDeclaration(node) || ts.isMethodDeclaration(node)) {
      const id = identity(ts, node, context);
      if (id) {
        const key = JSON.stringify([id.name, id.start_line]);
        counts.set(key, (counts.get(key) ?? 0) + 1);
      }
    }
    ts.forEachChild(node, count);
  }
  count(sf);
  tripleCache.set(sf, counts);
  return counts;
}

function grade(row, context, index, forms) {
  const { ts } = context.public;
  const call = exactCall(row, context);
  if (!call) return result('UNDECIDED', 'whole-call-unproved');
  const resolution = bodyFor(call, context);
  if (!resolution.body) return result('UNDECIDED', resolution.why);
  const expected = identity(ts, resolution.body, context);
  if (!expected || !forms.includes(expected.form)) return result('UNDECIDED', 'target-id-unproved');
  const claim = row.resolved_targets[index].function_id;
  const sf = context.sourceFile(claim.file);
  if (sf && countTriples(ts, sf, context).get(JSON.stringify([claim.name, claim.start_line])) > 1) {
    return result('UNDECIDED', 'non-unique-declaration');
  }
  return result(expected.file === claim.file && expected.name === claim.name
    && expected.start_line === claim.start_line ? 'CORRECT' : 'WRONG');
}

module.exports = { property, grade, callerContradicts, exactCall };
