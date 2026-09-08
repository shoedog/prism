export const SEMANTIC_CLOSURE_POLICY='prism.semantic-closure/exact-ambient-v1';
const LIMIT=100000;
const GLOBAL_REASONS=new Set([
  'budget_exceeded','unsupported_input','compiler_mismatch','unstable_snapshot',
  'compiler_diagnostics','unsupported_references','unsupported_plugins','outside_lookup',
  'unsupported_lookup','unresolved_module','unproven_path_reference',
  'unproven_type_lib_reference','invalid_config','worker_failed'
]);
const CATEGORIZED_GLOBAL=new Set([
  'compiler_mismatch','unstable_snapshot','compiler_diagnostics','unsupported_references',
  'unsupported_plugins','outside_lookup','unsupported_lookup','unproven_path_reference',
  'unproven_type_lib_reference','unresolved_module'
]);
const AGGREGATE_ORDER=[
  'compiler_unverified','unstable_snapshot','config_unproven',
  'entry_obligations_incomplete','no_resolve','compiler_diagnostics',
  'unsupported_project_references','unsupported_plugins','required_path_unproven',
  'type_lib_unproven','boundary_encounter','global_refusal','resolution_unproven'
];

function invalid(){throw Error('invalid_semantic_closure');}
const count=value=>Number.isSafeInteger(value)&&value>=0;
const record=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
function lane(value){return value===null||record(value)&&['observed','unproven'].includes(value.status);}
function validResolution(value){
  return record(value)&&(value.target===null||typeof value.target==='string')&&record(value.lookup)
    &&['observed','unproven'].includes(value.lookup.status)
    &&Object.hasOwn(value.lookup,'wildcard')&&lane(value.lookup.wildcard)
    &&Object.hasOwn(value.lookup,'merged_wildcard')&&lane(value.lookup.merged_wildcard);
}

// Pure classification over facts already authenticated by the packet parser and
// full reproduction. It does not recreate compiler binding or filesystem proof.
export function classifySemanticClosure(input,cap=LIMIT){
  if(!record(input)||!Number.isSafeInteger(cap)||cap<1||cap>LIMIT
    ||typeof input.compilerVerified!=='boolean'||typeof input.stableSnapshot!=='boolean'
    ||typeof input.configObserved!=='boolean'||typeof input.entryComplete!=='boolean'
    ||typeof input.noResolve!=='boolean'||!count(input.diagnosticCount)
    ||typeof input.outside!=='boolean'||!count(input.refusedCount)||!count(input.boundaryCount)
    ||!Array.isArray(input.globalReasons)||input.globalReasons.length>LIMIT
    ||!Array.isArray(input.programFiles)||input.programFiles.length>LIMIT
    ||!Array.isArray(input.resolutions)||input.resolutions.length>cap)invalid();
  if(!input.globalReasons.every(reason=>typeof reason==='string'&&GLOBAL_REASONS.has(reason))
    ||!input.programFiles.every(file=>typeof file==='string')
    ||!input.resolutions.every(validResolution))invalid();

  const globals=new Set(input.globalReasons);
  const hasNullTarget=input.resolutions.some(resolution=>resolution.target===null);
  if(globals.has('unresolved_module')!==hasNullTarget)invalid();

  const program=new Set(input.programFiles);
  const rows=input.resolutions.map((resolution,index)=>{
    if(resolution.target!==null)return program.has(resolution.target)
      ?{index,disposition:'filesystem_selected',reason:null}
      :{index,disposition:'unproven',reason:'target_not_in_program'};
    if(resolution.lookup.status==='observed')return {index,disposition:'exact_ambient',reason:null};
    if(resolution.lookup.wildcard?.status==='observed'||resolution.lookup.merged_wildcard?.status==='observed')
      return {index,disposition:'unproven',reason:'unadmitted_binding'};
    return {index,disposition:'unproven',reason:'unresolved_module'};
  });

  const found=new Set();
  if(!input.compilerVerified||globals.has('compiler_mismatch'))found.add('compiler_unverified');
  if(!input.stableSnapshot||globals.has('unstable_snapshot'))found.add('unstable_snapshot');
  if(!input.configObserved)found.add('config_unproven');
  if(!input.entryComplete)found.add('entry_obligations_incomplete');
  if(input.configObserved&&input.noResolve)found.add('no_resolve');
  if(input.diagnosticCount>0||globals.has('compiler_diagnostics'))found.add('compiler_diagnostics');
  if(globals.has('unsupported_references'))found.add('unsupported_project_references');
  if(globals.has('unsupported_plugins'))found.add('unsupported_plugins');
  if(globals.has('unproven_path_reference'))found.add('required_path_unproven');
  if(globals.has('unproven_type_lib_reference'))found.add('type_lib_unproven');
  if(input.outside||input.refusedCount>0||input.boundaryCount>0
    ||globals.has('outside_lookup')||globals.has('unsupported_lookup'))found.add('boundary_encounter');
  if(input.globalReasons.some(reason=>reason!=='unresolved_module'&&!CATEGORIZED_GLOBAL.has(reason)))found.add('global_refusal');
  if(rows.some(row=>row.disposition==='unproven'))found.add('resolution_unproven');
  const reasons=AGGREGATE_ORDER.filter(reason=>found.has(reason));
  return {policy:SEMANTIC_CLOSURE_POLICY,complete:reasons.length===0,reasons,rows};
}
