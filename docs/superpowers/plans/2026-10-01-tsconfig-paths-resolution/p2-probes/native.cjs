// READ: P2 measurement only. Uses each caller's real ProjectService program/checker.
// Usage: node native.cjs TS_JS ROOT ALIAS_SITES IMPORT_FACTS OUT_PREFIX
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const [tsPath, rootArg, aliasesPath, factsPath, out] = process.argv.slice(2);
const ts = require(tsPath), root = fs.realpathSync(rootArg);
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
if (ts.version !== '5.9.3' || sha(tsPath) !== '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675') throw Error('oracle drift');
const aliases = JSON.parse(fs.readFileSync(aliasesPath));
const facts = fs.readFileSync(factsPath, 'utf8').trim().split('\n').map(JSON.parse);
const indexed = new Set(facts.map(f => f.file));
const rel = p => path.relative(root, p).split(path.sep).join('/');
const inside = p => rel(p) !== '..' && !rel(p).startsWith('../') && !path.isAbsolute(rel(p));
const inputs = new Map();
const host = {...ts.sys, getCurrentDirectory: () => root,
  writeFile() {throw Error('native write refused');}, watchFile: () => ({close(){}}), watchDirectory: () => ({close(){}}),
  setTimeout: () => 0, clearTimeout(){}, readFile(p) {
    const text = ts.sys.readFile(p); if (text !== undefined && inside(p)) inputs.set(rel(p), sha(p)); return text;
  }};
const logger = {hasLevel:()=>false, loggingEnabled:()=>false, info(){}, msg(){}, perftrc(){}, startGroup(){}, endGroup(){}, getLogFileName:()=>undefined};
const service = new ts.server.ProjectService({host, logger, cancellationToken:{isCancellationRequested:()=>false},
  useSingleInferredProject:false, useInferredProjectPerProjectRoot:true, typingsInstaller:ts.server.nullTypingsInstaller});
