import test from 'node:test';
import assert from 'node:assert/strict';
import {classifySemanticClosure,SEMANTIC_CLOSURE_POLICY} from './semantic-closure.mjs';

const lookup=(status='unproven',wildcard=null,merged_wildcard=null)=>({status,wildcard,merged_wildcard});
const exact=()=>({target:null,lookup:lookup('observed')});
const unresolved=()=>({target:null,lookup:lookup()});
const wildcard=()=>({target:null,lookup:lookup('unproven',{status:'observed'},null)});
const merged=()=>({target:null,lookup:lookup('unproven',null,{status:'observed'})});
const selected=target=>({target,lookup:lookup()});
function base(overrides={}){
  return {compilerVerified:true,stableSnapshot:true,configObserved:true,entryComplete:true,
    noResolve:false,diagnosticCount:0,globalReasons:[],outside:false,refusedCount:0,
    boundaryCount:0,programFiles:[],resolutions:[],...overrides};
}
const classify=overrides=>classifySemanticClosure(base(overrides));
const reason=(name,overrides={})=>assert.deepEqual(classify(overrides).reasons,[name]);

test('fixed policy admits exact ambient while retaining old unresolved reason as per-row input',()=>{
  assert.deepEqual(classify({globalReasons:['unresolved_module'],resolutions:[exact()]}),{
    policy:SEMANTIC_CLOSURE_POLICY,complete:true,reasons:[],
    rows:[{index:0,disposition:'exact_ambient',reason:null}]
  });
});

test('row precedence covers selected, absent target, exact, wildcard, merged, and unresolved in input order',()=>{
  const resolutions=[selected('project/a.ts'),selected('project/missing.ts'),exact(),wildcard(),merged(),unresolved()];
  const result=classify({programFiles:['project/a.ts'],globalReasons:['unresolved_module'],resolutions});
  assert.deepEqual(result.rows,[
    {index:0,disposition:'filesystem_selected',reason:null},
    {index:1,disposition:'unproven',reason:'target_not_in_program'},
    {index:2,disposition:'exact_ambient',reason:null},
    {index:3,disposition:'unproven',reason:'unadmitted_binding'},
    {index:4,disposition:'unproven',reason:'unadmitted_binding'},
    {index:5,disposition:'unproven',reason:'unresolved_module'}
  ]);
  assert.deepEqual(result.reasons,['resolution_unproven']);assert.equal(result.complete,false);
});

test('nonnull target precedence ignores semantic lane status',()=>{
  const resolution={target:'project/a.ts',lookup:lookup('observed',{status:'observed'},{status:'observed'})};
  assert.deepEqual(classify({programFiles:['project/a.ts'],resolutions:[resolution]}).rows,
    [{index:0,disposition:'filesystem_selected',reason:null}]);
});

test('duplicate occurrences are retained and indexed without deduplication',()=>{
  const one=exact(),result=classify({globalReasons:['unresolved_module'],resolutions:[one,one,selected('project/a.ts')],programFiles:['project/a.ts']});
  assert.deepEqual(result.rows.map(row=>[row.index,row.disposition]),[[0,'exact_ambient'],[1,'exact_ambient'],[2,'filesystem_selected']]);
  assert.equal(result.complete,true);
});

test('one unrelated unresolved row blocks an otherwise exact population',()=>{
  const result=classify({globalReasons:['unresolved_module'],resolutions:[exact(),unresolved()]});
  assert.deepEqual(result.reasons,['resolution_unproven']);assert.equal(result.rows[1].reason,'unresolved_module');
});

test('explicit evidence booleans and counts map to each ordered aggregate reason',()=>{
  reason('compiler_unverified',{compilerVerified:false});
  reason('unstable_snapshot',{stableSnapshot:false});
  reason('config_unproven',{configObserved:false});
  reason('entry_obligations_incomplete',{entryComplete:false});
  reason('no_resolve',{noResolve:true});
  reason('compiler_diagnostics',{diagnosticCount:1});
  reason('boundary_encounter',{outside:true});
  reason('boundary_encounter',{refusedCount:1});
  reason('boundary_encounter',{boundaryCount:1});
});

test('noResolve is not separately inferred when configuration is unobserved',()=>{
  assert.deepEqual(classify({configObserved:false,noResolve:true}).reasons,['config_unproven']);
});

