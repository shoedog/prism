// Controller-only aggregate explanation of the old nonrelative_hop partition.
// TS_JS ROOT CURRENT_EVIDENCE [PRIOR_GAP_EVIDENCE]. Raw output stays in evidence.
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert');
const [tsPath,rootArg,out,prior]=process.argv.slice(2),ts=require(tsPath),root=fs.realpathSync(rootArg);
const read=(d,n)=>JSON.parse(fs.readFileSync(path.join(d,n),'utf8'));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
assert.equal(ts.version,'5.9.3');assert.equal(sha(tsPath),'3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675');
const native=read(out,'F-native-rows.json'),rows=new Map(native.map(n=>[JSON.stringify(n.key),n]));
const host={...ts.sys,getCurrentDirectory:()=>root,writeFile(){throw Error('write refused')},watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout(){}};
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
const service=new ts.server.ProjectService({host,logger,cancellationToken:{isCancellationRequested:()=>false},useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
const rel=p=>path.relative(root,p).split(path.sep).join('/'),key=n=>JSON.stringify(n.key),bump=(o,k)=>o[k]=(o[k]||0)+1;
function unalias(c,s){const seen=new Set();while(s&&(s.flags&ts.SymbolFlags.Alias)&&!seen.has(s)){seen.add(s);s=c.getAliasedSymbol(s);}return s;}
function span(c,s){s=unalias(c,s);const ds=(s?.declarations||[]).filter(d=>(ts.isFunctionDeclaration(d)&&d.body)||ts.isVariableDeclaration(d)||ts.isExportAssignment(d));if(ds.length!==1)return null;
 const d=ds[0],sf=d.getSourceFile();let x=ts.isVariableDeclaration(d)?d.initializer:ts.isExportAssignment(d)?d.expression:d;
 while(x&&(ts.isParenthesizedExpression(x)||ts.isAsExpression(x)||ts.isSatisfiesExpression(x)||ts.isNonNullExpression(x)))x=x.expression;
 if(x&&ts.isCallExpression(x)&&x.arguments.length&&(ts.isArrowFunction(x.arguments[0])||ts.isFunctionExpression(x.arguments[0])))x=x.arguments[0];
 if(!x||!(ts.isFunctionDeclaration(x)||ts.isFunctionExpression(x)||ts.isArrowFunction(x)))return null;
 return {file:rel(sf.fileName),name:d.name?.text||s.name,start_line:sf.getLineAndCharacterOfPosition(x.getStart(sf)).line+1,end_line:sf.getLineAndCharacterOfPosition(x.end-1).line+1};}
const same=(a,b)=>a&&b&&['file','name','start_line','end_line'].every(f=>a[f]===b[f]);
function sites(name){const m=new Map();for(const l of fs.readFileSync(path.join(out,name),'utf8').split('\n').filter(Boolean)){const r=JSON.parse(l);if(r.record_kind==='call_site'){const c=r.caller,s=r.source_span;m.set(JSON.stringify([c.file,c.name,c.start_line,s.file,s.start_byte,s.end_byte,r.callee_text]),r);}}return m;}
const before=sites('p1-sites.jsonl'),after=sites('p2-sites.jsonl');assert.equal(before.size,after.size);
let selected;
if(prior){
 assert.equal(sha(path.join(prior,'p1-sites.jsonl')),sha(path.join(out,'p1-sites.jsonl')),'prior P1 stream drift');
 for(const [p,h] of Object.entries(read(prior,'F-native-inputs.json')))assert.equal(sha(path.join(root,p)),h,'prior native input drift');
 selected=read(prior,'gap-private-rows.json').filter(d=>d.class==='nonrelative_hop');
}else selected=native.filter(n=>n.reason==='JS_EXPORT_HOP'&&n.native_callable&&n.ownership_agrees).flatMap(n=>n.hops.filter(h=>h.relative===false).map(h=>({key:n.key,event:{file:h.from,spec:h.specifier,member:h.member},typescript:{export_symbol:'NOT_PREVIOUSLY_CLASSIFIED',resolution:h.target?'RESOLVED':'UNRESOLVED'}})));
const result={prior_partition_bound:!!prior,rows:selected.length,old_export_symbols:{},absent_explanations:{},current_export_symbols:{},unresolved:{rows:0,stayed_at_p1:0},full_chain_span_agreement:{},limitations:prior?[]:['Prior 697/350/98 partition not bound; counts are hop occurrences, not original rows.']},details=[];
for(const d of selected){const n=rows.get(key(d));assert(n&&n.native_callable&&n.ownership_agrees,'population drift');
 const abs=path.join(root,n.key[0]);service.openClientFile(abs,undefined,undefined,root);
 const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(abs),true),program=project.getLanguageService().getProgram(),c=program.getTypeChecker();assert.equal(rel(project.getProjectName()),n.owner_config);
 const e=d.event,r=ts.resolveModuleName(e.spec,path.join(root,e.file),project.getCompilerOptions(),host).resolvedModule;
 const from=program.getSourceFile(path.join(root,e.file)),target=r&&program.getSourceFile(r.resolvedFileName),mod=target&&c.getSymbolAtLocation(target),exp=mod&&c.getExportsOfModule(mod).find(s=>s.name===e.member);
 const state=!r?'UNRESOLVED_MODULE':!target?'SOURCE_UNAVAILABLE':exp?'PRESENT':'ABSENT';bump(result.old_export_symbols,d.typescript.export_symbol);bump(result.current_export_symbols,state);
 let explanation=null;
 if(d.typescript.export_symbol==='ABSENT'){
  const star=from?.statements.some(s=>ts.isExportDeclaration(s)&&!s.exportClause&&s.moduleSpecifier?.text===e.spec);
  const owner=from&&c.getSymbolAtLocation(from),ownerExp=owner&&c.getExportsOfModule(owner).find(s=>s.name===e.member);
  if(state==='PRESENT')explanation='NOW_PRESENT_PROGRAM_OR_CHECKER_STATE';
  else if(star&&same(span(c,ownerExp),n.terminal))explanation='NONCONTRIBUTING_STAR_BRANCH_FULL_BARREL_BINDS_TERMINAL';
  else if(target?.statements.some(s=>ts.isExportAssignment(s)&&s.isExportEquals))explanation='EXPORT_EQUALS_REQUIRES_IMPORT_SYMBOL_CHECK';
  else if(target?.commonJsModuleIndicator)explanation='COMMONJS_REQUIRES_IMPORT_SYMBOL_CHECK';
  else explanation='UNEXPLAINED_REQUIRES_PRIVATE_FOLLOWUP';
  bump(result.absent_explanations,explanation);
 }
 if(d.typescript.resolution==='UNRESOLVED'){
  result.unresolved.rows++;const preserved=JSON.stringify(before.get(key(d)))===JSON.stringify(after.get(key(d)));result.unresolved.stayed_at_p1+=preserved;
  assert(preserved,'previously unresolved row changed');
 }
 // Actual site/import symbol is the full-chain certificate, not an individual branch.
 const sf=program.getSourceFile(abs);let pos=Buffer.from(sf.text,'utf8').subarray(0,n.key[4]).toString('utf8').length;if(sf.text[pos]==='<')pos++;
 const siteSpan=span(c,c.getSymbolAtLocation(ts.getTokenAtPosition(sf,pos))),agrees=same(siteSpan,n.terminal);bump(result.full_chain_span_agreement,agrees?'AGREES':'UNAVAILABLE_OR_DISAGREES');
 details.push({key:d.key,event:e,old:d.typescript,current:state,explanation,siteSpan,agrees});service.closeClientFile(abs);
}
fs.writeFileSync(path.join(out,'hop-audit-private.json'),JSON.stringify(details,null,2)+'\n');
result.probe_sha256=sha(__filename);fs.writeFileSync(path.join(out,'hop-audit-aggregate.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result));
