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

function constInitializer(ts, declaration) {
  return ts.isVariableDeclaration(declaration) && declaration.parent.flags & ts.NodeFlags.Const
    && !declaration.type ? unwrap(ts, declaration.initializer) : null;
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

function receiverProved(ts, checker, expression, body) {
  if (!ts.isPropertyAccessExpression(expression) && !ts.isElementAccessExpression(expression)) return true;
  const receiver = expression.expression;
  if (body.name && ts.isPrivateIdentifier(body.name)
      || receiver.kind === ts.SyntaxKind.SuperKeyword) return true;
  // Static access through the class value, or a namespace function.
  const symbols = bindingSymbol(ts, checker, receiver)?.declarations ?? [];
  if (symbols.length === 1 && (ts.isClassDeclaration(symbols[0]) || ts.isModuleDeclaration(symbols[0])
      || ts.isNamespaceImport(symbols[0]) || ts.isSourceFile(symbols[0]))) {
    return ts.isFunctionDeclaration(body) || ts.getModifiers(body)?.some(m => m.kind === ts.SyntaxKind.StaticKeyword);
  }
  const declaration = symbols.length === 1 ? symbols[0] : null;
  const initializer = declaration && constInitializer(ts, declaration);
  const value = unwrap(ts, receiver);
  const origin = initializer && uniqueReceiverUse(ts, checker, declaration, value) ? initializer : value;
  return ts.isNewExpression(origin) || ts.isObjectLiteralExpression(origin);
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
    const siblings = ts.isConstructorDeclaration(selected) ? selected.parent.members
      : (selected.name && checker.getSymbolAtLocation(selected.name)?.declarations) ?? [];
    const bodies = siblings.filter(node => bodyKind(node) && node.body);
    if (bodies.length !== 1) return { why: bodies.length ? 'ambiguous-dispatch' : 'type-only-declaration' };
    [body] = bodies;
  }
  if (!context.span(body)) return { why: 'external-callable' };
  const classField = ts.isPropertyDeclaration(body.parent) && body.parent.initializer === body;
  const objectField = ts.isPropertyAssignment(body.parent) && body.parent.initializer === body;
  const proofOwner = classField || objectField ? body.parent : body;
  if (!receiverProved(ts, checker, expression, proofOwner)) return { why: 'ambiguous-dispatch' };
  if (ts.isArrowFunction(body) || ts.isFunctionExpression(body)) {
    const owner = body.parent;
    if (ts.isVariableDeclaration(owner) && !constInitializer(ts, owner)) return { why: 'ambiguous-dispatch' };
    if (ts.isPropertyDeclaration(owner) &&
        !ts.getModifiers(owner)?.some(m => m.kind === ts.SyntaxKind.ReadonlyKeyword)) {
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
