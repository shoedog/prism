// Independent TypeScript oracle; no prism resolver or export logic is reused.
// Usage: node oracle.cjs TS_JS ROOT BASE_SITES IMPORT_FACTS OUT_PREFIX [CHANGES]
const fs=require('fs'), path=require('path'), crypto=require('crypto');
const [tsPath,rootArg,dumpPath,factsPath,outPrefix,changesPath]=process.argv.slice(2);
const ts=require(tsPath), root=path.resolve(rootArg);
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const facts=fs.readFileSync(factsPath,'utf8').trim().split('\n').map(JSON.parse);
const byFile=new Map(facts.map(x=>[x.file,x]));
const rows=fs.readFileSync(dumpPath,'utf8').trim().split('\n').map(JSON.parse).filter(x=>x.record_kind==='call_site');
const rel=p=>path.relative(root,p).split(path.sep).join('/');
const within=p=>{const r=rel(p);return r!== '..' && !r.startsWith('../') && !path.isAbsolute(r);};
const configCache=new Map(), chosen=new Map(), resolutions=new Map();
const inputHashes=new Map();
function config(p){
 if(configCache.has(p))return configCache.get(p);
 const read=[];const diagnostics=[];
 const host={...ts.sys,readFile:q=>{if(fs.existsSync(q)) {inputHashes.set(q,hash(q));read.push(q);}return ts.sys.readFile(q);},onUnRecoverableConfigFileDiagnostic:d=>diagnostics.push(d)};
 const c=ts.getParsedCommandLineOfConfigFile(p,{},host);
 if(c)diagnostics.push(...c.errors.filter(e=>![18002,18003].includes(e.code)));
 const v={p,c,members:new Set((c?.fileNames||[]).map(q=>path.resolve(q))),read:[...new Set(read)],diagnostics:diagnostics.map(d=>({code:d.code,message:ts.flattenDiagnosticMessageText(d.messageText,' ')}))};
 configCache.set(p,v);return v;
}
function select(file){
 const absolute=path.join(root,file);if(chosen.has(absolute))return chosen.get(absolute);
 let dir=path.dirname(absolute), selected=null;
 while(within(dir)){
  const p=path.join(dir,'tsconfig.json');
  if(fs.existsSync(p)) {const c=config(p);if(c.members.has(absolute)){selected=c;break;}}
  if(dir===root)break;dir=path.dirname(dir);
 }
 chosen.set(absolute,selected);return selected;
}
function resolve(file,spec){
 const k=file+'\0'+spec;if(resolutions.has(k))return resolutions.get(k);
 const cf=select(file);const opts=cf?.c?.options||{moduleResolution:ts.ModuleResolutionKind.Node10,allowJs:true};
 const host={...ts.sys,readFile:q=>{if(fs.existsSync(q))inputHashes.set(q,hash(q));return ts.sys.readFile(q);}};
 const mod=ts.resolveModuleName(spec,path.join(root,file),opts,host).resolvedModule;
 let pattern=null,via=null;
 if(!spec.startsWith('.') && !path.isAbsolute(spec) && cf){
  const patterns=Object.keys(opts.paths||{});
  if(patterns.includes(spec))pattern=spec;
  else pattern=patterns.filter(p=>{const n=p.indexOf('*');return n>=0 && spec.startsWith(p.slice(0,n))&&spec.endsWith(p.slice(n+1))&&spec.length>=p.length-1;}).sort((a,b)=>b.indexOf('*')-a.indexOf('*'))[0]||null;
  if(pattern){
   const stripped={...opts,paths:undefined};
   const without=ts.resolveModuleName(spec,path.join(root,file),stripped,host).resolvedModule;
   // Resolution equality does not prove causality; record the match and a removal control.
   via='paths';
   var removalSame=mod?.resolvedFileName===without?.resolvedFileName;
  }else if(opts.baseUrl && mod && within(mod.resolvedFileName) && !mod.isExternalLibraryImport)via='baseUrl';
  else if(mod)via='package_or_other';
 }
 const v={config:cf?rel(cf.p):null,diagnostics:cf?.diagnostics||[],chain:cf?.read.filter(p=>p!==cf.p&&p.endsWith('.json')).map(rel)||[],references:cf?.c?.projectReferences?.length||0,pattern,via,removal_same:removalSame||false,target:mod?rel(mod.resolvedFileName):null,external:mod?!within(mod.resolvedFileName):false,extension:mod?.extension||null,options:cf?{baseUrl:opts.baseUrl?rel(opts.baseUrl):null,pathsBasePath:opts.pathsBasePath?rel(opts.pathsBasePath):null,moduleResolution:opts.moduleResolution,paths:opts.paths||{}}:null};
 resolutions.set(k,v);return v;
}
// Independent checker, with TypeScript's resolver receiving each file's selected config.
const options={target:ts.ScriptTarget.ESNext,module:ts.ModuleKind.ESNext,jsx:ts.JsxEmit.Preserve,allowJs:true,skipLibCheck:true,noLib:true};
const host=ts.createCompilerHost(options);
host.resolveModuleNames=(names,containing)=>names.map(spec=>{const cfg=select(rel(containing));return ts.resolveModuleName(spec,containing,cfg?.c?.options||options,ts.sys).resolvedModule;});
const program=ts.createProgram(facts.map(f=>path.join(root,f.file)),options,host);
const checker=program.getTypeChecker();
function unalias(s){const seen=new Set();while(s&&(s.flags&ts.SymbolFlags.Alias)&&!seen.has(s)){seen.add(s);s=checker.getAliasedSymbol(s);}return s;}
function exportsFor(file){
 const sf=program.getSourceFile(path.join(root,file));if(!sf)return null;
 const symbol=checker.getSymbolAtLocation(sf);if(!symbol)return null;
 return new Map(checker.getExportsOfModule(symbol).map(s=>[s.name,unalias(s)]));
}
const exportsCache=new Map();
function reactWrapper(call){
 let name,ref;
 if(ts.isIdentifier(call.expression)){name=call.expression.text;ref=call.expression;}
 else if(ts.isPropertyAccessExpression(call.expression)&&ts.isIdentifier(call.expression.expression)){name=call.expression.name.text;ref=call.expression.expression;}
 else return null;
 const symbol=checker.getSymbolAtLocation(ref);const ds=symbol?.declarations||[];
 if(ds.length!==1)return null;
 const d=ds[0];let imp=d;while(imp&&!ts.isImportDeclaration(imp))imp=imp.parent;
 if(!imp||imp.moduleSpecifier.text!=='react'||imp.importClause?.isTypeOnly)return null;
 if(ts.isImportSpecifier(d)){if(d.isTypeOnly)return null;name=(d.propertyName||d.name).text;}
 else if(!ts.isNamespaceImport(d)&&!ts.isImportClause(d))return null;
 return ['memo','forwardRef'].includes(name)?name:null;
}
function terminal(file,member){
 if(!exportsCache.has(file))exportsCache.set(file,exportsFor(file));
 const symbol=exportsCache.get(file)?.get(member);if(!symbol)return {class:'missing_export'};
 const ds=symbol.declarations||[];
 const vals=ds.filter(d=>(ts.isFunctionDeclaration(d)&&d.body)||ts.isVariableDeclaration(d)||ts.isClassDeclaration(d)||ts.isExportAssignment(d));
 if(vals.length!==1)return {class:'ambiguous_or_nonvalue',count:vals.length};
 const d=vals[0], sf=d.getSourceFile();let init=ts.isVariableDeclaration(d)?d.initializer:ts.isExportAssignment(d)?d.expression:null;
 while(init&&(ts.isParenthesizedExpression(init)||ts.isAsExpression(init)||ts.isSatisfiesExpression(init)||ts.isNonNullExpression(init)))init=init.expression;
 const wrapper=init&&ts.isCallExpression(init)?reactWrapper(init):null;
 const wrapped=wrapper&&init.arguments.length&&((ts.isArrowFunction(init.arguments[0])||ts.isFunctionExpression(init.arguments[0])))?init.arguments[0]:null;
 let cls=ts.isFunctionDeclaration(d)?'function':init&&(ts.isArrowFunction(init)||ts.isFunctionExpression(init))?'function_variable':wrapped?'wrapped_function':ts.isClassDeclaration(d)?'class':'value_alias_or_noncallable';
 const spanNode=cls==='function_variable'?init:wrapped||d;
 return {class:cls,wrapper:wrapper||null,file:rel(sf.fileName),name:ts.isFunctionDeclaration(d)&&d.name?d.name.text:ts.isVariableDeclaration(d)&&ts.isIdentifier(d.name)?d.name.text:symbol.name,start_line:sf.getLineAndCharacterOfPosition(spanNode.getStart(sf)).line+1,end_line:sf.getLineAndCharacterOfPosition(spanNode.end-1).line+1};
}
function rowKey(r){const c=r.caller,s=r.source_span;return JSON.stringify([c.file,c.name,c.start_line,s.file,s.start_byte,s.end_byte,r.callee_text]);}
const metadata=new Map();for(const f of facts)for(const s of f.sites)metadata.set(JSON.stringify([s.caller.file,s.caller.name,s.caller.start_line,f.file,s.start_byte,s.end_byte,s.callee_text]),s);
function siteImportProof(r,a){
 const sf=program.getSourceFile(path.join(root,r.caller.file));if(!sf)return 'unavailable';
 let pos=Buffer.from(sf.text,'utf8').subarray(0,r.source_span.start_byte).toString('utf8').length;
 if(sf.text[pos]==='<')pos++;
 let token=ts.getTokenAtPosition(sf,pos);
 if(token.kind===ts.SyntaxKind.NewKeyword)token=ts.getTokenAtPosition(sf,token.end+1);
 const symbol=checker.getSymbolAtLocation(token),ds=symbol?.declarations||[];
 if(ds.length!==1)return 'unavailable';
 const d=ds[0];let imp=d;while(imp&&!ts.isImportDeclaration(imp))imp=imp.parent;
 if(!imp)return 'shadowed';
 return imp.moduleSpecifier.text===a.b.module_path&&d.name?.text===a.b.local?'import':'different_import';
}
function associate(r){
 const f=byFile.get(r.caller.file),s=metadata.get(rowKey(r));if(!f||!s||s.origin!=='Source')return null;
 const local=s.qualifier||r.callee_text;
 const bindings=(f.bindings||[]).filter(b=>b.eligible&&b.local===local);
 if(bindings.length!==1)return null;const b=bindings[0];
 if(b.module_path.startsWith('.')||path.isAbsolute(b.module_path))return null;
 if(s.qualifier && b.kind!=='ModuleImport')return null;
 if(!s.qualifier && b.kind!=='MemberImport')return null;
 return {b,s,member:s.qualifier?r.callee_text:(b.member||b.local)};
}
const candidates=[], allAliasRows=[];let imports=0, modules=0;const importDetails=[];
for(const f of facts){for(const b of f.bindings||[]){if(b.module_path.startsWith('.')||path.isAbsolute(b.module_path))continue;imports++;const m=resolve(f.file,b.module_path);if(m.target&&within(path.join(root,m.target)))modules++;importDetails.push({file:f.file,binding:b,...m});}}
for(const r of rows){
 const a=associate(r);if(!a)continue;const m=resolve(r.caller.file,a.b.module_path);
 const low=!(r.resolved_targets||[]).some(t=>t.confidence==='exact');
 const t=m.target&&!m.external?terminal(m.target,a.member):{class:'module_unresolved_or_external'};
 const record={key:JSON.parse(rowKey(r)),specifier:a.b.module_path,local:a.b.local,member:a.member,site_import_proof:siteImportProof(r,a),base_drop:r.drop,base_targets:r.resolved_targets,low,...m,terminal:t,mechanisms:[]};
 if(m.via==='paths')record.mechanisms.push(m.pattern.includes('*')?'paths_wildcard':'paths_exact');
 if(m.via==='baseUrl')record.mechanisms.push('baseUrl_bare');
 if(m.chain.length)record.mechanisms.push('extends_chain');
 if(m.config && m.config.includes('/'))record.mechanisms.push('per_package_config');
 if(m.references)record.mechanisms.push('project_references_present');
 if(m.target&&a.b.module_path.endsWith('.js')&&m.target.endsWith('.ts'))record.mechanisms.push('js_to_ts');
 if(m.target&&/\/index\.[cm]?[jt]sx?$/.test(m.target))record.mechanisms.push('index_file');
 if(m.via==='package_or_other')record.mechanisms.push('package_exports_or_workspace');
 record.recoverable=low&&record.site_import_proof==='import'&&['paths','baseUrl'].includes(m.via)&&['function','function_variable','wrapped_function'].includes(t.class)&&m.diagnostics.length===0&&(t.class!=='wrapped_function'||a.s.jsx_element);
 allAliasRows.push(record);if(low&&record.site_import_proof==='import')candidates.push(record);
}
const counts={total_sites:rows.length,nonrelative_import_bindings:imports,module_resolved_bindings:modules,associated_nonrelative_sites:allAliasRows.length,associated_low_sites:allAliasRows.filter(c=>c.low).length,site_binding_counts:{},low_sites:candidates.length,paths_baseurl_resolve_low:candidates.filter(c=>['paths','baseUrl'].includes(c.via)&&c.target&&!c.external).length,callable_recoverable:candidates.filter(c=>c.recoverable).length,mechanisms:{},terminal_classes:{}};
for(const c of allAliasRows.filter(c=>c.low))counts.site_binding_counts[c.site_import_proof]=(counts.site_binding_counts[c.site_import_proof]||0)+1;
for(const c of candidates){counts.terminal_classes[c.terminal.class]=(counts.terminal_classes[c.terminal.class]||0)+1;for(const m of c.mechanisms){counts.mechanisms[m]??={low:0,module_resolved:0,callable:0};counts.mechanisms[m].low++;if(c.target&&!c.external)counts.mechanisms[m].module_resolved++;if(c.recoverable)counts.mechanisms[m].callable++;}}
const report={oracle:{version:ts.version,path:tsPath,sha256:hash(tsPath)},root,counts,configurations:[...configCache.values()].map(c=>({path:rel(c.p),diagnostics:c.diagnostics,members:c.members.size,reads:c.read.map(rel)})),input_hashes:Object.fromEntries([...inputHashes.entries()].map(([p,h])=>[rel(p),h]))};
fs.writeFileSync(outPrefix+'-P0.json',JSON.stringify(report,null,2));
fs.writeFileSync(outPrefix+'-candidates.json',JSON.stringify(candidates,null,2));
fs.writeFileSync(outPrefix+'-imports.json',JSON.stringify(importDetails,null,2));
fs.writeFileSync(outPrefix+'-alias-sites.json',JSON.stringify(allAliasRows,null,2));
if(changesPath){
 const changes=JSON.parse(fs.readFileSync(changesPath,'utf8'));const lookup=new Map(allAliasRows.map(r=>[JSON.stringify(r.key),r]));
 const classified=changes.map(c=>{
  const a=lookup.get(JSON.stringify(c.key));let cls='UNPROVEN';
  if(a&&a.terminal){const t=a.terminal, targets=c.proto[1];
   if(a.recoverable&&c.proto[0]===null&&targets.length===1&&targets[0][4]==='exact'&&targets[0][5]==='import_member'){
    const x=targets[0];const equal=x[0]===t.file&&x[1]===t.name&&x[2]===t.start_line&&x[3]===t.end_line;
    cls=equal?'CORRECT_STATIC_BINDING':'WRONG_OR_SPAN_MISMATCH';
   } else if(a.site_import_proof==='import'&&a.target&&a.diagnostics.length===0&&t.class==='wrapped_function'&&!metadata.get(JSON.stringify(a.key)).jsx_element&&targets.length===0&&c.proto[0]==='WrappedExportNonJsx')cls='CORRECT_STATIC_REFUSAL';
   else if(targets.length===0)cls='REMOVAL_NEEDS_AUDIT';
  }
  return {...c,class:cls,oracle:a||null};
 });
 const classes={};for(const c of classified)classes[c.class]=(classes[c.class]||0)+1;
 fs.writeFileSync(outPrefix+'-classified.json',JSON.stringify(classified,null,2));report.changed_rows=classified.length;report.classes=classes;
 fs.writeFileSync(outPrefix+'-P0.json',JSON.stringify(report,null,2));
}
console.log(JSON.stringify(counts));if(report.classes)console.log(JSON.stringify(report.classes));
