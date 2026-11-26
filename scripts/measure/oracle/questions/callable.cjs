'use strict';

const path = require('node:path');
const { headerStart } = require('../bytes.cjs');

function wrapper(ts, node) {
  return ts.isParenthesizedExpression(node) || ts.isAsExpression(node)
    || ts.isTypeAssertionExpression(node) || ts.isNonNullExpression(node) || ts.isSatisfiesExpression(node);
}

// Every wrapper: the whole actual of an argument is the same value however it is asserted.
function unwrap(ts, node) {
  while (node && wrapper(ts, node)) node = node.expression;
  return node;
}

// The value a callee or receiver denotes. Only parentheses, `!` and `satisfies` leave the
// value and its static type alone; `as` and `<T>` select another signature than the body
// that runs, so they are never stripped and prove nothing.
function strip(ts, node) {
  while (node && (ts.isParenthesizedExpression(node) || ts.isNonNullExpression(node)
      || ts.isSatisfiesExpression(node))) {
    node = node.expression;
  }
  return node;
}

function memberName(ts, expression) {
  if (ts.isPropertyAccessExpression(expression)) return expression.name.text;
  const key = expression.argumentExpression;
  return key && (ts.isStringLiteral(key) || ts.isNoSubstitutionTemplateLiteral(key)) ? key.text : null;
}

// What the analysed sources write: member names (`x.name = ..`, `x['name'] = ..`,
// `delete x.name`, destructuring-assignment leaves, `for (x.name of ..)`) and identifier
// symbols (assignment, update, destructuring and loop-head targets). A written member or
// symbol may hold a callable other than its declaration. Computed keys, Object.assign
// and reflective writes are outside the model (README accept-list).
const writeIndex = new WeakMap();

function writes(context) {
  const { ts, program, checker } = context.public;
  if (writeIndex.has(program)) return writeIndex.get(program);
  const members = new Set();
  const symbols = new Set();
  const root = program.getCurrentDirectory() + path.sep;
  function target(node) {
    node = unwrap(ts, node);
    if (!node) return;
    if (ts.isIdentifier(node)) {
      const symbol = checker.getSymbolAtLocation(node);
      if (symbol) symbols.add(symbol);
    } else if (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) {
      const name = memberName(ts, node);
      if (name !== null) members.add(name);
    } else if (ts.isArrayLiteralExpression(node)) {
      for (const element of node.elements) target(ts.isSpreadElement(element) ? element.expression : element);
    } else if (ts.isObjectLiteralExpression(node)) {
      for (const property of node.properties) {
        if (ts.isPropertyAssignment(property)) target(property.initializer);
        else if (ts.isSpreadAssignment(property)) target(property.expression);
        else if (ts.isShorthandPropertyAssignment(property)) {
          const symbol = checker.getShorthandAssignmentValueSymbol(property);
          if (symbol) symbols.add(symbol);
        }
      }
    } else if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
      target(node.left);
    }
  }
  function visit(node) {
    if (ts.isBinaryExpression(node) && node.operatorToken.kind >= ts.SyntaxKind.FirstAssignment
        && node.operatorToken.kind <= ts.SyntaxKind.LastAssignment) target(node.left);
    else if (ts.isDeleteExpression(node)) target(node.expression);
    else if ((ts.isPrefixUnaryExpression(node) || ts.isPostfixUnaryExpression(node))
        && (node.operator === ts.SyntaxKind.PlusPlusToken || node.operator === ts.SyntaxKind.MinusMinusToken)) {
      target(node.operand);
    } else if ((ts.isForInStatement(node) || ts.isForOfStatement(node))
        && !ts.isVariableDeclarationList(node.initializer)) target(node.initializer);
    ts.forEachChild(node, visit);
  }
  for (const sf of program.getSourceFiles()) {
    if (!sf.isDeclarationFile && sf.fileName.startsWith(root)) visit(sf);
  }
  const index = { members, symbols };
  writeIndex.set(program, index);
  return index;
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

function constDeclaration(ts, declaration) {
  return ts.isVariableDeclaration(declaration) && declaration.parent.flags & ts.NodeFlags.Const
    && !declaration.type;
}

