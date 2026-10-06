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

for (const wrap of [x => `(${x})`, x => `${x}!`, x => `(${x} as any)`,
  x => `(${x} satisfies any)`, x => `(<any>${x})`]) {
  for (const member of [false, true]) {
    const access = member ? 'obj.x' : 'x';
    test(`wrapped ${wrap(access)} write/read role`, () => {
      for (const [operator, written] of [['=', true], ['+=', false]]) {
        const sf = ts.createSourceFile('case.ts', `${wrap(access)} ${operator} 1;`, ts.ScriptTarget.ESNext, true);
        let node;
        const visit = n => {
          if (member ? ts.isPropertyAccessExpression(n) : ts.isIdentifier(n) && n.text === 'x') node = n;
          ts.forEachChild(n, visit);
        };
        visit(sf); assert.ok(node);
        assert.equal(member ? memberNotARead(ts, node.expression, ['x']) : valueNotARead(ts, node), written);
      }
    });
  }
}
