'use strict';

const { isIdentifier } = require('../bytes.cjs');
const { bodyFor, unwrap } = require('./callable.cjs');
const { neighbour, role, roles } = require('./lexical_binding.cjs');

const property = 'implementation-slot';
const result = (verdict, why = property) => ({ verdict, property, why });

function grade(row, context) {
  const { ts, program, enclosing_call, node_at_bytes, identifier_at_bytes } = context.public;
  const { from, to } = row;
  if (from.path.fields.length || to.path.fields.length) return result('UNDECIDED', 'structured-path');
  const call = enclosing_call(from.file, from.start_byte, from.end_byte);
  const actual = node_at_bytes(program, from.file, from.start_byte, from.end_byte);
  if (!call || !actual) return result('UNDECIDED', 'slot-unproved');
  const index = call.arguments?.findIndex(arg => unwrap(ts, arg) === actual) ?? -1;
  if (index < 0 || call.arguments.slice(0, index + 1).some(ts.isSpreadElement)) {
    return result('UNDECIDED', 'slot-unproved');
  }
  const primitive = new Set([ts.SyntaxKind.NumericLiteral, ts.SyntaxKind.BigIntLiteral,
    ts.SyntaxKind.StringLiteral, ts.SyntaxKind.NoSubstitutionTemplateLiteral,
    ts.SyntaxKind.TrueKeyword, ts.SyntaxKind.FalseKeyword, ts.SyntaxKind.NullKeyword]);
  if (!isIdentifier(ts, actual) && !primitive.has(actual.kind)) return result('UNDECIDED', 'slot-unproved');
  const resolved = bodyFor(call, context);
  if (!resolved.body) return result('UNDECIDED', resolved.why);
  function fixedSlot(declaration) {
    const formals = declaration.parameters.filter(p => p.name?.text !== 'this');
    const formal = formals[index];
    return formal && isIdentifier(ts, formal.name)
      && !formals.slice(0, index + 1).some(p => p.dotDotDotToken || p.isBracketed && !p.name);
  }
  if (!fixedSlot(resolved.selected) || !fixedSlot(resolved.body)) return result('UNDECIDED', 'slot-unproved');
  const formal = resolved.body.parameters.filter(p => p.name?.text !== 'this')[index].name;
  const wire = context.span(formal);
  // The coordinate rule of the lexical question (lexical_binding.cjs): a spelling or position
  // at a proved slot is WRONG only when the claimed name is nowhere else on that endpoint's
  // line in the needed role (from: a read; to: the proved formal itself, which must share
  // the claimed line); otherwise the producer's line-anchored span may have landed on a
  // neighbour, UNDECIDED `endpoint-coordinate-off`. A different name at exactly the
  // formal's bytes, and a signature formal of an overload, stay WRONG whatever else the
  // line holds (FORMATS section 4 is stricter there). An endpoint that is not at exact identifier
  // bytes is UNDECIDED `identifier-unproved`.
  const status = [];
  const spelled = isIdentifier(ts, actual) ? actual.text : actual.getText();
  if (spelled !== from.path.base) {
    status.push(neighbour(ts, actual, from.line, candidate => (isIdentifier(ts, candidate)
      ? candidate.text === from.path.base && roles.read.includes(role(ts, candidate))
      : primitive.has(candidate.kind) && candidate.getText() === from.path.base)) ? 'off' : 'wrong');
  }
  const bytes = wire.file === to.file && wire.start_byte === to.start_byte && wire.end_byte === to.end_byte;
  if (bytes) {
    if (formal.text !== to.path.base) status.push('wrong');
  } else {
    const claimed = identifier_at_bytes(to.file, to.start_byte, to.end_byte);
    if (!claimed) status.push('unproved');
    else {
      // Only the proved formal itself can serve: a signature formal, or another definition of
      // the name, is the wrong position FORMATS section 4 (oracle-032, oracle-037) keeps WRONG.
      status.push(formal.text === to.path.base && neighbour(ts, claimed, to.line,
        candidate => candidate === formal) ? 'off' : 'wrong');
    }
  }
  if (status.includes('wrong')) return result('WRONG');
  if (status.includes('off')) return result('UNDECIDED', 'endpoint-coordinate-off');
  if (status.includes('unproved')) return result('UNDECIDED', 'identifier-unproved');
  return result('CORRECT');
}

module.exports = { property, grade };
