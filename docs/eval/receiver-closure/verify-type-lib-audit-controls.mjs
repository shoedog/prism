// Fixed-source audit controls; mutations are disposable packets, never app files.
// node verify-type-lib-audit-controls.mjs <compiler> <source> <task-root>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
const [compiler,root,task]=process.argv.slice(2);assert(compiler&&root&&task);
const audit=fileURLToPath(new URL('./audit-type-lib-observations.mjs',import.meta.url));
const previous='/private/tmp/prism-required-path-o41JkY/packet.json',cache='/private/tmp/prism-required-path-o41JkY/reference-capture-aah3aa/reference-cache.json';
const dir=fs.mkdtempSync(path.join(task,'audit-controls-')),records=[];
try{const packet=JSON.parse(fs.readFileSync(path.join(task,'packet.json'))),manifest=path.join(task,'source-before.json');
  function run(name,file,expected){const r=spawnSync(process.execPath,[audit,compiler,root,file,previous,cache,manifest],{encoding:'utf8',maxBuffer:2*1024*1024});assert.equal(r.status,expected,r.stderr);
    if(!expected){const p=JSON.parse(r.stdout);assert.deepEqual(p.source_directives,{types:27,lib:112,total:139,observed:138,unproven:1});}
    else assert.match(r.stderr,/AssertionError/);records.push({name,status:r.status,stderr:r.stderr});}
  run('unchanged actual packet and pinned source/cache',path.join(task,'packet.json'),0);
  for(const [name,mutate] of [
    ['omitted occurrence',p=>p.type_lib_references.splice(p.type_lib_references.findIndex(r=>r.status==='observed'),1)],
    ['two genuine Program targets substituted',p=>{const a=p.type_lib_references.find(r=>r.target),b=p.type_lib_references.find(r=>r.target&&r.target!==a.target);a.target=b.target;}],
    ['same-name span moved within source',p=>{const r=p.type_lib_references[0].request;r.start_utf16++;r.end_utf16++;r.start_byte++;r.end_byte++;}],
    ['unproved react-scripts promoted and refusal erased',p=>{const r=p.type_lib_references.find(r=>r.name==='react-scripts');r.status='observed';r.reason=null;r.inclusion=true;r.target=p.type_lib_references.find(r=>r.target).target;p.reasons=p.reasons.filter(r=>r!=='unproven_type_lib_reference');p.closure.references=true;}],
  ]){const p=structuredClone(packet);mutate(p);const file=path.join(dir,records.length+'.json');fs.writeFileSync(file,JSON.stringify(p));run(name,file,1);}
  console.log(JSON.stringify({audit_only:true,controls:records},null,2));
}finally{fs.rmSync(dir,{recursive:true,force:true});}
