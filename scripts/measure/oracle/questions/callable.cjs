'use strict';

const path = require('node:path');
const { headerStart } = require('../bytes.cjs');

function wrapper(ts, node) {
  return ts.isParenthesizedExpression(node) || ts.isAsExpression(node)
    || ts.isTypeAssertionExpression(node) || ts.isNonNullExpression(node) || ts.isSatisfiesExpression(node);
}

function unwrap(ts, node) {
  while (node && wrapper(ts, node)) node = node.expression;
  return node;
}

// The value a receiver expression denotes when an assertion does not change its type.
// A type assertion to a different type selects a different static signature than the
// runtime body, so it proves nothing about which body runs.
function origin(ts, checker, node) {
  while (node) {
    if (ts.isParenthesizedExpression(node) || ts.isNonNullExpression(node)
        || ts.isSatisfiesExpression(node)) {
      node = node.expression;
    } else if (ts.isAsExpression(node) || ts.isTypeAssertionExpression(node)) {
      if (checker.getTypeAtLocation(node.expression) !== checker.getTypeAtLocation(node)) return null;
      node = node.expression;
    } else {
      break;
    }
  }
  return node;
}

function memberName(ts, expression) {
  if (ts.isPropertyAccessExpression(expression)) return expression.name.text;
  const key = expression.argumentExpression;
  return key && (ts.isStringLiteral(key) || ts.isNoSubstitutionTemplateLiteral(key)) ? key.text : null;
}

// Names written through `x.name = ..`, `x['name'] = ..` or `delete x.name` anywhere in the
// analysed sources. A written member may hold a callable other than its declaration, so
// no declaration of that name proves the current value. Computed keys, Object.assign and
// reflective writes are outside the model (README accept-list).
const writeIndex = new WeakMap();

function writtenMembers(context) {
  const { ts, program } = context.public;
  if (writeIndex.has(program)) return writeIndex.get(program);
  const names = new Set();
  const root = program.getCurrentDirectory() + path.sep;
  function record(target) {
    let node = target;
    while (node && (ts.isParenthesizedExpression(node) || ts.isNonNullExpression(node)
        || ts.isAsExpression(node) || ts.isTypeAssertionExpression(node) || ts.isSatisfiesExpression(node))) {
      node = node.expression;
    }
    if (node && (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node))) {
      const name = memberName(ts, node);
      if (name !== null) names.add(name);
    }
  }
  function visit(node) {
    if (ts.isBinaryExpression(node) && node.operatorToken.kind >= ts.SyntaxKind.FirstAssignment
        && node.operatorToken.kind <= ts.SyntaxKind.LastAssignment) record(node.left);
    else if (ts.isDeleteExpression(node)) record(node.expression);
    ts.forEachChild(node, visit);
  }
  for (const sf of program.getSourceFiles()) {
    if (!sf.isDeclarationFile && sf.fileName.startsWith(root)) visit(sf);
  }
  writeIndex.set(program, names);
  return names;
}

function bindingSymbol(ts, checker, expression) {
  let symbol = checker.getSymbolAtLocation(ts.isPropertyAccessExpression(expression)
    ? expression.name : expression);
  if (symbol && symbol.flags & ts.SymbolFlags.Alias) symbol = checker.getAliasedSymbol(symbol);
  return symbol;
}

function escapingImport(ts, checker, call, root) {
  const expr = call.expression;
  const symbol = checker.getSymbolAtLocation(ts.isPropertyAccessExpression(expr) ? expr.expression : expr);
  return symbol?.declarations?.some(declaration => {
    let node = declaration;
    while (node && !ts.isImportDeclaration(node)) node = node.parent;
    if (!node || !ts.isStringLiteral(node.moduleSpecifier)) return false;
    const wire = node.moduleSpecifier.text;
    if (!wire.startsWith('.')) return true;
    const absolute = path.resolve(path.dirname(node.getSourceFile().fileName), wire);
    return absolute !== root && !absolute.startsWith(root + path.sep);
  });
}

function constInitializer(ts, checker, declaration) {
  return ts.isVariableDeclaration(declaration) && declaration.parent.flags & ts.NodeFlags.Const
    && !declaration.type && declaration.initializer ? origin(ts, checker, declaration.initializer) : null;
}

function constDeclaration(ts, declaration) {
  return ts.isVariableDeclaration(declaration) && declaration.parent.flags & ts.NodeFlags.Const
    && !declaration.type;
}

function uniqueReceiverUse(ts, checker, declaration, receiver) {
  const statement = declaration.parent?.parent;
  if (statement && ts.canHaveModifiers(statement)
      && ts.getModifiers(statement)?.some(m => m.kind === ts.SyntaxKind.ExportKeyword)) return false;
  const symbol = checker.getSymbolAtLocation(declaration.name);
  let distinctUse = false;
  function visit(node) {
    if (node !== receiver && node !== declaration.name && ts.isIdentifier(node)
        && node.text === declaration.name?.text && checker.getSymbolAtLocation(node) === symbol) {
      distinctUse = true;
    }
    if (!distinctUse) ts.forEachChild(node, visit);
  }
  visit(declaration.getSourceFile());
  return !distinctUse;
}

