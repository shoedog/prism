'use strict';

const property = 'declaration-and-read';
const result = (verdict, why = property) => ({ verdict, property, why });

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
  if (left.text !== from.path.base || right.text !== to.path.base) return result('WRONG', 'different-identifier');
  const a = role(ts, left);
  const b = role(ts, right);
  if (a === 'read' || a === 'type') return result('WRONG', 'not-a-definition');
  if (b === 'decl' || b === 'write' || b === 'type') return result('WRONG', b === 'type' ? 'type-position' : 'write-position');
  const source = declaration(ts, checker, left);
  const read = declaration(ts, checker, right);
  if (!source || !read) return result('UNDECIDED', 'declaration-unproved');
  return result(source === read ? 'CORRECT' : 'WRONG');
}

module.exports = { property, grade };
