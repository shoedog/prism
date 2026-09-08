import {createHash} from "node:crypto";
import path from "node:path";
import {libFileForReference,libraryNameFromLibFile} from './lib-search.mjs';
import {classifyEntryObligations} from './entry-obligations.mjs';
import {classifySemanticClosure,SEMANTIC_CLOSURE_POLICY} from './semantic-closure.mjs';
import {classifySemanticClosureV2,SEMANTIC_CLOSURE_POLICY_V2} from './semantic-closure-v2.mjs';
const SCHEMA14="prism.callable-observation/14";
const SCHEMA15="prism.callable-observation/15";
const SCHEMA16="prism.callable-observation/16";
const SCHEMA17="prism.callable-observation/17";
const SCHEMA18="prism.callable-observation/18";
export const SCHEMA="prism.callable-observation/19";
export const REFERENCE_LIMIT=100000;
export const referenceKey=r=>JSON.stringify([r.request.file,r.kind,r.index]);
export const entryKey=r=>JSON.stringify([r.kind,r.index]);
export const COMPILER_HASH="3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675";
export const LIMITS={files:20000,bytes:128*1024*1024,read_bytes:128*1024*1024,link_steps:32,depth:64,timeout_ms:30000,observations:2000,provenance_steps:32,nested_depth:8,nested_calls:128,props_type_args:8};
export const PACKET_BYTES=8*1024*1024;
export const PROFILES={
  default:{limits:LIMITS,packet_bytes:PACKET_BYTES,heap_mb:512},
  installed:{limits:{...LIMITS,files:100000,bytes:1024*1024*1024,read_bytes:32*1024*1024,timeout_ms:120000},packet_bytes:32*1024*1024,heap_mb:1024},
};
export const MAX_PACKET_BYTES=32*1024*1024;
export const hash=b=>createHash("sha256").update(b).digest("hex");
export const canonical=x=>JSON.stringify(sort(x));
function sort(x) {
  if(Array.isArray(x)) return x.map(sort);
  if(x && typeof x==="object") return Object.fromEntries(Object.keys(x).sort().map(k=>[k,sort(x[k])]));
  return x;
}
export const relative=s=>typeof s==="string" && s.length>0 && s.length<=4096
  && !/[\\:\x00-\x1f]/.test(s) && s.split("/").every(p=>p && p!=="." && p!=="..");
const id=s=>relative(s) && /^(project|compiler)(\/|$)/.test(s);
const str=s=>typeof s==="string" && s.length<=65536;
const integer=n=>Number.isSafeInteger(n) && n>=0;
const boolean=x=>typeof x==="boolean";
const digest=s=>typeof s==="string" && /^[a-f0-9]{64}$/.test(s);
const literal=v=>x=>x===v;
const array=rule=>x=>Array.isArray(x) && x.length<=100000 && x.every(rule);
const nullable=rule=>x=>x===null || rule(x);
const object=shape=>x=>x!==null && typeof x==="object" && !Array.isArray(x)
  && Object.keys(x).length===Object.keys(shape).length
  && Object.entries(shape).every(([k,v])=>Object.hasOwn(x,k) && v(x[k]));
const anchor=object({file:id,sha256:digest,kind:str,start_utf16:integer,end_utf16:integer,start_byte:integer,end_byte:integer});
const wildcard=object({status:x=>['observed','unproven'].includes(x),
  reason:nullable(x=>['unsupported_request','filesystem_target','augmentation','duplicate_provider','competing_pattern',
    'ambiguous_binding','exact_provider','binding_mismatch','unsupported_provider'].includes(x)),
  pattern:x=>str(x)&&x.split('*').length===2,providers:array(anchor),augmentations:array(anchor),matches:array(anchor)});
const moduleEntry=object({declaration:anchor,shape:x=>['empty_block','shorthand','other'].includes(x)});
const mergedWildcard=object({status:x=>['observed','unproven'].includes(x),
  reason:nullable(x=>['unsupported_request','filesystem_target','augmentation','provider_count','competing_pattern',
    'exact_provider','binding_mismatch','unsupported_provider','selected_declaration'].includes(x)),
  declarations:array(moduleEntry),selected:nullable(moduleEntry)});
const lookup=object({status:x=>['observed','unproven'].includes(x),
  reason:nullable(x=>['synthetic_request','augmentation_request','unsupported_request','filesystem_target',
    'augmentation','duplicate_provider','unresolved_symbol','ambiguous_binding','non_exact_binding',
    'binding_mismatch','unsupported_provider'].includes(x)),
  context:x=>['synthetic','import','export','import_type','import_equals','augmentation_name','dynamic_import','require','other'].includes(x),
  request:nullable(anchor),declarations:array(anchor),providers:array(anchor),augmentations:array(anchor),wildcard:nullable(wildcard),merged_wildcard:nullable(mergedWildcard)});
const alias=object({declarations:array(anchor),target:array(anchor),module:nullable(anchor),
  module_declarations:array(anchor),module_exports:array(anchor),module_bindings:array(anchor)});
