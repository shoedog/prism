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
