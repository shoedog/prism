// Audit-only disposable worker copy. Never modifies the production worker or app.
// node capture-required-path-proof.mjs <options.json> <normal-packet> <existing-task-root>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
import {parsePacket,hash,COMPILER_HASH} from '../../../scripts/callable-observations/schema.mjs';
import {settings} from '../../../scripts/callable-observations/index.mjs';
const [optionsFile,packetFile,task]=process.argv.slice(2);assert(optionsFile&&packetFile&&task);
const options=settings(JSON.parse(fs.readFileSync(optionsFile))),normalBytes=fs.readFileSync(packetFile),normal=parsePacket(normalBytes.toString());
assert.equal(normal.producer.version,'0.11.1');assert.equal(normal.compiler.sha256,COMPILER_HASH);
assert.equal(normal.snapshot.program_files.length,1445,'fixed public population');
const taskPath=fs.realpathSync(task),rootPath=fs.realpathSync(options.root);
assert(taskPath!==rootPath&&!taskPath.startsWith(rootPath+path.sep),'capture must stay outside audited root');
const original=fileURLToPath(new URL('../../../scripts/callable-observations/',import.meta.url));
const copy=fs.mkdtempSync(path.join(taskPath,'path-capture-'));fs.cpSync(original,copy,{recursive:true});
const worker=path.join(copy,'worker.mjs'),source=fs.readFileSync(worker,'utf8');assert.equal(source.split('  return packet;').length,2);
const replacement=`  const frozen=JSON.stringify(packet),references=[];
  const sources=new Set(program.getSourceFiles()),inclusions=program.getFileIncludeReasons();
  for(const sf of sources) {
    const original=sf.redirectInfo?.unredirected??sf;
    for(const [index,ref] of original.referencedFiles.entries()) {
      const target=program.getSourceFileFromReference(original,ref);
      const from=toId(original.fileName),targetId=target?toId(target.fileName):null,bytes=first.read(from);
      if(!bytes || new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes)!==original.text
        || original.text.slice(ref.pos,ref.end)!==ref.fileName)throw Error('capture_source_mismatch');
      const reasons=target?inclusions.get(target.path)??[]:[];
      references.push({from,sha256:hash(bytes),index,name:ref.fileName,start_utf16:ref.pos,end_utf16:ref.end,
        start_byte:Buffer.byteLength(original.text.slice(0,ref.pos)),end_byte:Buffer.byteLength(original.text.slice(0,ref.end)),
        redirected:!!sf.redirectInfo,target:targetId,target_in_program:!!target&&sources.has(target),
        self_reference:targetId===from,inclusion:reasons.some(r=>r.kind===4&&r.file===sf.path&&r.index===index)});
    }
  }
  process.stderr.write(JSON.stringify(references));
  return JSON.parse(frozen);`;
fs.writeFileSync(worker,source.replace('  return packet;',replacement));
const run=spawnSync(process.execPath,['--max-old-space-size=1024',worker],{input:JSON.stringify(options),maxBuffer:40*1024*1024,timeout:options.limits.timeout_ms});
assert.equal(run.status,0,run.stderr?.toString());const instrumented=parsePacket(run.stdout.toString()),references=JSON.parse(run.stderr);
const projection=p=>{p=structuredClone(p);delete p.producer;return p;};
assert.deepEqual(projection(instrumented),projection(normal),'frozen packet agrees in every non-producer field');
assert.equal(references.length,78);assert.equal(new Set(references.map(r=>JSON.stringify([r.from,r.index]))).size,78);
const inventory=new Map(normal.snapshot.files.map(f=>[f.id,f])),program=new Set(normal.snapshot.program_files);
for(const r of references){assert.equal(r.sha256,inventory.get(r.from)?.sha256);assert(program.has(r.from));
  assert(r.target&&program.has(r.target)&&inventory.has(r.target));assert(r.target_in_program&&r.inclusion&&!r.self_reference);}
assert(!normal.reasons.includes('unproven_path_reference'));
fs.writeFileSync(path.join(copy,'packet.json'),run.stdout);fs.writeFileSync(path.join(copy,'path-cache.json'),run.stderr);
console.log(JSON.stringify({schema:'prism.required-path-proof-audit/1',audit_only:true,authorizes_runtime_edge:false,
  packet_sha256:hash(normalBytes),producer:normal.producer,instrumented_packet_sha256:hash(run.stdout),path_cache_sha256:hash(run.stderr),
  non_producer_packet_equal:true,program_sources:1445,path_directives:78,proved:78,unproven:0,references},null,2));
