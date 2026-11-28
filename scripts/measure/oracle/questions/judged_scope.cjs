'use strict';

// The oracle's judged scope, stated positively and decided here, once, for every question.
//
// An endpoint is judged only when ALL of these hold:
//   1. it has no field path;
//   2. the claimed bytes are a plain identifier (not a private name, a string or numeric
//      key, a keyword or a literal), except that the actual of a call may also be a
//      primitive literal, judged against the declared formal (LITERAL);
//   3. the identifier stands at a BINDING position (BINDING table: the declared name of a
//      variable, parameter, function, class, import, binding element or shorthand property)
//      or at a REFERENCE position (REFERENCE table: an evaluated operand, argument,
//      initializer or value tag);
//   4. its symbol, when it has one, is a value binding (VALUE), not a member of anything.
// Everything else is UNDECIDED `outside-judged-scope`: it gets no verdict, not even a WRONG.
const scopeReason = 'outside-judged-scope';

const BINDING = [
  'isVariableDeclaration', 'isParameter', 'isFunctionDeclaration', 'isFunctionExpression',
  'isClassDeclaration', 'isClassExpression', 'isImportClause', 'isImportSpecifier',
  'isNamespaceImport', 'isImportEqualsDeclaration', 'isBindingElement', 'isShorthandPropertyAssignment',
  'isModuleDeclaration', 'isEnumDeclaration',
];

// The slots in which an identifier operand is evaluated (read, or written as a value binding).
const OPERAND = [
  'isParenthesizedExpression', 'isNonNullExpression', 'isAsExpression', 'isTypeAssertionExpression',
  'isSatisfiesExpression', 'isAwaitExpression', 'isSpreadElement', 'isSpreadAssignment', 'isVoidExpression',
  'isTypeOfExpression', 'isDeleteExpression', 'isYieldExpression', 'isDecorator', 'isExpressionStatement',
  'isReturnStatement', 'isThrowStatement', 'isIfStatement', 'isWhileStatement', 'isDoStatement',
  'isSwitchStatement', 'isCaseClause', 'isExportAssignment', 'isComputedPropertyName', 'isTemplateSpan',
  'isJsxExpression', 'isJsxSpreadAttribute',
];
const INITIALIZED = [
  'isVariableDeclaration', 'isParameter', 'isBindingElement', 'isPropertyDeclaration', 'isEnumMember',
  'isPropertyAssignment',
];

const VALUE_SYMBOL = (ts) => ts.SymbolFlags.Variable | ts.SymbolFlags.Function | ts.SymbolFlags.Class
  | ts.SymbolFlags.Alias | ts.SymbolFlags.Enum | ts.SymbolFlags.ValueModule;

function bindingPosition(ts, node) {
  const p = node.parent;
  return BINDING.some(kind => ts[kind](p)) && p.name === node;
}

// An intrinsic JSX tag (`<div />`) names an element type, not a value in scope.
const intrinsicTag = name => /^[a-z]/.test(name) || name.includes('-');

function referencePosition(ts, node) {
  const p = node.parent;
  if (!p) return false;
  if (ts.isCallExpression(p) || ts.isNewExpression(p)) return p.expression === node || !!p.arguments?.includes(node);
  if (ts.isPropertyAccessExpression(p)) return p.expression === node;
  if (ts.isElementAccessExpression(p)) return p.expression === node || p.argumentExpression === node;
  if (ts.isTaggedTemplateExpression(p)) return p.tag === node;
  if (ts.isPrefixUnaryExpression(p) || ts.isPostfixUnaryExpression(p)) return p.operand === node;
  if (ts.isBinaryExpression(p)) return p.left === node || p.right === node;
  if (ts.isConditionalExpression(p)) return [p.condition, p.whenTrue, p.whenFalse].includes(node);
  if (ts.isArrayLiteralExpression(p)) return p.elements.includes(node);
  if (ts.isArrowFunction(p)) return p.body === node;
  if (ts.isShorthandPropertyAssignment(p)) return p.objectAssignmentInitializer === node;
  if (ts.isForStatement(p)) return [p.initializer, p.condition, p.incrementor].includes(node);
  if (ts.isForInStatement(p) || ts.isForOfStatement(p)) return p.initializer === node || p.expression === node;
  if (ts.isJsxOpeningElement(p) || ts.isJsxSelfClosingElement(p) || ts.isJsxClosingElement(p)) {
    return p.tagName === node && !intrinsicTag(node.text);
  }
  if (ts.isExpressionWithTypeArguments(p) && ts.isHeritageClause(p.parent)
      && p.parent.token === ts.SyntaxKind.ExtendsKeyword
      && (ts.isClassDeclaration(p.parent.parent) || ts.isClassExpression(p.parent.parent))) {
    return p.expression === node;
  }
  if (INITIALIZED.some(kind => ts[kind](p)) && p.initializer === node) return true;
  return OPERAND.some(kind => ts[kind](p)) && p.expression === node;
}

const LITERAL = (ts) => new Set([ts.SyntaxKind.NumericLiteral, ts.SyntaxKind.BigIntLiteral,
  ts.SyntaxKind.StringLiteral, ts.SyntaxKind.NoSubstitutionTemplateLiteral, ts.SyntaxKind.TrueKeyword,
  ts.SyntaxKind.FalseKeyword, ts.SyntaxKind.NullKeyword]);

function callActualLiteral(ts, node) {
  if (!LITERAL(ts).has(node.kind)) return false;
  let operand = node;
  while (ts.isParenthesizedExpression(operand.parent)) operand = operand.parent;
  const call = operand.parent;
  return (ts.isCallExpression(call) || ts.isNewExpression(call)) && !!call.arguments?.includes(operand);
}

function valueSymbol(ts, checker, node) {
  const p = node.parent;
  const symbol = ts.isShorthandPropertyAssignment(p) && p.name === node
    ? checker.getShorthandAssignmentValueSymbol(p) : checker.getSymbolAtLocation(node);
  return !symbol || (symbol.flags & VALUE_SYMBOL(ts)) !== 0;
}

// True when the claimed endpoint is inside the judged scope.
function judgedEndpoint(ts, checker, endpoint, node, { literalActual = false } = {}) {
  if (endpoint.path.fields.length || !node?.parent) return false;
  if (literalActual && callActualLiteral(ts, node)) return true;
  if (node.kind !== ts.SyntaxKind.Identifier) return false;
  return (bindingPosition(ts, node) || referencePosition(ts, node)) && valueSymbol(ts, checker, node);
}

module.exports = { judgedEndpoint, scopeReason };