// The runtime-value declarations of a symbol: a type-only declaration (interface, alias)
// merged into it creates no value.
function valueDeclarations(ts, symbol) {
  return (symbol?.declarations ?? []).filter(d => !ts.isInterfaceDeclaration(d) && !ts.isTypeAliasDeclaration(d));
}

const maxDepth = 8;

// The class chain `[C, base, ..]` an expression certainly denotes: an identifier bound to
// one class declaration or to an unannotated `const` holding a class (alias chains of
// such consts allowed), whose symbol is never written, with no class decorator (a
// decorator may return a replacement) and an `extends` that is itself certified.
function classChain(context, expression, depth = 0) {
  const { ts, checker } = context.public;
  const value = strip(ts, expression);
  if (!value || depth > maxDepth) return null;
  let cls = null;
  if (ts.isClassExpression(value)) {
    cls = value;
  } else if (ts.isIdentifier(value)) {
    const symbol = bindingSymbol(ts, checker, value);
    const declarations = valueDeclarations(ts, symbol);
    if (!symbol || declarations.length !== 1 || writes(context).symbols.has(symbol)) return null;
    const [declaration] = declarations;
    if (ts.isClassDeclaration(declaration)) cls = declaration;
    else if (constDeclaration(ts, declaration) && declaration.initializer) {
      return classChain(context, declaration.initializer, depth + 1);
    }
  }
  if (!cls || ts.getDecorators(cls)?.length) return null;
  const extension = cls.heritageClauses?.find(clause => clause.token === ts.SyntaxKind.ExtendsKeyword);
  if (!extension) return [cls];
  const base = classChain(context, extension.types[0].expression, depth + 1);
  return base ? [cls, ...base] : null;
}

// Construction returns the instance only when no constructor of the chain returns a value.
function returnsInstance(ts, cls) {
  function valueReturn(node) {
    if (ts.isReturnStatement(node)) return Boolean(node.expression);
    if (ts.isFunctionLike(node) || ts.isClassLike(node)) return false;
    return ts.forEachChild(node, valueReturn) ?? false;
  }
  return !cls.members.some(member => ts.isConstructorDeclaration(member) && member.body
    && ts.forEachChild(member.body, valueReturn));
}

function staticMember(ts, member) {
  return Boolean(ts.canHaveModifiers(member) && ts.getModifiers(member)?.some(m => m.kind === ts.SyntaxKind.StaticKeyword));
}

// The first class of the chain that declares the member decides it: it must be exactly the
// proved owner (a method with its overloads, or one field). A computed name, a parameter
// property or an accessor of that name makes the member unproved.
function memberOrigin(ts, chain, name, isStatic, owner) {
  for (const cls of chain) {
    const same = [];
    for (const member of cls.members) {
      if (ts.isConstructorDeclaration(member)) {
        if (!isStatic) {
          for (const parameter of member.parameters) {
            if (ts.getModifiers(parameter)?.length && ts.isIdentifier(parameter.name)
                && parameter.name.text === name) same.push(parameter);
          }
        }
        continue;
      }
      if (!member.name || staticMember(ts, member) !== isStatic) continue;
      if (ts.isComputedPropertyName(member.name)) return false;
      if (member.name.text === name) same.push(member);
    }
    if (!same.length) continue;
    return ts.isPropertyDeclaration(owner)
      ? same.length === 1 && same[0] === owner
      : same.every(ts.isMethodDeclaration) && same.includes(owner);
  }
  return false;
}

// An object literal owns a member only through one plain property: no spread, no computed
// key and no second property of the name.
function literalOwns(ts, literal, name, owner) {
  if (owner.parent !== literal) return false;
  let count = 0;
  for (const property of literal.properties) {
    if (ts.isSpreadAssignment(property) || (property.name && ts.isComputedPropertyName(property.name))) return false;
    if (property.name && property.name.text === name) count++;
  }
  return count === 1;
}

