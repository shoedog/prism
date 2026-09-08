import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {classifySemanticClosureV3,SEMANTIC_CLOSURE_POLICY_V3} from './semantic-closure-v3.mjs';
import {hash} from './schema.mjs';

const lookup=({status='unproven',wildcard=null,merged_wildcard=null}={})=>({status,wildcard,merged_wildcard});
const base=(overrides={})=>({compilerVerified:true,stableSnapshot:true,configObserved:true,entryComplete:true,
  noResolve:false,diagnosticCount:0,globalReasons:['unresolved_module'],outside:false,refusedCount:0,boundaryCount:0,
  programFiles:['project/src/app.ts'],resolutions:[{target:null,lookup:lookup()}],...overrides});
const merged={status:'observed',reason:null,declarations:[{shape:'empty_block'},{shape:'shorthand'}],selected:{shape:'empty_block'}};

test('v3 promotes only the v2-unadmitted observed merged null row',()=>{
  const result=classifySemanticClosureV3(base({resolutions:[{target:null,lookup:lookup({merged_wildcard:merged})}]}));
  assert.deepEqual(result,{policy:SEMANTIC_CLOSURE_POLICY_V3,complete:true,reasons:[],rows:[{index:0,disposition:'merged_side_effect',reason:null}]});
});

test('v3 leaves target, wildcard, and unobserved merged lanes outside its promotion',()=>{
  const result=classifySemanticClosureV3(base({resolutions:[
    {target:'project/src/app.ts',lookup:lookup({merged_wildcard:merged})},
    {target:null,lookup:lookup({wildcard:{status:'observed'},merged_wildcard:null})},
    {target:null,lookup:lookup({merged_wildcard:{...merged,status:'unproven',reason:'augmentation'}})},
  ]}));
  assert.deepEqual(result.rows,[
    {index:0,disposition:'filesystem_selected',reason:null},
    {index:1,disposition:'singleton_wildcard',reason:null},
    {index:2,disposition:'unproven',reason:'unresolved_module'},
  ]);
  assert.deepEqual(result.reasons,['resolution_unproven']);assert.equal(result.complete,false);
});

test('v3 recomputes only resolution_unproven and retains inherited barriers/order',()=>{
  const result=classifySemanticClosureV3(base({outside:true,boundaryCount:1,
    resolutions:[{target:null,lookup:lookup({merged_wildcard:merged})},{target:null,lookup:lookup()}]}));
  assert.deepEqual(result.rows,[{index:0,disposition:'merged_side_effect',reason:null},{index:1,disposition:'unproven',reason:'unresolved_module'}]);
  assert.deepEqual(result.reasons,['boundary_encounter','resolution_unproven']);assert.equal(result.complete,false);
});

test('v1 and v2 source bytes remain frozen',()=>{
  assert.equal(hash(readFileSync(new URL('./semantic-closure.mjs',import.meta.url))),'1ac30091a62bc03e267f3ff55c906a5e26db06dfc937752dfcabf6bbeb20e4b4');
  assert.equal(hash(readFileSync(new URL('./semantic-closure-v2.mjs',import.meta.url))),'82c55f11d4e0a6e10a5ae7ca0fc379a1177c75f00995a04eed9b42a3685e651c');
});
