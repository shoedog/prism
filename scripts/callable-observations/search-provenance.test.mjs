import test from 'node:test';
import assert from 'node:assert/strict';
import {createSearchProvenance,classifyBoundary} from './search-provenance.mjs';

test('boundary classifier normalizes admitted, refused, outside and unsafe paths',()=>{
  assert.deepEqual(classifyBoundary('/__prism__/project/src/a.ts/'),{kind:'in_root',normalized:'/__prism__/project/src/a.ts',id:'project/src/a.ts'});
  assert.equal(classifyBoundary('/__prism__/project/virtual:foo').kind,'refused');
  assert.equal(classifyBoundary('/outside/a.ts').kind,'outside');
  assert.equal(classifyBoundary('/__prism__/project/.git/config').kind,'unsafe');
});

test('nested request contexts restore ownership after nested throw',()=>{
  const ledger=createSearchProvenance(s=>`h:${s}`);
  ledger.request({id:0});ledger.request({id:1});
  ledger.withRequest({id:0},()=>{
    ledger.boundary('outside','fileExists','outer');
    assert.throws(()=>ledger.withRequest({id:1},()=>{ledger.boundary('refused','readFile','inner');throw Error('expected');}),/expected/);
    ledger.boundary('outside','realpath','outer-again');
  });
  ledger.boundary('refused','identity','unattributed');
  assert.deepEqual(ledger.boundary_events.map(e=>e.owner?.id??null),[0,1,0,null]);
  assert.throws(()=>ledger.withRequest({id:0},()=>{throw Error('outer failure');}),/outer failure/);
  ledger.boundary('outside','identity','after-outer-throw');assert.equal(ledger.boundary_events.at(-1).owner,null);
  ledger.withRequest({id:1},()=>{const resolved=(()=>'/external/target')();ledger.boundary('outside','identity',resolved);});
  assert.deepEqual(ledger.boundary_events.at(-1).owner,{channel:'module',id:1});
});

test('independent ledgers cap module and boundary populations fail closed',()=>{
  const ledger=createSearchProvenance(s=>s,1);
  assert.equal(ledger.claimRequest(),0);assert.throws(()=>ledger.claimRequest(),/budget_exceeded/);
  ledger.request({id:0});assert.throws(()=>ledger.request({id:1}),/budget_exceeded/);
  ledger.boundary('outside','identity','a');assert.throws(()=>ledger.boundary('outside','identity','b'),/budget_exceeded/);
});

test('type request and execution caps are independent and type contexts restore',()=>{
  const ledger=createSearchProvenance(s=>s,1),search={id:ledger.claimTypeSearch()};
  assert.throws(()=>ledger.claimTypeSearch(),/budget_exceeded/);assert.equal(ledger.claimTypeRequest(),0);assert.throws(()=>ledger.claimTypeRequest(),/budget_exceeded/);
  ledger.typeRequest({id:0});assert.throws(()=>ledger.typeRequest({id:1}),/budget_exceeded/);
  ledger.typeSearch(search);assert.throws(()=>ledger.typeSearch({id:1}),/budget_exceeded/);
  assert.throws(()=>ledger.withTypeSearch(search,()=>{throw Error('type failure');}),/type failure/);
  const other=createSearchProvenance(s=>s);other.withTypeSearch({id:0},()=>other.boundary('outside','identity','owned'));
  other.boundary('outside','identity','unowned');assert.deepEqual(other.boundary_events.map(e=>e.owner),[{channel:'type',id:0},null]);
});

test('type batch claims and records have an independent cap',()=>{
  const ledger=createSearchProvenance(s=>s,1);
  assert.equal(ledger.claimTypeBatch(),0);assert.throws(()=>ledger.claimTypeBatch(),/budget_exceeded/);
  assert.equal(ledger.claimTypeRequest(),0);assert.equal(ledger.claimTypeSearch(),0);
  ledger.typeBatch({id:0});assert.throws(()=>ledger.typeBatch({id:1}),/budget_exceeded/);
});

test('lib execution and beneficiary caps are independent and nested owners restore',()=>{
  const ledger=createSearchProvenance(s=>s,1),lib={id:ledger.claimLibSearch()};
  assert.throws(()=>ledger.claimLibSearch(),/budget_exceeded/);
  assert.equal(ledger.claimRequest(),0);assert.equal(ledger.claimTypeSearch(),0);
  ledger.libSearch({...lib,beneficiaries:[{index:0}]});
  assert.throws(()=>ledger.libSearch({id:1,beneficiaries:[]}),/budget_exceeded/);
  const each=createSearchProvenance(s=>s,1);
  assert.throws(()=>each.libSearch({id:0,beneficiaries:[{index:0},{index:1}]}),/budget_exceeded/);
  const relationships=createSearchProvenance(s=>s,2);
  relationships.libSearch({id:0,beneficiaries:[{index:0},{index:1}]});
  assert.throws(()=>relationships.libSearch({id:1,beneficiaries:[{index:2}]}),/budget_exceeded/);
  const preappend=createSearchProvenance(s=>s,2),beneficiaries=[];
  assert.throws(()=>preappend.libBeneficiaries(beneficiaries,[{index:0},{index:1},{index:2}]),/budget_exceeded/);
  assert.deepEqual(beneficiaries,[]);
  preappend.libBeneficiaries(beneficiaries,[{index:0},{index:1}]);assert.equal(beneficiaries.length,2);
  const other=[];assert.throws(()=>preappend.libBeneficiaries(other,[{index:2}]),/budget_exceeded/);assert.deepEqual(other,[]);
  const nested=createSearchProvenance(s=>s),module={id:0},type={id:0},search={id:0};
  nested.withRequest(module,()=>nested.withTypeSearch(type,()=>{
    assert.throws(()=>nested.withLibSearch(search,()=>{nested.boundary('outside','identity','lib');throw Error('lib failure');}),/lib failure/);
    nested.boundary('outside','identity','type');
  }));
  nested.boundary('outside','identity','none');
  assert.deepEqual(nested.boundary_events.map(e=>e.owner),[{channel:'lib',id:0},{channel:'type',id:0},null]);
});