test('categorized legacy reasons map without an extra global refusal',()=>{
  const cases=[
    ['compiler_mismatch','compiler_unverified'],['unstable_snapshot','unstable_snapshot'],
    ['compiler_diagnostics','compiler_diagnostics'],['unsupported_references','unsupported_project_references'],
    ['unsupported_plugins','unsupported_plugins'],['unproven_path_reference','required_path_unproven'],
    ['unproven_type_lib_reference','type_lib_unproven'],['outside_lookup','boundary_encounter'],
    ['unsupported_lookup','boundary_encounter']
  ];
  for(const [old,aggregate] of cases)reason(aggregate,{globalReasons:[old]});
});

test('all other known legacy refusals map to global_refusal',()=>{
  for(const old of ['budget_exceeded','unsupported_input','invalid_config','worker_failed'])
    reason('global_refusal',{globalReasons:[old]});
});

test('aggregate reasons are deduplicated and returned in fixed order',()=>{
  const input=base({compilerVerified:false,stableSnapshot:false,entryComplete:false,noResolve:true,diagnosticCount:2,
    globalReasons:['worker_failed','compiler_diagnostics','unsupported_references','unsupported_plugins',
      'unproven_path_reference','unproven_type_lib_reference','outside_lookup','unsupported_lookup','unresolved_module'],
    outside:true,refusedCount:2,boundaryCount:3,resolutions:[unresolved()]});
  assert.deepEqual(classifySemanticClosure(input).reasons,[
    'compiler_unverified','unstable_snapshot','entry_obligations_incomplete','no_resolve',
    'compiler_diagnostics','unsupported_project_references','unsupported_plugins',
    'required_path_unproven','type_lib_unproven','boundary_encounter','global_refusal',
    'resolution_unproven'
  ]);
  assert.deepEqual(classify({compilerVerified:false,stableSnapshot:false,configObserved:false,
    entryComplete:false}).reasons,
    ['compiler_unverified','unstable_snapshot','config_unproven','entry_obligations_incomplete']);
});

test('qualifying rows never clear outside, refused, or ownerless boundary counts',()=>{
  for(const change of [{outside:true},{refusedCount:1},{boundaryCount:1}]){
    const result=classify({globalReasons:['unresolved_module'],resolutions:[exact()],...change});
    assert.equal(result.rows[0].disposition,'exact_ambient');assert.deepEqual(result.reasons,['boundary_encounter']);
  }
});

test('eventless unresolved cache-like row stays unproven',()=>{
  const result=classify({globalReasons:['unresolved_module'],resolutions:[unresolved()],outside:false,refusedCount:0,boundaryCount:0});
  assert.deepEqual(result.reasons,['resolution_unproven']);assert.equal(result.complete,false);
});

test('unresolved_module presence is exactly consistent with null targets',()=>{
  assert.throws(()=>classify({globalReasons:['unresolved_module'],programFiles:['project/a.ts'],resolutions:[selected('project/a.ts')]}),/invalid_semantic_closure/);
  assert.throws(()=>classify({globalReasons:[],resolutions:[exact()]}),/invalid_semantic_closure/);
});

test('cap is validated and checked before any resolution row is read',()=>{
  assert.throws(()=>classifySemanticClosure(base(),0),/invalid_semantic_closure/);
  assert.throws(()=>classifySemanticClosure(base(),100001),/invalid_semantic_closure/);
  const rows=new Array(2);Object.defineProperty(rows,0,{get(){throw Error('row_was_read');}});
  assert.throws(()=>classifySemanticClosure(base({resolutions:rows}),1),error=>error.message==='invalid_semantic_closure');
});

test('invalid input shapes, counts, reasons, and minimum lookup facts reject',()=>{
  for(const input of [null,[],base({compilerVerified:1}),base({diagnosticCount:-1}),base({refusedCount:1.5}),
    base({programFiles:[1]}),base({globalReasons:['new_unknown_reason']}),base({resolutions:[{target:null,lookup:{status:'observed'}}]}),
    base({resolutions:[{target:false,lookup:lookup()}]})])assert.throws(()=>classifySemanticClosure(input),/invalid_semantic_closure/);
});