const provenance=object({status:x=>["traced","unproven"].includes(x),
  reason:nullable(x=>["step_limit","cycle","ambiguous_declaration","unresolved_symbol",
    "unsupported_declaration","unsupported_type","unsupported_heritage","unsupported_export_star"].includes(x)),
  steps_used:integer,terminal:nullable(anchor),hops:array(object({reference:anchor,type_arguments:array(anchor),
    type_parameters:array(anchor),declarations:array(anchor),aliases:array(alias),
    qualifiers:array(object({use:anchor,aliases:array(alias),declarations:array(anchor)}))}))});
const propsClass=object({status:x=>["observed","unproven"].includes(x),
  reason:nullable(x=>["binding_unproven","explicit_parameter","callable_unproven","signature_ambiguity","unsupported_path","unsupported_props","ambiguous_declaration","unsupported_property","unsupported_class","type_argument_limit","program_unproven"].includes(x)),
  signatures:array(anchor),props_type:nullable(str),props_declarations:array(anchor),
  instantiation:array(object({parameter:anchor,argument_type:str,argument_declarations:array(anchor)})),
  property_name:nullable(str),property_declarations:array(anchor),property_type:nullable(str),
  declared_type_declarations:array(anchor),class_declaration:nullable(anchor)});
const nested=object({calls:array(object({call:anchor,receiver:anchor,receiver_type:str,declarations:array(anchor),functions:array(anchor),props_class:propsClass,
  binding:object({status:x=>["linked","unproven"].includes(x),
    reason:nullable(x=>["unsupported_receiver","unresolved_symbol","unsupported_parameter","other_binding","duplicate_binding","write_barrier"].includes(x)),
    use:nullable(anchor),parameter:nullable(anchor),declarations:array(anchor),writes:array(anchor)})})),
  barriers:array(object({scope:anchor,reason:x=>["unsupported_scope","depth_limit","call_limit"].includes(x)}))});
const reasons=array(x=>[
  "budget_exceeded","unsupported_input","compiler_mismatch","unstable_snapshot",
  "compiler_diagnostics","unsupported_references","unsupported_plugins",
  "outside_lookup","unsupported_lookup","unresolved_module","unproven_path_reference","unproven_type_lib_reference","invalid_config","worker_failed",
].includes(x));
const packetShape={
  schema:literal("prism.callable-observation/10"),authorizes_runtime_edge:literal(false),
  // Historical schema10 packets remain readable for pinned audit tooling.
  // validate() still requires exact reproduction by the current producer.
  producer:object({version:x=>["0.11.0","0.11.1"].includes(x),sha256:digest}),
  compiler:object({version:literal("5.9.3"),sha256:literal(COMPILER_HASH),verified:boolean,library_sha256:digest}),
  scope:object({config:id,acquisition_profile:x=>typeof x==="string" && Object.hasOwn(PROFILES,x),link_policy:x=>["reject","in-root"].includes(x),callable_scope:literal("direct-annotated-function"),class_authority:literal(false),case_sensitive:nullable(boolean)}),
  status:x=>["observed","unproven"].includes(x),reasons,
  closure:object({stable_snapshot:boolean,dependencies:boolean,references:boolean,augmentation:boolean,resolution:boolean}),
  limits:object(Object.fromEntries(Object.keys(LIMITS).map(k=>[k,integer]))),
  snapshot:object({sha256:digest,files:array(object({id,sha256:digest,size:integer})),directories:array(id),links:array(object({id,target:id,sha256:digest})),
    roots:array(id),config_files:array(id),program_files:array(id),reads:array(id),failed_lookups:array(id),
    refused_lookup_sha256:array(digest),outside_lookups:boolean,options_sha256:digest}),
  resolutions:array(object({from:id,specifier:str,target:nullable(id),lookup})),
  diagnostics:array(object({code:integer,file:nullable(id),start:nullable(integer)})),
  observations:array(object({annotation:anchor,implementation:anchor,parameter:nullable(anchor),
    explicit_parameter:boolean,signatures:array(anchor),callable_declarations:array(anchor),provenance,nested,
    calls:array(object({call:anchor,receiver:anchor,receiver_type:str,declarations:array(anchor)}))})),
};
const packet10=object(packetShape);
const typeLibReference=object({kind:x=>['types','lib'].includes(x),index:integer,name:str,
  mode:nullable(x=>['import','require'].includes(x)),request:anchor,status:x=>['observed','unproven'].includes(x),
  reason:nullable(x=>['unprocessed','unresolved','target_not_in_program','missing_inclusion'].includes(x)),target:nullable(id),inclusion:boolean});
const referenceShape={...packetShape,type_lib_references:array(typeLibReference)};
const packet11=object({...referenceShape,schema:literal('prism.callable-observation/11'),producer:object({version:literal('0.12.0'),sha256:digest})});
const typeLibEntry=object({kind:x=>['types','lib'].includes(x),origin:x=>['configured','automatic','default'].includes(x),
  index:integer,name:str,mode:literal(null),status:x=>['observed','unproven'].includes(x),
  reason:nullable(x=>['unprocessed','unresolved','target_not_in_program','missing_inclusion'].includes(x)),target:nullable(id),inclusion:boolean});
