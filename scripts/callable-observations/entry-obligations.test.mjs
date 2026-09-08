import test from 'node:test';
import assert from 'node:assert/strict';
import {classifyEntryObligations} from './entry-obligations.mjs';

const selected={kind:'types',origin:'configured',index:0,status:'observed',reason:null};
const unprocessedType={kind:'types',origin:'configured',index:1,status:'unproven',reason:'unprocessed'};
const unprocessedLib={kind:'lib',origin:'configured',index:2,status:'unproven',reason:'unprocessed'};
const input=overrides=>({entries:[],rootCount:1,configObserved:true,noLib:false,explicitTypes:true,...overrides});

test('selected rows preserve order and cannot be disabled by contradictory predicates',()=>{
  const result=classifyEntryObligations(input({rootCount:0,noLib:true,entries:[selected]}));
  assert.deepEqual(result,{complete:true,reasons:[],rows:[{kind:'types',origin:'configured',index:0,disposition:'selected',reason:null}]});
});

test('no roots disables only old unprocessed type and lib rows',()=>{
  const automatic={kind:'types',origin:'automatic',index:2,status:'unproven',reason:'unprocessed'};
  const defaultLib={kind:'lib',origin:'default',index:0,status:'unproven',reason:'unprocessed'};
  const result=classifyEntryObligations(input({rootCount:0,entries:[unprocessedType,automatic,unprocessedLib,defaultLib,{kind:'lib',origin:'configured',index:3,status:'unproven',reason:'missing_inclusion'}]}));
  assert.deepEqual(result.rows.map(row=>[row.kind,row.origin,row.index,row.disposition,row.reason]),[
    ['types','configured',1,'disabled','no_roots'],['types','automatic',2,'disabled','no_roots'],
    ['lib','configured',2,'disabled','no_roots'],['lib','default',0,'disabled','no_roots'],
    ['lib','configured',3,'unproven','missing_inclusion'],
  ]);
  assert.deepEqual(result.reasons,['unproven_entry']);
});

test('proven noLib disables only unprocessed lib rows, never type rows',()=>{
  const result=classifyEntryObligations(input({noLib:true,entries:[unprocessedType,unprocessedLib]}));
  assert.deepEqual(result.rows.map(row=>[row.disposition,row.reason]),[['unproven','unprocessed'],['disabled','no_lib']]);
  assert.deepEqual(result.reasons,['unproven_entry']);
});

test('unobserved config cannot manufacture noLib and unprocessed libs remain suppression-unproven',()=>{
  const result=classifyEntryObligations(input({configObserved:false,noLib:true,explicitTypes:false,entries:[unprocessedLib]}));
  assert.deepEqual(result.rows,[{kind:'lib',origin:'configured',index:2,disposition:'unproven',reason:'suppression_unproven'}]);
  assert.deepEqual(result.reasons,['configuration_unproven','unproven_entry']);
});

test('automatic discovery remains unproven without explicit types even with no entries',()=>{
  assert.deepEqual(classifyEntryObligations(input({explicitTypes:false})),{complete:false,reasons:['automatic_discovery_unproven'],rows:[]});
  assert.deepEqual(classifyEntryObligations(input({explicitTypes:true,entries:[]})),{complete:true,reasons:[],rows:[]});
});

test('duplicates, empty explicit arrays and old failure reasons retain their exact rows',()=>{
  const duplicate={kind:'lib',origin:'configured',index:3,status:'unproven',reason:'unresolved'};
  const duplicateOccurrence={...duplicate,index:4};
  const result=classifyEntryObligations(input({entries:[duplicate,duplicateOccurrence,{kind:'types',origin:'automatic',index:5,status:'unproven',reason:'target_not_in_program'}]}));
  assert.deepEqual(result.rows.map(row=>[row.origin,row.index,row.disposition,row.reason]),[
    ['configured',3,'unproven','unresolved'],['configured',4,'unproven','unresolved'],['automatic',5,'unproven','target_not_in_program'],
  ]);
  assert.deepEqual(result.reasons,['unproven_entry']);
});

test('reasons are deterministic configuration, automatic universe, then row completeness',()=>{
  const result=classifyEntryObligations(input({configObserved:false,explicitTypes:false,entries:[unprocessedType]}));
  assert.deepEqual(result.reasons,['configuration_unproven','unproven_entry']);
  const observed=classifyEntryObligations(input({explicitTypes:false,entries:[unprocessedType]}));
  assert.deepEqual(observed.reasons,['automatic_discovery_unproven','unproven_entry']);
});

test('cap rejects before building rows and malformed inputs fail closed',()=>{
  assert.throws(()=>classifyEntryObligations(input({entries:[selected,unprocessedType]}),1),/invalid_entry_obligations/);
  assert.deepEqual(classifyEntryObligations(input({entries:[selected]}),1).rows.length,1);
  for(const bad of [null,{},input({rootCount:-1}),input({rootCount:1.5}),input({configObserved:null}),input({entries:[{...selected,reason:'unresolved'}]}),input({entries:[{...unprocessedLib,origin:'automatic'}]})]) {
    assert.throws(()=>classifyEntryObligations(bad),/invalid_entry_obligations/);
  }
  assert.throws(()=>classifyEntryObligations(input(),0),/invalid_entry_obligations/);
});
