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

// What the analysed sources write: member names on ANY receiver (`x.name = ..`,
// `x['name'] = ..`, `this.name = ..`, `delete x.name`, destructuring-assignment leaves,
// `for (x.name of ..)`), identifier symbols and identifier NAMES (assignment, update,
// destructuring and loop-head targets). A written member or symbol may hold a callable
// other than its declaration. The names are matched across every analysed file by spelling,
// so a global object reached through any form (a parameter, a later assignment, a nested
// property) is covered by the member name it writes. Computed keys, Object.assign and
// reflective writes are outside the model (README accept-list).
const writeIndex = new WeakMap();

function writes(context) {
  const { ts, program, checker } = context.public;
  if (writeIndex.has(program)) return writeIndex.get(program);
  const members = new Set();
  const symbols = new Set();
  const names = new Set();
  const root = program.getCurrentDirectory() + path.sep;
  function target(node) {
    node = unwrap(ts, node);
    if (!node) return;
    if (ts.isIdentifier(node)) {
      names.add(node.text);
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
          names.add(property.name.text);
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
  const scripts = [];
  for (const sf of program.getSourceFiles()) {
    if (sf.isDeclarationFile || !sf.fileName.startsWith(root)) continue;
    visit(sf);
    if (!ts.isExternalModule(sf) && !sf.commonJsModuleIndicator) scripts.push(sf);
  }
  const index = { members, symbols, names, scripts };
  writeIndex.set(program, index);
  return index;
}

// The declarations of `name` made by var, let, const, function, class, namespace or enum
// anywhere in `container`, without entering nested functions.
function declaredIn(ts, container, name) {
  const found = [];
  function binds(pattern) {
    if (ts.isIdentifier(pattern)) return pattern.text === name;
    return pattern.elements.some(element => !ts.isOmittedExpression(element) && binds(element.name));
  }
  function visit(node) {
    if (node !== container && (ts.isFunctionLike(node) || ts.isClassStaticBlockDeclaration(node))) {
      // The declaration of a nested function is a declaration of its own name here.
      if (ts.isFunctionDeclaration(node) && node.name?.text === name) found.push(node);
      return;
    }
    if (ts.isVariableDeclaration(node) && binds(node.name)) found.push(node);
    else if ((ts.isClassDeclaration(node) || ts.isModuleDeclaration(node) || ts.isEnumDeclaration(node))
        && node.name?.text === name) found.push(node);
    ts.forEachChild(node, visit);
  }
  visit(container);
  return found;
}

// Closed world for the global scope: `name` (a script function, or a namespace the script
// declares) is claimed only when, in every analysed file, it is written by no identifier
// assignment, no member write on any receiver (`globalThis.name = ..`, `root.name = ..`,
// `this.name = ..`) and declared in no script file except by the declarations in `own`.
function globalClosed(context, name, own) {
  const { ts } = context.public;
  const index = writes(context);
  return !index.names.has(name) && !index.members.has(name)
    && index.scripts.every(sf => declaredIn(ts, sf, name).every(d => own.includes(d)));
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

// Closed world for erroneous programs. The checker reports these when a class chain or a
// function is declared in a way no single body certainly runs: duplicate implementations,
// an instance field shadowing a method across a chain, `super` reaching a field, and an
// assignment to a function. A callee file or owner class chain that carries one decides
// nothing. The check reads only the diagnostics the program itself reports (a suppressed
// diagnostic is not seen; the structural rules of memberOrigin and the write index carry
// their own proofs for those).
const gateCodes = new Set([2393, 2425, 2426, 2855, 2630]);
const diagnosticIndex = new WeakMap();

function carriesDiagnostic(context, sf) {
  const { program } = context.public;
  let files = diagnosticIndex.get(program);
  if (!files) diagnosticIndex.set(program, files = new Map());
  if (!files.has(sf.fileName)) {
    files.set(sf.fileName, program.getSemanticDiagnostics(sf).some(d => gateCodes.has(d.code)));
  }
  return files.get(sf.fileName);
}

// The source files of a class and of every class its `extends` chain names.
function chainFiles(context, cls, found = new Set(), depth = 0) {
  const { ts, checker } = context.public;
  if (!cls || depth > maxDepth || found.has(cls.getSourceFile())) return found;
  found.add(cls.getSourceFile());
  const extension = cls.heritageClauses?.find(clause => clause.token === ts.SyntaxKind.ExtendsKeyword);
  const expression = extension && strip(ts, extension.types[0].expression);
  const symbol = expression && ts.isIdentifier(expression) ? bindingSymbol(ts, checker, expression) : null;
  for (const declaration of symbol?.declarations ?? []) {
    if (ts.isClassLike(declaration)) chainFiles(context, declaration, found, depth + 1);
    else if (constDeclaration(ts, declaration) && declaration.initializer
        && ts.isClassLike(strip(ts, declaration.initializer))) {
      chainFiles(context, strip(ts, declaration.initializer), found, depth + 1);
    }
  }
  return found;
}

function erroneous(context, body) {
  const { ts } = context.public;
  const files = chainFiles(context, ts.findAncestor(body, ts.isClassLike));
  files.add(body.getSourceFile());
  return [...files].some(sf => carriesDiagnostic(context, sf));
}

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
  const method = ts.isMethodDeclaration(owner);
  // A computed name may evaluate to the method's name: it is a possible match.
  const ownsInstance = member => !staticMember(ts, member) && member.name
    && (ts.isComputedPropertyName(member.name) || member.name.text === name);
  // An instance field or parameter property is an own property of the instance: it wins
  // over a method of the same name anywhere in the chain.
  if (method && !isStatic && chain.some(cls => cls.members.some(member =>
    (ts.isPropertyDeclaration(member) && ownsInstance(member))
      || (ts.isConstructorDeclaration(member) && member.parameters.some(parameter =>
        ts.getModifiers(parameter)?.length && ts.isIdentifier(parameter.name) && parameter.name.text === name))))) {
    return false;
  }
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
    // One method: its overloads and exactly one implementation.
    return ts.isPropertyDeclaration(owner)
      ? same.length === 1 && same[0] === owner
      : same.every(ts.isMethodDeclaration) && same.includes(owner)
        && same.filter(member => member.body).length === 1;
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

// A `var` binding of the function's name in the function's own var scope (the nearest
// function, static block, namespace body or file; nested functions are not entered)
// replaces the function when it assigns: an initializer, a destructuring leaf or a
// `for..in/of` head. TypeScript gives such a binding a fresh symbol (TS2300), so the
// function's symbol never lists it. A plain `var f;` assigns nothing and the function stays
// (a destructuring declaration always has an initializer or a loop head).
function varReassigns(ts, fn) {
  const name = fn.name?.text;
  if (name === undefined) return true;
  const container = ts.findAncestor(fn.parent, node => ts.isFunctionLike(node) || ts.isSourceFile(node)
    || ts.isModuleBlock(node) || ts.isClassStaticBlockDeclaration(node));
  if (!container) return true;
  function binds(pattern) {
    if (ts.isIdentifier(pattern)) return pattern.text === name;
    return pattern.elements.some(element => !ts.isOmittedExpression(element) && binds(element.name));
  }
  let found = false;
  function visit(node) {
    if (found) return;
    if (node !== container && (ts.isFunctionLike(node) || ts.isClassStaticBlockDeclaration(node))) return;
    if (ts.isVariableDeclaration(node) && !(node.parent.flags & ts.NodeFlags.BlockScoped) && binds(node.name)) {
      const head = ts.isForInStatement(node.parent.parent) || ts.isForOfStatement(node.parent.parent);
      if (node.initializer || head) found = true;
    }
    ts.forEachChild(node, visit);
  }
  visit(container);
  return found;
}

// A top-level function of a non-module script is a property of the global object, so a
// write by name anywhere in the program may replace it. A file with module syntax or
// CommonJS exports is module-local.
function globalBacked(ts, fn) {
  const sf = fn.getSourceFile();
  return fn.parent === sf && !ts.isExternalModule(sf) && !sf.commonJsModuleIndicator;
}

// The function symbol's declarations are all function declarations (overloads allowed) with
// exactly one body, and that body is `body`; no assigning `var` of the name in its own var
// scope replaces it, and a global-scope function is claimed only under the closed world of
// `globalClosed` (every script file, every receiver).
function oneFunctionBody(context, declarations, body) {
  const { ts } = context.public;
  if (!declarations.length || !declarations.every(ts.isFunctionDeclaration)) return false;
  const bodies = declarations.filter(d => d.body);
  return bodies.length === 1 && bodies[0] === body && !varReassigns(ts, body)
    && (!globalBacked(ts, body) || globalClosed(context, body.name.text, declarations));
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
    return oneFunctionBody(context, declarations, body) ? { fresh: false } : null;
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
    // `super.f` never reaches an instance field: it is an own property of the instance.
    if (!isStatic && ts.isPropertyDeclaration(owner)) return null;
    return chain && memberOrigin(ts, chain, name, isStatic, owner) ? { fresh: false } : null;
  }
  const symbol = ts.isIdentifier(receiver) ? bindingSymbol(ts, checker, receiver) : null;
  const declarations = valueDeclarations(ts, symbol);
  // Static access through the class value, or a namespace function.
  if (declarations.length === 1 && (ts.isModuleDeclaration(declarations[0])
      || ts.isNamespaceImport(declarations[0]) || ts.isSourceFile(declarations[0]))) {
    // The receiver must be unwritten by name (`N = {..}` replaces every member) and the export a
    // function symbol with exactly one body, which is the owner, however many diagnostics
    // are suppressed (the identifier callee rule). The receiver and member names are also
    // closed over the whole program: no write of either name by any receiver or global
    // object form, and no other declaration of the receiver name in its scope or in any
    // script file.
    const exported = bindingSymbol(ts, checker, expression);
    const index = writes(context);
    if (index.names.has(receiver.text) || index.members.has(receiver.text) || index.names.has(name)) return null;
    const own = declarations;
    const sameFile = declaredIn(ts, receiver.getSourceFile(), receiver.text);
    if (!sameFile.every(d => own.includes(d)) || !index.scripts.every(sf =>
      declaredIn(ts, sf, receiver.text).every(d => own.includes(d)))) return null;
    return ts.isFunctionDeclaration(owner) && exported && !index.symbols.has(exported)
      && oneFunctionBody(context, valueDeclarations(ts, exported), owner) ? { fresh: false } : null;
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

// `options.decorated === 'identity'` keeps a decorated METHOD for call-target identity under
// a matching qualification (which declaration the checker resolves, not which body runs).
function bodyFor(call, context, options = {}) {
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
  if (erroneous(context, body)) return { why: 'ambiguous-dispatch' };
  // A member decorator may return a replacement implementation: no certificate form
  // proves a decorated member's body is the one that runs.
  if (ts.canHaveDecorators(proofOwner) && ts.getDecorators(proofOwner)?.length
      && !(options.decorated === 'identity' && ts.isMethodDeclaration(proofOwner))) {
    return { why: 'ambiguous-dispatch' };
  }
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