const packet12=object({...referenceShape,schema:literal('prism.callable-observation/12'),producer:object({version:literal('0.13.0'),sha256:digest}),type_lib_entries:array(typeLibEntry)});
const moduleRequest=object({id:integer,from:id,specifier:str,mode:nullable(x=>['import','require'].includes(x)),request:nullable(anchor),target:nullable(id)});
const searchMode=nullable(x=>['import','require'].includes(x));
const typeBatch=object({id:integer,origin:x=>['source','configured','automatic'].includes(x),from:id,size:integer});
const typeRequest=object({id:integer,origin:x=>['source','configured','automatic'].includes(x),from:id,name:str,mode:searchMode,
  request:nullable(anchor),index:integer,execution:integer,batch:integer});
const typeSearch=object({id:integer,from:id,name:str,mode:searchMode,target:nullable(id)});
const libBeneficiary=object({origin:x=>['source','configured'].includes(x),request:nullable(anchor),index:integer});
const libSearch=object({id:integer,name:str,from:id,lib_file:str,target:nullable(id),selected:nullable(id),beneficiaries:array(libBeneficiary)});
const boundaryEvent=ownerRule=>object({id:integer,kind:x=>['outside','refused'].includes(x),operation:x=>['identity','readFile','entries','fileExists','directoryExists','realpath'].includes(x),probe_sha256:digest,owner:nullable(ownerRule)});
const moduleOwner=object({channel:literal('module'),id:integer});
const typeSearchOwner=object({channel:x=>['module','type'].includes(x),id:integer});
const searchOwner=object({channel:x=>['module','type','lib'].includes(x),id:integer});
const configOptionNames=['types','lib','typeRoots','noLib','libReplacement','noResolve','target'];
const configReason=x=>['chain_limit','unavailable','invalid_config','duplicate_property','unsupported_extends','cycle','missing_target','selection_mismatch','unsupported_option','value_mismatch'].includes(x);
const configProvenance=object({status:x=>['observed','unproven'].includes(x),reason:nullable(configReason),files:array(anchor),
  extends:array(object({source:anchor,target:anchor})),options:array(object({name:x=>configOptionNames.includes(x),present:boolean,value_sha256:digest,origin:nullable(anchor)}))});
const entryObligation=object({kind:x=>['types','lib'].includes(x),origin:x=>['configured','automatic','default'].includes(x),index:integer,
  disposition:x=>['selected','disabled','unproven'].includes(x),reason:nullable(x=>['no_roots','no_lib','suppression_unproven','unprocessed','unresolved','target_not_in_program','missing_inclusion'].includes(x))});
const entryObligations=object({complete:boolean,reasons:array(x=>['configuration_unproven','automatic_discovery_unproven','unproven_entry'].includes(x)),rows:array(entryObligation)});
const semanticClosure=policy=>object({policy:literal(policy),complete:boolean,
  reasons:array(x=>['compiler_unverified','unstable_snapshot','config_unproven','entry_obligations_incomplete','no_resolve',
    'compiler_diagnostics','unsupported_project_references','unsupported_plugins','required_path_unproven','type_lib_unproven',
    'boundary_encounter','global_refusal','resolution_unproven'].includes(x)),
  rows:array(object({index:integer,disposition:x=>['filesystem_selected','exact_ambient','singleton_wildcard','merged_side_effect','unproven'].includes(x),
    reason:nullable(x=>['target_not_in_program','unresolved_module','unadmitted_binding'].includes(x))}))});
