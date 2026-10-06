'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const ts = require('/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js');
const { variableEnvironment, varMerged } = require('./oracle-environments.cjs');

for (const [name, source, same] of [
  ['one var environment', 'function f(){ var p; {var p;} }', true],
  ['function and var', 'function f(){ function p(){} var p; }', true],
  ['separate functions', 'function f(){var p;} function g(){var p;}', false],
  ['separate static blocks', 'function f(){class C {static {var p;} static {var p;}}}', false],
  ['static versus function', 'function f(){var p; class C {static {var p;}}}', false],
  ['module bodies', 'namespace A {var p;} namespace B {var p;}', false],
  ['lexical is not var', 'function f(){let p; {var p;}}', false],
]) test(name, () => {
  const sf = ts.createSourceFile('case.ts', source, ts.ScriptTarget.ESNext, true);
  const declarations = [];
  const visit = n => {
    if ((ts.isVariableDeclaration(n) || ts.isFunctionDeclaration(n)) && n.name?.text === 'p') declarations.push(n);
    ts.forEachChild(n, visit);
  };
  visit(sf); assert.equal(declarations.length, 2);
  const symbols = declarations.map(d => ({ name: 'p', declarations: [d] }));
  assert.equal(varMerged(ts, ...symbols), same);
  if (!same && !name.startsWith('lexical')) assert.notEqual(variableEnvironment(ts, declarations[0]), variableEnvironment(ts, declarations[1]));
});
