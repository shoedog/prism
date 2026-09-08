// Audit-verifier controls; does not test or modify production closure policy.
// node verify-closure-policy-proof-controls.mjs <compiler> <source> <task-root>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
const [compiler,source,task]=process.argv.slice(2);assert(compiler&&source&&task);
const audit=fileURLToPath(new URL('./audit-closure-policy-proof.mjs',import.meta.url));
const temporary=fs.mkdtempSync(path.join(task,'verifier-controls-')),rows=[];
try{
  const packet=path.join(task,'packet.json'),instrumented=path.join(task,'instrumented-packet.json'),cache=path.join(task,'reference-cache.json');
  function run(name,root,capture,expected){const r=spawnSync(process.execPath,[audit,compiler,root,packet,instrumented,capture],{encoding:'utf8',maxBuffer:2*1024*1024});
    assert.equal(r.status,expected,r.stderr);if(expected===0){const p=JSON.parse(r.stdout);assert.equal(p.program_sources_checked,1445);assert.equal(p.additional_reference_gaps[0].name,'react-scripts');}
    else assert.match(r.stderr,/AssertionError/);rows.push({name,status:r.status,assertion:expected!==0,stderr:r.stderr});
  }
  run('unchanged pinned capture',source,cache,0);
  const altered=JSON.parse(fs.readFileSync(cache));const target=altered.find(r=>r.target!==null);const other=altered.find(r=>r.target!==null&&r.target!==target.target);assert(target&&other);target.target=other.target;
  const substituted=path.join(temporary,'substituted-cache.json');fs.writeFileSync(substituted,JSON.stringify(altered,null,2));
  run('two genuine Program targets cannot substitute',source,substituted,1);
  const root=path.join(temporary,'source');fs.mkdirSync(root);
  for(const name of fs.readdirSync(source))if(name!=='tsconfig.json')fs.symlinkSync(path.join(source,name),path.join(root,name));
  fs.writeFileSync(path.join(root,'tsconfig.json'),Buffer.concat([fs.readFileSync(path.join(source,'tsconfig.json')),Buffer.from(' ')]));
  run('same-options config cannot retain stale byte hash',root,cache,1);
  console.log(JSON.stringify({audit_only:true,controls:rows},null,2));
}finally{fs.rmSync(temporary,{recursive:true,force:true});}
