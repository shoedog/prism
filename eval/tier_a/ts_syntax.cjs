// Independent TypeScript syntax census. No project execution or package acquisition.
const fs = require('node:fs');
const path = require('node:path');
const ts = require(process.argv[2]);
const root = process.argv[3];
const files = JSON.parse(fs.readFileSync(0, 'utf8'));
const declarations = [], sites = [];
for (const file of files) {
  const text = fs.readFileSync(path.join(root, file), 'utf8');
  const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
  function pos(n) { const p = sf.getLineAndCharacterOfPosition(n.getStart(sf)); return {line:p.line+1, character:p.character}; }
  function name(n) { return n && (ts.isIdentifier(n) || ts.isStringLiteral(n) || ts.isNumericLiteral(n)) ? n.text : null; }
  function visit(n) {
    let token = n.name, shape = null;
    if (ts.isGetAccessor(n)) shape='getter';
    else if (ts.isMethodDeclaration(n) && n.body) shape='method';
    else if (ts.isFunctionDeclaration(n) && n.body) shape='function';
    else if ((ts.isPropertyAssignment(n) || ts.isPropertyDeclaration(n) || ts.isVariableDeclaration(n)) && n.initializer &&
             (ts.isArrowFunction(n.initializer) || ts.isFunctionExpression(n.initializer)))
      shape = ts.isVariableDeclaration(n) ? 'variable_callable' : 'property_callable';
    if (shape && name(token)) declarations.push({file, name:name(token), shape, ...pos(token), end_line:sf.getLineAndCharacterOfPosition(n.end).line+1});
    if (ts.isCallExpression(n)) {
      const e=n.expression, token=ts.isPropertyAccessExpression(e) ? e.name : e;
      if (name(token)) sites.push({file, name:name(token), kind:'call', ...pos(token)});
    } else if (ts.isPropertyAccessExpression(n) && !(ts.isCallExpression(n.parent) && n.parent.expression===n)) {
      sites.push({file, name:name(n.name), kind:'access', ...pos(n.name)});
    }
    ts.forEachChild(n, visit);
  }
  visit(sf);
}
process.stdout.write(JSON.stringify({declarations,sites}));