const packet13=object({...referenceShape,schema:literal('prism.callable-observation/13'),producer:object({version:literal('0.14.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),search_provenance:object({module_requests:array(moduleRequest),boundary_events:array(boundaryEvent(moduleOwner))})});
const packet14=object({...referenceShape,schema:literal(SCHEMA14),producer:object({version:literal('0.15.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),
  search_provenance:object({module_requests:array(moduleRequest),type_batches:array(typeBatch),type_requests:array(typeRequest),type_searches:array(typeSearch),boundary_events:array(boundaryEvent(typeSearchOwner))})});
const packet15=object({...referenceShape,schema:literal(SCHEMA15),producer:object({version:literal('0.16.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),
  search_provenance:object({module_requests:array(moduleRequest),type_batches:array(typeBatch),type_requests:array(typeRequest),type_searches:array(typeSearch),lib_searches:array(libSearch),boundary_events:array(boundaryEvent(searchOwner))})});
const packet16=object({...referenceShape,schema:literal(SCHEMA16),producer:object({version:literal('0.17.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),
  search_provenance:object({module_requests:array(moduleRequest),type_batches:array(typeBatch),type_requests:array(typeRequest),type_searches:array(typeSearch),lib_searches:array(libSearch),boundary_events:array(boundaryEvent(searchOwner))}),config_provenance:configProvenance});
const packet17=object({...referenceShape,schema:literal(SCHEMA17),producer:object({version:literal('0.18.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),
  search_provenance:object({module_requests:array(moduleRequest),type_batches:array(typeBatch),type_requests:array(typeRequest),type_searches:array(typeSearch),lib_searches:array(libSearch),boundary_events:array(boundaryEvent(searchOwner))}),config_provenance:configProvenance,entry_obligations:entryObligations});
const packet18=object({...referenceShape,schema:literal(SCHEMA18),producer:object({version:literal('0.19.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),
  search_provenance:object({module_requests:array(moduleRequest),type_batches:array(typeBatch),type_requests:array(typeRequest),type_searches:array(typeSearch),lib_searches:array(libSearch),boundary_events:array(boundaryEvent(searchOwner))}),config_provenance:configProvenance,entry_obligations:entryObligations,semantic_closure:semanticClosure(SEMANTIC_CLOSURE_POLICY)});
const packet19=object({...referenceShape,schema:literal(SCHEMA),producer:object({version:literal('0.20.0'),sha256:digest}),type_lib_entries:array(typeLibEntry),
  search_provenance:object({module_requests:array(moduleRequest),type_batches:array(typeBatch),type_requests:array(typeRequest),type_searches:array(typeSearch),lib_searches:array(libSearch),boundary_events:array(boundaryEvent(searchOwner))}),config_provenance:configProvenance,entry_obligations:entryObligations,semantic_closure:semanticClosure(SEMANTIC_CLOSURE_POLICY_V2)});
export function parsePacket(text) {
  if(typeof text!=="string" || Buffer.byteLength(text)>MAX_PACKET_BYTES) throw Error("invalid_packet");
  const value=JSON.parse(text);
  if(!packet10(value)&&!packet11(value)&&!packet12(value)&&!packet13(value)&&!packet14(value)&&!packet15(value)&&!packet16(value)&&!packet17(value)&&!packet18(value)&&!packet19(value)) throw Error("invalid_packet");
  const profile=PROFILES[value.scope.acquisition_profile];
  if(Buffer.byteLength(text)>profile.packet_bytes)throw Error("invalid_packet");
  const files=new Map(value.snapshot.files.map(f=>[f.id,f]));
  if(files.size!==value.snapshot.files.length) throw Error("invalid_packet");
  if(Object.values(value.limits).some(n=>n<1) || Object.entries(value.limits).some(([k,v])=>v>profile.limits[k])) throw Error("invalid_packet");
  if(!value.scope.config.startsWith("project/")) throw Error("invalid_packet");
  if(value.status==="observed" && (value.reasons.length || !value.compiler.verified || Object.values(value.closure).some(x=>!x))) throw Error("invalid_packet");
  if(value.status==="unproven" && !value.reasons.length) throw Error("invalid_packet");
  if(value.reasons.includes("unproven_path_reference") && (!["0.11.1","0.12.0","0.13.0","0.14.0","0.15.0","0.16.0","0.17.0","0.18.0","0.19.0","0.20.0"].includes(value.producer.version)
    || value.status!=="unproven" || value.closure.dependencies || value.closure.references
    || value.closure.augmentation || value.closure.resolution))throw Error("invalid_packet");
  const refused=value.snapshot.refused_lookup_sha256;
  if(refused.some((v,i)=>i>0 && v<=refused[i-1])
    || !!refused.length!==value.reasons.includes("unsupported_lookup")
    || refused.length && (value.status!=="unproven" || value.closure.dependencies
      || value.closure.augmentation || value.closure.resolution))throw Error("invalid_packet");
  const dirs=new Set(value.snapshot.directories),links=value.snapshot.links;
  if(dirs.size!==value.snapshot.directories.length || (value.scope.link_policy==="reject" && links.length))throw Error("invalid_packet");
  if(files.size+dirs.size+links.length>value.limits.files || value.snapshot.files.reduce((n,f)=>n+f.size,0)>value.limits.bytes)throw Error("invalid_packet");
  if(links.some((l,i)=>i>0 && l.id<=links[i-1].id || files.has(l.id) || dirs.has(l.id)
      || !files.has(l.target) && !dirs.has(l.target) || l.id.split("/")[0]!==l.target.split("/")[0]
      || [l.id,l.target].some(v=>v.split("/").some(p=>p.toLowerCase()===".git"))))throw Error("invalid_packet");
  if(value.snapshot.files.length && value.snapshot.sha256!==hash(canonical({files:value.snapshot.files,directories:value.snapshot.directories,links}))) throw Error("invalid_packet");
  for(const name of ["roots","program_files","config_files","reads"]) {
    if(value.snapshot[name].some(id=>!files.has(id))) throw Error("invalid_packet");
  }
  function checkAnchor(a) {
    if(!a)return;
    const f=files.get(a.file);
    if(!f || f.sha256!==a.sha256 || a.start_utf16>a.end_utf16 || a.start_byte>a.end_byte
      || a.end_byte>f.size || a.end_utf16>a.end_byte) throw Error("invalid_packet");
  }
  const programFiles=new Set(value.snapshot.program_files);
  if(value.config_provenance) {
    const record=value.config_provenance;
    if(record.status==='unproven') {
      if(!record.reason||record.files.length||record.extends.length||record.options.length)throw Error('invalid_packet');
    } else {
      if(record.reason!==null||record.files.length<1||record.files.length>32||record.extends.length!==record.files.length-1
          ||record.options.length!==configOptionNames.length)throw Error('invalid_packet');
      const ids=new Set(record.files.map(a=>a.file)),configFiles=value.snapshot.config_files;
      if(ids.size!==record.files.length||new Set(configFiles).size!==configFiles.length
          ||canonical([...ids].sort())!==canonical([...configFiles].sort()))throw Error('invalid_packet');
      for(const anchor of record.files) {
        checkAnchor(anchor);const file=files.get(anchor.file);
        if(anchor.kind!=='SourceFile'||anchor.start_utf16!==0||anchor.start_byte!==0||anchor.end_byte!==file.size)throw Error('invalid_packet');
      }
      for(const [i,edge] of record.extends.entries()) {
        checkAnchor(edge.source);checkAnchor(edge.target);
        if(edge.source.kind!=='StringLiteral'||edge.source.file!==record.files[i].file
            ||canonical(edge.target)!==canonical(record.files[i+1]))throw Error('invalid_packet');
      }
      const absent=hash(canonical({present:false,value:null}));
      for(const [i,row] of record.options.entries()) {
        checkAnchor(row.origin);
        if(row.name!==configOptionNames[i]||row.present!==!!row.origin
            ||row.origin&&(row.origin.kind!=='PropertyAssignment'||!ids.has(row.origin.file))
            ||!row.present&&row.value_sha256!==absent)throw Error('invalid_packet');
      }
    }
  }
  if(value.search_provenance) {
    const {module_requests:requests,boundary_events:events}=value.search_provenance;
    const requestIds=new Set();
    for(const [i,r] of requests.entries()) {
      if(r.id!==i||requestIds.has(r.id)||!programFiles.has(r.from))throw Error('invalid_packet');
      requestIds.add(r.id);checkAnchor(r.request);
      if(r.request&&(!programFiles.has(r.request.file)||r.request.file!==r.from))throw Error('invalid_packet');
    }
    const key=r=>canonical({from:r.from,specifier:r.specifier,target:r.target,request:r.request});
    const actual=value.resolutions.map(r=>({from:r.from,specifier:r.specifier,target:r.target,request:r.lookup.request})).map(key).sort();
    if(canonical(requests.map(key).sort())!==canonical(actual))throw Error('invalid_packet');
    const typeBatches=value.search_provenance.type_batches??[],typeRequests=value.search_provenance.type_requests??[],typeSearches=value.search_provenance.type_searches??[],libSearches=value.search_provenance.lib_searches??[];
    if(typeBatches.length>REFERENCE_LIMIT||typeRequests.length>REFERENCE_LIMIT||typeSearches.length>REFERENCE_LIMIT||libSearches.length>REFERENCE_LIMIT)throw Error('invalid_packet');
    const batchById=new Map();
    for(const [i,b] of typeBatches.entries()){if(b.id!==i||b.size>REFERENCE_LIMIT||batchById.has(b.id))throw Error('invalid_packet');batchById.set(b.id,b);}
    const typeSearchIds=new Set();
    for(const [i,s] of typeSearches.entries()){if(s.id!==i||typeSearchIds.has(s.id))throw Error('invalid_packet');typeSearchIds.add(s.id);}
    const references=value.type_lib_references??[],entries=value.type_lib_entries??[],usedLinks=new Set(),usedSearches=new Set();
    const requestsByBatch=new Map(typeBatches.map(b=>[b.id,[]])),executionBatches=new Map(),keyExecutions=new Map(),coveredRows=new Set();
    const inferredFrom=path.posix.join(path.posix.dirname(value.scope.config),'__inferred type names__.ts');
    let priorBatch=-1;
    for(const [i,r] of typeRequests.entries()) {
      const batch=batchById.get(r.batch);
      if(r.id!==i||r.index>=REFERENCE_LIMIT||!batch||r.batch<priorBatch||batch.origin!==r.origin||batch.from!==r.from||!typeSearchIds.has(r.execution))throw Error('invalid_packet');
      priorBatch=r.batch;requestsByBatch.get(r.batch).push(r);
      const search=typeSearches[r.execution];
      if(search.from!==r.from||search.name!==r.name||search.mode!==r.mode)throw Error('invalid_packet');
      const executionBatch=executionBatches.get(r.execution);
      if(executionBatch!==undefined&&executionBatch!==r.batch)throw Error('invalid_packet');executionBatches.set(r.execution,r.batch);
      const batchKey=canonical({batch:r.batch,name:r.name,mode:r.mode}),keyExecution=keyExecutions.get(batchKey);
      if(keyExecution!==undefined&&keyExecution!==r.execution)throw Error('invalid_packet');keyExecutions.set(batchKey,r.execution);
      usedSearches.add(r.execution);checkAnchor(r.request);
      let row,rowLink;
      if(r.origin==='source') {
        if(!r.request||!programFiles.has(r.from)||r.request.file!==r.from)throw Error('invalid_packet');
        row=references.find(x=>x.kind==='types'&&x.request.file===r.from&&x.index===r.index);
        rowLink=canonical({origin:r.origin,from:r.from,index:r.index});
        if(!row||canonical(row.request)!==canonical(r.request))throw Error('invalid_packet');
      } else {
        if(r.request!==null||r.from!==inferredFrom)throw Error('invalid_packet');
        row=entries.find(x=>x.kind==='types'&&x.origin===r.origin&&x.index===r.index);
        rowLink=canonical({origin:r.origin,index:r.index});
      }
      const link=canonical({batch:r.batch,row:rowLink});
      if(!row||usedLinks.has(link)||row.reason==='unprocessed'||row.name!==r.name||row.mode!==r.mode
        ||row.status==='observed'&&row.target!==search.target||row.reason==='unresolved'&&search.target!==null)throw Error('invalid_packet');
      usedLinks.add(link);coveredRows.add(rowLink);
    }
    const expectedRows=new Set();
    if([SCHEMA14,SCHEMA15,SCHEMA16,SCHEMA17,SCHEMA18,SCHEMA].includes(value.schema)) {
      for(const batch of typeBatches) {
        if(batch.origin==='source'?!programFiles.has(batch.from):batch.from!==inferredFrom)throw Error('invalid_packet');
        const rows=(batch.origin==='source'?references.filter(r=>r.kind==='types'&&r.request.file===batch.from)
          :entries.filter(r=>r.kind==='types'&&r.origin===batch.origin)).filter(r=>r.reason!=='unprocessed').sort((a,b)=>a.index-b.index);
        const batchRequests=requestsByBatch.get(batch.id);
        if(batch.size!==rows.length||batchRequests.length!==rows.length
          ||rows.some((row,index)=>batchRequests[index].index!==row.index))throw Error('invalid_packet');
        for(const row of rows)expectedRows.add(batch.origin==='source'?canonical({origin:'source',from:batch.from,index:row.index}):canonical({origin:batch.origin,index:row.index}));
      }
      for(const row of references)if(row.kind==='types'&&row.reason!=='unprocessed')expectedRows.add(canonical({origin:'source',from:row.request.file,index:row.index}));
      for(const row of entries)if(row.kind==='types'&&row.reason!=='unprocessed')expectedRows.add(canonical({origin:row.origin,index:row.index}));
    }
    if(usedSearches.size!==typeSearches.length||[...expectedRows].some(link=>!coveredRows.has(link)))throw Error('invalid_packet');
    const libSearchIds=new Set(),libFiles=new Set();let beneficiaryCount=0;
    for(const [i,s] of libSearches.entries()) {
      if(s.id!==i||libSearchIds.has(s.id)||libFiles.has(s.lib_file)
        ||libraryNameFromLibFile(s.lib_file)!==s.name
        ||s.from!==path.posix.join(path.posix.dirname(value.scope.config),`__lib_node_modules_lookup_${s.lib_file}__.ts`))throw Error('invalid_packet');
      libSearchIds.add(s.id);libFiles.add(s.lib_file);
      beneficiaryCount+=s.beneficiaries.length;if(beneficiaryCount>REFERENCE_LIMIT)throw Error('invalid_packet');
      for(const beneficiary of s.beneficiaries)checkAnchor(beneficiary.request);
      const expected=[
        ...references.filter(row=>row.kind==='lib'&&row.status==='observed'
          &&libFileForReference(row.name)===s.lib_file&&row.target===s.selected)
          .map(row=>({origin:'source',request:row.request,index:row.index})),
        ...entries.filter(row=>row.kind==='lib'&&row.origin==='configured'&&row.status==='observed'
          &&row.name===s.lib_file&&row.target===s.selected)
          .map(row=>({origin:'configured',request:null,index:row.index})),
      ];
      if(canonical(s.beneficiaries)!==canonical(expected))throw Error('invalid_packet');
    }
    for(const [i,event] of events.entries()) {
      const ownerIds=event.owner?.channel==='module'?requestIds:event.owner?.channel==='type'?typeSearchIds:libSearchIds;
      if(event.id!==i||event.owner&&!ownerIds.has(event.owner.id))throw Error('invalid_packet');
    }
    const refused=new Set(events.filter(e=>e.kind==='refused').map(e=>e.probe_sha256));
    if(canonical([...refused].sort())!==canonical(value.snapshot.refused_lookup_sha256)
      ||events.some(e=>e.kind==='outside')!==value.snapshot.outside_lookups)throw Error('invalid_packet');
  }
  const references=value.type_lib_references??[],unproven=references.some(r=>r.status==='unproven');
  if(unproven!==value.reasons.includes('unproven_type_lib_reference')
    || unproven&&(!['prism.callable-observation/11','prism.callable-observation/12','prism.callable-observation/13',SCHEMA14,SCHEMA15,SCHEMA16,SCHEMA17,SCHEMA18,SCHEMA].includes(value.schema)||value.status!=='unproven'||value.closure.dependencies
      ||value.closure.references||value.closure.augmentation||value.closure.resolution))throw Error('invalid_packet');
  for(const [i,r] of references.entries()) {
    checkAnchor(r.request);
    if(i>0&&referenceKey(references[i-1])>=referenceKey(r)
      ||!programFiles.has(r.request.file)||r.index>=REFERENCE_LIMIT
      ||r.request.kind!==(r.kind==='types'?'TypeReferenceDirective':'LibReferenceDirective')
      ||r.kind==='lib'&&r.mode!==null
      ||r.request.end_utf16-r.request.start_utf16!==r.name.length
      ||r.request.end_byte-r.request.start_byte!==Buffer.byteLength(r.name)
      ||(r.status==='observed'
        ? r.reason!==null||!r.target||!programFiles.has(r.target)||!r.inclusion||!value.compiler.verified
        : !r.reason||r.target!==null||r.inclusion))throw Error('invalid_packet');
  }
  const entries=value.type_lib_entries??[];
  for(const [i,r] of entries.entries()) {
    if(i>0&&entryKey(entries[i-1])>=entryKey(r)||r.index>=REFERENCE_LIMIT
      ||r.kind==='types'&&r.origin==='default'||r.kind==='lib'&&r.origin==='automatic'
      ||r.origin==='default'&&r.index!==0
      ||(r.status==='observed'
        ? r.reason!==null||!r.target||!programFiles.has(r.target)||!r.inclusion||!value.compiler.verified
        : !r.reason||r.target!==null||r.inclusion))throw Error('invalid_packet');
  }
  if(value.entry_obligations) {
    const configObserved=value.config_provenance.status==='observed';
    const option=name=>configObserved&&value.config_provenance.options.find(row=>row.name===name);
    const noLib=option('noLib')?.present===true
      &&option('noLib').value_sha256===hash(canonical({present:true,value:true}));
    const expected=classifyEntryObligations({entries,rootCount:value.snapshot.roots.length,configObserved,
      noLib,explicitTypes:option('types')?.present===true});
    if(canonical(value.entry_obligations)!==canonical(expected))throw Error('invalid_packet');
  }
  for(const r of value.resolutions) {
    const l=r.lookup,synthetic=l.context==='synthetic';
    for(const a of [l.request,...l.declarations,...l.providers,...l.augmentations]) {
      checkAnchor(a);if(a && !programFiles.has(a.file))throw Error('invalid_packet');
    }
    if(!programFiles.has(r.from) || (l.request===null)!==synthetic
      || r.target===null && (!value.reasons.includes('unresolved_module') || value.status!=='unproven'
        || value.closure.dependencies || value.closure.augmentation || value.closure.resolution)
      || l.request && l.request.file!==r.from
      || [...l.providers,...l.augmentations].some(a=>a.kind!=='ModuleDeclaration')
      || (l.status==='observed'
        ? l.reason!==null || r.target!==null || !value.compiler.verified || !l.request
          || l.request.kind!=='StringLiteral' || !['import','export','import_type','import_equals'].includes(l.context)
          || l.declarations.length!==1 || l.providers.length!==1 || l.augmentations.length
          || canonical(l.declarations[0])!==canonical(l.providers[0])
        : !l.reason)
      || synthetic!==(l.reason==='synthetic_request')
      || (l.context==='augmentation_name')!==(l.reason==='augmentation_request')
      || l.reason==='filesystem_target' && r.target===null
      || l.reason==='augmentation' && !l.augmentations.length
      || l.reason==='duplicate_provider' && l.providers.length<2
      || l.reason==='unresolved_symbol' && l.declarations.length
      || l.reason==='ambiguous_binding' && l.declarations.length<2)throw Error('invalid_packet');
    const w=l.wildcard;
    if(w) {
      for(const a of [...w.providers,...w.augmentations,...w.matches]) {
        checkAnchor(a);if(!programFiles.has(a.file)||a.kind!=='ModuleDeclaration')throw Error('invalid_packet');
      }
      const [prefix,suffix]=w.pattern.split('*');
      if(!l.declarations.length || (w.status==='observed'
        ? w.reason!==null || r.target!==null || !value.compiler.verified || !l.request || l.request.kind!=='StringLiteral'
          || !['import','export','import_type','import_equals'].includes(l.context) || l.status!=='unproven'
          || l.declarations.length!==1 || l.providers.length || l.augmentations.length
          || w.providers.length!==1 || w.matches.length!==1 || w.augmentations.length
          || canonical(w.providers[0])!==canonical(l.declarations[0]) || canonical(w.matches[0])!==canonical(w.providers[0])
          || r.specifier.length<prefix.length+suffix.length || !r.specifier.startsWith(prefix) || !r.specifier.endsWith(suffix)
        : !w.reason)
        || w.reason==='filesystem_target' && r.target===null
        || w.reason==='augmentation' && !w.augmentations.length
        || w.reason==='duplicate_provider' && w.providers.length<2
        || w.reason==='competing_pattern' && w.matches.length<2
        || w.reason==='ambiguous_binding' && l.declarations.length<2
        || w.reason==='exact_provider' && !l.providers.length)throw Error('invalid_packet');
    }
  }
  for(const r of value.resolutions) {
    const l=r.lookup,w=l.wildcard,m=l.merged_wildcard;if(!m)continue;
    if(!w || w.providers.length<2 && l.declarations.length<2)throw Error('invalid_packet');
    for(const e of [...m.declarations,...(m.selected?[m.selected]:[])]) {
      checkAnchor(e.declaration);
      if(!programFiles.has(e.declaration.file)||e.declaration.kind!=='ModuleDeclaration')throw Error('invalid_packet');
    }
    const sameSet=(a,b)=>canonical(a.map(canonical).sort())===canonical(b.map(canonical).sort());
    const [prefix,suffix]=w.pattern.split('*');
    if((m.status==='observed'
      ? m.reason!==null || r.target!==null || !value.compiler.verified || !l.request || l.request.kind!=='StringLiteral'
        || l.context!=='import' || l.status!=='unproven' || w.status!=='unproven' || w.reason!=='duplicate_provider'
        || l.providers.length || l.augmentations.length || w.augmentations.length || m.declarations.length!==2
        || canonical(m.declarations.map(e=>e.declaration))!==canonical(l.declarations)
        || new Set(m.declarations.map(e=>canonical(e.declaration))).size!==2
        || !sameSet(w.providers,l.declarations) || !sameSet(w.matches,l.declarations)
        || m.declarations.filter(e=>e.shape==='empty_block').length!==1 || m.declarations.filter(e=>e.shape==='shorthand').length!==1
        || !m.selected || !m.declarations.some(e=>canonical(e)===canonical(m.selected))
        || r.specifier.length<prefix.length+suffix.length || !r.specifier.startsWith(prefix) || !r.specifier.endsWith(suffix)
      : !m.reason)
      || m.reason==='filesystem_target' && r.target===null
      || m.reason==='augmentation' && !w.augmentations.length && !l.augmentations.length
      || m.reason==='provider_count' && w.providers.length===2 && l.declarations.length===2
      || m.reason==='competing_pattern' && w.matches.length===2
      || m.reason==='exact_provider' && !l.providers.length
      || m.reason==='selected_declaration' && m.selected && m.declarations.some(e=>canonical(e)===canonical(m.selected)))throw Error('invalid_packet');
  }
  for(const o of value.observations) {
    [o.annotation,o.implementation,o.parameter,...o.signatures,...o.callable_declarations].forEach(checkAnchor);
    for(const c of o.calls)[c.call,c.receiver,...c.declarations].forEach(checkAnchor);
    if(o.nested.calls.length>value.limits.nested_calls)throw Error("invalid_packet");
    o.nested.barriers.forEach(b=>checkAnchor(b.scope));
    for(const c of o.nested.calls) {
      const b=c.binding;
      [c.call,c.receiver,...c.declarations,...c.functions,b.use,b.parameter,...b.declarations,...b.writes].forEach(checkAnchor);
      if(!c.functions.length || c.functions.length>value.limits.nested_depth
        || (b.status==="linked" ? b.reason!==null || !b.use || !b.parameter || b.declarations.length!==1 || b.writes.length : !b.reason))throw Error("invalid_packet");
      const k=c.props_class;
      [...k.signatures,...k.props_declarations,...k.property_declarations,...k.declared_type_declarations,k.class_declaration].forEach(checkAnchor);
      if(k.class_declaration && k.class_declaration.kind!=="ClassDeclaration")throw Error("invalid_packet");
      k.instantiation.forEach(i=>[i.parameter,...i.argument_declarations].forEach(checkAnchor));
      if(k.instantiation.length>value.limits.props_type_args || (k.status==="observed"
        ? k.reason!==null || !k.class_declaration || !k.property_name || k.property_declarations.length!==1
          || k.signatures.length!==1 || !k.props_type || !k.property_type || b.status!=="linked"
          || o.explicit_parameter || o.provenance.status!=="traced" || value.status!=="observed"
        : !k.reason))throw Error("invalid_packet");
    }
    const p=o.provenance;
    if(p.steps_used>value.limits.provenance_steps || (p.status==="traced" ? p.reason!==null || !p.terminal : !p.reason || p.terminal!==null))throw Error("invalid_packet");
    checkAnchor(p.terminal);
    const checkAlias=a=>[a.module,...a.declarations,...a.target,...a.module_declarations,...a.module_exports,...a.module_bindings].forEach(checkAnchor);
    for(const h of p.hops) {
      [h.reference,...h.type_arguments,...h.type_parameters,...h.declarations].forEach(checkAnchor);
      h.aliases.forEach(checkAlias);
      for(const q of h.qualifiers){[q.use,...q.declarations].forEach(checkAnchor);q.aliases.forEach(checkAlias);}
    }
  }
  if(value.schema===SCHEMA18||value.schema===SCHEMA) {
    const configObserved=value.config_provenance.status==='observed';
    const noResolveOption=configObserved&&value.config_provenance.options.find(row=>row.name==='noResolve');
    const classify=value.schema===SCHEMA18?classifySemanticClosure:classifySemanticClosureV2;
    const expected=classify({compilerVerified:value.compiler.verified,
      stableSnapshot:value.closure.stable_snapshot,configObserved,entryComplete:value.entry_obligations.complete,
      noResolve:noResolveOption?.present===true
        &&noResolveOption.value_sha256===hash(canonical({present:true,value:true})),
      diagnosticCount:value.diagnostics.length,globalReasons:value.reasons,
      outside:value.snapshot.outside_lookups,refusedCount:value.snapshot.refused_lookup_sha256.length,
      boundaryCount:value.search_provenance.boundary_events.length,programFiles:value.snapshot.program_files,
      resolutions:value.resolutions});
    if(canonical(value.semantic_closure)!==canonical(expected))throw Error('invalid_packet');
  }
  return value;
}
