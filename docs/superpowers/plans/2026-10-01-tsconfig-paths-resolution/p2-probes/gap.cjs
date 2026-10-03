// Aggregate-only diagnosis. Raw keys, paths, export facts and traces stay in OUT.
// Usage: node gap.cjs TS_JS ROOT EVIDENCE_DIR
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert');
const {bindingGate,projectionGate}=require('./gap-projection.cjs');
const [tsPath,rootArg,out]=process.argv.slice(2),ts=require(path.resolve(tsPath)),root=fs.realpathSync(rootArg);
const read=n=>JSON.parse(fs.readFileSync(path.join(out,n),'utf8'));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
assert.equal(ts.version,'5.9.3');
assert.equal(sha(tsPath),'3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675');
const native=read('F-native-rows.json'),aliases=read('F-alias-sites.json'),side=read('gap-sidecar.json');
const factRows=fs.readFileSync(path.join(out,'import-facts.jsonl'),'utf8').trim().split('\n').map(JSON.parse);
const factHashes=new Map(factRows.map(r=>[r.file,r.hash])),siteFacts=new Map();
for(const f of factRows)assert.equal(side.file_hashes[f.file],f.hash,'fact/source drift');
const verifyNativeInputs=()=>{
  for(const [p,h] of Object.entries(read('F-native-inputs.json')))assert.equal(sha(path.join(root,p)),h,'native input drift');
};
verifyNativeInputs();
for(const f of factRows)for(const s of f.sites)siteFacts.set(JSON.stringify([
  s.caller.file,s.caller.name,s.caller.start_line,f.file,s.start_byte,s.end_byte,s.callee_text]),s);
const key=r=>JSON.stringify(r.key),aliasMap=new Map(aliases.map(r=>[key(r),r]));
const entryMap=new Map(side.entries.map(r=>[key(r),r]));
const hopMap=new Map(side.hops.map(r=>[JSON.stringify([r.from,r.specifier,r.allow_js]),r.explanation]));
const loadSites=n=>{
  const m=new Map();
  for(const l of fs.readFileSync(path.join(out,n),'utf8').trim().split('\n')) {
    const r=JSON.parse(l);if(r.record_kind!=='call_site')continue;
    const c=r.caller,s=r.source_span,k=JSON.stringify([c.file,c.name,c.start_line,s.file,s.start_byte,s.end_byte,r.callee_text]);
    assert(!m.has(k),'duplicate site');m.set(k,r);
  }
  assert(m.size,'zero-site probe');return m;
};
const before=loadSites('p1-sites.jsonl'),after=loadSites('p2-sites.jsonl');
const comparison=read('p2-comparison.json');
assert.equal(sha(path.join(out,'p1-sites.jsonl')),comparison.p1_dump_sha256,'P1 evidence drift');
assert.equal(sha(path.join(out,'p2-sites.jsonl')),comparison.p2_dump_sha256,'P2 evidence drift');
assert.equal(sha(path.join(out,'F-native-rows.json')),comparison.native_rows_sha256,'native row drift');
assert.deepEqual([...before.keys()].sort(),[...after.keys()].sort());
const population=native.filter(r=>r.reason==='JS_EXPORT_HOP'&&r.native_callable&&r.ownership_agrees);
const correct=(r,n)=>!r.drop&&r.resolved_targets?.length===1&&r.resolved_targets[0].confidence==='exact'&&
  r.resolved_targets[0].kind==='import_member'&&['file','name','start_line','end_line'].every(f=>r.resolved_targets[0].function_id[f]===n.terminal[f]);
