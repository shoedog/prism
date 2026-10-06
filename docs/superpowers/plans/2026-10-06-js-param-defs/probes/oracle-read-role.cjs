'use strict';

function valueNotARead(ts, node) {
  for (let child = node, p = node.parent; p; child = p, p = p.parent) {
    if (ts.isParenthesizedExpression(p) || ts.isAsExpression(p) || ts.isNonNullExpression(p)) continue;
    if (ts.isArrayLiteralExpression(p) || ts.isObjectLiteralExpression(p) || ts.isArrayBindingPattern(p) || ts.isObjectBindingPattern(p)) continue;
    if (ts.isPropertyAssignment(p) && p.initializer === child || ts.isBindingElement(p) && p.name === child) continue;
    if (ts.isBinaryExpression(p)) return p.left === child && p.operatorToken.kind === ts.SyntaxKind.EqualsToken;
    if (ts.isForInStatement(p) || ts.isForOfStatement(p)) return p.initializer === child;
    if (ts.isVariableDeclaration(p) || ts.isParameter(p) || ts.isEnumDeclaration(p) ||
        ts.isFunctionDeclaration(p) || ts.isFunctionExpression(p) || ts.isClassDeclaration(p) || ts.isClassExpression(p)) return p.name === child;
    return false;
  }
  return false;
}

// Examine the complete matched access, rather than its receiver identifier.
function memberNotARead(ts, base, fields) {
  let access = base;
  for (const field of fields) {
    const p = access.parent;
    if (field === '[]' && ts.isElementAccessExpression(p) && p.expression === access ||
        ts.isPropertyAccessExpression(p) && p.expression === access && p.name.text === field.replace(/\(.*$/, '')) access = p;
    else return false;
  }
  return valueNotARead(ts, access);
}

module.exports = { memberNotARead, valueNotARead };
