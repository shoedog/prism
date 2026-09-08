// Disposable diagnostic copy only; never changes the production worker or source.
// node capture-closure-reference-cache.mjs <options.json> <existing task-root>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
const [optionsFile,task]=process.argv.slice(2);assert(optionsFile&&task);
const original=fileURLToPath(new URL('../../../scripts/callable-observations/',import.meta.url)),copy=fs.mkdtempSync(path.join(task,'reference-capture-'));
fs.cpSync(original,copy,{recursive:true});const worker=path.join(copy,'worker.mjs'),source=fs.readFileSync(worker,'utf8');assert.equal(source.split('  return packet;').length,2);
const replacement=`  const frozen=JSON.stringify(packet),references=[];
  for(const sf of program.getSourceFiles()) {
    for(const ref of sf.typeReferenceDirectives??[]) {
      const resolved=program.getResolvedTypeReferenceDirectiveFromTypeReferenceDirective(ref,sf)?.resolvedTypeReferenceDirective;
      references.push({from:sf.fileName,kind:'types',name:ref.fileName,start_utf16:ref.pos,end_utf16:ref.end,target:resolved?.resolvedFileName??null});
    }
    for(const ref of sf.libReferenceDirectives??[]) {
      const lib=ts.libMap.get(ref.fileName.toLowerCase()),resolved=lib?program.resolvedLibReferences?.get(lib):null;
      references.push({from:sf.fileName,kind:'lib',name:ref.fileName,start_utf16:ref.pos,end_utf16:ref.end,target:resolved?.actual??null});
    }
  }
  process.stderr.write(JSON.stringify(references,null,2));
  return JSON.parse(frozen);`;
fs.writeFileSync(worker,source.replace('  return packet;',replacement));
const run=spawnSync(process.execPath,['--max-old-space-size=1024',worker],{input:fs.readFileSync(optionsFile),maxBuffer:40*1024*1024,timeout:180000});
assert.equal(run.status,0,run.stderr?.toString());const packet=JSON.parse(run.stdout),cache=JSON.parse(run.stderr);assert.equal(packet.schema,'prism.callable-observation/10');assert.equal(cache.length,139);
fs.writeFileSync(path.join(copy,'packet.json'),run.stdout);fs.writeFileSync(path.join(copy,'reference-cache.json'),run.stderr);
console.log(JSON.stringify({copy,packet:path.join(copy,'packet.json'),cache:path.join(copy,'reference-cache.json')}));
