'use strict';

const { isIdentifier } = require('../bytes.cjs');
const { bodyFor, unwrap } = require('./callable.cjs');
const { neighbour, role, roles } = require('./lexical_binding.cjs');
const { memberEndpoint, memberPathReason } = require('./member_scope.cjs');

const property = 'implementation-slot';
const result = (verdict, why = property) => ({ verdict, property, why });

function grade(row, context) {
  const { ts, program, checker, enclosing_call, node_at_bytes } = context.public;
  const { from, to } = row;
  const actual = node_at_bytes(program, from.file, from.start_byte, from.end_byte);
  const destination = node_at_bytes(program, to.file, to.start_byte, to.end_byte);
  if (memberEndpoint(ts, checker, from, actual) || memberEndpoint(ts, checker, to, destination)) {
    return result('UNDECIDED', memberPathReason);
  }
  const call = enclosing_call(from.file, from.start_byte, from.end_byte);
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
  // A serving neighbour gives `endpoint-coordinate-off`. The actual can have a nearby
  // read or primitive with the claimed spelling. Only the proved formal can serve at
  // the destination, on the claimed token's wire or physical line. A different
  // name at the proved formal's bytes or an overload signature's formal always gives WRONG.
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
    const claimed = isIdentifier(ts, destination) ? destination : null;
    // A formal of an overload signature of the called function is the wrong position
    // whatever else its line holds, even on the implementation's own line.
    // A constructor has no name: its overloads are the containing class's constructor declarations.
    const siblings = ts.isConstructorDeclaration(resolved.body)
      ? resolved.body.parent.members.filter(ts.isConstructorDeclaration)
      : (resolved.body.name && checker.getSymbolAtLocation(resolved.body.name)?.declarations) ?? [];
    const signatureFormal = claimed && ts.isParameter(claimed.parent) && claimed.parent.name === claimed
      && claimed.parent.parent !== resolved.body && siblings.includes(claimed.parent.parent);
    if (!claimed) status.push('unproved');
    else if (signatureFormal) status.push('wrong');
    else {
      // Only the proved formal itself can serve: another definition of the name is the wrong
      // position FORMATS section 4 (oracle-032, oracle-037) keeps WRONG.
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
