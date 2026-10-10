'use strict';

// ECMAScript variable environments include class static blocks and TS module
// bodies. A common surrounding function/file alone does not prove equivalence.
function variableEnvironment(ts, declaration) {
  for (let n = declaration.parent; n; n = n.parent) {
    if (ts.isClassStaticBlockDeclaration(n) || ts.isModuleBlock(n) ||
        ts.isFunctionLike(n) || ts.isSourceFile(n)) return n;
  }
  return null;
}

function varLike(ts, d) {
  return !!d && (ts.isFunctionDeclaration(d) && d.parent &&
    (ts.isSourceFile(d.parent) || ts.isBlock(d.parent) && ts.isFunctionLike(d.parent.parent)) ||
    ts.isVariableDeclaration(d) && d.parent && ts.isVariableDeclarationList(d.parent) &&
    !(d.parent.flags & (ts.NodeFlags.Let | ts.NodeFlags.Const)));
}

function varMerged(ts, sa, sb) {
  if (sa.name !== sb.name) return false;
  const declarations = [...(sa.declarations || []), ...(sb.declarations || [])];
  if (!declarations.length || !declarations.every(d => varLike(ts, d))) return false;
  const environment = variableEnvironment(ts, declarations[0]);
  return !!environment && declarations.every(d => variableEnvironment(ts, d) === environment);
}

module.exports = { variableEnvironment, varMerged };