const moduleGate=(from,spec,allow)=>{
  const d=hopMap.get(JSON.stringify([from,spec,allow]));assert(d,'missing module diagnostic');return d;
};
const event=(gate,file,member,spec)=>({gate,file,member,spec});
// Mechanically matches resolve_one_inner: named overrides star; forwardability
// follows child resolution; any BlockedClaim poisons a star, distinct from a miss.
function lookup(file,name,allow,depth=0,seen=new Set()) {
  const k=JSON.stringify([file,name]);
  if(seen.has(k))return {state:'none',events:[event('EXPORT_CYCLE',file,name)]};
  seen=new Set(seen);seen.add(k);
  const f=side.exports[file];if(!f)return {state:'none',events:[event('EXPORT_FACTS_FILE_MISSING',file,name)]};
  if(f.conflicted.includes(name))return {state:'blocked',events:[event('EXPORT_NAME_CONFLICT',file,name)]};
  const t=f.named[name];
  if(t) {
    const [kind,v]=Object.entries(t)[0];
    if(kind==='UnprovenLocal')return {state:'blocked',events:[event('EXPORT_UNPROVEN_LOCAL',file,name)]};
    if(['VerifiedLocal','SpannedLocal'].includes(kind))return {state:'resolved',hit:{file,local_name:v.local,span:[v.start_line,v.end_line],wrapped:kind==='SpannedLocal',is_class:false},events:[]};
    if(['Local','Class'].includes(kind))return {state:'resolved',hit:{file,local_name:v,span:null,wrapped:false,is_class:kind==='Class'},events:[]};
    assert(['ReExport','ImportForward'].includes(kind),'unknown export fact');
    if(depth+1>side.max_depth)return {state:'none',events:[event('EXPORT_DEPTH_LIMIT',file,name,v.module_path)]};
    const m=moduleGate(file,v.module_path,allow);
    if(!m.target)return {state:'none',events:[event(m.gate,file,v.imported,v.module_path)]};
    const child=lookup(m.target,v.imported,allow,depth+1,seen);
    if(child.state==='resolved'&&kind==='ImportForward'&&(child.hit.is_class||!side.exports[child.hit.file]?.forwardable_function_locals.includes(child.hit.local_name)))
      return {state:'blocked',events:[event('IMPORT_FORWARDABILITY',file,name,v.module_path),...child.events]};
    return child;
  }
  if(name==='default'||!f.star_reexports.length)return {state:'none',events:[event('EXPORT_NAME_MISSING',file,name)]};
  if(depth+1>side.max_depth)return {state:'none',events:[event('EXPORT_DEPTH_LIMIT',file,name)]};
  const candidates=new Map(),events=[];
  for(const spec of f.star_reexports) {
    const m=moduleGate(file,spec,allow);
    if(!m.target){events.push(event(m.gate,file,name,spec));continue;}
    const c=lookup(m.target,name,allow,depth+1,seen);events.push(...c.events);
    if(c.state==='blocked')return {state:'blocked',events:[...c.events,...events.slice(0,events.length-c.events.length)]};
    if(c.state==='resolved')candidates.set(JSON.stringify(c.hit),c.hit);
  }
  if(candidates.size>1)return {state:'blocked',events:[event('STAR_COMPETITION',file,name),...events]};
  if(candidates.size===1)return {state:'resolved',hit:[...candidates.values()][0],events};
  return {state:'none',events:events.length?events:[event('EXPORT_NAME_MISSING',file,name)]};
}
function skipped(file,name,allow,seen=new Set()) {
  const k=JSON.stringify([file,name]);
  if(seen.size>side.max_depth)return [event('STAR_CLOSURE_DEPTH',file,name)];
  if(seen.has(k))return [event('STAR_CLOSURE_CYCLE',file,name)];
  seen=new Set(seen);seen.add(k);
  const f=side.exports[file];if(!f)return [event('EXPORT_FACTS_FILE_MISSING',file,name)];
  const t=f.named[name];
  if(t) {
    const [kind,v]=Object.entries(t)[0];
    if(!['ReExport','ImportForward'].includes(kind))return [];
    const m=moduleGate(file,v.module_path,allow);
    return m.target?skipped(m.target,v.imported,allow,seen):[event(m.gate,file,v.imported,v.module_path)];
  }
  if(f.skipped_expr_count>Object.values(f.skipped_decl_reasons).reduce((a,b)=>a+b,0)||
     Object.keys(f.skipped_decl_reasons).length&&f.module_value_bindings.includes(name))
    return [event('STAR_OPAQUE_EXPORT_FACT',file,name)];
  const result=[];
  for(const spec of f.star_reexports) {
    const m=moduleGate(file,spec,allow);
    result.push(...(m.target?skipped(m.target,name,allow,seen):[event(m.gate,file,name,spec)]));
  }
  return result;
}
// Validate replay against the actual unmodified export table for each entry.
function closure(file,name,allow) {
  const r=lookup(file,name,allow),actual=side.path_exports[String(allow)]?.[file]?.[name];
  const emitted=r.state==='resolved'&&!r.hit.is_class;
  assert.equal(!!actual,emitted,'export table replay disagreement');
  if(actual) {
    const {is_class,...expected}=r.hit;
    assert.deepEqual({...actual,via_unresolved_star:undefined},{...expected,via_unresolved_star:undefined},'export identity replay disagreement');
    const skippedEvents=skipped(file,name,allow);
    assert.equal(actual.via_unresolved_star,!!skippedEvents.length,'star provenance replay disagreement');
    if(skippedEvents.length)return {gate:'UNRESOLVED_STAR_BRANCH',event:skippedEvents[0],events:skippedEvents,actual};
    return {gate:'PASS',events:[],actual};
  }
  const first=r.events[0]||event('EXPORT_CLASS_NOT_CALLABLE',file,name);
  return {gate:first.gate,event:first,events:r.events,actual:null};
}
const host={...ts.sys,getCurrentDirectory:()=>root,writeFile(){throw Error('oracle write refused')},
  watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout(){}};
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
const service=new ts.server.ProjectService({host,logger,cancellationToken:{isCancellationRequested:()=>false},
  useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
const rel=p=>path.relative(root,p).split(path.sep).join('/'),oracleCache=new Map();
function typescript(project,program,e,n) {
  const file=e?.file||n.entry.from,spec=e?.spec;
  let result={resolution:'NOT_A_MODULE_GATE',shape:'EXPORT_OR_SITE',redirect:'NONE'};
  if(spec) {
    const k=JSON.stringify([project.getProjectName(),file,spec]);
    if(oracleCache.has(k))result=oracleCache.get(k);
    else {
      const traces=[];
      const r=ts.resolveModuleName(spec,path.join(root,file),{...project.getCompilerOptions(),traceResolution:true},{...host,trace:s=>traces.push(s)}).resolvedModule;
      const target=r&&rel(r.resolvedFileName);
      const isJS=/\.[cm]?jsx?$/.test(target||'');
      result={target,resolution:!r?'UNRESOLVED':target.startsWith('../')?'OUTSIDE_ROOT':/\.d\.[cm]?ts$/.test(target)?'DECLARATION':
        isJS?(traces.some(s=>s.includes('target file types: JavaScript'))?'JS_SECONDARY_PASS':'JS_PRIORITY_PASS'):'TS_OR_OTHER_SOURCE',
        shape:(ts.isExternalModuleNameRelative(spec)?'RELATIVE':'NONRELATIVE')+(/\.[cm]?[jt]sx?$/.test(spec)?'_EXPLICIT':'_IMPLICIT'),
        redirect:traces.some(s=>s.includes('typesVersions')&&s.includes('matches compiler version'))?'TYPES_VERSIONS':
          traces.some(s=>/package\.json.*has.*(?:main|types|typings).*field/.test(s))?'PACKAGE_FIELD':
          /\/index\.[cm]?[jt]sx?$/.test(target||'')?'DIRECTORY_INDEX':
          /\.[cm]?[jt]sx?$/.test(spec)&&target&&!target.endsWith(spec.replace(/^\.\//,''))?'SUFFIX_REPLACEMENT':'NONE'};
      oracleCache.set(k,result);
    }
  }
  const sf=program.getSourceFile(path.join(root,file));
  const diagnostics=sf?[...new Set(program.getSemanticDiagnostics(sf).map(d=>String(d.code)))].sort():[];
  const exportSf=spec?(result.target&&program.getSourceFile(path.join(root,result.target))):sf;
  const checker=program.getTypeChecker(),symbol=exportSf&&checker.getSymbolAtLocation(exportSf);
  const export_symbol=['IMPORT_BINDING_GATE','SITE_BINDING_GATE'].includes(e?.gate)?'NOT_AN_EXPORT_GATE':
    !exportSf?(spec&&!result.target?'UNRESOLVED_MODULE':'SOURCE_UNAVAILABLE'):
    symbol&&checker.getExportsOfModule(symbol).some(s=>s.name===(e?.member||n.terminal.name))?'PRESENT':'ABSENT';
  return {...result,diagnostics,export_symbol,terminal:n.terminal.class};
}
const catalog=read('gap-class-catalog.json'),counts={},latent={},details=[];
const bump=(o,k)=>{o[k]=(o[k]||0)+1;};
let recovered=0;
for(const n of population) {
  const k=key(n),a=aliasMap.get(k),s=entryMap.get(k),p2=after.get(k),site=siteFacts.get(k);assert(a&&s&&p2&&site);
  // The sidecar and facts binary must have parsed the identical physical bytes.
  assert.equal(side.file_hashes[n.key[0]],readFactHash(n.key[0]));
  if(correct(p2,n)){recovered++;continue;}
  let gate,e,events=[],c;
  const callerFacts=side.exports[n.key[0]];
  gate=bindingGate(s.binding,callerFacts,a.local,site);
  if(gate) {e=event(gate,n.key[0],a.member);}
  else if(!s.module){gate=s.explanation.gate;e=event(gate,n.key[0],a.member,a.specifier);assert.notEqual(gate,'PASS');}
  else {
    c=closure(s.module.target,a.member,s.module.allow_js);gate=c.gate;e=c.event;events=c.events;
    if(gate==='PASS') {
      gate=projectionGate(c.actual,site,side.functions);
      e=event(gate,s.module.target,a.member);
    }
  }
  // Later native-entry gates are explicitly counterfactual when entry was refused.
  if(!s.module&&n.entry.target&&side.exports[n.entry.target]) {
    const later=lookup(n.entry.target,a.member,s.allow_js);
    for(const g of new Set(later.events.map(x=>x.gate)))bump(latent,g);
  }
  assert(catalog.gates[gate],`uncatalogued gate ${gate}`);
  const cls=catalog.gates[gate],v=counts[cls]||={rows:0,member_written_rows:0,exact_gate_histogram:{},typescript_resolution:{},typescript_shape:{},typescript_redirect:{},typescript_terminal:{},typescript_export_symbol:{},typescript_diagnostic_codes:{},encountered_gate_rows:{}};
  v.rows++;v.member_written_rows+=n.refusal_detail_codes.includes('TERMINAL_MEMBER_WRITTEN');bump(v.exact_gate_histogram,gate);
  const absolute=path.join(root,n.key[0]);service.openClientFile(absolute,undefined,undefined,root);
  const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(absolute),true),program=project.getLanguageService().getProgram();
  assert(program,'missing native program');
  const t=typescript(project,program,e,n);
  assert.equal(rel(project.getProjectName()),n.owner_config,'native ownership drift');
  for(const [field,name] of [['typescript_resolution','resolution'],['typescript_shape','shape'],['typescript_redirect','redirect'],['typescript_terminal','terminal'],['typescript_export_symbol','export_symbol']])bump(v[field],t[name]);
  for(const d of t.diagnostics)bump(v.typescript_diagnostic_codes,d);
  for(const g of new Set(events.map(x=>x.gate)))bump(v.encountered_gate_rows,g);
  details.push({key:n.key,class:cls,gate,event:e,events,typescript:t});
  service.closeClientFile(absolute);
}
function readFactHash(file) {return factHashes.get(file);}
assert.equal(Object.values(counts).reduce((a,b)=>a+b.rows,0),population.length-recovered);
verifyNativeInputs();
const unresolved=counts.post_export_site_gate?.rows||0;
const result={status:unresolved?'MEASURED_WITH_UNRESOLVED_SITE_ROWS':'MEASURED',population:{bucket:'JS_EXPORT_HOP',native_callable_ownership_agrees:population.length,
  p2_recovered:recovered,p2_unrecovered:population.length-recovered,
  relative_js_resolution_ceiling:read('p2-comparison.json').relative_js_resolution_ceiling_by_reason.JS_EXPORT_HOP||0,
  p2_recovered_member_written:population.filter(n=>correct(after.get(key(n)),n)&&n.refusal_detail_codes.includes('TERMINAL_MEMBER_WRITTEN')).length,
  excluded_native_ownership_disagreements:native.filter(r=>r.reason==='JS_EXPORT_HOP'&&r.native_callable&&!r.ownership_agrees).length},
  classes:counts,counterfactual_gates_after_native_entry:latent,
  taxonomy:catalog.classes,prototype_changed_rows:read('p2-comparison.json').classes,
  source_binding:read('gap-driver-build/binding.json'),
  binary_hashes:read('gap-source-binding.json').binary_hashes,
  diagnostic_probe_hashes:{classifier:sha(__filename),projection:sha(require.resolve('./gap-projection.cjs')),
    catalog:sha(path.join(out,'gap-class-catalog.json')),oracle:sha(tsPath)},
  assertions:{complete_partition:true,kernel_replay:true,export_table_replay:true,keys_unchanged:true,corpus_identifiers_in_stdout:false},
  limitations:{post_export_site_gate_rows:unresolved,
    forecast:'ASSUMPTION',secondary_counts_are_not_additive:true}};
fs.writeFileSync(path.join(out,'gap-private-rows.json'),JSON.stringify(details,null,2)+'\n');
fs.writeFileSync(path.join(out,'gap-aggregate.json'),JSON.stringify(result)+'\n');
console.log(JSON.stringify(result));
