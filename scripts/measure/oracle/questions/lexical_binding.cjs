'use strict';

const { writes } = require('./callable.cjs');

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

// Type references and computed keys erased at a signature rather than evaluated.
function erasedRole(ts, identifier) {
  let node = identifier;
  while (ts.isPropertyAccessExpression(node.parent) && node.parent.expression === node) node = node.parent;
  const heritage = node.parent;
  if (ts.isExpressionWithTypeArguments(heritage) && ts.isHeritageClause(heritage.parent)) {
    const owner = heritage.parent.parent;
    if (heritage.parent.token === ts.SyntaxKind.ImplementsKeyword || ts.isInterfaceDeclaration(owner)) return 'type';
  }
  for (let p = identifier.parent; p; p = p.parent) {
    if (ts.isComputedPropertyName(p)) {
      const member = p.parent;
      const typeMember = ts.isTypeLiteralNode(member.parent) || ts.isInterfaceDeclaration(member.parent);
      if (typeMember && (ts.isPropertySignature(member) || ts.isMethodSignature(member))) return 'type';
      if (typeMember && (ts.isGetAccessorDeclaration(member) || ts.isSetAccessorDeclaration(member))) {
        return 'erased-computed-key';
      }
      // Ambient declaration and declare-field discrepancies have their own pinned limits.
      if (!(member.flags & ts.NodeFlags.Ambient)
          && ((ts.getCombinedModifierFlags(member) & ts.ModifierFlags.Abstract)
            || (ts.isMethodDeclaration(member) && !member.body))) return 'erased-computed-key';
      break;
    }
    if (ts.isFunctionLike(p) || ts.isStatement(p) || ts.isDeclaration(p)) break;
  }
  return null;
}

function objectAssignment(ts, identifier) {
  const key = identifier.parent;
  if (!ts.isPropertyAssignment(key) || key.name !== identifier) return null;
  const pattern = key.parent;
  const assignment = pattern.parent;
  return ts.isObjectLiteralExpression(pattern) && ts.isBinaryExpression(assignment)
    && assignment.left === pattern && assignment.operatorToken.kind === ts.SyntaxKind.EqualsToken
    ? assignment : null;
}

