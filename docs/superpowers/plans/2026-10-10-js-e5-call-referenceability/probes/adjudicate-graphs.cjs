'use strict';
// PR-B physical binding/role checks plus the primary E5 callee binding proof.
// Signature identity never upgrades an undecided callee value binding.
const fs=require('fs'),path=require('path'),cp=require('child_process');
const [tsPath,root,changesFile,callsFile,out,...flags]=process.argv.slice(2),ts=require(tsPath);
if(ts.version!=='5.9.3')throw Error('wrong oracle');
const read=p=>fs.existsSync(p)?fs.readFileSync(p,'utf8').split('\n').filter(Boolean).map(JSON.parse):[];
const changes=read(changesFile),calls=read(callsFile),indices=new Map(),roles=require('./oracle-read-role.cjs');
function index(file){
 const absolute=path.resolve(root,file),scope=path.resolve(root);
 if(absolute.split(path.sep).includes('frontend-portal')&&!(flags.includes('--controller-private')&&(absolute===scope||absolute.startsWith(scope+path.sep))))throw Error('forbidden source');
 if(indices.has(file))return indices.get(file);
 const raw=fs.readFileSync(path.resolve(root,file)),text=raw.toString('utf8');
 const sf=ts.createSourceFile(file,text,ts.ScriptTarget.ESNext,true),functions=[],nodes=new Map();
 const byte=p=>Buffer.byteLength(text.slice(0,p));
 function start(n){let s=n.getStart(sf);for(const m of n.modifiers||[])if(m.kind===ts.SyntaxKind.ExportKeyword||m.kind===ts.SyntaxKind.DefaultKeyword)s=m.end;const scanner=ts.createScanner(ts.ScriptTarget.ESNext,true,sf.languageVariant,text);scanner.setTextPos(s);scanner.scan();return scanner.getTokenPos();}
 function name(n){if(n.name)return n.name.text??n.name.getText(sf);const p=n.parent;if(ts.isVariableDeclaration(p)||ts.isPropertyAssignment(p)||ts.isPropertyDeclaration(p))return p.name.text??p.name.getText(sf);if(ts.isBinaryExpression(p)&&p.right===n)return p.left.name?.text??p.left.text;if(ts.isCallExpression(p)&&ts.isVariableDeclaration(p.parent))return p.parent.name.text??p.parent.name.getText(sf);const s=start(n),line=sf.getLineAndCharacterOfPosition(s).line+1,column=byte(s)-byte(text.lastIndexOf('\n',s-1)+1)+1;return `<cb@${line}:${column}>`;}
 function visit(n){const key=byte(n.getStart(sf))+':'+byte(n.end);if(!nodes.has(key))nodes.set(key,n);if(ts.isFunctionLike(n)&&n.body)functions.push({node:n,name:name(n),start_line:sf.getLineAndCharacterOfPosition(start(n)).line+1,start_byte:byte(start(n)),end_byte:byte(n.end)});ts.forEachChild(n,visit);}visit(sf);
 const i={sf,functions,nodes,byte,parse_errors:sf.parseDiagnostics.length};indices.set(file,i);return i;
}
function endpoint(wrapped){
 const v=wrapped.Variable;if(!v)return null;
 const i=index(v.file),matches=i.functions.filter(f=>f.name===v.function&&f.start_line===v.function_start_line);
 const owner=matches.length===1?{name:v.function,start_line:v.function_start_line,start_byte:matches[0].start_byte,end_byte:matches[0].end_byte}:{name:v.function,start_line:v.function_start_line,ambiguous:matches.length};
 const parameter=matches.length===1&&matches[0].node.parameters.some(p=>i.byte(p.name.getStart(i.sf))===v.start_byte&&i.byte(p.name.end)===v.end_byte);
 return {...v,access:v.access.toLowerCase(),owner,parameter};
}
function label(edge){const l=edge.DataFlow;if(l==='Exact')return {confidence:'exact',doubt:null,kill_line:null};const d=l.NameOnly;if(typeof d==='object')return {confidence:'nameonly',doubt:'killed',kill_line:d.Killed.kill_line};return {confidence:'nameonly',doubt:{SameLine:'sameline',CfgIncomplete:'cfg_incomplete',AliasUnstable:'alias_unstable',CallNameOnly:'call_nameonly'}[d],kill_line:null};}
const normalized=[];for(const c of changes){if(c.row.kind!=='DataFlow')continue;const from=endpoint(c.row.from),to=endpoint(c.row.to);if(!from||!to)throw Error('unexpected DataFlow endpoint');const lab=label(c.row.edge);normalized.push({class:c.change==='removed'?'LOST':'ADDED',row:{from,to},[c.change==='removed'?'base_label':'head_label']:lab});}
fs.mkdirSync(out,{recursive:true});const input=path.join(out,'flow.normalized.jsonl'),detail=path.join(out,'flow.prb-details.jsonl');
fs.writeFileSync(input,normalized.map(JSON.stringify).join('\n')+(normalized.length?'\n':''));
cp.execFileSync(process.execPath,[path.join(__dirname,'adjudicate-prb.cjs'),tsPath,root,input,path.join(out,'flow.prb-summary.json'),'--details',detail],{stdio:['ignore','ignore','pipe']});
const prb=read(detail);if(prb.length!==normalized.length)throw Error('lost PR-B row');let flowIndex=0;
function owner(w){const v=w.Variable||w.ReturnValue||w.Function;return v&&{file:v.file,name:v.function??v.name,start_line:v.function_start_line??v.start_line};}
function same(a,b){return a&&b&&a.file===b.file&&a.name===b.name&&a.start_line===b.start_line;}
function causal(c){const r=c.row;let caller,target,span=null;
 if(r.kind==='DataFlow'&&r.from.Variable.access==='Use'&&r.to.Variable.access==='Def'){caller=owner(r.from);target=owner(r.to);span=r.from.Variable;}
 else if(r.kind==='Call'){caller=owner(r.from);target=owner(r.to);}
 else if(r.kind==='Return'||r.kind==='ReturnFlow'){caller=owner(r.to);target=owner(r.from);}
 else return [];
 return calls.filter(x=>x.change===c.change&&same(caller,x.site.caller)&&same(target,x.target.function_id)&&(!span||x.site.source_span.start_byte<=span.start_byte&&span.end_byte<=x.site.source_span.end_byte));
}
function unanimous(xs){if(!xs.length)return {verdict:'UNDECIDED',why:'no_unique_call_provenance'};const vs=new Set(xs.map(x=>x.verdict));return vs.size===1?{verdict:xs[0].verdict,why:'primary_callee_binding_proof',calls:xs.map(x=>x.site.source_span)}:{verdict:'UNDECIDED',why:'mixed_call_provenance',counts:Object.fromEntries([...vs].map(v=>[v,xs.filter(x=>x.verdict===v).length]))};}
const result=[],counts={};
for(const c of changes){let verdict,raw=null,cause=causal(c);
 const files=[c.row.from,c.row.to].map(w=>(w.Variable||w.ReturnValue||w.Function)?.file).filter(Boolean);
 const syntaxBad=files.filter(f=>/\.(js|jsx|mjs|cjs|ts|tsx)$/.test(f)).some(f=>index(f).parse_errors);
 if(c.row.kind==='DataFlow'){
  raw=prb[flowIndex++].verdict;verdict={verdict:raw.step1,why:raw.why??'PR-B_physical_binding_and_owner_proof',role:raw.e4??null,time:raw.step2??null};
  if(raw.e4||raw.step2==='EXACT_PRIOR_WRITE')verdict={...verdict,verdict:'WRONG',why:raw.e4??raw.step2};
  if(c.row.from.Variable.access==='Use'&&c.row.to.Variable.access==='Def'){
   const cv=unanimous(cause);
   if(cv.verdict!=='CORRECT')verdict=cv;
  }
 }else if(['Call','Return','ReturnFlow'].includes(c.row.kind))verdict=unanimous(cause);
 else if(c.row.kind==='ReturnInput'){
  const v=c.row.from.Variable,ret=c.row.to.ReturnValue,i=index(v.file),n=i.nodes.get(v.start_byte+':'+v.end_byte);
  verdict=v.file===ret.file&&v.function===ret.function&&v.function_start_line===ret.function_start_line&&ret.start_byte<=v.start_byte&&v.end_byte<=ret.end_byte&&n&&!roles.valueNotARead(ts,n)?{verdict:'CORRECT',why:'physical_value_read_inside_return_expression'}:{verdict:'UNDECIDED',why:'return_input_identity_or_role_unproved'};
 }else if(c.row.kind==='ReturnContains')verdict={verdict:'CORRECT',why:'Exact_consumer_generated_return_endpoint_containment'};
 else verdict={verdict:'INADMISSIBLE',why:'non_call_digest_difference_STOP'};
 if(syntaxBad)verdict={verdict:'INADMISSIBLE',why:'parse_diagnostics_in_endpoint_file',underlying:verdict};
 const row={...c,raw_prb:raw,...verdict};result.push(row);const key=[c.change,c.row.kind,verdict.verdict].join('|');counts[key]=(counts[key]||0)+1;
}
fs.writeFileSync(path.join(out,'graph.adjudicated.jsonl'),result.map(JSON.stringify).join('\n')+(result.length?'\n':''));
fs.writeFileSync(path.join(out,'graph.adjudication-summary.json'),JSON.stringify(counts,null,2)+'\n');console.log(JSON.stringify(counts));
