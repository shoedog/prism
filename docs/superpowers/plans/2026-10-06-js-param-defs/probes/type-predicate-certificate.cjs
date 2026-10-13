'use strict';
// Independent static TypeScript AST/checker proof. No product/oracle predicates.
const fs=require('fs'),path=require('path'),crypto=require('crypto');
function prove(ts,root,entries){
 const files=[...new Set(entries.flatMap(x=>[x.row.from.file,x.row.to.file]))];
 const program=ts.createProgram(files.map(f=>path.resolve(root,f)),{noLib:true,noResolve:true,types:[],allowJs:true,jsx:ts.JsxEmit.Preserve,target:ts.ScriptTarget.ESNext});
 const checker=program.getTypeChecker(),cache=new Map();
 function file(f){if(!cache.has(f)){
  const raw=fs.readFileSync(path.resolve(root,f)),source=raw.toString('utf8'),sf=program.getSourceFile(path.resolve(root,f));
  if(!sf)throw Error('missing AST '+f);
  const nodes=new Map(),byte=p=>Buffer.byteLength(source.slice(0,p));
  function visit(n){if(ts.isIdentifier(n))nodes.set(byte(n.getStart(sf))+':'+byte(n.end),n);ts.forEachChild(n,visit);}visit(sf);
  cache.set(f,{sf,raw,nodes,sha256:crypto.createHash('sha256').update(raw).digest('hex')});
 }return cache.get(f);}
 function runtime(n){
  let target=n;while(target.parent&&ts.isParenthesizedExpression(target.parent))target=target.parent;
  if(ts.isBinaryExpression(target.parent)&&target.parent.left===target&&target.parent.operatorToken.kind===ts.SyntaxKind.EqualsToken)return false;
  if((ts.isForOfStatement(target.parent)||ts.isForInStatement(target.parent))&&target.parent.initializer===target)return false;
  for(let c=n;c.parent;c=c.parent){const p=c.parent;
   if(ts.isTypeNode(p))return false;
   if((ts.isParameter(p)||ts.isVariableDeclaration(p)||ts.isFunctionLike(p)||ts.isClassLike(p)||ts.isBindingElement(p))&&p.name===c)return false;
   if((ts.isPropertyAccessExpression(p)||ts.isPropertyAssignment(p)||ts.isMethodDeclaration(p)||ts.isPropertyDeclaration(p))&&p.name===c)return false;
   if(ts.isJsxOpeningElement(p)||ts.isJsxClosingElement(p)||ts.isJsxSelfClosingElement(p)||ts.isJsxAttribute(p)||ts.isImportDeclaration(p)||ts.isImportClause(p)||ts.isImportSpecifier(p))return false;
  }return true;
 }
 return entries.map(entry=>{
  const {row,direction}=entry,a=row.from,b=row.to,af=file(a.file),bf=file(b.file);
  const def=af.nodes.get(a.start_byte+':'+a.end_byte),use=bf.nodes.get(b.start_byte+':'+b.end_byte);
  const symbol=n=>n&&checker.getSymbolAtLocation(n),ds=symbol(def),us=symbol(use);
  const formal=def&&ts.isParameter(def.parent)&&def.parent.name===def;
  const owner=formal&&def.parent.parent;
  const sameOwner=owner&&Buffer.byteLength(af.sf.text.slice(0,owner.getStart(af.sf)))===a.owner.start_byte&&Buffer.byteLength(af.sf.text.slice(0,owner.end))===a.owner.end_byte;
  const predicate=use&&ts.isTypePredicateNode(use.parent)&&use.parent.parameterName===use;
  const sameLine=!!use&&af.sf.getLineAndCharacterOfPosition(def?.getStart(af.sf)||0).line+1===a.line&&bf.sf.getLineAndCharacterOfPosition(use.getStart(bf.sf)).line+1===b.line&&a.line===b.line;
  const bound=!!ds&&ds===us&&ds.declarations?.includes(def?.parent);
  const pass=!!(a.parameter&&formal&&sameOwner&&a.file===b.file&&use&&b.access==='use'&&(direction==='LOST'?predicate:direction==='ADDED'&&sameLine&&runtime(use)&&bound));
  return {...entry,proof:{class:'TYPE_PREDICATE (F9)',pass,typescript:ts.version,file_sha256:bf.sha256,def_text:def?.getText(af.sf),endpoint_text:use?.getText(bf.sf),formal:!!formal,owner_matches:!!sameOwner,type_predicate_parameter:!!predicate,runtime_identifier:!!use&&runtime(use),same_line:sameLine,checker_same_formal:bound,def_symbol_declarations:ds?.declarations?.map(n=>({kind:ts.SyntaxKind[n.kind],start:n.getStart(af.sf),end:n.end})),endpoint_kind:use&&ts.SyntaxKind[use.kind]}};
 });
}
if(require.main===module){const [tsPath,root,input,output]=process.argv.slice(2);const results=prove(require(tsPath),root,JSON.parse(fs.readFileSync(input,'utf8')));fs.writeFileSync(output,JSON.stringify(results,null,1)+'\n');}
module.exports={prove};
