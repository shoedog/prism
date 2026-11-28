'use strict';

// The out-of-model belt of the callable certificate (README, amended accept-list). A callable
// verdict is a STATIC-BINDING grade: it is decided over statically bound bodies only. Two
// classes are out of model, zero credit either way (UNDECIDED with a named reason):
//   `out-of-model-module-mutation`: replacement or mutation of a module object, namespace
//     object, `module.exports`/`exports` or a global, by any spelling (aliases of `module`,
//     `exports` and the global object included);
//   `out-of-model-suppression`: a program that only compiles under diagnostic suppression, and
//     duplicate declarations of the callee or its receiver name.
// Detection is a FILE-WIDE BELT BY SPELLING for the callee's file F (and, for a namespace
// member, every file that contributes to the namespace), added to the name-based
// program-wide rules of callable.cjs, never instead of them. A belt component applies where the
// mutation can reach the body's binding: a write to the module object matters to a call that
// crosses a module boundary or reaches a namespace member, a write to the global object to a
// script-global function or a namespace member; suppression always applies.

const moduleRoots = ['module', 'exports'];
const globalRoots = ['globalThis', 'window', 'self', 'global'];
const reflective = new Set(['assign', 'defineProperty', 'defineProperties', 'setPrototypeOf',
  'set', 'deleteProperty']);
const suppression = /@ts-(?:nocheck|ignore|expect-error)\b/;
const facts = new WeakMap();

function peel(ts, node) {
  while (node && (ts.isParenthesizedExpression(node) || ts.isAsExpression(node)
      || ts.isTypeAssertionExpression(node) || ts.isNonNullExpression(node) || ts.isSatisfiesExpression(node))) {
    node = node.expression;
  }
  return node;
}

// The root of a member or element chain: an identifier's text, 'this', or null.
function rootOf(ts, node) {
  node = peel(ts, node);
  while (node && (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node))) {
    node = peel(ts, node.expression);
  }
  if (!node) return null;
  if (ts.isIdentifier(node)) return node.text;
  return node.kind === ts.SyntaxKind.ThisKeyword ? 'this' : null;
}

// Every write target of a file (assignment, update, delete, destructuring-assignment leaf, loop
// head), as expressions, with the call targets of reflective writers.
function writeTargets(ts, sf) {
  const targets = [];
  function leaf(node) {
    node = peel(ts, node);
    if (!node) return;
    if (ts.isArrayLiteralExpression(node)) {
      for (const element of node.elements) leaf(ts.isSpreadElement(element) ? element.expression : element);
    } else if (ts.isObjectLiteralExpression(node)) {
      for (const property of node.properties) {
        if (ts.isPropertyAssignment(property)) leaf(property.initializer);
        else if (ts.isSpreadAssignment(property)) leaf(property.expression);
        else if (ts.isShorthandPropertyAssignment(property)) targets.push(property.name);
      }
    } else if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
      leaf(node.left);
    } else {
      targets.push(node);
    }
  }
  function visit(node) {
    if (ts.isBinaryExpression(node) && node.operatorToken.kind >= ts.SyntaxKind.FirstAssignment
        && node.operatorToken.kind <= ts.SyntaxKind.LastAssignment) leaf(node.left);
    else if (ts.isDeleteExpression(node)) leaf(node.expression);
    else if ((ts.isPrefixUnaryExpression(node) || ts.isPostfixUnaryExpression(node))
        && (node.operator === ts.SyntaxKind.PlusPlusToken || node.operator === ts.SyntaxKind.MinusMinusToken)) {
      leaf(node.operand);
    } else if ((ts.isForInStatement(node) || ts.isForOfStatement(node))
        && !ts.isVariableDeclarationList(node.initializer)) leaf(node.initializer);
    else if (ts.isCallExpression(node) && node.arguments.length) {
      const callee = peel(ts, node.expression);
      if (ts.isPropertyAccessExpression(callee) && reflective.has(callee.name.text)
          && ts.isIdentifier(callee.expression) && ['Object', 'Reflect'].includes(callee.expression.text)) {
        targets.push(node.arguments[0]);
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(sf);
  return targets;
}

// The names that hold one of `seeds` (`const alias = module`, `alias = module.exports`,
// `const e = exports`), transitively and by spelling.
function aliasesOf(ts, sf, seeds) {
  const names = new Set(seeds);
  let grown = true;
  while (grown) {
    grown = false;
    (function visit(node) {
      let name = null;
      let value = null;
      if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
        name = node.name.text;
        value = node.initializer;
      } else if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken
          && ts.isIdentifier(peel(ts, node.left))) {
        name = peel(ts, node.left).text;
        value = node.right;
      }
      if (name !== null && !names.has(name) && names.has(rootOf(ts, value))) {
        names.add(name);
        grown = true;
      }
      ts.forEachChild(node, visit);
    })(sf);
  }
  return names;
}

function fileFacts(context, sf) {
  const { ts } = context.public;
  let known = facts.get(sf);
  if (known) return known;
  const targets = writeTargets(ts, sf);
  const modules = aliasesOf(ts, sf, moduleRoots);
  const globals = aliasesOf(ts, sf, globalRoots);
  const script = !ts.isExternalModule(sf) && !sf.commonJsModuleIndicator;
  const topLevelThis = target => {
    if (rootOf(ts, target) !== 'this' || !script) return false;
    for (let p = target.parent; p; p = p.parent) {
      if ((ts.isFunctionLike(p) && !ts.isArrowFunction(p)) || ts.isClassLike(p)) return false;
    }
    return true;
  };
  known = {
    module: targets.some(target => modules.has(rootOf(ts, target))),
    global: targets.some(target => globals.has(rootOf(ts, target)) || topLevelThis(target)),
    suppressed: suppression.test(sf.text),
  };
  facts.set(sf, known);
  return known;
}

