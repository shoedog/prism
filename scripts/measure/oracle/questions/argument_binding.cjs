'use strict';

const { isIdentifier } = require('../bytes.cjs');
const { bodyFor, unwrap } = require('./callable.cjs');

const property = 'implementation-slot';
const result = (verdict, why = property) => ({ verdict, property, why });

function grade(row, context) {
  const { ts, program, enclosing_call, node_at_bytes } = context.public;
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
  return result((actual.text !== undefined && isIdentifier(ts, actual) ? actual.text : actual.getText())
    === from.path.base && formal.text === to.path.base && wire.file === to.file
    && wire.start_byte === to.start_byte && wire.end_byte === to.end_byte ? 'CORRECT' : 'WRONG');
}

module.exports = { property, grade };
