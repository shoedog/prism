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
 const sf = program.getSourceFile(file), ids = new Map();
 if(sf&&byte(sf,0)===null){const result={sf,ids,why:'physical_source_text_mismatch'};indices.set(file,result);return result;}
 if(sf) { const visit=n=> { if(ts.isIdentifier(n)) ids.set(`${byte(sf,n.getStart(sf))}:${byte(sf,n.end)}`,n);ts.forEachChild(n,visit); }; visit(sf); }
 const result={sf,ids};indices.set(file,result);return result;
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
 return checker.getSymbolAtLocation(id);
}
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
function defUse(c) {
 const from=exactId(c.row.from);if(!from.id)return {step1:from.wrong?'WRONG':'UNDECIDED',why:from.why};
 let uses=[],collapsed=false;
 const endpoint=exactId(c.row.to);
 if(endpoint.id)uses=[endpoint.id];
 else if(endpoint.why==='zero_width_endpoint') {
  collapsed=true;const {sf,ids,why}=index(c.row.to.file),owner=c.row.to.owner;
  if(why)return {step1:'UNDECIDED',why};
  if(!sf||!owner||'ambiguous' in owner)return {step1:'UNDECIDED',why:'collapsed_owner_missing'};
  uses=[...ids.values()].filter(id=>id.text===c.row.from.path.base &&
   sf.getLineAndCharacterOfPosition(id.getStart(sf)).line+1===c.row.to.line &&
   owner.start_byte<=byte(sf,id.getStart(sf))&&byte(sf,id.end)<=owner.end_byte);
  if(!uses.length)return {step1:'UNDECIDED',why:'collapsed_no_identifier'};
 }else return {step1:endpoint.wrong?'WRONG':'UNDECIDED',why:endpoint.why};
 const verdicts=uses.map(use=> {
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
  return {step1:'CORRECT',shape,step2:unreachable(use)?'UNREACHABLE':label==='exact'&&ts.isParameter(decl)?writesBefore(decl,use)?'EXACT_PRIOR_WRITE':'EXACT_OK':'n/a',
   checked_use_span:[byte(use.getSourceFile(),use.getStart()),byte(use.getSourceFile(),use.end)]};
 });
 if(collapsed) {
  if(verdicts.some(v=>v.step1==='UNDECIDED')||verdicts.some(v=>v.step1!==verdicts[0].step1))
   return {step1:'UNDECIDED',why:'collapsed_mixed_bindings',occurrences:verdicts};
  if(verdicts[0].step1==='WRONG')return {...verdicts[0],occurrences:verdicts};
  return {...verdicts[0],step2:verdicts.some(v=>v.step2==='UNREACHABLE')?'UNREACHABLE':verdicts.some(v=>v.step2==='EXACT_PRIOR_WRITE')?'EXACT_PRIOR_WRITE':verdicts[0].step2,
   collapsed:true,occurrences:verdicts};
 }return verdicts[0];
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
 if(v.step2&&v.step2!=='n/a')inc(`${c.class}|step2|${v.step2}`);
 if(v.why)inc(`${c.class}|${kind}|${v.step1}|${v.why}`);details.push({...c,verdict:v});
}
fs.writeFileSync(outFile,JSON.stringify(agg,null,1)+'\n');
if(detailsFile)fs.writeFileSync(detailsFile,details.map(d=>JSON.stringify(d)).join('\n')+(details.length?'\n':''));
console.log(JSON.stringify(agg));
