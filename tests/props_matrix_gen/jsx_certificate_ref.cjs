'use strict';

// Independent reference for DESIGN 5.3.4 Rule R (positive certificate P2), written over the
// TypeScript AST of the generated source and limited to the forms the generator emits. It never
// reads product code. A representative read of `name` in `owner` is certified only when the
// owner's own spelled occurrences prove no rebinding or write (clauses a2 and b2; scope: it does not model
// result positions, `await` or destructuring-assignment targets, which no generated form uses; clause c's
// eval/arguments scan is not exercised by the generated cells and is asserted absent here).

function certified(ts, owner, name) {
  const occurrences = [];
  (function visit(node) {
    if (ts.isIdentifier(node) && node.text === name && isVariableToken(ts, node)) occurrences.push(node);
    ts.forEachChild(node, visit);
  })(owner);
  const text = owner.getText();
  if (/\b(eval|arguments)\b/.test(text.replace(/(["'`])(?:\\.|(?!\1).)*\1/g, ''))) return false;
  const declarations = occurrences.filter(node => declarationKind(ts, node));
  if (declarations.length !== 1) return false;
  if (declarationKind(ts, declarations[0]) !== 'eligible') {
    return false;
  }
  if (!eligibleScope(ts, declarations[0], owner)) return false;
  return occurrences.every(node => node === declarations[0]
    ? !inLoopHead(ts, node, owner) : readPosition(ts, node) && !inLoopHead(ts, node, owner));
}

function isVariableToken(ts, node) {
  const p = node.parent;
  if (ts.isPropertyAccessExpression(p) && p.name === node) return false;
  if (ts.isPropertyAssignment(p) && p.name === node) return false;
  if (ts.isJsxAttribute(p) && p.name === node) return false;
  if ((ts.isMethodDeclaration(p) || ts.isPropertyDeclaration(p)) && p.name === node) return false;
  if (ts.isLabeledStatement(p) && p.label === node) return false;
  return true;
}

function declarationKind(ts, node) {
  const p = node.parent;
  if (ts.isParameter(p) && p.name === node) return 'eligible';
  if (ts.isVariableDeclaration(p) && p.name === node) return 'eligible';
  if (ts.isBindingElement(p) && p.name === node) return 'eligible';
  if ((ts.isFunctionDeclaration(p) || ts.isClassDeclaration(p) || ts.isClassExpression(p)
    || ts.isFunctionExpression(p)) && p.name === node) return 'ineligible';
  return null;
}

// A declaration is eligible only as an initialized `const` outside every loop, or as a formal of
// the owner itself.
function eligibleScope(ts, name, owner) {
  const p = name.parent;
  if (ts.isParameter(p)) return p.parent === owner;
  if (ts.isVariableDeclaration(p)) {
    const list = p.parent;
    if (!ts.isVariableDeclarationList(list) || !(list.flags & ts.NodeFlags.Const) || !p.initializer) return false;
    return !ancestors(ts, name, owner).some(n => ts.isIterationStatement(n, false));
  }
  return false;
}

function ancestors(ts, node, owner) {
  const out = [];
  for (let n = node.parent; n && n !== owner; n = n.parent) out.push(n);
  return out;
}

function inLoopHead(ts, node, owner) {
  let child = node;
  for (let n = node.parent; n && n !== owner.parent; child = n, n = n.parent) {
    if (ts.isIterationStatement(n, false) && n.statement !== child) return true;
  }
  return false;
}

function readPosition(ts, node) {
  let child = node;
  for (let p = node.parent; p; child = p, p = p.parent) {
    if (ts.isPropertyAccessExpression(p) || ts.isElementAccessExpression(p)) {
      if (p.expression === child) continue;
      return ts.isElementAccessExpression(p) && p.argumentExpression === child;
    }
    if (ts.isParenthesizedExpression(p)) continue;
    if (ts.isBinaryExpression(p)) {
      const k = p.operatorToken.kind;
      return !(k >= ts.SyntaxKind.FirstAssignment && k <= ts.SyntaxKind.LastAssignment) || p.right === child;
    }
    if (ts.isPrefixUnaryExpression(p)) return !(p.operator === ts.SyntaxKind.PlusPlusToken
      || p.operator === ts.SyntaxKind.MinusMinusToken);
    if (ts.isPostfixUnaryExpression(p) || ts.isDeleteExpression(p)) return false;
    if (ts.isForOfStatement(p) || ts.isForInStatement(p)) return p.expression === child;
    if (ts.isArrayLiteralExpression(p) || ts.isObjectLiteralExpression(p)) {
      if (ts.isArrayLiteralExpression(p) && p.parent && ts.isBinaryExpression(p.parent)
        && p.parent.left === p) return false;
      return true;
    }
    if (ts.isShorthandPropertyAssignment(p)) return true;
    if (ts.isComputedPropertyName(p)) return false;
    if (ts.isJsxOpeningLikeElement(p) || ts.isJsxClosingElement(p)) return false;
    return ts.isCallExpression(p) || ts.isNewExpression(p) || ts.isJsxExpression(p)
      || ts.isReturnStatement(p) || ts.isConditionalExpression(p) || ts.isExpressionStatement(p)
      || ts.isVariableDeclaration(p) || ts.isIfStatement(p) || ts.isWhileStatement(p)
      || ts.isDoStatement(p) || ts.isForStatement(p) || ts.isSwitchStatement(p)
      || ts.isCaseClause(p) || ts.isTemplateSpan(p) || ts.isSpreadElement(p)
      || ts.isAwaitExpression(p) || ts.isYieldExpression(p) || ts.isThrowStatement(p)
      || ts.isVoidExpression(p) || ts.isTypeOfExpression(p) || ts.isArrowFunction(p)
      || ts.isPropertyAssignment(p) || ts.isCallLikeExpression(p);
  }
  return false;
}

module.exports = { certified };