// The declaration node a body is declared by, and its name: a function declaration, or a
// function value held by one variable.
function declarationOf(ts, body) {
  if (ts.isFunctionDeclaration(body)) return body.name ? { node: body, name: body.name.text, own: body } : null;
  if (!ts.isArrowFunction(body) && !ts.isFunctionExpression(body)) return null;
  let top = body;
  while (top.parent && (ts.isParenthesizedExpression(top.parent) || ts.isAsExpression(top.parent)
      || ts.isNonNullExpression(top.parent) || ts.isSatisfiesExpression(top.parent))) top = top.parent;
  const holder = top.parent;
  if (!holder || !ts.isVariableDeclaration(holder) || !ts.isIdentifier(holder.name)
      || !ts.isVariableDeclarationList(holder.parent) || !holder.parent.parent) return null;
  return { node: holder.parent.parent, name: holder.name.text, own: holder };
}

function innerNamespace(ts, node) {
  return ts.isModuleBlock(node.parent) ? ts.findAncestor(node.parent, ts.isModuleDeclaration) : null;
}

// The files that contribute to the namespace (every declaration of its merged symbol).
function contributing(ts, checker, namespace) {
  const files = new Set();
  for (let p = namespace; p; p = p.parent) {
    if (!ts.isModuleDeclaration(p)) continue;
    const symbol = checker.getSymbolAtLocation(p.name);
    for (const declaration of symbol?.declarations ?? [p]) files.add(declaration.getSourceFile());
  }
  return files;
}

function blockOf(ts, declaration) {
  let body = declaration.body;
  while (body && ts.isModuleDeclaration(body)) body = body.body;
  return body && ts.isModuleBlock(body) ? body : null;
}

// A declaration of the name that is not the proved function (its overloads are function
// declarations) in the same scope: `let`, `const`, `class`, `enum`, `namespace`, or a `var` that
// assigns. A plain `var f;` assigns nothing and is no duplicate (README, var-scope rule).
function duplicates(ts, found, scope, own) {
  return found.filter(declaration => {
    if (declaration === own || ts.isFunctionDeclaration(declaration)) return false;
    if (!ts.isVariableDeclaration(declaration)) return sameScope(ts, declaration, scope);
    if (!(declaration.parent.flags & ts.NodeFlags.BlockScoped)) {
      const head = ts.isForInStatement(declaration.parent.parent) || ts.isForOfStatement(declaration.parent.parent);
      return Boolean(declaration.initializer) || head;
    }
    return sameScope(ts, declaration, scope);
  });
}

function sameScope(ts, declaration, scope) {
  const owner = ts.findAncestor(declaration.parent, node => ts.isBlock(node) || ts.isModuleBlock(node)
    || ts.isSourceFile(node) || ts.isCaseBlock(node) || ts.isForStatement(node) || ts.isForInStatement(node)
    || ts.isForOfStatement(node) || ts.isCatchClause(node) || ts.isFunctionLike(node));
  return owner === scope;
}

// `callee` is the called expression stripped of parentheses. Returns a named reason or null.
function belt(context, call, body, callee, helpers) {
  const { ts, checker } = context.public;
  const { declaredIn, writes, valueDeclarations, bindingSymbol } = helpers;
  const file = body.getSourceFile();
  const declaration = declarationOf(ts, body);
  const space = declaration && innerNamespace(ts, declaration.node);
  const files = space ? contributing(ts, checker, space) : new Set();
  files.add(file);
  const crossFile = call.getSourceFile() !== file;
  const global = declaration && ts.isSourceFile(declaration.node.parent)
    && !ts.isExternalModule(file) && !file.commonJsModuleIndicator;
  const index = writes(context);
  let mutated = false;
  for (const sf of files) {
    const known = fileFacts(context, sf);
    if ((crossFile || space) && known.module) mutated = true;
    if ((global || space) && known.global) mutated = true;
  }
  // A namespace member of this name written on any receiver, anywhere (`(N as any).f = ..`).
  if (space && declaration.name && index.members.has(declaration.name)) mutated = true;
  if (mutated) return 'out-of-model-module-mutation';
  for (const sf of files) if (fileFacts(context, sf).suppressed) return 'out-of-model-suppression';
  if (declaration) {
    const scopes = space
      ? (checker.getSymbolAtLocation(space.name)?.declarations ?? [space]).map(d => blockOf(ts, d)).filter(Boolean)
      : [declaration.node.parent];
    for (const scope of scopes) {
      if (duplicates(ts, declaredIn(ts, scope, declaration.name), scope, declaration.own).length) return 'out-of-model-suppression';
    }
  }
  if (ts.isPropertyAccessExpression(callee) || ts.isElementAccessExpression(callee)) {
    const receiver = peel(ts, callee.expression);
    const symbol = receiver && ts.isIdentifier(receiver) ? bindingSymbol(ts, checker, receiver) : null;
    const own = valueDeclarations(ts, symbol);
    if (own.length === 1 && (ts.isModuleDeclaration(own[0]) || ts.isNamespaceImport(own[0]) || ts.isSourceFile(own[0]))) {
      const other = d => !own.includes(d);
      if (declaredIn(ts, receiver.getSourceFile(), receiver.text).some(other)
          || index.scripts.some(sf => declaredIn(ts, sf, receiver.text).some(other))) {
        return 'out-of-model-suppression';
      }
    }
  }
  return null;
}

module.exports = { belt };
