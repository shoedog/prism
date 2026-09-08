// Fixed-public packet controls; no app files are modified.
// node verify-entry-audit-controls.mjs <task-root>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {reconcileEntries} from './audit-entry-observations.mjs';
const task=process.argv[2];assert(task);const json=f=>JSON.parse(fs.readFileSync(path.join(task,f)));
const p=json('public-current.json'),base=json('public-base.json'),cache=json('entry-cache.json'),captured=json('public-capture.json');
assert.equal(reconcileEntries(p,base,cache,captured).counts.observed,5);
const controls=[{name:'actual fixed packet and entry cache',result:'accepted'}];
for(const [name,mutate] of [
  ['omitted entry',q=>q.type_lib_entries.pop()],
  ['two genuine targets substituted',q=>q.type_lib_entries[0].target=q.type_lib_entries[1].target],
  ['configured origin forged as default',q=>q.type_lib_entries[0].origin='default'],
  ['entry inclusion erased',q=>q.type_lib_entries[0].inclusion=false],
  ['react-scripts refusal erased',q=>{q.type_lib_references=q.type_lib_references.filter(r=>r.name!=='react-scripts');q.reasons=q.reasons.filter(r=>r!=='unproven_type_lib_reference');}],
]){const q=structuredClone(p);mutate(q);assert.throws(()=>reconcileEntries(q,base,cache,captured),assert.AssertionError);controls.push({name,result:'rejected'});}
const changedCache=structuredClone(cache);changedCache.entry_inclusions[0].reason.typeReference='other';assert.throws(()=>reconcileEntries(p,base,changedCache,captured),assert.AssertionError);controls.push({name:'borrowed type inclusion name',result:'rejected'});
console.log(JSON.stringify({controls,passed:controls.length,failures:[]},null,2));
