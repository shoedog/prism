#!/usr/bin/env node
// Byte-level checker binding proof. Input: rowdiff output from byte_dump.rs.
// Exact identifier bytes, declaration-name span, file and callable byte identity
// must agree. Collapsed endpoints require unanimous byte-identified occurrences;
// ambiguous owners remain UNDECIDED, never a proved mismatch.
'use strict';
const fs = require('fs'), path = require('path');
const [tsPath, root, changedFile, outFile, ...args] = process.argv.slice(2);
const ts = require(tsPath), S = ts.SyntaxKind;
const detailsFile = args[0] === '--details' ? args[1] : null;
const changed = fs.readFileSync(changedFile,'utf8').split('\n').filter(Boolean).map(JSON.parse);
const files = [...new Set(changed.flatMap(c => [c.row.from.file,c.row.to.file]))].map(f=>path.resolve(root,f));
const program = ts.createProgram(files,{allowJs:true,noLib:true,types:[],jsx:ts.JsxEmit.Preserve,target:ts.ScriptTarget.ESNext,
 module:ts.ModuleKind.ESNext,moduleResolution:ts.ModuleResolutionKind.Bundler,skipLibCheck:true,noEmit:true});
const checker = program.getTypeChecker(), indices = new Map();
const byteMaps = new WeakMap();
const byte = (sf,pos) => {
 if(!byteMaps.has(sf)) {
  const raw=fs.readFileSync(sf.fileName),encoded=Buffer.from(sf.text,'utf8');
  const strippedBom=raw.length===encoded.length+3&&raw.subarray(0,3).equals(Buffer.from([0xef,0xbb,0xbf]))&&raw.subarray(3).equals(encoded);
  if(!raw.equals(encoded)&&!strippedBom){byteMaps.set(sf,null);return null;}
  const map=new Uint32Array(sf.text.length+1);let bytes=strippedBom?3:0;
  for(let i=0;i<sf.text.length;i++) {
   map[i]=bytes;const cp=sf.text.codePointAt(i);
   if(cp>0xffff){map[++i]=bytes;bytes+=4;}else bytes+=cp<0x80?1:cp<0x800?2:3;
  }map[sf.text.length]=bytes;byteMaps.set(sf,map);
 }return byteMaps.get(sf)?.[pos]??null;
};
function index(rel) {
 const file = path.resolve(root,rel); if(indices.has(file)) return indices.get(file);
 const sf = program.getSourceFile(file), ids = new Map(), members = new Map();
 if(sf&&byte(sf,0)===null){const result={sf,ids,members,why:'physical_source_text_mismatch'};indices.set(file,result);return result;}
 // PR-B: member-path endpoints (`a.b`) are keyed by their exact bytes too.
 if(sf) { const visit=n=> { const k=`${byte(sf,n.getStart(sf))}:${byte(sf,n.end)}`;
  if(ts.isIdentifier(n)) ids.set(k,n);
  else if((ts.isPropertyAccessExpression(n)||ts.isElementAccessExpression(n)||ts.isCallExpression(n)||ts.isNonNullExpression(n))&&!members.has(k)) members.set(k,n);
  ts.forEachChild(n,visit); }; visit(sf); }
 const result={sf,ids,members};indices.set(file,result);return result;
}
// PR-B helpers: synthetic owners, member bases, innermost function-like container.
const isSynthetic=e=>!!(e.owner&&typeof e.owner.name==='string'&&e.owner.name.startsWith('<cb@'));
function memberBase(n){let b=n;while(ts.isPropertyAccessExpression(b)||ts.isElementAccessExpression(b)||ts.isNonNullExpression(b)||ts.isParenthesizedExpression(b)||ts.isCallExpression(b)||ts.isAsExpression(b)||ts.isTypeAssertionExpression(b)||(ts.isSatisfiesExpression&&ts.isSatisfiesExpression(b)))b=b.expression;return ts.isIdentifier(b)||b.kind===S.ThisKeyword?b:null;}
// `this` has no symbol: two `this` tokens bind alike iff they share the non-arrow this-container.
const thisKey=n=>n.kind===S.ThisKeyword?ts.getThisContainer(n,false,false):null;
const sameBinding=(a,b)=>{if(a.kind===S.ThisKeyword||b.kind===S.ThisKeyword)return a.kind===b.kind&&thisKey(a)===thisKey(b)?'CORRECT':'WRONG';
 const sa=symbolOf(a),sb=symbolOf(b);
 if(sa&&sb)return sa===sb||varMerged(sa,sb)?'CORRECT':'WRONG';
 // noLib program: an unresolved name is an ambient global; two unresolved same-text references
 // bind the same global (any local declaration would have produced a symbol).
 if(!sa&&!sb&&a.text===b.text)return 'CORRECT';
 // A free (undeclared) name and a declared binding are different bindings.
 if(a.text===b.text&&(!sa)!==(!sb))return 'WRONG';
 return 'UNDECIDED';};