function role(ts, identifier) {
  let p = identifier.parent;
  const erased = erasedRole(ts, identifier);
  if (erased) return erased;
  if (objectAssignment(ts, identifier)) return 'read';
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

// An endpoint anywhere inside the operand of a `delete` is graded by no one: `delete o.m`
// reads nothing of `m`, `delete (c ? x : 0)` evaluates `x`, and a receiver or key is read.
function insideDelete(ts, node) {
  for (let p = node.parent; p; p = p.parent) if (ts.isDeleteExpression(p)) return true;
  return false;
}

// A declaration that binds a value by its own name (variable, parameter, function, class,
// import, binding element). A property token (member name, object or pattern key)
// resolves to a property declaration, which is no binding.
function valueBinding(ts, declaration) {
  return ts.isVariableDeclaration(declaration) || ts.isParameter(declaration)
    || ts.isFunctionDeclaration(declaration) || ts.isClassDeclaration(declaration)
    || ts.isBindingElement(declaration) || ts.isImportClause(declaration)
    || ts.isImportSpecifier(declaration) || ts.isNamespaceImport(declaration)
    || ts.isImportEqualsDeclaration(declaration);
}

// The identifier is a property token BY SYNTAX: the name of a member access or qualified name, an
// object or pattern key. Whatever it resolves to, it is no binding of its own.
function propertyToken(ts, id) {
  const p = id.parent;
  return (ts.isPropertyAccessExpression(p) && p.name === id) || (ts.isQualifiedName(p) && p.right === id)
    || (ts.isPropertyAssignment(p) && p.name === id) || (ts.isBindingElement(p) && p.propertyName === id)
    || (ts.isDeclaration(p) && p.name === id && !valueBinding(ts, p));
}

function memberReceiver(ts, id) {
  const p = id.parent;
  return ts.isPropertyAccessExpression(p) && p.name === id ? p.expression : null;
}

function patternReceiver(ts, id) {
  const key = id.parent;
  if (ts.isBindingElement(key) && key.propertyName === id && ts.isObjectBindingPattern(key.parent)) {
    const owner = key.parent.parent;
    if (ts.isVariableDeclaration(owner)) return owner.initializer;
  }
  return objectAssignment(ts, id)?.right;
}

// Compare a receiver's identifier symbol or its nearest non-arrow `this` scope.
function receiverIdentity(ts, checker, receiver) {
  if (!receiver) return null;
  while (ts.isParenthesizedExpression(receiver) || ts.isNonNullExpression(receiver)) receiver = receiver.expression;
  if (ts.isIdentifier(receiver)) return checker.getSymbolAtLocation(receiver) ?? null;
  if (receiver.kind === ts.SyntaxKind.ThisKeyword) {
    return ts.findAncestor(receiver.parent, n => (ts.isFunctionLike(n) && !ts.isArrowFunction(n)) || ts.isClassLike(n)) ?? null;
  }
  return null;
}

function declaration(ts, checker, id) {
  let symbol;
  if (objectAssignment(ts, id)) symbol = checker.getPropertySymbolOfDestructuringAssignment(id);
  else if (ts.isShorthandPropertyAssignment(id.parent) && id.parent.name === id) {
    symbol = checker.getShorthandAssignmentValueSymbol(id.parent);
  } else symbol = checker.getSymbolAtLocation(id);
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
  if (insideDelete(ts, left) || insideDelete(ts, right)) return result('UNDECIDED', 'delete-operand');
  const a = role(ts, left);
  const b = role(ts, right);
  if (a === 'erased-computed-key' || b === 'erased-computed-key') return result('UNDECIDED', 'erased-computed-key');
  // A spelling mismatch (the span's identifier is not the claimed base) proves nothing: the
  // producer's alias twins carry the alias variable's span for a def of the base. It is
  // UNDECIDED. A role or position fault at an endpoint spelled as claimed is a proof, WRONG
  // unless another occurrence of the base on that line could serve the role.
  const point = { from: { node: left, wire: from, base: from.path.base }, to: { node: right, wire: to, base: to.path.base } };
  const served = end => neighbour(ts, point[end].node, point[end].wire.line, candidate =>
    (ts.isIdentifier(candidate) || ts.isPrivateIdentifier(candidate)) && candidate.text === point[end].base
    && needed[end].includes(role(ts, candidate)));
  const faults = [];
  const mismatched = [];
  if (left.text !== from.path.base) mismatched.push('from');
  else if (a === 'read' || a === 'type') faults.push({ end: 'from', why: 'not-a-definition' });
  if (right.text !== to.path.base) mismatched.push('to');
  else if (b === 'decl' || b === 'write' || b === 'type') {
    faults.push({ end: 'to', why: b === 'type' ? 'type-position' : 'write-position' });
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
  if (source === read) {
    // A member write needs evidence that the read uses the same receiver. Even one
    // identifier symbol can denote a different receiver when it is reassigned.
    const receiver = memberReceiver(ts, left);
    if (receiver) {
      const written = receiverIdentity(ts, checker, receiver);
      const observed = receiverIdentity(ts, checker, memberReceiver(ts, right) ?? patternReceiver(ts, right));
      if (!written || written !== observed || writes(context).symbols.has(written)) {
        return result('UNDECIDED', 'receiver-identity-unproved');
      }
    }
    return result('CORRECT');
  }
  // Both endpoints at exact identifier bytes spelled as the claimed base and resolving to
  // different VALUE bindings are a proved different binding, so no neighbour can rescue it.
  // A property token (member name, object or pattern key) is no binding: its span may be the
  // line-anchored neighbour of the real one, so a serving neighbour makes it coordinate-off.
  // The endpoint's SYNTAX decides before what it resolves to: a member access name resolves to a
  // parameter property or a namespace-exported const, which is still a property token.
  const token = propertyToken(ts, left) || propertyToken(ts, right);
  if ((token || !(valueBinding(ts, source) && valueBinding(ts, read))) && (served('from') || served('to'))) {
    return result('UNDECIDED', 'endpoint-coordinate-off');
  }
  return result('WRONG', 'different-declaration');
}

module.exports = { property, grade, role, neighbour, roles };
