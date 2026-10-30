// Conservative lexical binding and independent TypeScript callee resolution.
import path from 'node:path';
const isFunction = (ts,n) => ts.isFunctionLike(n) && Boolean(n.body);
function walk(ts,n,fn) { fn(n); ts.forEachChild(n,c=>walk(ts,c,fn)); }
function scope(ts,n,block=true) {
  for(let p=n.parent;p;p=p.parent)
    if(isFunction(ts,p)||ts.isSourceFile(p)||block&&(ts.isBlock(p)||ts.isCatchClause(p)))return p;
}
export function lexicalBindings(ts,sf) {
  const declarations=[],writes=[];
  walk(ts,sf,n=>{
    if(ts.isVariableDeclaration(n)||ts.isParameter(n)||ts.isFunctionDeclaration(n)||ts.isClassDeclaration(n)||ts.isBindingElement(n)) {
      if(n.name&&ts.isIdentifier(n.name))declarations.push(n);
    }
    if(ts.isBinaryExpression(n)&&n.operatorToken.kind>=ts.SyntaxKind.FirstAssignment&&n.operatorToken.kind<=ts.SyntaxKind.LastAssignment
      ||(ts.isPrefixUnaryExpression(n)||ts.isPostfixUnaryExpression(n))&&[ts.SyntaxKind.PlusPlusToken,ts.SyntaxKind.MinusMinusToken].includes(n.operator))writes.push(n);
  });
  function declScope(n) {
    if(ts.isParameter(n))return n.parent;
    let v=n;while(ts.isBindingElement(v))v=v.parent.parent;
    if(ts.isVariableDeclaration(v))return scope(ts,v,Boolean(v.parent.flags&ts.NodeFlags.BlockScoped));
    return scope(ts,n);
  }
  function declaration(name,at) {
    for(let s=at;s;s=s.parent) {
      const found=declarations.filter(d=>d.name.text===name&&declScope(d)===s);
      if(found.length)return found.length===1?found[0]:null;
    }
    return null;
  }
  function stable(name,at) {
    const d=declaration(name,at);if(!d)return null;
    if(writes.some(w=>{
      const lhs=ts.isBinaryExpression(w)?w.left:w.operand;
      return ts.isIdentifier(lhs)&&lhs.text===name&&declaration(name,lhs)===d;
    }))return null;
    return d;
  }
  function value(n,seen=new Set()) {
    if(!n||seen.has(n))return null;
    if(ts.isParenthesizedExpression(n))return value(n.expression,seen);
    if(!ts.isIdentifier(n))return n;
    const d=stable(n.text,n);if(!d||ts.isParameter(d))return null;
    return value(d.initializer||d,new Set(seen).add(n));
  }
  return {declaration,stable,value,writes,declarations};
}

export function checkerResolver(ts,test,dep,root,resolveFile) {
  const options={allowJs:true,checkJs:false,noLib:true,noResolve:false,
    target:ts.ScriptTarget.Latest,module:ts.ModuleKind.CommonJS,moduleResolution:ts.ModuleResolutionKind.Node10};
  const host=ts.createCompilerHost(options,true);
  host.resolveModuleNames=(specs,containing)=>specs.map(spec=>{
    const file=spec===dep||spec.startsWith(dep+'/')?resolveFile(path.join(root,spec.slice(dep.length).replace(/^\//,'')))
      :spec.startsWith('.')?resolveFile(path.resolve(path.dirname(containing),spec)):null;
    return file?{resolvedFileName:file,extension:file.endsWith('.ts')?ts.Extension.Ts:ts.Extension.Js,isExternalLibraryImport:false}:undefined;
  });
  const program=ts.createProgram([test],options,host),checker=program.getTypeChecker();
  const sf=program.getSourceFile(test),binders=new Map();
  function binder(file) {if(!binders.has(file))binders.set(file,lexicalBindings(ts,program.getSourceFile(file)));return binders.get(file);}
  function resolve(start,end,construct=false) {
    let call;walk(ts,sf,n=>{if((ts.isCallExpression(n)||ts.isNewExpression(n))&&n.getStart(sf)===start&&n.end===end)call=n;});
    if(!call)return null;
    let expression=call.expression,mode='direct',shift=0;
    if(ts.isPropertyAccessExpression(expression)&&['call','apply'].includes(expression.name.text)) {
      mode=expression.name.text;shift=1;expression=expression.expression;
      if(mode==='apply'&&(!call.arguments?.[1]||!ts.isArrayLiteralExpression(call.arguments[1])))return null;
    }
    const type=checker.getTypeAtLocation(expression);
    if(type.isUnion()||type.flags&ts.TypeFlags.Any)return null;
    const signatures=construct?type.getConstructSignatures():type.getCallSignatures();
    const declarations=[...new Set(signatures.map(s=>s.getDeclaration()).filter(Boolean))];
    if(declarations.length!==1)return null;
    const node=declarations[0],file=node.getSourceFile().fileName;
    if(!file.startsWith(root+path.sep)||!isFunction(ts,node)||node.getSourceFile().parseDiagnostics.length)return null;
    // JS inference is not reaching-definition analysis. Refuse top-level
    // mutable aliases and repeated whole exports rather than recovering W3.
    const b=binder(file);
    if(b.writes.some(w=>{const lhs=ts.isBinaryExpression(w)?w.left:w.operand;return ts.isIdentifier(lhs)&&b.declaration(lhs.text,lhs)&&!b.stable(lhs.text,lhs)&&ts.isSourceFile(scope(ts,b.declaration(lhs.text,lhs),false));}))return null;
    let exports=0;walk(ts,node.getSourceFile(),n=>{if(ts.isBinaryExpression(n)&&n.left.getText()==='module.exports')exports++;});
    if(exports>1)return null;
    return {file,node,mode,shift};
  }
  return {resolve};
}