function constInitializer(ts, declaration) {
  return constDeclaration(ts, declaration) && declaration.initializer ? strip(ts, declaration.initializer) : null;
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

// Callee certificate (a): an identifier bound to the body's declaration, or to an
// unannotated `const` holding it (alias chains of such consts allowed). A `new` through an
// identifier needs the certified class whose first constructor is the body.
function identifierProof(context, call, callee, body, depth = 0) {
  const { ts, checker } = context.public;
  if (depth > maxDepth) return null;
  if (ts.isNewExpression(call) && ts.isConstructorDeclaration(body)) {
    const owner = classChain(context, callee)?.find(cls => cls.members.some(ts.isConstructorDeclaration));
    return owner && body.parent === owner ? { fresh: false } : null;
  }
  const symbol = bindingSymbol(ts, checker, callee);
  const declarations = valueDeclarations(ts, symbol);
  if (!symbol || writes(context).symbols.has(symbol) || !declarations.length) return null;
  if (declarations.every(ts.isFunctionDeclaration)) {
    const bodies = declarations.filter(d => d.body);
    return bodies.length === 1 && bodies[0] === body ? { fresh: false } : null;
  }
  if (declarations.length !== 1) return null;
  const value = constInitializer(ts, declarations[0]);
  if (value === body) return { fresh: false };
  return value && ts.isIdentifier(value) ? identifierProof(context, call, value, body, depth + 1) : null;
}

// Callee certificate (b): a member access whose receiver has a value origin. Returns null
// when it does not prove the body, else whether the receiver is a fresh `new`.
function receiverProved(context, expression, owner) {
  const { ts, checker } = context.public;
  const name = memberName(ts, expression);
  if (name === null || writes(context).members.has(name)) return null;
  const receiver = strip(ts, expression.expression);
  const isStatic = staticMember(ts, owner);
  if (owner.name && ts.isPrivateIdentifier(owner.name)) return { fresh: false };
  if (receiver.kind === ts.SyntaxKind.SuperKeyword) {
    const cls = ts.findAncestor(receiver, ts.isClassLike);
    const extension = cls?.heritageClauses?.find(clause => clause.token === ts.SyntaxKind.ExtendsKeyword);
    const chain = extension && classChain(context, extension.types[0].expression);
    return chain && memberOrigin(ts, chain, name, isStatic, owner) ? { fresh: false } : null;
  }
  const symbol = ts.isIdentifier(receiver) ? bindingSymbol(ts, checker, receiver) : null;
  const declarations = valueDeclarations(ts, symbol);
  // Static access through the class value, or a namespace function.
  if (declarations.length === 1 && (ts.isModuleDeclaration(declarations[0])
      || ts.isNamespaceImport(declarations[0]) || ts.isSourceFile(declarations[0]))) {
    return ts.isFunctionDeclaration(owner) ? { fresh: false } : null;
  }
  const chain = ts.isIdentifier(receiver) && classChain(context, receiver);
  if (chain) return isStatic && memberOrigin(ts, chain, name, true, owner) ? { fresh: false } : null;
  const declaration = declarations.length === 1 ? declarations[0] : null;
  const initializer = declaration && constInitializer(ts, declaration);
  const root = initializer && uniqueReceiverUse(ts, checker, declaration, receiver) ? initializer : receiver;
  if (ts.isObjectLiteralExpression(root)) return literalOwns(ts, root, name, owner) ? { fresh: false } : null;
  if (!ts.isNewExpression(root)) return null;
  const made = classChain(context, root.expression);
  return made && made.every(cls => returnsInstance(ts, cls))
    && memberOrigin(ts, made, name, isStatic, owner) ? { fresh: true } : null;
}

function bodyFor(call, context) {
  const { ts, checker, program } = context.public;
  const expression = unwrap(ts, call.expression);
  const callee = strip(ts, call.expression);
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
  // Closed world: only a certificate form decides. (c) an immediately invoked body,
  // (a) an identifier bound to the body, (b) a member access with a proved receiver.
  let proof = null;
  if (callee === body) proof = { fresh: false };
  else if (ts.isIdentifier(callee)) proof = identifierProof(context, call, callee, body);
  else if (ts.isPropertyAccessExpression(callee) || ts.isElementAccessExpression(callee)) {
    proof = receiverProved(context, callee, proofOwner);
  }
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
