// Every changed full row is checked by the writer's actual pinned TS project.
// node compare.cjs TS_JS ROOT BASE_SITES HEAD_SITES HEAD_FACTS OUT_DIRECTORY
'use strict';
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const [tsPath,rootArg,basePath,headPath,factsPath,out]=process.argv.slice(2);
const ts=require(tsPath),root=fs.realpathSync(rootArg),sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
if(ts.version!=='5.9.3'||sha(tsPath)!=='3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675')throw Error('oracle drift');
const read=p=>fs.readFileSync(p,'utf8').trim().split('\n').filter(Boolean).map(JSON.parse);
const key=r=>JSON.stringify([r.caller.file,r.caller.name,r.caller.start_line,r.source_span.start_byte,r.source_span.end_byte,r.callee_text]);
function rows(p){const a=read(p).filter(r=>r.record_kind==='call_site'),m=new Map(a.map(r=>[key(r),r]));if(!a.length||m.size!==a.length)throw Error('empty/duplicate rows');return m;}
const base=rows(basePath),head=rows(headPath),facts=new Map(read(factsPath).map(f=>[f.file,f]));
if(base.size!==head.size||[...base.keys()].some(k=>!head.has(k)))throw Error('changed site population');
const rel=p=>path.relative(root,p).split(path.sep).join('/'),inputs=new Map();
const host={...ts.sys,getCurrentDirectory:()=>root,writeFile(){throw Error('oracle write refused');},watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout(){},readFile(p){const t=ts.sys.readFile(p);if(t!==undefined)inputs.set(p,sha(p));return t;}};
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
const service=new ts.server.ProjectService({host,logger,cancellationToken:{isCancellationRequested:()=>false},useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
function unalias(c,s){const seen=new Set();while(s&&(s.flags&ts.SymbolFlags.Alias)&&!seen.has(s)){seen.add(s);s=c.getAliasedSymbol(s);}return s;}
function terminal(c,s){s=unalias(c,s);const ds=s?.declarations||[];if(ds.length!==1)return null;const d=ds[0],sf=d.getSourceFile();let span=d,init=ts.isVariableDeclaration(d)?d.initializer:null;
 while(init&&(ts.isParenthesizedExpression(init)||ts.isAsExpression(init)||ts.isSatisfiesExpression(init)||ts.isNonNullExpression(init)))init=init.expression;
 if(ts.isVariableDeclaration(d)){
  if(init&&(ts.isArrowFunction(init)||ts.isFunctionExpression(init)))span=init;
  else if(init&&ts.isCallExpression(init)&&init.arguments[0]&&(ts.isArrowFunction(init.arguments[0])||ts.isFunctionExpression(init.arguments[0]))){
   const ref=ts.isPropertyAccessExpression(init.expression)?init.expression.expression:init.expression;
   const bd=c.getSymbolAtLocation(ref)?.declarations||[];if(bd.length!==1)return null;let imp=bd[0];while(imp&&!ts.isImportDeclaration(imp))imp=imp.parent;
   const name=ts.isPropertyAccessExpression(init.expression)?init.expression.name.text:ts.isImportSpecifier(bd[0])?(bd[0].propertyName||bd[0].name).text:null;
   if(!imp||imp.moduleSpecifier.text!=='react'||!['memo','forwardRef'].includes(name))return null;span=init.arguments[0];
  }else return null;
 }else if(!ts.isFunctionDeclaration(d)||!d.body)return null;
 return {file:rel(sf.fileName),name:d.name?.text||s.name,start_line:sf.getLineAndCharacterOfPosition(span.getStart(sf)).line+1,end_line:sf.getLineAndCharacterOfPosition(span.end-1).line+1};
}
const changed=[],summary={sites:base.size,base_bound_sites:[...base.values()].filter(r=>r.resolved_targets.length).length,changed:0,correct:0,unproven:0,lost_base_edges:0,keys_added:0,keys_removed:0};
for(const [k,b]of base){const h=head.get(k);const meta=r=>Object.fromEntries(Object.entries(r).filter(([k])=>!['drop','resolved_targets','exact_target'].includes(k)));if(JSON.stringify(meta(b))!==JSON.stringify(meta(h)))throw Error('changed row metadata');
 summary.lost_base_edges+=b.resolved_targets.filter(t=>!h.resolved_targets.some(u=>JSON.stringify(t)===JSON.stringify(u))).length;
 if(JSON.stringify(b)===JSON.stringify(h))continue;summary.changed++;
 const abs=path.join(root,b.caller.file);service.openClientFile(abs,undefined,undefined,root);const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(abs),true),program=project.getLanguageService().getProgram(),sf=program?.getSourceFile(abs),checker=program?.getTypeChecker();let found=[];
 function visit(n){if(ts.isCallExpression(n)||ts.isJsxSelfClosingElement(n)||ts.isJsxOpeningElement(n)){const expr=n.expression||n.tagName;const start=Buffer.byteLength(sf.text.slice(0,n.getStart(sf)));if(start===b.source_span.start_byte)found.push(expr);}ts.forEachChild(n,visit);}if(sf)visit(sf);
 const native=found.length===1?terminal(checker,checker.getSymbolAtLocation(found[0])):null;
 const bindings=(facts.get(b.caller.file)?.bindings||[]).filter(x=>x.local===b.callee_text||x.local===b.qualifier),binding=bindings.length===1?bindings[0]:null;
 const proof=binding&&facts.get(b.caller.file)?.modules?.[binding.module_path],owner=project.projectKind===ts.server.ProjectKind.Configured?rel(project.getProjectName()):null;
 const module=binding&&ts.resolveModuleName(binding.module_path,abs,project.getCompilerOptions(),host,undefined,undefined,ts.getModeForUsageLocation(sf,findImport(sf,binding.module_path),project.getCompilerOptions())).resolvedModule;
 let nativeModule=null;try{if(module)nativeModule=rel(fs.realpathSync(module.resolvedFileName));}catch{}
 const target=h.resolved_targets.length===1?h.resolved_targets[0]:null,correct=!!(target&&target.confidence==='exact'&&native&&proof&&proof[0]===nativeModule&&proof[1]===owner&&['file','name','start_line','end_line'].every(x=>target.function_id[x]===native[x]));
 summary[correct?'correct':'unproven']++;changed.push({key:JSON.parse(k),class:correct?'CORRECT_STATIC_BINDING':'UNPROVEN_OR_WRONG',native,owner,nativeModule,proof,before:b,after:h});service.closeClientFile(abs);
}
function findImport(sf,spec){return sf.statements.find(n=>(ts.isImportDeclaration(n)||ts.isExportDeclaration(n))&&n.moduleSpecifier?.text===spec)?.moduleSpecifier;}
for(const [p,h]of inputs)if(sha(p)!==h)throw Error('oracle input drift');
fs.mkdirSync(out,{recursive:true});fs.writeFileSync(path.join(out,'comparison.json'),JSON.stringify({summary,changed,inputs:Object.fromEntries(inputs),hashes:{base:sha(basePath),head:sha(headPath),facts:sha(factsPath),oracle:sha(tsPath)}},null,2)+'\n');console.log(JSON.stringify(summary));
if(summary.unproven||summary.lost_base_edges)throw Error('uncertified change/lost edge');
