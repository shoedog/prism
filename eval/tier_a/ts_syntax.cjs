// Independent TypeScript syntax census. No project execution or package acquisition.
const fs = require('node:fs');
const path = require('node:path');
const ts = require(process.argv[2]);
const root = process.argv[3];
const files = JSON.parse(fs.readFileSync(0, 'utf8'));
const declarations = [], sites = [], bindings = [];
for (const file of files) {
  const text = fs.readFileSync(path.join(root, file), 'utf8');
  const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
  function pos(n) { const p = sf.getLineAndCharacterOfPosition(n.getStart(sf)); return {line:p.line+1, character:p.character}; }
  function name(n) { return n && (ts.isIdentifier(n) || ts.isStringLiteral(n) || ts.isNumericLiteral(n)) ? n.text : null; }
  function binding(n) {
    const positions = [pos(n.name)];
    if (n.initializer && (ts.isArrowFunction(n.initializer) || ts.isFunctionExpression(n.initializer))) {
      positions.push(pos(n.initializer));
      if (n.initializer.name) positions.push(pos(n.initializer.name));
    }
    // Overloads are contiguous declarations in one lexical container. Stop at
    // the implementation; a nested or unrelated same-name callable is distinct.
    const siblings = n.parent.statements || n.parent.members || [];
    const isOverload = x => x && x.kind === n.kind && name(x.name) === name(n.name) &&
      !!x.modifiers?.some(m => m.kind === ts.SyntaxKind.StaticKeyword) ===
      !!n.modifiers?.some(m => m.kind === ts.SyntaxKind.StaticKeyword);
    let first = siblings.indexOf(n), last = first;
    if (ts.isFunctionDeclaration(n) || ts.isMethodDeclaration(n)) {
      while (first > 0 && isOverload(siblings[first-1]) && !siblings[first-1].body) first--;
      while (last >= 0 && !siblings[last].body && isOverload(siblings[last+1])) last++;
      for (let i=first; i>=0 && i<=last; i++) positions.push(pos(siblings[i].name));
    }
    const start = (first >= 0 ? siblings[first] : n).getStart(sf);
    const end = (last >= 0 ? siblings[last] : n).end;
    const p = sf.getLineAndCharacterOfPosition(start), q = sf.getLineAndCharacterOfPosition(end);
    return {file, name:name(n.name), ...pos(n.name),
      declaration_start:{line:p.line+1, character:p.character},
      declaration_end:{line:q.line+1, character:q.character}, definition_positions:positions};
  }
  function visit(n) {
    let token = n.name, shape = null;
    if (ts.isGetAccessor(n)) shape='getter';
    else if (ts.isMethodDeclaration(n) && n.body) shape='method';
    else if (ts.isFunctionDeclaration(n) && n.body) shape='function';
    else if ((ts.isPropertyAssignment(n) || ts.isPropertyDeclaration(n) || ts.isVariableDeclaration(n)) && n.initializer &&
             (ts.isArrowFunction(n.initializer) || ts.isFunctionExpression(n.initializer)))
      shape = ts.isVariableDeclaration(n) ? 'variable_callable' : 'property_callable';
    if ((shape || ts.isFunctionDeclaration(n) || ts.isMethodDeclaration(n)) && name(token)) {
      const identity = binding(n);
      bindings.push(identity);
      if (shape) declarations.push({...identity, shape, end_line:sf.getLineAndCharacterOfPosition(n.end).line+1});
    }
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
process.stdout.write(JSON.stringify({declarations,sites,bindings}));