const resolutions = new Map();
const configFeatures = new Map();
function packageOrArrayExtends(file) {
  if(configFeatures.has(file))return configFeatures.get(file);
  const p=path.resolve(root,file),text=host.readFile(p);
  const raw=text===undefined?{}:ts.parseConfigFileTextToJson(p,text).config||{},e=raw.extends;
  const value=Array.isArray(e)||(typeof e==='string'&&!e.startsWith('.'));
  configFeatures.set(file,value);return value;
}
function physical(p) {
  if (!inside(p)) return 'outside';
  let cur = root;
  try {
    const parts = rel(p).split('/');
    for (let i=0;i<parts.length;i++) {
      cur = path.join(cur, parts[i]); const s = fs.lstatSync(cur);
      if (s.isSymbolicLink()) return 'symlink';
      if (i < parts.length-1 && !s.isDirectory()) return 'not_directory';
      if (s.isDirectory()) fs.readdirSync(cur);
    }
    return 'present';
  } catch(e) {return e.code === 'ENOENT' ? 'absent' : 'opaque';}
}
function resolve(project, from, spec) {
  const key = project.getProjectName()+'\0'+from+'\0'+spec;
  if (resolutions.has(key)) return resolutions.get(key);
  const traces = [], lookups = [];
  const opts = {...project.getCompilerOptions(), traceResolution:true};
  const h = {...host, trace: s => traces.push(s), fileExists: p => {const exists=ts.sys.fileExists(p);lookups.push({path:rel(p), exists, physical:physical(p)});return exists;}};
  const r = ts.resolveModuleName(spec, path.join(root,from), opts, h);
  const m = r.resolvedModule;
  const v = {from, specifier:spec, relative:ts.isExternalModuleNameRelative(spec), target:m?rel(m.resolvedFileName):null,
    extension:m?.extension||null, external:m?!inside(m.resolvedFileName):false,
    indexed:m?indexed.has(rel(m.resolvedFileName)):false, mode:opts.moduleResolution,
    failed_lookups:(r.failedLookupLocations||[]).map(rel), affecting:(r.affectingLocations||[]).map(rel), lookups, traces};
  v.js_family = /\.[cm]?jsx?$/.test(v.target||'');
  // Explicit paths substitutions can return JS directly in the priority pass (46431-46435).
  v.js_secondary = opts.moduleResolution === ts.ModuleResolutionKind.Node10 && v.js_family &&
    traces.some(s=>s.includes('target file types: JavaScript'));
  v.physical_doubt = lookups.filter(x => !['present','absent','not_directory'].includes(x.physical));
  for(const p of [...v.failed_lookups,...v.affecting]){
    const fact=physical(path.resolve(root,p));
    if(!['present','absent','not_directory'].includes(fact))v.physical_doubt.push({path:p,physical:fact});
  }
  resolutions.set(key,v); return v;
}
function unalias(checker, s) {const seen=new Set();while(s&&(s.flags&ts.SymbolFlags.Alias)&&!seen.has(s)){seen.add(s);s=checker.getAliasedSymbol(s);}return s;}
// READ: same accepted React wrapper model as the P1 oracle, using this project's checker.
function reactWrapper(checker, call) {
  let name,ref;
  if(ts.isIdentifier(call.expression)){name=call.expression.text;ref=call.expression;}
  else if(ts.isPropertyAccessExpression(call.expression)&&ts.isIdentifier(call.expression.expression)){name=call.expression.name.text;ref=call.expression.expression;}
  else return false;
  const ds=checker.getSymbolAtLocation(ref)?.declarations||[];if(ds.length!==1)return false;
  const d=ds[0];let imp=d;while(imp&&!ts.isImportDeclaration(imp))imp=imp.parent;
  if(!imp||imp.moduleSpecifier.text!=='react'||imp.importClause?.isTypeOnly)return false;
  if(ts.isImportSpecifier(d)){if(d.isTypeOnly)return false;name=(d.propertyName||d.name).text;}
  else if(!ts.isNamespaceImport(d)&&!ts.isImportClause(d))return false;
  return ['memo','forwardRef'].includes(name);
}
function terminal(checker, s) {
  s=unalias(checker,s);
  const vals=(s?.declarations||[]).filter(d=>(ts.isFunctionDeclaration(d)&&d.body)||ts.isVariableDeclaration(d)||ts.isClassDeclaration(d)||ts.isExportAssignment(d));
  if(vals.length!==1)return {class:'non_unique_or_unavailable',count:vals.length};
  const d=vals[0],sf=d.getSourceFile();let init=ts.isVariableDeclaration(d)?d.initializer:ts.isExportAssignment(d)?d.expression:null;
  while(init&&(ts.isParenthesizedExpression(init)||ts.isAsExpression(init)||ts.isSatisfiesExpression(init)||ts.isNonNullExpression(init)))init=init.expression;
  const wrapped=init&&ts.isCallExpression(init)&&reactWrapper(checker,init)&&init.arguments.length&&
    (ts.isArrowFunction(init.arguments[0])||ts.isFunctionExpression(init.arguments[0]))?init.arguments[0]:null;
  let cls=ts.isFunctionDeclaration(d)?'function':init&&(ts.isArrowFunction(init)||ts.isFunctionExpression(init))?'function_variable':wrapped?'wrapped_function':ts.isClassDeclaration(d)?'class':'value_alias_or_noncallable';
  const span=cls==='function_variable'?init:wrapped||d;
  return {class:cls,file:rel(sf.fileName),name:d.name?.text||s.name,start_line:sf.getLineAndCharacterOfPosition(span.getStart(sf)).line+1,end_line:sf.getLineAndCharacterOfPosition(span.end-1).line+1};
}
function exportPaths(project, program, checker, file, member, seen=new Set(), depth=0) {
  const key=file+'\0'+member;
  if(seen.has(key)||depth>12)return [{stop:'cycle_or_depth',from:file,member}];
  seen=new Set(seen);seen.add(key);
  const sf=program.getSourceFile(path.join(root,file));if(!sf)return [{stop:'source_unavailable',from:file,member}];
  const result=[];
  function hop(spec,name,kind) {
    const r=resolve(project,file,spec); result.push({kind,member:name,...r});
    if(r.target&&!r.external)result.push(...exportPaths(project,program,checker,r.target,name,seen,depth+1));
  }
  function local(name) {
    for(const st of sf.statements){if(!ts.isImportDeclaration(st)||!st.importClause)continue;
      const c=st.importClause;
      if(c.name?.text===name)hop(st.moduleSpecifier.text,'default','import_forward');
      if(c.namedBindings&&ts.isNamedImports(c.namedBindings))for(const e of c.namedBindings.elements)if(e.name.text===name)hop(st.moduleSpecifier.text,(e.propertyName||e.name).text,'import_forward');
    }
  }
  for(const st of sf.statements){
    if(ts.isExportAssignment(st)&&member==='default'&&ts.isIdentifier(st.expression))local(st.expression.text);
    if(!ts.isExportDeclaration(st))continue;
    if(st.exportClause&&ts.isNamedExports(st.exportClause)){
      for(const e of st.exportClause.elements)if(e.name.text===member&&!e.isTypeOnly&&!st.isTypeOnly){const n=(e.propertyName||e.name).text;st.moduleSpecifier?hop(st.moduleSpecifier.text,n,'named'):local(n);}
    } else if(!st.exportClause&&st.moduleSpecifier&&!st.isTypeOnly&&member!=='default'){
      const m=resolve(project,file,st.moduleSpecifier.text), child=m.target&&program.getSourceFile(path.join(root,m.target));
      const sym=child&&checker.getSymbolAtLocation(child);
      if(sym&&checker.getExportsOfModule(sym).some(s=>s.name===member))hop(st.moduleSpecifier.text,member,'star');
      else if(!m.target)result.push({kind:'unresolved_star',member,...m});
    }
  }
  return result;
}
const rows=[];
const low=aliases.filter(a=>a.low);
for(const a of low) {
  const file=a.key[0],absolute=path.join(root,file);
  service.openClientFile(absolute,undefined,undefined,root);
  const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(absolute),true);
  const program=project.getLanguageService().getProgram(),checker=program?.getTypeChecker(),sf=program?.getSourceFile(absolute);
  let symbol,proof='unavailable';
  if(sf&&checker){let pos=Buffer.from(sf.text,'utf8').subarray(0,a.key[4]).toString('utf8').length;if(sf.text[pos]==='<')pos++;
    let tok=ts.getTokenAtPosition(sf,pos);if(tok.kind===ts.SyntaxKind.NewKeyword)tok=ts.getTokenAtPosition(sf,tok.end+1);
    // Namespace calls retain the imported namespace as their binding proof.
    symbol=checker.getSymbolAtLocation(tok);const ds=symbol?.declarations||[];
    if(ds.length===1){let imp=ds[0];while(imp&&!ts.isImportDeclaration(imp))imp=imp.parent;
      proof=imp&&imp.moduleSpecifier.text===a.specifier&&ds[0].name?.text===a.local?'import':'shadowed_or_different';}
  }
  const m=resolve(project,file,a.specifier);
  let term=checker?terminal(checker,symbol):{class:'unavailable'};
  // Namespace token's symbol points to a module; ask that project's checker for the requested member.
  if(checker&&term.class==='non_unique_or_unavailable'&&m.target){const targetSf=program.getSourceFile(path.join(root,m.target));const module=targetSf&&checker.getSymbolAtLocation(targetSf);if(module)term=terminal(checker,checker.getExportsOfModule(module).find(s=>s.name===a.member));}
  const configured=project.projectKind===ts.server.ProjectKind.Configured;
  const config=configured?rel(project.getProjectName()):null;
  const hops=m.target&&!m.external&&checker?exportPaths(project,program,checker,m.target,a.member):[];
  const agrees=!!term.file&&term.file===a.terminal?.file&&term.name===a.terminal?.name&&term.start_line===a.terminal?.start_line&&term.end_line===a.terminal?.end_line;
  const opts=project.getCompilerOptions();
  const parked_traits={ordered_substitutions:Object.values(opts.paths||{}).some(v=>v.length>1),
    tied_pattern:a.refusal_reason==='TIED_PATTERN',empty_capture:a.refusal_reason==='EMPTY_CAPTURE',
    explicit_substitution_extension:a.refusal_reason==='EXPLICIT_EXTENSION_OUTSIDE_P1',
    node_next_or_bundler:[3,99,100].includes(opts.moduleResolution),
    package_or_array_extends:[a.config,...(a.chain||[])].filter(Boolean).some(packageOrArrayExtends)};
  rows.push({key:a.key,reason:a.refusal_reason,old_recoverable:a.recoverable,old_mechanisms:a.mechanisms,
    site_import_proof:proof,owner_config:config,root_config:a.root_config,ownership_agrees:config===a.root_config,
    refusal_detail_codes:a.refusal_detail_codes||[],parked_traits,
    compiler_mode:project.getCompilerOptions().moduleResolution,allow_js:!!project.getCompilerOptions().allowJs,
    terminal:term,terminal_agrees:agrees,entry:m,hops,
    native_callable:proof==='import'&&inside(path.join(root,term.file||'..'))&&
      (['function','function_variable'].includes(term.class)||(term.class==='wrapped_function'&&a.recoverable)),
    js_hop_count:hops.filter(h=>h.js_family).length,
    js_secondary_hop_count:hops.filter(h=>h.js_secondary).length,
    js_to_ts_hop_count:hops.filter(h=>h.specifier?.endsWith('.js')&&h.target?.endsWith('.ts')&&!h.target?.endsWith('.d.ts')).length,
    nonrelative_hop_count:hops.filter(h=>h.relative===false).length,
    hop_stops:hops.filter(h=>h.stop||!h.target||h.external||!h.indexed||h.physical_doubt?.length).length});
  service.closeClientFile(absolute);
}
const hist=(rs,f)=>{const c={};for(const r of rs){const k=f(r);c[k]=(c[k]||0)+1;}return c;};
const recoverable=aliases.filter(a=>a.recoverable&&a.low);
const summary={claim:'MEASURED',oracle_version:ts.version,oracle_sha256:sha(tsPath),probe_sha256:sha(__filename),total_sites:0,
  associated_low_sites:low.length,remaining_p1_callable:recoverable.length,
  refusal_histogram:hist(recoverable,r=>r.refusal_reason),all_low_refusal_histogram:hist(low,r=>r.refusal_reason),
  all_low_terminal_histogram:hist(rows,r=>r.terminal.class),
  native_callable:rows.filter(r=>r.native_callable).length,
  native_callable_ownership_agrees:rows.filter(r=>r.native_callable&&r.ownership_agrees).length,
  old_recoverable_native_terminal_disagreements:rows.filter(r=>r.old_recoverable&&!r.terminal_agrees).length,
  old_recoverable_ownership_disagreements:rows.filter(r=>r.old_recoverable&&!r.ownership_agrees).length,
  js_hops:{rows:rows.filter(r=>r.old_recoverable&&r.js_hop_count).length,
    secondary_pass_rows:rows.filter(r=>r.old_recoverable&&r.js_secondary_hop_count).length,
    native_agrees:rows.filter(r=>r.old_recoverable&&r.js_hop_count&&r.native_callable&&r.terminal_agrees&&r.ownership_agrees).length,
    physical_or_closure_doubt:rows.filter(r=>r.old_recoverable&&r.js_hop_count&&r.hop_stops).length,
    allow_js_off:rows.filter(r=>r.old_recoverable&&r.js_hop_count&&!r.allow_js).length,
    relative_only_resolution_ceiling:rows.filter(r=>r.old_recoverable&&r.js_hop_count&&r.native_callable&&r.terminal_agrees&&r.ownership_agrees&&r.allow_js&&!r.hop_stops&&!r.nonrelative_hop_count).length,
    spellings:hist(rows.filter(r=>r.old_recoverable&&r.js_hop_count),r=>r.hops.filter(h=>h.js_family).map(h=>/\.[cm]?jsx?$/.test(h.specifier)?'explicit_js':'extensionless').sort().join('+'))},
  nonrelative_hops:{rows:rows.filter(r=>r.old_recoverable&&r.nonrelative_hop_count).length,
    native_agrees:rows.filter(r=>r.old_recoverable&&r.nonrelative_hop_count&&r.native_callable&&r.terminal_agrees&&r.ownership_agrees).length},
  js_to_ts_hops:{rows:rows.filter(r=>r.js_to_ts_hop_count).length,
    old_recoverable:rows.filter(r=>r.old_recoverable&&r.js_to_ts_hop_count).length},
  mechanisms:{},buckets:{},parked_trait_rows:{},input_files:inputs.size};