function innermostFunction(n){for(let p=n.parent;p;p=p.parent)if(ts.isFunctionLike(p)&&!ts.isFunctionTypeNode(p)&&!ts.isConstructorTypeNode(p)&&!ts.isCallSignatureDeclaration(p)&&!ts.isMethodSignature(p)&&!ts.isIndexSignatureDeclaration(p))return p;return null;}
function exactEndpoint(e){
 if(!Number.isInteger(e.start_byte)||!Number.isInteger(e.end_byte)) return {why:'missing_byte_identity'};
 if(e.start_byte>=e.end_byte) return {why:'zero_width_endpoint'};
 const {sf,ids,members,why}=index(e.file);if(why)return {why};if(!sf) return {why:'no_source'};
 const key=`${e.start_byte}:${e.end_byte}`,id=ids.get(key);
 if(id&&(!e.path.fields||!e.path.fields.length)) return {id,base:id,sf};
 const m=members.get(key);
 if(m&&e.path.fields&&e.path.fields.length){const base=memberBase(m);return base?{id:base,base,member:m,sf}:{wrong:true,why:'member_without_identifier_base'};}
 if(id) return {id,base:id,sf};
 return {wrong:true,why:'not_exact_identifier_bytes'};
}
function ownerScope(c,defId,useIds){
 // Synthetic owners own their own scope only: the Def's innermost function-like container
 // IS the owner, and every Use lies inside the owner's bytes (captures into nested callables allowed).
 const o=c.row.from.owner;if(!isSynthetic(c.row.from))return null;
 if('ambiguous' in o)return {step1:'UNDECIDED',why:'synthetic_owner_ambiguous'};
 const fn=innermostFunction(defId),sf=defId.getSourceFile();
 if(!fn||!ownerMatches(c.row.from,fn,sf))return {step1:'WRONG',why:'synthetic_owner_scope'};
 for(const u of useIds){const b=byte(sf,u.getStart(sf));if(b<o.start_byte||byte(sf,u.end)>o.end_byte)return {step1:'WRONG',why:'use_outside_synthetic_owner'};}
 return null;
}
// ECMAScript: `var` declarations and function declarations of one name in one function (or file)
// scope are a single binding. TS's JS binder can split them (expando constructor functions,
// function + var redeclaration); treat such pairs as the same binding.
const environments=require('./oracle-environments.cjs');
const varMerged=(sa,sb)=>environments.varMerged(ts,sa,sb);
const roles=require('./oracle-read-role.cjs');
// Binding scope of a declaration: parameters -> their function; `var` -> nearest function or
// file; everything else (let/const/class/function/catch/for-head) -> nearest block-like scope.
function bindingScope(d){
 const isVar=ts.isVariableDeclaration(d)&&d.parent&&ts.isVariableDeclarationList(d.parent)&&!(d.parent.flags&(ts.NodeFlags.Let|ts.NodeFlags.Const));
 if(isVar)return environments.variableEnvironment(ts,d)||d.getSourceFile();
 let n=ts.isParameter(d)?d.parent:d.parent;
 for(;n;n=n.parent){
  if(ts.isFunctionLike(n)||ts.isSourceFile(n)||ts.isModuleBlock(n))return n;
  if(!isVar&&!ts.isParameter(d)&&(ts.isBlock(n)||ts.isForStatement(n)||ts.isForInStatement(n)||ts.isForOfStatement(n)||ts.isCatchClause(n)||ts.isCaseBlock(n)))return n;
 }return d.getSourceFile();
}
// PR-B: a synthetic pass's zero-width anchor can only stand for occurrences its walk admits:
// reads (B-D12), outside nested binders that exclude the Def (B-D3 fence). Occurrences whose
// checker binding is declared in a scope that does not contain the Def are those fenced ones.
// If no candidate survives, keep all (the row then decides on the unfiltered occurrences).
function syntheticCandidates(c,uses){
 const keep=uses.filter(u=>{if(notARead(u))return false;const sym=symbolOf(u),d=sym&&(sym.valueDeclaration||(sym.declarations||[])[0]);
  if(!d)return true;const sc=bindingScope(d),ds=d.getSourceFile();
  if(path.resolve(ds.fileName)!==path.resolve(root,c.row.from.file))return true;
  return byte(ds,sc.getStart(ds))<=c.row.from.start_byte&&c.row.from.start_byte<byte(ds,sc.end);});
 return keep.length?keep:uses;
}
// A collapsed member-path Use (`p.x`, `p[…]`) stands only for occurrences of that access.
function accessMatches(id,p){
 const f=(p.fields||[])[0];if(f===undefined)return true;
 const par=id.parent;
 if(f==='[]')return ts.isElementAccessExpression(par)&&par.expression===id;
 return ts.isPropertyAccessExpression(par)&&par.expression===id&&par.name.text===f.replace(/\(.*$/,'');
}
function aliasTarget(defId,base){
 // `q = req` / `var q = req` / `const {a} = obj` / `const [a] = arr`: the alias twin Def of
 // the RHS name sits on the bytes of the LHS binding; its binding is the RHS identifier's.
 let p=defId.parent;
 if(ts.isBinaryExpression(p)&&p.left===defId&&p.operatorToken.kind===S.EqualsToken&&ts.isIdentifier(p.right)&&p.right.text===base)return p.right;
 while(p&&(ts.isBindingElement(p)||ts.isObjectBindingPattern(p)||ts.isArrayBindingPattern(p)))p=p.parent;
 if(p&&ts.isVariableDeclaration(p)&&p.initializer){let i=p.initializer;while(ts.isParenthesizedExpression(i)||ts.isAsExpression(i)||ts.isNonNullExpression(i))i=i.expression;
  const b=ts.isIdentifier(i)?i:memberBase(i);if(b&&ts.isIdentifier(b)&&b.text===base)return b;}
 return null;
}
function exactId(e) {
 if(!Number.isInteger(e.start_byte)||!Number.isInteger(e.end_byte)) return {why:'missing_byte_identity'};
 if(e.start_byte>=e.end_byte) return {why:'zero_width_endpoint'};
 const {sf,ids,why}=index(e.file);if(why)return {why};if(!sf) return {why:'no_source'};
 const id=ids.get(`${e.start_byte}:${e.end_byte}`);
 return id ? {id,sf} : {wrong:true,why:'not_exact_identifier_bytes'};
}
function symbolOf(id) {
 if(ts.isShorthandPropertyAssignment(id.parent)&&id.parent.name===id) return checker.getShorthandAssignmentValueSymbol(id.parent);
 const sym=checker.getSymbolAtLocation(id);
 // PR-B: in JS files the TS binder turns `exports.x = …` / `module.exports = …` into CommonJS
 // export declarations, so the base identifier resolves to that synthesized symbol even when an
 // enclosing formal or local named `exports`/`module` shadows it (AMD `define(…, function
 // (require, exports) {…})`). ECMAScript scoping binds the lexical name: re-resolve in scope.
 const d=sym&&(sym.valueDeclaration||(sym.declarations||[])[0]);
 if(ts.isIdentifier(id)&&(id.text==='exports'||id.text==='module')&&(!d||!(ts.isParameter(d)||ts.isVariableDeclaration(d)||ts.isBindingElement(d)||ts.isFunctionDeclaration(d)))){
  const lexical=checker.getSymbolsInScope(id,ts.SymbolFlags.Value).find(x=>x.name===id.text);
  const ld=lexical&&(lexical.valueDeclaration||(lexical.declarations||[])[0]);
  if(ld&&(ts.isParameter(ld)||ts.isVariableDeclaration(ld)||ts.isBindingElement(ld)))return lexical;
  // No lexical binding: the free CommonJS `exports`/`module` name; every such reference binds it.
  if(!ld)return FREE_COMMONJS[id.text];
 }
 return sym;
}
const FREE_COMMONJS={exports:{name:'<free exports>'},module:{name:'<free module>'}};
function ownerStart(fn,sf) {
 let start=fn.getStart(sf);
 for(const m of fn.modifiers||[]) if(m.kind===S.ExportKeyword||m.kind===S.DefaultKeyword) start=m.end;
 const scanner=ts.createScanner(ts.ScriptTarget.ESNext,true,sf.languageVariant,sf.text);
 scanner.setTextPos(start);scanner.scan();return byte(sf,scanner.getTokenPos());
}
function ownerMatches(e,fn,sf) {
 return e.owner && !('ambiguous' in e.owner) && e.owner.start_byte===ownerStart(fn,sf) && e.owner.end_byte===byte(sf,fn.end);
}
function writesBefore(decl,use) {
 const sym=symbolOf(decl.name);let bad=false;
 const visit=n=> {if(bad)return;
  if(ts.isIdentifier(n)&&n!==decl.name&&symbolOf(n)===sym) {
   const p=n.parent;const write=ts.isBinaryExpression(p)&&p.left===n&&p.operatorToken.kind>=S.FirstAssignment&&p.operatorToken.kind<=S.LastAssignment ||
    (ts.isPrefixUnaryExpression(p)||ts.isPostfixUnaryExpression(p))&&(p.operator===S.PlusPlusToken||p.operator===S.MinusMinusToken);
   const rhs=ts.isBinaryExpression(p)&&p.right.pos<=use.pos&&use.end<=p.right.end;
   if(write&&!rhs&&n.getStart()<use.getStart())bad=true;
  }ts.forEachChild(n,visit);
 };if(decl.parent.body)visit(decl.parent.body);return bad;
}
function unreachable(use) {
 // Independent syntax proof for unconditional return/throw before a sibling
 // statement; this decides only proved dead code, never claims complete CFG QA.
 for(let child=use,p=use.parent;p;child=p,p=p.parent) {
  if(ts.isBlock(p)||ts.isSourceFile(p)) {
   for(const stmt of p.statements) {
    if(stmt.pos>=child.pos)break;
    if(ts.isReturnStatement(stmt)||ts.isThrowStatement(stmt))return true;
   }
  }
  if(ts.isFunctionLike(p))break;
 }return false;
}
// PR-B E4 probe: a Use whose identifier only binds or is only written (`=` target).
function notARead(n){return roles.valueNotARead(ts,n);}
// Identifiers in property-name/attribute-name positions are not variable references
// (TS checker semantics); a zero-width collapsed endpoint must only consider references.
function isReferencePosition(n) {
 const p=n.parent; if(!p)return true;
 if(ts.isPropertyAccessExpression(p)&&p.name===n)return false;
 if(ts.isQualifiedName(p)&&p.right===n)return false;
 if(ts.isPropertyAssignment(p)&&p.name===n)return false;
 if(ts.isJsxAttribute(p)&&p.name===n)return false;
 if(ts.isBindingElement(p)&&p.propertyName===n)return false;
 if((ts.isImportSpecifier(p)||ts.isExportSpecifier(p))&&p.propertyName===n)return false;
 if((ts.isPropertySignature(p)||ts.isPropertyDeclaration(p)||ts.isMethodDeclaration(p)||ts.isMethodSignature(p)||
     ts.isGetAccessorDeclaration(p)||ts.isSetAccessorDeclaration(p)||ts.isEnumMember(p))&&p.name===n)return false;
 if((ts.isLabeledStatement(p)||ts.isBreakStatement(p)||ts.isContinueStatement(p))&&p.label===n)return false;
 return true;
}
function defUse(c) {
 const fields=c.row.from.path.fields&&c.row.from.path.fields.length;
 const from=fields?exactEndpoint(c.row.from):exactId(c.row.from);if(!from.id)return {step1:from.wrong?'WRONG':'UNDECIDED',why:from.why};
 // PR-B mechanisms beyond PR-A's parameter rows: local/assignment Defs (same checker symbol),
 // member paths (same base symbol, syntactic path), alias twins (alias RHS symbol).
 if(!c.row.from.parameter) return defUseLocal(c,from);
 let uses=[],collapsed=false;
 const endpoint=exactId(c.row.to);
 if(endpoint.id)uses=[endpoint.id];
 else if(endpoint.why==='zero_width_endpoint') {
  collapsed=true;const {sf,ids,why}=index(c.row.to.file),owner=c.row.to.owner;
  if(why)return {step1:'UNDECIDED',why};
  if(!sf||!owner||'ambiguous' in owner)return {step1:'UNDECIDED',why:'collapsed_owner_missing'};
  uses=[...ids.values()].filter(id=>id.text===c.row.from.path.base && isReferencePosition(id) &&
   sf.getLineAndCharacterOfPosition(id.getStart(sf)).line+1===c.row.to.line &&
   owner.start_byte<=byte(sf,id.getStart(sf))&&byte(sf,id.end)<=owner.end_byte);
  if(isSynthetic(c.row.to))uses=syntheticCandidates(c,uses);
  if(!uses.length)return {step1:'UNDECIDED',why:'collapsed_no_identifier'};
 }else return {step1:endpoint.wrong?'WRONG':'UNDECIDED',why:endpoint.why};
 const verdicts=uses.map(use=> {
  if(!collapsed&&!isReferencePosition(use))return {step1:'WRONG',why:'E7_non_reference_use'};
  const sym=symbolOf(use),decl=sym&&(sym.valueDeclaration||(sym.declarations||[])[0]);
  if(!decl)return {step1:'UNDECIDED',why:'no_symbol_declaration'};
  const name=decl.name,ds=decl.getSourceFile();
  if(!name||!ts.isIdentifier(name))return {step1:'WRONG',why:'non_identifier_declaration'};
  const spanMatches=path.resolve(root,c.row.from.file)===path.resolve(ds.fileName)&&
   byte(ds,name.getStart(ds))===c.row.from.start_byte&&byte(ds,name.end)===c.row.from.end_byte;
  if(!spanMatches)return {step1:'WRONG',why:'declaration_span_mismatch'};
  if(c.row.from.parameter) {
   if(!ts.isParameter(decl))return {step1:'WRONG',why:'def_not_parameter_declaration'};
   if(!c.row.from.owner||'ambiguous' in c.row.from.owner)return {step1:'UNDECIDED',why:'parameter_owner_ambiguous'};
   if(!ownerMatches(c.row.from,decl.parent,ds))return {step1:'WRONG',why:'parameter_owner_mismatch'};
  }else if(ts.isParameter(decl))return {step1:'WRONG',why:'def_not_parameter_record'};
  const shape=ts.isParameter(decl)?decl.dotDotDotToken?'rest':ts.isArrowFunction(decl.parent)&&!decl.parent.getChildren(ds).some(n=>n.kind===S.OpenParenToken)?'bare_arrow':'other_formal':'non_parameter';
  const label=(c.head_label||c.base_label||{}).confidence;
  return {step1:'CORRECT',shape,e4:notARead(use)?'USE_NOT_READ':undefined,step2:unreachable(use)?'UNREACHABLE':label==='exact'&&ts.isParameter(decl)?writesBefore(decl,use)?'EXACT_PRIOR_WRITE':'EXACT_OK':'n/a',
   checked_use_span:[byte(use.getSourceFile(),use.getStart()),byte(use.getSourceFile(),use.end)]};
 });
 if(collapsed) {
  if(verdicts.some(v=>v.step1==='UNDECIDED')||verdicts.some(v=>v.step1!==verdicts[0].step1))
   return {step1:'UNDECIDED',why:'collapsed_mixed_bindings',occurrences:verdicts};
  if(verdicts[0].step1==='WRONG')return {...verdicts[0],occurrences:verdicts};
  return {...verdicts[0],e4:verdicts.every(v=>v.e4)?'USE_NOT_READ':undefined,step2:verdicts.some(v=>v.step2==='UNREACHABLE')?'UNREACHABLE':verdicts.some(v=>v.step2==='EXACT_PRIOR_WRITE')?'EXACT_PRIOR_WRITE':verdicts[0].step2,
   collapsed:true,occurrences:verdicts};
 }return verdicts[0];
}
function defUseLocal(c,from) {
 const defId=from.id,sf=from.sf,base=c.row.from.path.base;
 let uses=[],collapsed=false;
 const fields=c.row.to.path.fields&&c.row.to.path.fields.length;
 const endpoint=fields?exactEndpoint(c.row.to):exactId(c.row.to);
 if(endpoint.id)uses=[endpoint.base||endpoint.id];
 else if(endpoint.why==='zero_width_endpoint') {
  collapsed=true;const {sf:usf,ids,why}=index(c.row.to.file),owner=c.row.to.owner;
  if(why)return {step1:'UNDECIDED',why};
  if(!usf||!owner||'ambiguous' in owner)return {step1:'UNDECIDED',why:'collapsed_owner_missing'};
  const onLine=n=>usf.getLineAndCharacterOfPosition(n.getStart(usf)).line+1===c.row.to.line&&
   owner.start_byte<=byte(usf,n.getStart(usf))&&byte(usf,n.end)<=owner.end_byte;
  uses=[...ids.values()].filter(id=>id.text===c.row.to.path.base && isReferencePosition(id) && onLine(id));
  {const matched=uses.filter(id=>accessMatches(id,c.row.to.path));if(matched.length)uses=matched;}
  // PR-B: a synthetic pass's walk counts reads only (B-D12), so its collapsed anchor stands for the
  // reads on that line; declaration names / `=` targets are dropped when a read remains.
  if(isSynthetic(c.row.to))uses=syntheticCandidates(c,uses);
  // `this.x` paths: `this` is a keyword, not an identifier; collapse onto `this.<field>` accesses.
  if(!uses.length&&c.row.to.path.base==='this'){const f0=(c.row.to.path.fields||[])[0];
   const visit=n=>{if(ts.isPropertyAccessExpression(n)&&n.expression.kind===S.ThisKeyword&&n.name.text===f0&&onLine(n))uses.push(n.expression);ts.forEachChild(n,visit);};visit(usf);}
  if(!uses.length)return {step1:'UNDECIDED',why:'collapsed_no_identifier'};
 }else return {step1:endpoint.wrong?'WRONG':'UNDECIDED',why:endpoint.why};
 const scope=ownerScope(c,defId,uses);if(scope)return {...scope,mechanism:'owner_scope'};
 let target=defId,mechanism=fields?'member':'local';
 if((defId.kind===S.ThisKeyword?'this':defId.text)!==base){const t=aliasTarget(defId,base);
  if(!t){
   // A simple-path twin on a line whose LHS is NOT assigned the alias target: re-assigning the
   // alias name breaks the alias; it never writes the target (flow-insensitive twin, E11).
   const p=defId.parent,lhs=(ts.isBinaryExpression(p)&&p.left===defId)||(ts.isVariableDeclaration(p)&&p.name===defId)||
    (ts.isBindingElement(p)&&p.name===defId)||(ts.isShorthandPropertyAssignment(p)&&p.name===defId);
   if(!fields&&lhs)return {step1:'WRONG',why:'alias_flow_insensitive',mechanism:'alias'};
   // A twin can only define a binding that is visible at the Def. A Use whose checker binding
   // is declared in a scope that does not contain the Def bytes binds something else.
   const notVisible=uses.length&&uses.every(u=>{const sym=symbolOf(u),d=sym&&(sym.valueDeclaration||(sym.declarations||[])[0]);
    if(!d)return false;const sc=bindingScope(d),ds=d.getSourceFile();
    return path.resolve(ds.fileName)===path.resolve(root,c.row.from.file)&&!(byte(ds,sc.getStart(ds))<=c.row.from.start_byte&&c.row.from.start_byte<byte(ds,sc.end));});
   if(notVisible)return {step1:'WRONG',why:'alias_binding_not_visible_at_def',mechanism:'alias'};
   return {step1:'UNDECIDED',why:fields?'alias_member':'alias_shape',mechanism:'alias'};}
  target=t;mechanism='alias';}
 const e4=uses.every(u=>fields?roles.memberNotARead(ts,u,c.row.to.path.fields):notARead(u));
 const verdicts=uses.map(u=>{
  if(!collapsed&&!isReferencePosition(u))return {step1:'WRONG',why:'E7_non_reference_use'};
  if(fields&&roles.memberNotARead(ts,u,c.row.to.path.fields))return {step1:'WRONG',why:'member_write_only_use'};
  const v=sameBinding(target,u);return v==='CORRECT'?{step1:'CORRECT'}:v==='WRONG'?{step1:'WRONG',why:'symbol_mismatch'}:{step1:'UNDECIDED',why:'unresolved_symbol'};});
 if(verdicts.some(v=>v.step1==='UNDECIDED')||verdicts.some(v=>v.step1!==verdicts[0].step1))
  return {step1:collapsed?'UNDECIDED':verdicts[0].step1,why:collapsed?'collapsed_mixed_bindings':verdicts[0].why,mechanism,collapsed};
 return {...verdicts[0],mechanism,collapsed,shape:'non_parameter',e4:e4?'USE_NOT_READ':undefined,step2:uses.some(unreachable)?'UNREACHABLE':'n/a'};
}
function useDef(c) {
 const from=exactId(c.row.from),to=exactId(c.row.to);
 for(const x of [from,to])if(!x.id)return {step1:x.wrong?'WRONG':'UNDECIDED',why:x.why};
 let call=from.id.parent;while(call&&!ts.isCallExpression(call))call=call.parent;
 if(!call)return {step1:'UNDECIDED',why:'no_containing_call'};
 const k=call.arguments.findIndex(a=>a.pos<=from.id.pos&&from.id.end<=a.end);
 const sig=checker.getResolvedSignature(call),decl=sig&&sig.declaration;
 if(!decl||!decl.parameters)return {step1:'UNDECIDED',why:'callee_unresolved'};
 const p=decl.parameters[k];if(!p||p.dotDotDotToken||!ts.isIdentifier(p.name))return {step1:'WRONG',why:'position_or_rest'};
 const sf=p.getSourceFile();
 if(byte(sf,0)===null)return {step1:'UNDECIDED',why:'physical_source_text_mismatch'};
 if(!c.row.to.owner||'ambiguous' in c.row.to.owner)return {step1:'UNDECIDED',why:'callee_owner_ambiguous'};
 return path.resolve(sf.fileName)===path.resolve(root,c.row.to.file)&&byte(sf,p.name.getStart(sf))===c.row.to.start_byte&&
  byte(sf,p.name.end)===c.row.to.end_byte&&ownerMatches(c.row.to,decl,sf)?{step1:'CORRECT'}:{step1:'WRONG',why:'callee_parameter_identity_mismatch'};
}
const agg={},details=[];const inc=k=>agg[k]=(agg[k]||0)+1;
for(const c of changed) {
 const kind=`${c.row.from.access}->${c.row.to.access}`;
 const v=kind==='def->use'?defUse(c):kind==='use->def'?useDef(c):{step1:'UNDECIDED',why:'kind'};
 inc(`${c.class}|${kind}|${v.step1}`);if(v.shape)inc(`${c.class}|shape|${v.shape}`);
 if(v.mechanism)inc(`${c.class}|mechanism|${v.mechanism}|${v.step1}`);
 if(v.e4)inc(`${c.class}|e4|${v.e4}`);
 if(isSynthetic(c.row.from)||isSynthetic(c.row.to))inc(`${c.class}|owner|synthetic|${v.step1}`);
 if(v.step2&&v.step2!=='n/a')inc(`${c.class}|step2|${v.step2}`);
 if(v.why)inc(`${c.class}|${kind}|${v.step1}|${v.why}`);details.push({...c,verdict:v});
}
fs.writeFileSync(outFile,JSON.stringify(agg,null,1)+'\n');
if(detailsFile)fs.writeFileSync(detailsFile,details.map(d=>JSON.stringify(d)).join('\n')+(details.length?'\n':''));
console.log(JSON.stringify(agg));
