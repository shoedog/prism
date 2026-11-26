'use strict';

const property = 'declaration-and-read';
const result = (verdict, why = property) => ({ verdict, property, why });

// The coordinate rule. WRONG needs an endpoint at exact identifier bytes that the oracle has
// proved to be another binding or position AND no other occurrence of the claimed base on
// that endpoint's physical line could serve the endpoint's role. The producer anchors an
// endpoint by line, so its span can land on a neighbour (the key `app:` in
// `{app: {value: app}}`); that is UNDECIDED `endpoint-coordinate-off`, never WRONG.
const roles = { definition: ['decl', 'write', 'update'], read: ['read', 'update'] };
const needed = { from: roles.definition, to: roles.read };
const lineStarts = new WeakMap();

function lfLine(sf, position) {
  let starts = lineStarts.get(sf);
  if (!starts) {
    starts = [0];
    for (let i = sf.text.indexOf('\n'); i >= 0; i = sf.text.indexOf('\n', i + 1)) starts.push(i + 1);
    lineStarts.set(sf, starts);
  }
  let lo = 0;
  let hi = starts.length;
  while (lo + 1 < hi) {
    const mid = lo + ((hi - lo) >> 1);
    if (starts[mid] > position) hi = mid;
    else lo = mid;
  }
  return lo + 1;
}

// Is there a node other than `node`, starting on the endpoint's wire line or on the line of
// the node itself, that `matches`?
function neighbour(ts, node, wireLine, matches) {
  const sf = node.getSourceFile();
  const lines = new Set([wireLine, lfLine(sf, node.getStart(sf))]);
  const low = Math.min(...lines);
  const high = Math.max(...lines);
  let found = false;
  function visit(candidate) {
    if (found) return;
    const start = candidate.getStart(sf);
    if (lfLine(sf, candidate.end) < low || lfLine(sf, start) > high) return;
    if (candidate !== node && lines.has(lfLine(sf, start)) && matches(candidate)) {
      found = true;
      return;
    }
    ts.forEachChild(candidate, visit);
  }
  visit(sf);
  return found;
}

function role(ts, identifier) {
  let p = identifier.parent;
  if (ts.isDeclaration(p) && p.name === identifier && !ts.isShorthandPropertyAssignment(p)) return 'decl';
  for (let node = identifier; p; node = p, p = p.parent) {
    // A class `extends` expression is evaluated at run time, so it reads the value binding.
    if (ts.isExpressionWithTypeArguments(p) && ts.isHeritageClause(p.parent)
        && p.parent.token === ts.SyntaxKind.ExtendsKeyword
        && (ts.isClassDeclaration(p.parent.parent) || ts.isClassExpression(p.parent.parent))) return 'read';
    // The default of a shorthand destructuring property is an evaluated read.
    if (ts.isShorthandPropertyAssignment(p) && p.objectAssignmentInitializer === node) return 'read';
    if (ts.isTypeNode(p) || ts.isTypeQueryNode(p)) return 'type';
    if (ts.isPropertyAccessExpression(p) || ts.isElementAccessExpression(p)) {
      if (!ts.isPropertyAccessExpression(p) || p.name !== node) return 'read';
      continue;
    }
    // `delete o.m` removes the member and never reads its value (`void o.m` reads it); the
    // receiver and a computed key are read before this point.
    if (ts.isDeleteExpression(p) && p.expression === node) return 'delete';
    if (ts.isBinaryExpression(p) && p.left === node &&
        p.operatorToken.kind >= ts.SyntaxKind.FirstAssignment &&
        p.operatorToken.kind <= ts.SyntaxKind.LastAssignment) {
      if (p.operatorToken.kind !== ts.SyntaxKind.EqualsToken) return 'update';
      // A default nested within an assignment pattern is also a write to the leaf.
      return 'write';
    }
    if ((ts.isPrefixUnaryExpression(p) || ts.isPostfixUnaryExpression(p))
        && (p.operator === ts.SyntaxKind.PlusPlusToken || p.operator === ts.SyntaxKind.MinusMinusToken)) {
      return 'update';
    }
    if ((ts.isForInStatement(p) || ts.isForOfStatement(p)) && p.initializer === node) return 'write';
    if (ts.isFunctionLike(p) || ts.isStatement(p)) break;
    if (ts.isComputedPropertyName(p) || ts.isCallExpression(p)) return 'read';
    if (ts.isPropertyAssignment(p) && p.initializer !== node) return 'read';
    if (ts.isBinaryExpression(p) && p.right === node) return 'read';
  }
  return 'read';
}