for(const trait of ['ordered_substitutions','tied_pattern','empty_capture','explicit_substitution_extension','node_next_or_bundler','package_or_array_extends']){
  const rs=rows.filter(r=>r.parked_traits[trait]);
  summary.parked_trait_rows[trait]={rows:rs.length,native_callable:rs.filter(r=>r.native_callable).length,
    ownership_agrees_callable:rs.filter(r=>r.native_callable&&r.ownership_agrees).length};
}
for(const reason of new Set(low.map(r=>r.refusal_reason))){
  const rs=rows.filter(r=>r.reason===reason);
  summary.buckets[reason]={rows:rs.length,old_recoverable:rs.filter(r=>r.old_recoverable).length,
    native_callable:rs.filter(r=>r.native_callable).length,
    ownership_agrees_callable:rs.filter(r=>r.native_callable&&r.ownership_agrees).length,
    terminal_agrees:rs.filter(r=>r.terminal_agrees).length,
    detail_histogram:hist(rs.flatMap(r=>r.refusal_detail_codes),r=>r)};
}
for(const mechanism of ['baseUrl_bare','js_to_ts','project_references_present','package_exports_or_workspace','extends_chain','per_package_config']){
  const rs=rows.filter(r=>r.old_mechanisms.includes(mechanism));summary.mechanisms[mechanism]={rows:rs.length,native_callable:rs.filter(r=>r.native_callable).length,ownership_agrees_callable:rs.filter(r=>r.native_callable&&r.ownership_agrees).length};}
fs.writeFileSync(out+'-native-rows.json',JSON.stringify(rows,null,2));
fs.writeFileSync(out+'-native-inputs.json',JSON.stringify(Object.fromEntries(inputs),null,2));
fs.writeFileSync(out+'-native-summary.json',JSON.stringify(summary,null,2));
console.log(JSON.stringify(summary));