// Returns null when the receiver does not prove the body, else whether it is a fresh `new`.
function receiverProved(context, expression, body) {
  const { ts, checker } = context.public;
  if (!ts.isPropertyAccessExpression(expression) && !ts.isElementAccessExpression(expression)) {
    return { fresh: false };
  }
  const name = memberName(ts, expression);
  if (name === null || writtenMembers(context).has(name)) return null;
  const receiver = expression.expression;
  if (body.name && ts.isPrivateIdentifier(body.name)
      || receiver.kind === ts.SyntaxKind.SuperKeyword) return { fresh: false };
  // Static access through the class value, or a namespace function.
  const symbols = bindingSymbol(ts, checker, receiver)?.declarations ?? [];
  if (symbols.length === 1 && (ts.isClassDeclaration(symbols[0]) || ts.isModuleDeclaration(symbols[0])
      || ts.isNamespaceImport(symbols[0]) || ts.isSourceFile(symbols[0]))) {
    return ts.isFunctionDeclaration(body) || ts.getModifiers(body)?.some(m => m.kind === ts.SyntaxKind.StaticKeyword)
      ? { fresh: false } : null;
  }
  const declaration = symbols.length === 1 ? symbols[0] : null;
  const value = origin(ts, checker, receiver);
  if (!value) return null;
  const initializer = declaration && constInitializer(ts, checker, declaration);
  const root = initializer && uniqueReceiverUse(ts, checker, declaration, value) ? initializer : value;
  if (!root || !(ts.isNewExpression(root) || ts.isObjectLiteralExpression(root))) return null;
  return { fresh: ts.isNewExpression(root) };
}

function bodyFor(call, context) {
  const { ts, checker, program } = context.public;
  const expression = unwrap(ts, call.expression);
  const expressionType = checker.getTypeAtLocation(expression);
  const type = checker.getBaseConstraintOfType(expressionType) ?? expressionType;
  const signature = checker.getResolvedSignature(call);
  let selected = signature?.declaration;
  if (type.isUnionOrIntersection() && type.types.length > 1) {
    const signatures = type.types.flatMap(member => member.getCallSignatures());
    const owners = new Set(signatures.map(s => s.declaration).filter(Boolean));
    if (owners.size > 1) return { why: 'ambiguous-dispatch' };
  }
  if (!selected) {
    return { why: escapingImport(ts, checker, call, program.getCurrentDirectory())
      ? 'external-callable' : (ts.isNewExpression(call) ? 'type-only-declaration' : 'unresolved-callable') };
  }
  const sf = selected.getSourceFile();
  if (!context.span(selected)) return { why: 'external-callable' };
  if (sf.isDeclarationFile) return { why: 'type-only-declaration' };
  const selectedSignature = selected;
  if (ts.isJSDocSignature(selected) && ts.isJSDocOverloadTag(selected.parent)) {
    selected = ts.getHostSignatureFromJSDoc(selected);
    if (!selected) return { why: 'type-only-declaration' };
  }
  const bodyKind = node => ts.isFunctionDeclaration(node) || ts.isFunctionExpression(node)
    || ts.isArrowFunction(node) || ts.isMethodDeclaration(node) || ts.isConstructorDeclaration(node);
  if (!bodyKind(selected)) return { why: 'type-only-declaration' };
  let body = selected;
  if (!body.body) {
    const siblings = ts.isConstructorDeclaration(selected)
      ? selected.parent.members.filter(ts.isConstructorDeclaration)
      : (selected.name && checker.getSymbolAtLocation(selected.name)?.declarations) ?? [];
    const bodies = siblings.filter(node => bodyKind(node) && node.body);
    if (bodies.length !== 1) return { why: bodies.length ? 'ambiguous-dispatch' : 'type-only-declaration' };
    [body] = bodies;
  }
  if (!context.span(body)) return { why: 'external-callable' };
  // A parenthesized or asserted value is the same value for ownership purposes.
  let top = body;
  while (wrapper(ts, top.parent)) top = top.parent;
  const holder = top.parent;
  const classField = ts.isPropertyDeclaration(holder) && holder.initializer === top;
  const objectField = ts.isPropertyAssignment(holder) && holder.initializer === top;
  const proofOwner = classField || objectField ? holder : body;
  const proof = receiverProved(context, expression, proofOwner);
  if (!proof) return { why: 'ambiguous-dispatch' };
  if (ts.isArrowFunction(body) || ts.isFunctionExpression(body)) {
    if (ts.isVariableDeclaration(holder) && !constDeclaration(ts, holder)) return { why: 'ambiguous-dispatch' };
    // A field arrow is the current value of a fresh instance (no write to that member name
    // exists); otherwise only a readonly field is stable.
    if (ts.isPropertyDeclaration(holder) && !ts.getModifiers(holder)?.some(m => m.kind === ts.SyntaxKind.ReadonlyKeyword)
        && !(proof.fresh && !holder.type)) {
      return { why: 'ambiguous-dispatch' };
    }
  }
  return { body, selected: selectedSignature };
}

function identity(ts, body, context) {
  const name = body.name;
  if (!name || !ts.isIdentifier(name)) return null;
  const line = context.line(name);
  if (context.line(body, headerStart(ts, body)) !== line) return null;
  let form;
  if (ts.isFunctionDeclaration(body)) form = 'named-function';
  if (ts.isMethodDeclaration(body)) {
    form = ts.getDecorators(body)?.length ? 'decorated-method' : 'named-method';
  }
  return form ? { file: context.span(body).file, name: name.text, start_line: line, form } : null;
}

module.exports = { bodyFor, identity, unwrap };