function declaration(ts, checker, id) {
  const symbol = ts.isShorthandPropertyAssignment(id.parent) && id.parent.name === id
    ? checker.getShorthandAssignmentValueSymbol(id.parent)
    : checker.getSymbolAtLocation(id);
  if (!symbol || !(symbol.flags & (ts.SymbolFlags.Value | ts.SymbolFlags.Alias))) return undefined;
  const ds = symbol.declarations;
  if (ds?.length !== 1) return undefined;
  if (symbol.flags & ts.SymbolFlags.Alias) {
    for (let node = ds[0]; node && !ts.isImportDeclaration(node); node = node.parent) {
      if ((ts.isImportClause(node) || ts.isImportSpecifier(node)) && node.isTypeOnly) return undefined;
    }
  }
  return ds[0];
}

function grade(row, context) {
  const { ts, checker, identifier_at_bytes } = context.public;
  const { from, to } = row;
  if (from.path.fields.length || to.path.fields.length) return result('UNDECIDED', 'structured-path');
  const left = identifier_at_bytes(from.file, from.start_byte, from.end_byte);
  const right = identifier_at_bytes(to.file, to.start_byte, to.end_byte);
  if (!left || !right) return result('UNDECIDED', 'identifier-unproved');
  const a = role(ts, left);
  const b = role(ts, right);
  // A spelling mismatch (the span's identifier is not the claimed base) proves nothing: the
  // producer's alias twins carry the alias variable's span for a def of the base. It is
  // UNDECIDED. A role or position fault at an endpoint spelled as claimed is a proof, WRONG
  // unless another occurrence of the base on that line could serve the role.
  const point = { from: { node: left, wire: from, base: from.path.base }, to: { node: right, wire: to, base: to.path.base } };
  const served = end => neighbour(ts, point[end].node, point[end].wire.line, candidate =>
    ts.isIdentifier(candidate) && candidate.text === point[end].base && needed[end].includes(role(ts, candidate)));
  const faults = [];
  const mismatched = [];
  if (left.text !== from.path.base) mismatched.push('from');
  else if (a === 'read' || a === 'type' || a === 'delete') faults.push({ end: 'from', why: 'not-a-definition' });
  if (right.text !== to.path.base) mismatched.push('to');
  else if (b === 'decl' || b === 'write' || b === 'type' || b === 'delete') {
    faults.push({ end: 'to', why: b === 'type' ? 'type-position' : b === 'delete' ? 'delete-position' : 'write-position' });
  }
  const proved = faults.find(fault => !served(fault.end));
  if (proved) return result('WRONG', proved.why);
  if (faults.length || mismatched.length) {
    // The neighbour reason comes first when any faulted or mismatched endpoint has one.
    const off = faults.length || mismatched.some(served);
    return result('UNDECIDED', off ? 'endpoint-coordinate-off' : 'endpoint-spelling-mismatch');
  }
  const source = declaration(ts, checker, left);
  const read = declaration(ts, checker, right);
  if (!source || !read) return result('UNDECIDED', 'declaration-unproved');
  if (source === read) return result('CORRECT');
  // Both endpoints are at exact identifier bytes spelled as the claimed base and resolve to
  // different declarations: a proved different binding, so no neighbour can rescue it.
  return result('WRONG', 'different-declaration');
}

module.exports = { property, grade, role, neighbour, roles };
