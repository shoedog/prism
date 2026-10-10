'use strict';
// Static controller helper. Compile callable fragments only; never invoke package source.
const fs=require('fs'),path=require('path');
const [tsPath,root,detailsPath,censusPath,outPath]=process.argv.slice(2);
const ts=require(tsPath),{classify}=require('./r9-early-errors.cjs');
const {bucket,rowBucket,afterOverride}=require('./r10-buckets.cjs');
const details=fs.readFileSync(detailsPath,'utf8').split('\n').filter(Boolean).map(JSON.parse);
const census=fs.readFileSync(censusPath,'utf8').split('\n').filter(Boolean).map(JSON.parse);
const key=e=>JSON.stringify([e.file,e.owner.name,e.owner.start_line,e.owner.start_byte,e.owner.end_byte]);
const owners=new Map(census.map(c=>[key(c),c]));
const files=[...new Set(details.flatMap(a=>[a.row.from.file,a.row.to.file]))];
const program=ts.createProgram(files.map(f=>path.resolve(root,f)),{allowJs:true,noLib:true,noResolve:true,types:[],checkJs:false,jsx:ts.JsxEmit.Preserve,target:ts.ScriptTarget.ESNext});
const typed=new Set(),other=new Set(),diagnostics={},buckets=new Map();
for(const file of files)if(/\.(js|jsx)$/.test(file)){
 const sf=program.getSourceFile(path.resolve(root,file));if(!sf)throw Error('missing source '+file);
 const ds=[...sf.parseDiagnostics,...program.getSyntacticDiagnostics(sf)];
 const b=bucket(file,ds);buckets.set(file,b);
 if(b==='1')typed.add(file);else if(b==='2')other.add(file);
 if(ds.length)diagnostics[file]=ds.map(d=>({code:d.code,start:d.start,length:d.length,message:ts.flattenDiagnosticMessageText(d.messageText,' ')}));
}
function directive(body){if(!body?.statements)return false;for(const s of body.statements){if(!ts.isExpressionStatement(s)||!ts.isStringLiteral(s.expression))break;const raw=s.expression.getText();if(raw==='"use strict"'||raw==="'use strict'")return true;}return false;}
function contextualStrict(c){const file=path.resolve(root,c.file),source=fs.readFileSync(file,'utf8'),sf=ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true);let found=null;
 const byte=p=>Buffer.byteLength(source.slice(0,p));
 function v(n){if(ts.isFunctionLike(n)&&n.body&&byte(n.end)===c.owner.end_byte&&byte(n.getStart(sf))<=c.owner.start_byte)found=n;ts.forEachChild(n,v);}v(sf);
 for(let n=found;n;n=n.parent)if(ts.isClassLike(n)||directive(n.body)||(ts.isSourceFile(n)&&(ts.isExternalModule(n)||directive(n))))return true;
 return false;
}
const counts={},raw={},overrides=[],inadmissible_proofs=[],stop=[],cache=new Map();
for(const a of details){const side=rowBucket(a.row,buckets);let v=a.verdict.step1,proof=null;const rawkey=[side,a.class,v].join('|');raw[rawkey]=(raw[rawkey]||0)+1;const c=owners.get(key(a.row.from));
 if(a.class==='LOST'&&c&&!c.admitted){proof=cache.get(key(c));if(!proof){const prefix=contextualStrict(c)?'"use strict";':'';const errors=[prefix+'return ('+c.source+');',prefix+'return ({'+c.source+'});'].map(s=>{try{new Function(s);return null;}catch(e){return {name:e.name,message:e.message};}});proof={...classify(errors),errors};cache.set(key(c),proof);}
  v=afterOverride(v,proof);
  if(proof.status==='INADMISSIBLE')inadmissible_proofs.push({original:a,proof});
  if(v!==a.verdict.step1)overrides.push({original:a,effective:v,proof});
 }
 const k=[side,a.class,v].join('|');counts[k]=(counts[k]||0)+1;
 if(side!=='1'&&((a.class==='LOST'&&a.verdict.step1==='CORRECT'&&proof?.status!=='EARLY_ERROR')||(a.class==='LOST'&&v==='CORRECT')||(['ADDED','RE-OWNED'].includes(a.class)&&(a.verdict.step1==='WRONG'||v==='WRONG'))))stop.push(a);
}
fs.writeFileSync(outPath,JSON.stringify({typescript:ts.version,node:process.version,counts,raw,after_override:counts,overrides,inadmissible_proofs,STOP:!!stop.length,STOP_rows:stop,type_annotated_JS:[...typed],other_diagnostic_JS:[...other],diagnostics},null,2)+'\n');
