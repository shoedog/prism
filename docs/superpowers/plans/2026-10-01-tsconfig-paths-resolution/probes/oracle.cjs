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
const configCache=new Map(), rootChosen=new Map(), barriers=new Map(), resolutions=new Map();
const inputHashes=new Map();
function config(p){
 if(configCache.has(p))return configCache.get(p);
 const read=[];const diagnostics=[];
 const host={...ts.sys,readFile:q=>{if(fs.existsSync(q)) {inputHashes.set(q,hash(q));read.push(q);}return ts.sys.readFile(q);},onUnRecoverableConfigFileDiagnostic:d=>diagnostics.push(d)};
 const c=ts.getParsedCommandLineOfConfigFile(p,{},host);
 if(c)diagnostics.push(...c.errors.filter(e=>![18002,18003].includes(e.code)));
 const v={p,c,members:new Set((c?.fileNames||[]).map(q=>path.resolve(q))),read:[...new Set(read)],diagnostics:diagnostics.map(d=>({code:d.code,message:ts.flattenDiagnosticMessageText(d.messageText,' ')}))};
 // Independent reachability check using the native type-directive resolver.
 v.p1TypeKeys=[...new Set(v.read.filter(q=>q.endsWith('.json')).flatMap(q=>{
  const opts=ts.parseConfigFileTextToJson(q,ts.sys.readFile(q)||'').config?.compilerOptions||{};
  const safe=q=>within(q)&&(!fs.existsSync(q)||!fs.lstatSync(q).isSymbolicLink());
  const roots=opts.typeRoots?.map(r=>path.resolve(path.dirname(q),r));
  const effective={...(c?.options||{}),...(roots?{typeRoots:roots}:{})};
  const bad=[];
  if(roots?.some(r=>!safe(r)))bad.push('typeRoots');
  if(opts.types?.some(name=>{
   const direct=name.startsWith('.')||path.isAbsolute(name)?path.resolve(path.dirname(q),name):null;
   const resolved=direct||ts.resolveTypeReferenceDirective(name,q,effective,ts.sys).resolvedTypeReferenceDirective?.resolvedFileName;
   return !!resolved&&!safe(resolved);
  }))bad.push('types');
  return bad;
 }))];
 const visited=new Set();
 const uncovered=q=>{
  if(!within(q))return true;
  if(!fs.existsSync(q))return false;
  if(fs.lstatSync(q).isSymbolicLink())return true;
  if(visited.has(q))return false;visited.add(q);
  const refs=ts.preProcessFile(ts.sys.readFile(q)||'',true);
  return refs.referencedFiles.some(r=>uncovered(path.resolve(path.dirname(q),r.fileName)))||refs.typeReferenceDirectives.some(r=>{
   const name=r.fileName;
   const resolved=name.startsWith('.')||path.isAbsolute(name)?path.resolve(path.dirname(q),name):ts.resolveTypeReferenceDirective(name,q,c?.options||{},ts.sys).resolvedTypeReferenceDirective?.resolvedFileName;
   return !!resolved&&uncovered(resolved);
  });
 };
 v.p1Triple=[...v.members].some(uncovered);
 configCache.set(p,v);return v;
}
function rootSelect(file){
 const absolute=path.join(root,file);if(rootChosen.has(absolute))return rootChosen.get(absolute);
 let dir=path.dirname(absolute), selected=null;
 while(within(dir)){
  const p=path.join(dir,'tsconfig.json');
  const js=path.join(dir,'jsconfig.json');
  // Root-file reference model only; it does not supply oracle ownership.
  if(!fs.existsSync(p)&&fs.existsSync(js)){config(js);barriers.set(absolute,'JSCONFIG_BARRIER');break;}
  if(fs.existsSync(p)) {
   const c=config(p), raw=c.c?.raw;
   if(raw && (Object.hasOwn(raw,'references') || (Array.isArray(raw.files)&&raw.files.length===0))){barriers.set(absolute,'DELEGATED_CONFIG_BARRIER');break;}
   if(c.diagnostics.length){barriers.set(absolute,'INVALID_CONFIG_BARRIER');break;}
   if(c.members.has(absolute)){selected=c;break;}
   if(fs.existsSync(js)){barriers.set(absolute,'EXCLUDING_TSCONFIG_WITH_JSCONFIG');}
   if(c.c?.options.disableSolutionSearching){barriers.set(absolute,'DISABLE_SOLUTION_SEARCHING');}
  }
  if(dir===root)break;dir=path.dirname(dir);
 }
 rootChosen.set(absolute,selected);return selected;
}
// Offline real tsserver: no watchers, timers, package installs or writes.
const serviceHost={...ts.sys,getCurrentDirectory:()=>root,writeFile:()=>{throw Error('oracle write refused');},
 watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout:()=>{},
 readFile:p=>{if(within(p)&&fs.existsSync(p)&&p.endsWith('.json'))inputHashes.set(p,hash(p));return ts.sys.readFile(p);}};
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
const service=new ts.server.ProjectService({host:serviceHost,logger,cancellationToken:{isCancellationRequested:()=>false},
 useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
const owners=new Map();
function select(file){
 const absolute=path.resolve(root,file);if(owners.has(absolute))return owners.get(absolute).config;
 service.openClientFile(absolute,undefined,undefined,root);
 const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(absolute),true);
 const p=project?.projectKind===ts.server.ProjectKind.Configured?project.getProjectName():null;
 const actual=p&&within(p)?config(p):null,reference=rootSelect(file);
 const disagreement=(actual?.p||null)!==(reference?.p||null);
 owners.set(absolute,{config:actual,project:p?rel(p):null,kind:project?.projectKind,root_config:reference?rel(reference.p):null,disagreement});
 service.closeClientFile(absolute);
 return actual;
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
 const owner=owners.get(path.resolve(root,file));
 const v={config:cf?rel(cf.p):null,tsserver_project:owner.project,tsserver_project_kind:owner.kind,root_config:owner.root_config,tsserver_disagreement:owner.disagreement,ownership_barrier:barriers.get(path.join(root,file))||null,diagnostics:cf?.diagnostics||[],chain:cf?.read.filter(p=>p!==cf.p&&p.endsWith('.json')).map(rel)||[],references:cf?.c?.projectReferences?.length||0,pattern,via,removal_same:removalSame||false,target:mod?rel(mod.resolvedFileName):null,external:mod?!within(mod.resolvedFileName):false,extension:mod?.extension||null,options:cf?{baseUrl:opts.baseUrl?rel(opts.baseUrl):null,pathsBasePath:opts.pathsBasePath?rel(opts.pathsBasePath):null,moduleResolution:opts.moduleResolution,outDir:opts.outDir,rootDirs:opts.rootDirs,moduleSuffixes:opts.moduleSuffixes,noResolve:opts.noResolve,paths:opts.paths||{}}:null};
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
const exportsCache=new Map(), barrelDiagnostics=new Map();
// TS first-wins symbol lookup does not certify an ambiguous star binding.
function ambiguousBarrel(file,seen=new Set()){
 if(seen.has(file))return false;seen.add(file);
 if(barrelDiagnostics.has(file))return barrelDiagnostics.get(file);
 const sf=program.getSourceFile(path.join(root,file));if(!sf)return false;
 const exports=sf.statements.filter(st=>ts.isExportDeclaration(st)&&st.moduleSpecifier);
 if(!exports.length)return false;
 if(program.getSemanticDiagnostics(sf).some(d=>d.code===2308)){barrelDiagnostics.set(file,true);return true;}
 for(const st of exports){const m=resolve(file,st.moduleSpecifier.text);if(m.target&&!m.external&&ambiguousBarrel(m.target,seen)){barrelDiagnostics.set(file,true);return true;}}
 return false;
}
function nonrelativeExportHop(file,member,seen=new Set()){
 const key=file+'\0'+member;if(seen.has(key))return false;seen.add(key);
 const sf=program.getSourceFile(path.join(root,file));if(!sf)return false;
 function hop(spec,name){
  if(!spec.startsWith('.'))return true;
  const m=resolve(file,spec);return !!(m.target&&!m.external&&nonrelativeExportHop(m.target,name,seen));
 }
 function local(name){
  for(const st of sf.statements){
   if(!ts.isImportDeclaration(st)||!st.importClause)continue;
   const c=st.importClause;
   if(c.name?.text===name)return hop(st.moduleSpecifier.text,'default');
   if(c.namedBindings&&ts.isNamedImports(c.namedBindings)){
    const binding=c.namedBindings.elements.find(e=>e.name.text===name);
    if(binding)return hop(st.moduleSpecifier.text,(binding.propertyName||binding.name).text);
   }
  }
  return false;
 }
 for(const st of sf.statements){
  if(ts.isExportAssignment(st)&&member==='default'&&ts.isIdentifier(st.expression)&&local(st.expression.text))return true;
  if(!ts.isExportDeclaration(st))continue;
  if(st.exportClause&&ts.isNamedExports(st.exportClause)){
   const binding=st.exportClause.elements.find(e=>e.name.text===member);
   if(binding){const name=(binding.propertyName||binding.name).text;if(st.moduleSpecifier?hop(st.moduleSpecifier.text,name):local(name))return true;}
  }else if(!st.exportClause&&st.moduleSpecifier&&member!=='default'){
   const m=resolve(file,st.moduleSpecifier.text);
   if(m.target&&!m.external&&exportsFor(m.target)?.has(member)&&hop(st.moduleSpecifier.text,member))return true;
  }
 }
 return false;
}
function importForwardHop(file,member,seen=new Set()){
 const key=file+'\0'+member;if(seen.has(key))return false;seen.add(key);
 const sf=program.getSourceFile(path.join(root,file));if(!sf)return false;
 function imported(name){return sf.statements.some(st=>ts.isImportDeclaration(st)&&st.importClause&&
  (st.importClause.name?.text===name||(st.importClause.namedBindings&&ts.isNamedImports(st.importClause.namedBindings)&&st.importClause.namedBindings.elements.some(e=>e.name.text===name))));}
 for(const st of sf.statements){
  if(ts.isExportAssignment(st)&&member==='default'&&ts.isIdentifier(st.expression)&&imported(st.expression.text))return true;
  if(!ts.isExportDeclaration(st))continue;
  if(st.exportClause&&ts.isNamedExports(st.exportClause)){
   const e=st.exportClause.elements.find(e=>e.name.text===member);if(!e)continue;
   const name=(e.propertyName||e.name).text;if(!st.moduleSpecifier&&imported(name))return true;
   if(st.moduleSpecifier){const m=resolve(file,st.moduleSpecifier.text);if(m.target&&!m.external&&importForwardHop(m.target,name,seen))return true;}
  }else if(!st.exportClause&&st.moduleSpecifier&&member!=='default'){
   const m=resolve(file,st.moduleSpecifier.text);if(m.target&&!m.external&&exportsFor(m.target)?.has(member)&&importForwardHop(m.target,member,seen))return true;
  }
 }
 return false;
}
function memberWritten(sf,symbol){
 let written=false;
 function visit(n){
  if(ts.isBinaryExpression(n)&&n.operatorToken.kind>=ts.SyntaxKind.FirstAssignment&&n.operatorToken.kind<=ts.SyntaxKind.LastAssignment){
   let left=n.left;
   if(ts.isPropertyAccessExpression(left)||ts.isElementAccessExpression(left)){
    while(ts.isPropertyAccessExpression(left)||ts.isElementAccessExpression(left))left=left.expression;
    if(ts.isIdentifier(left)&&unalias(checker.getSymbolAtLocation(left))===symbol)written=true;
   }
  }
  ts.forEachChild(n,visit);
 }
 visit(sf);return written;
}
const duplicateConfigCache=new Map();
function duplicateConfig(p){
 if(duplicateConfigCache.has(p))return duplicateConfigCache.get(p);
 const sf=ts.parseJsonText(p,fs.readFileSync(p,'utf8'));let duplicate=false;
 function visit(n){
  if(ts.isObjectLiteralExpression(n)){
   const names=new Set();for(const prop of n.properties){const name=prop.name?.text;if(name!==undefined){if(names.has(name))duplicate=true;names.add(name);}}
  }
  ts.forEachChild(n,visit);
 }
 visit(sf);duplicateConfigCache.set(p,duplicate);return duplicate;
}
function opaqueExportHop(file,seen=new Set()){
 if(seen.has(file))return false;seen.add(file);
 const sf=program.getSourceFile(path.join(root,file));if(!sf)return false;
 for(const st of sf.statements){
  if(ts.isExportAssignment(st)&&!ts.isIdentifier(st.expression))return true;
  if(ts.isExpressionStatement(st)&&ts.isBinaryExpression(st.expression)){
   const x=st.expression,left=x.left.getText(sf).replace(/\s/g,'');
   if((left==='module.exports'||left.startsWith('module.exports.')||left.startsWith('exports.'))&&!ts.isIdentifier(x.right))return true;
  }
  if(ts.isExportDeclaration(st)&&st.moduleSpecifier){const m=resolve(file,st.moduleSpecifier.text);if(m.target&&!m.external&&opaqueExportHop(m.target,seen))return true;}
 }
 return false;
}
// Identify remaining finite refusal shapes without invoking prism's matcher.
function membershipCut(file){
 const absolute=path.resolve(root,file);let d=path.dirname(absolute);
 while(within(d)){
  if(fs.readdirSync(d).some(n=>n.toLowerCase()==='tsconfig.json'&&n!=='tsconfig.json'))return 'CASE_VARIANT_CONFIG_NAME';
  const p=path.join(d,'tsconfig.json');
  if(fs.existsSync(p)){
   const c=config(p),raw=c.c?.raw||{};
   if(raw.files?.some(f=>path.resolve(d,f)===absolute))return null;
   const inc=raw.include||(raw.files?[]:['**/*']);
   if(file.split('/').some(p=>['bower_components','jspm_packages','node_modules'].includes(p)))return 'PACKAGE_FOLDER_MEMBERSHIP';
   if(file.endsWith('.d.ts')&&fs.existsSync(absolute.slice(0,-5)+'.ts'))return 'WILDCARD_DECLARATION_PRIORITY';
   if(inc.some(p=>p.split('/').some(s=>s.startsWith('?'))))return 'QUESTION_PREFIX_INCLUDE';
   if(inc.some(p=>/[?*]/.test(p))&&/[^\x00-\x7f]/.test(file+inc.join('')))return 'NONASCII_WILDCARD_MEMBERSHIP';
   for(const spec of [...inc,...(raw.exclude||[])]){
    const prefix=path.resolve(d,spec.split(/[?*]/)[0]);
    if(absolute.toLowerCase().startsWith(prefix.toLowerCase())&&!absolute.startsWith(prefix))return 'CASE_VARIANT_MEMBERSHIP_SPEC';
   }
   return null;
  }
  if(d===root)break;d=path.dirname(d);
 }
 return null;
}
function starCut(file,seen=new Set()){
 if(seen.has(file))return null;seen.add(file);
 const sf=program.getSourceFile(path.join(root,file));if(!sf)return 'UNPROVEN_STAR_BRANCH';
 if(sf.parseDiagnostics.length)return 'EXPORT_SYNTAX_DIAGNOSTIC';
 for(const st of sf.statements){
  if(!ts.isExportDeclaration(st)||st.exportClause||!st.moduleSpecifier)continue;
  if(!st.moduleSpecifier.text.startsWith('.'))return 'UNPROVEN_STAR_BRANCH';
  const m=resolve(file,st.moduleSpecifier.text);
  if(!m.target||m.external)return 'UNPROVEN_STAR_BRANCH';
  const cut=starCut(m.target,seen);if(cut)return cut;
 }
 return null;
}
// Ordered independent explanations of P1 cuts, not production telemetry.
// The explicit fallback remains an open reason; it is never called a scope proof.
function refusalReason(a,m,t){
 const o=m.options||{}, paths=o.paths||{};
 if(m.tsserver_disagreement)return 'TSSERVER_OWNERSHIP_DISAGREEMENT';
 if(m.ownership_barrier)return m.ownership_barrier;
 if(!m.config)return 'NO_OWNING_CONFIG';
 if(m.diagnostics.length)return 'CONFIG_DIAGNOSTIC';
 if(o.moduleResolution!==ts.ModuleResolutionKind.Node10)return 'MODULE_RESOLUTION_OUTSIDE_P1';
 if(Object.keys(paths).some(k=>k.split('*').length>2 || !Array.isArray(paths[k]) || paths[k].length!==1 || paths[k].some(v=>v.split('*').length>2)))return 'PATHS_SHAPE_OUTSIDE_P1';
 if(o.rootDirs!==undefined||o.moduleSuffixes!==undefined||o.noResolve!==undefined)return 'OPTIONS_OUTSIDE_P1';
 const cf=config(path.join(root,m.config));
 if(cf.p1TypeKeys.length)return 'CONFIG_TYPES_SCOPE_BARRIER';
 if(cf.p1Triple)return 'TRIPLE_REFERENCE_SCOPE_BARRIER';
 if(cf.read.filter(p=>/^tsconfig(?:\..+)?\.json$/.test(path.basename(p))).some(duplicateConfig))return 'DUPLICATE_CONFIG_KEY';
 if(o.outDir && cf.c?.raw?.exclude===undefined)return 'OUTDIR_BARRIER';
 const membership=membershipCut(a.s.caller.file);if(membership)return membership;
 if(m.via!=='paths')return 'BARE_BASEURL_OUTSIDE_P1';
 const key=m.pattern,cap=key?.includes('*')?a.b.module_path.slice(key.indexOf('*'),a.b.module_path.length-(key.length-key.indexOf('*')-1)):'';
 if(key?.includes('*')&&!cap)return 'EMPTY_CAPTURE';
 const prefix=key?.indexOf('*');
 if(prefix>=0&&Object.keys(paths).filter(k=>k.indexOf('*')===prefix&&a.b.module_path.startsWith(k.slice(0,prefix))&&a.b.module_path.endsWith(k.slice(prefix+1))).length>1)return 'TIED_PATTERN';
 const target=paths[key]?.[0];
 if(target?.includes('*')&&!key?.includes('*'))return 'UNMATCHED_TARGET_STAR';
 if(target){
  const subst=path.resolve(root,o.baseUrl||o.pathsBasePath||path.dirname(m.config),target.replace('*',cap));
  if(/\.(jsx?|[cm][jt]s|json)$/.test(subst))return 'EXPLICIT_EXTENSION_OUTSIDE_P1';
  if(fs.existsSync(path.join(subst,'package.json')))return 'PACKAGE_BOUNDARY';
  if(!/\.tsx?$/.test(subst)){
   const probes=['.ts','.tsx','.d.ts','.js','.jsx'].flatMap(e=>[subst+e,path.join(subst,'index'+e)]).filter(p=>fs.existsSync(p));
   if(probes.length!==1)return 'CANDIDATE_COMPETITION_OR_ABSENCE';
   if(probes[0].endsWith('.d.ts'))return 'DECLARATION_BLOCKER';
  }
 }
 if(t.file&&t.file!==m.target&&/\.(?:js|jsx|mjs|cjs)$/.test(t.file))return 'JS_EXPORT_HOP';
 if(m.target&&nonrelativeExportHop(m.target,a.member))return 'NONRELATIVE_EXPORT_HOP';
 if(m.target&&opaqueExportHop(m.target))return 'OPAQUE_EXPORT_BRANCH';
 const star=m.target&&starCut(m.target);if(star)return star;
 if(['function_variable','wrapped_function','class'].includes(t.class) && m.target && importForwardHop(m.target,a.member))return 'IMPORT_FORWARD_NOT_FORWARDABLE';
 if(t.class==='value_alias_or_noncallable')return 'TERMINAL_VALUE_ALIAS_OR_NONCALLABLE';
 if(t.class==='ambiguous_or_nonvalue')return 'TERMINAL_AMBIGUOUS_OR_NONVALUE';
 if(t.class==='missing_export')return 'MISSING_EXPORT';
 if(a.s.local_binding?.Unproven!=='import'||a.b.kind!=='MemberImport')return 'BINDING_OR_SITE_GUARD';
 return 'UNCLASSIFIED_P1_PROOF';
}
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
 if(ambiguousBarrel(file))return {class:'ambiguous_star_diagnostic'};
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
 return {class:cls,member_written:memberWritten(sf,symbol),wrapper:wrapper||null,file:rel(sf.fileName),name:ts.isFunctionDeclaration(d)&&d.name?d.name.text:ts.isVariableDeclaration(d)&&ts.isIdentifier(d.name)?d.name.text:symbol.name,start_line:sf.getLineAndCharacterOfPosition(spanNode.getStart(sf)).line+1,end_line:sf.getLineAndCharacterOfPosition(spanNode.end-1).line+1};
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
 record.refusal_detail_codes=t.member_written?['TERMINAL_MEMBER_WRITTEN']:[];
 record.refusal_reason=refusalReason(a,m,t);
 record.recoverable=m.options?.moduleResolution===ts.ModuleResolutionKind.Node10&&!m.ownership_barrier&&!m.tsserver_disagreement&&low&&record.site_import_proof==='import'&&['paths','baseUrl'].includes(m.via)&&['function','function_variable','wrapped_function'].includes(t.class)&&m.diagnostics.length===0&&(t.class!=='wrapped_function'||a.s.jsx_element);
 allAliasRows.push(record);if(low&&record.site_import_proof==='import')candidates.push(record);
}
const counts={tsserver_disagreement_sites:allAliasRows.filter(r=>r.tsserver_disagreement).length,tsserver_disagreement_files:new Set(allAliasRows.filter(r=>r.tsserver_disagreement).map(r=>r.key[0])).size,total_sites:rows.length,nonrelative_import_bindings:imports,module_resolved_bindings:modules,associated_nonrelative_sites:allAliasRows.length,associated_low_sites:allAliasRows.filter(c=>c.low).length,site_binding_counts:{},low_sites:candidates.length,paths_baseurl_resolve_low:candidates.filter(c=>['paths','baseUrl'].includes(c.via)&&c.target&&!c.external).length,callable_recoverable:candidates.filter(c=>c.recoverable).length,mechanisms:{},terminal_classes:{}};
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
  if(a&&a.terminal&&!a.tsserver_disagreement){const t=a.terminal, targets=c.proto[1];
   if(a.recoverable&&c.proto[0]===null&&targets.length===1&&targets[0][4]==='exact'&&targets[0][5]==='import_member'){
    const x=targets[0];const equal=x[0]===t.file&&x[1]===t.name&&x[2]===t.start_line&&x[3]===t.end_line;
    cls=equal?'CORRECT_STATIC_BINDING':'WRONG_OR_SPAN_MISMATCH';
   } else if(!a.ownership_barrier&&a.site_import_proof==='import'&&a.target&&a.diagnostics.length===0&&t.class==='wrapped_function'&&!metadata.get(JSON.stringify(a.key)).jsx_element&&targets.length===0&&c.proto[0]==='WrappedExportNonJsx')cls='CORRECT_STATIC_REFUSAL';
   else if(targets.length===0)cls='REMOVAL_NEEDS_AUDIT';
  }
  return {...c,class:cls,oracle:a||null};
 });
 const classes={};for(const c of classified)classes[c.class]=(classes[c.class]||0)+1;
 fs.writeFileSync(outPrefix+'-classified.json',JSON.stringify(classified,null,2));report.changed_rows=classified.length;report.classes=classes;report.changed_tsserver_disagreements=classified.filter(c=>c.oracle?.tsserver_disagreement).length;
 fs.writeFileSync(outPrefix+'-P0.json',JSON.stringify(report,null,2));
}
console.log(JSON.stringify(counts));if(report.classes)console.log(JSON.stringify(report.classes));
