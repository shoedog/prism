// Diagnostic copy only. No production source or application files are modified.
// node capture-search-boundaries.mjs <options.json> <existing-task-root>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
import {hash,parsePacket} from '../../../scripts/callable-observations/schema.mjs';

export function instrumentBoundaryWorker(source) {
  const replace=(old,next)=>{assert.equal(source.split(old).length,2,'unique instrumentation anchor: '+old);source=source.replace(old,next);};
  replace('  // Virtual paths make observations portable across equivalent caller-owned roots.',`  const boundaryTrace=[],moduleTrace=[];let activeModule=null;
  Error.stackTraceLimit=40;
  function traceBoundary(kind,input,normalized,operation) {
    if(boundaryTrace.length>=100000)throw Error('budget_exceeded');
    boundaryTrace.push({index:boundaryTrace.length,kind,input,normalized,operation,owner:activeModule,
      stack:new Error().stack.split('\\n').slice(2)});
  }
  // Virtual paths make observations portable across equivalent caller-owned roots.`);
  replace('  const toId=f=>{','  const toId=(f,operation="identity")=>{');
  replace('if(!relative(id)){refused.add(hash(n));','if(!relative(id)){traceBoundary("refused",f,n,operation);refused.add(hash(n));');
  replace('    outside=true;return null;','    traceBoundary("outside",f,n,operation);outside=true;return null;');
  replace('const id=toId(f);if(!id)return undefined;','const id=toId(f,"readFile");if(!id)return undefined;');
  replace('const id=toId(f);if(!id)return {files:[],directories:[]};','const id=toId(f,"entries");if(!id)return {files:[],directories:[]};');
  replace('const basic={readFile:read,fileExists:f=>{const id=toId(f);','const basic={readFile:read,fileExists:f=>{const id=toId(f,"fileExists");');
  replace('directoryExists:f=>{const id=toId(f);','directoryExists:f=>{const id=toId(f,"directoryExists");');
  replace('realpath:f=>{const id=toId(f);','realpath:f=>{const id=toId(f,"realpath");');
  replace('    resolveModuleNameLiterals:(literals,from,redirected,compilerOptions,source)=>literals.map(l=>{',`    resolveModuleNameLiterals:(literals,from,redirected,compilerOptions,source)=>literals.map(l=>{
      const previousOwner=activeModule;
      const current={index:moduleTrace.length,from,specifier:l.text,request:l.pos>=0?anchorInSource(l,source):null,
        mode:ts.getModeForUsageLocation(source,l,compilerOptions)??null,target:null};
      moduleTrace.push(current);activeModule=current;`);
  replace('      const resolution={from:toId(from),specifier:l.text,target};',`      current.target=target;activeModule=previousOwner;
      const resolution={from:toId(from),specifier:l.text,target};`);
  replace('  return packet;',`  process.stderr.write(JSON.stringify({schema:'prism.search-boundary-capture/1',events:boundaryTrace,modules:moduleTrace}));
  return packet;`);
  return source;
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [optionsFile,task]=process.argv.slice(2);assert(optionsFile&&task);
  const original=fileURLToPath(new URL('../../../scripts/callable-observations/',import.meta.url));
  const copy=fs.mkdtempSync(path.join(task,'boundary-capture-'));fs.cpSync(original,copy,{recursive:true});
  const worker=path.join(copy,'worker.mjs'),before=fs.readFileSync(worker,'utf8');fs.writeFileSync(worker,instrumentBoundaryWorker(before));
  const run=spawnSync(process.execPath,['--max-old-space-size=1024',worker],{input:fs.readFileSync(optionsFile),maxBuffer:64*1024*1024,timeout:180000});
  assert.equal(run.status,0,run.stderr?.toString());const packet=parsePacket(run.stdout.toString()),trace=JSON.parse(run.stderr.toString());
  assert.equal(trace.schema,'prism.search-boundary-capture/1');assert(!packet.reasons.includes('worker_failed'));assert(!packet.reasons.includes('budget_exceeded'));
  fs.writeFileSync(path.join(copy,'packet.json'),run.stdout);fs.writeFileSync(path.join(copy,'boundaries.json'),run.stderr);
  console.log(JSON.stringify({copy,packet:path.join(copy,'packet.json'),capture:path.join(copy,'boundaries.json'),
    worker_before_sha256:hash(before),worker_after_sha256:hash(fs.readFileSync(worker)),packet_sha256:hash(run.stdout),capture_sha256:hash(run.stderr)}));
}
