import test from 'node:test';
import assert from 'node:assert/strict';
import {cpSync,mkdtempSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {classifySemanticClosure} from './semantic-closure.mjs';
import {classifySemanticClosureV2,SEMANTIC_CLOSURE_POLICY_V2} from './semantic-closure-v2.mjs';

const lookup=(status='unproven',wildcard=null,merged_wildcard=null)=>({status,wildcard,merged_wildcard});
const exact=()=>({target:null,lookup:lookup('observed')});
const unresolved=()=>({target:null,lookup:lookup()});
const wildcard=()=>({target:null,lookup:lookup('unproven',{status:'observed'},null)});
const merged=()=>({target:null,lookup:lookup('unproven',{status:'unproven'},{status:'observed'})});
const selected=target=>({target,lookup:lookup()});
function base(overrides={}) {
  return {compilerVerified:true,stableSnapshot:true,configObserved:true,entryComplete:true,
    noResolve:false,diagnosticCount:0,globalReasons:[],outside:false,refusedCount:0,
    boundaryCount:0,programFiles:[],resolutions:[],...overrides};
}
const classify=overrides=>classifySemanticClosureV2(base(overrides));

test('v2 promotes only the v1 unadmitted singleton wildcard row',()=>{
  const input=base({globalReasons:['unresolved_module'],resolutions:[wildcard()]});
  assert.deepEqual(classifySemanticClosure(input).rows,[{index:0,disposition:'unproven',reason:'unadmitted_binding'}]);
  assert.deepEqual(classifySemanticClosureV2(input),{policy:SEMANTIC_CLOSURE_POLICY_V2,complete:true,reasons:[],
    rows:[{index:0,disposition:'singleton_wildcard',reason:null}]});
});

test('row order and every non-singleton disposition remain v1-owned',()=>{
  const resolutions=[selected('project/a.ts'),selected('project/missing.ts'),exact(),wildcard(),merged(),unresolved()];
  const result=classify({programFiles:['project/a.ts'],globalReasons:['unresolved_module'],resolutions});
  assert.deepEqual(result.rows,[
    {index:0,disposition:'filesystem_selected',reason:null},
    {index:1,disposition:'unproven',reason:'target_not_in_program'},
    {index:2,disposition:'exact_ambient',reason:null},
    {index:3,disposition:'singleton_wildcard',reason:null},
    {index:4,disposition:'unproven',reason:'unadmitted_binding'},
    {index:5,disposition:'unproven',reason:'unresolved_module'},
  ]);
  assert.deepEqual(result.reasons,['resolution_unproven']);assert.equal(result.complete,false);
});

test('promotion recomputes only resolution_unproven and preserves every global aggregate',()=>{
  const input=base({compilerVerified:false,stableSnapshot:false,configObserved:false,entryComplete:false,
    diagnosticCount:1,globalReasons:['worker_failed','compiler_diagnostics','unsupported_references',
      'unsupported_plugins','unproven_path_reference','unproven_type_lib_reference','outside_lookup',
      'unsupported_lookup','unresolved_module'],outside:true,refusedCount:1,boundaryCount:1,
    resolutions:[wildcard()]});
  const before=classifySemanticClosure(input),after=classifySemanticClosureV2(input);
  assert.deepEqual(after.reasons,before.reasons.filter(reason=>reason!=='resolution_unproven'));
  assert.equal(after.complete,false);assert.equal(after.rows[0].disposition,'singleton_wildcard');
});

test('duplicates are retained per occurrence without mutation',()=>{
  const one=wildcard(),input=base({globalReasons:['unresolved_module'],resolutions:[one,one]});
  const saved=structuredClone(input),result=classifySemanticClosureV2(input);
  assert.deepEqual(result.rows.map(row=>[row.index,row.disposition]),[[0,'singleton_wildcard'],[1,'singleton_wildcard']]);
  assert.deepEqual(input,saved);
});

test('one unrelated unresolved row prevents a promoted row from completing the set',()=>{
  const result=classify({globalReasons:['unresolved_module'],resolutions:[wildcard(),unresolved()]});
  assert.deepEqual(result.rows,[{index:0,disposition:'singleton_wildcard',reason:null},
    {index:1,disposition:'unproven',reason:'unresolved_module'}]);
  assert.deepEqual(result.reasons,['resolution_unproven']);assert.equal(result.complete,false);
});

test('v1 validation and cap refusal happen before v2 row access',()=>{
  assert.throws(()=>classifySemanticClosureV2(base(),0),/invalid_semantic_closure/);
  assert.throws(()=>classifySemanticClosureV2(base(),100001),/invalid_semantic_closure/);
  const rows=new Array(2);Object.defineProperty(rows,0,{get(){throw Error('row_was_read');}});
  assert.throws(()=>classifySemanticClosureV2(base({resolutions:rows}),1),error=>error.message==='invalid_semantic_closure');
  assert.throws(()=>classify({globalReasons:[],resolutions:[wildcard()]}),/invalid_semantic_closure/);
});

test('v2 remains portable with only its explicit local v1 dependency',async()=>{
  const source=readFileSync(new URL('./semantic-closure-v2.mjs',import.meta.url),'utf8');
  assert(!source.includes('node:'));assert.match(source,/from '\.\/semantic-closure\.mjs'/);
  const root=mkdtempSync(path.join(tmpdir(),'prism-s8-portable-'));
  try {
    cpSync(new URL('./semantic-closure.mjs',import.meta.url),path.join(root,'semantic-closure.mjs'));
    cpSync(new URL('./semantic-closure-v2.mjs',import.meta.url),path.join(root,'semantic-closure-v2.mjs'));
    const copied=await import(pathToFileURL(path.join(root,'semantic-closure-v2.mjs')).href+'?portable');
    assert.deepEqual(copied.classifySemanticClosureV2(base({globalReasons:['unresolved_module'],resolutions:[wildcard()]})),
      classifySemanticClosureV2(base({globalReasons:['unresolved_module'],resolutions:[wildcard()]})));
  } finally {rmSync(root,{recursive:true,force:true});}
});
