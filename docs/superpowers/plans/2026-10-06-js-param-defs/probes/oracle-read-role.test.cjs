'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const ts = require('/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js');
const { memberNotARead, valueNotARead } = require('./oracle-read-role.cjs');
for (const [source, written] of [
  ['obj.x = 1;', true], ['this.x = 1;', true], ['obj.x += 1;', false], ['obj.x++;', false],
  ['use(obj.x);', false], ['({x: obj.x} = other);', true], ['[obj.x] = other;', true],
  ['for (obj.x of list) {}', true], ['for (obj.x in list) {}', true], ['use({x: obj.x});', false],
]) test(source, () => {
  const sf = ts.createSourceFile('case.js', source, ts.ScriptTarget.ESNext, true);
  let base;
  const visit = n => { if (ts.isPropertyAccessExpression(n) && n.name.text === 'x') base = n.expression; ts.forEachChild(n, visit); };
  visit(sf); assert.ok(base);
  assert.equal(memberNotARead(ts, base, ['x']), written);
  assert.equal(valueNotARead(ts, base), false, 'a member receiver is still evaluated');
});
