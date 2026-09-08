// Read-only fixed-public-source/cache reconciliation; never installs or edits apps.
// node audit-entry-observations.mjs <compiler> <source> <task-root> <source-manifest>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {createRequire} from 'node:module';import {fileURLToPath} from 'node:url';
import {parsePacket,canonical,hash,COMPILER_HASH} from '../../../scripts/callable-observations/schema.mjs';

export function reconcileEntries(packet,base,capture,capturedPacket) {
  assert.equal(packet.schema,'prism.callable-observation/12');assert.equal(packet.producer.version,'0.13.0');
  assert.equal(base.schema,'prism.callable-observation/11');
  for(const key of Object.keys(base)) {
    if(!['schema','producer'].includes(key))assert.deepEqual(packet[key],base[key],key);
    if(key!=='producer')assert.deepEqual(capturedPacket[key],base[key],'capture '+key);
  }
  assert.deepEqual(capture.names,capture.types);assert.equal(capture.types.length,2);assert.equal(capture.lib.length,3);
  assert.equal(capture.entry_inclusions.length,5);
  const id=f=>{assert(f.startsWith('/__prism__/'));return f.slice(11);};
  const expected=[];
  for(const [kind,names] of [['types',capture.types],['lib',capture.lib]])for(const [index,name] of names.entries()) {
    const selected=kind==='types'?capture.type_cache[index].resolution.resolvedTypeReferenceDirective.resolvedFileName
      :new Map(capture.lib_cache).get(name).actual;
    if(kind==='types')assert.equal(capture.type_cache[index].name,name);
    const ii=capture.entry_inclusions.filter(r=>r.file===selected&&(kind==='types'
      ?r.reason.kind===8&&r.reason.typeReference===name:r.reason.kind===6&&r.reason.index===index));assert.equal(ii.length,1);
    expected.push({kind,origin:'configured',index,name,mode:null,status:'observed',reason:null,target:id(selected),inclusion:true});
  }
  assert.deepEqual(packet.type_lib_entries.map(canonical).sort(),expected.map(canonical).sort());
  const gaps=packet.type_lib_references.filter(r=>r.status==='unproven');assert.equal(gaps.length,1);assert.equal(gaps[0].name,'react-scripts');
  assert.equal(packet.status,'unproven');assert.equal(packet.authorizes_runtime_edge,false);assert.equal(packet.scope.class_authority,false);
  return {entries:packet.type_lib_entries,source_directives:{total:139,observed:138,unproven:1},gaps,
    unchanged_fields:Object.keys(base).filter(k=>!['schema','producer'].includes(k)),changed_previous_fields:['schema','producer'],
    counts:{configured_types:2,configured_libs:3,automatic:0,default:0,observed:5,unproven:0},
    lookup:{total:packet.resolutions.length,nulls:packet.resolutions.filter(r=>r.target===null).length,refusal_digests:packet.snapshot.refused_lookup_sha256.length,outside:packet.snapshot.outside_lookups},
    receivers:{observations:packet.observations.length,nested_calls:packet.observations.reduce((n,o)=>n+o.nested.calls.length,0)},
    disposition:'Owner decision: keep react-scripts unresolved; no install, removal, shim, substitution or waiver.'};
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [compiler,root,task,manifestFile]=process.argv.slice(2);assert(manifestFile);
  assert.equal(hash(fs.readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);
  const read=f=>fs.readFileSync(path.join(task,f)),packet=read('public-current.json'),base=read('public-base.json'),cache=read('entry-cache.json'),captured=read('public-capture.json');
  assert.equal(hash(base),'25ea7dba031a0f4d9a1d6f3022d68a8e4ed6c8c495a93dabcc664150f51bbca5');
  const p=parsePacket(packet.toString()),b=parsePacket(base.toString()),c=JSON.parse(cache),cp=parsePacket(captured.toString());
  const result=reconcileEntries(p,b,c,cp),manifest=JSON.parse(fs.readFileSync(manifestFile));
  assert.equal(manifest.manifest_sha256,'353187a695df2683a3631e4739c173cb6d33190901093549b61e28efeda60cbb');assert.equal(hash(JSON.stringify(manifest.manifest)),manifest.manifest_sha256);assert.equal(manifest.manifest.length,1229);
  const inventory=new Map(p.snapshot.files.map(f=>[f.id,f]));
  for(const f of manifest.manifest){const bytes=fs.readFileSync(path.join(root,f.file));assert.equal(hash(bytes),f.sha256,f.file);assert.equal(bytes.length,f.size);assert.equal(inventory.get('project/'+f.file)?.sha256,f.sha256);}
  const config=JSON.parse(fs.readFileSync(path.join(root,'tsconfig.json'))),converted=ts.convertCompilerOptionsFromJson(config.compilerOptions,'/__prism__/project');assert.deepEqual(converted.errors,[]);assert.deepEqual(converted.options.types,c.types);assert.deepEqual(converted.options.lib,c.lib);
  assert.equal(p.type_lib_references.length,139);assert.equal(p.type_lib_references.filter(r=>r.status==='observed').length,138);
  for(const r of p.type_lib_entries){const f=r.target,physical=f.startsWith('project/')?path.join(root,f.slice(8)):path.join(path.dirname(compiler),f.slice(9));assert.equal(hash(fs.readFileSync(physical)),inventory.get(f)?.sha256);}
  console.log(JSON.stringify({schema:'prism.entry-observation-audit/1',audit_only:true,closure_admission:false,authorizes_runtime_edge:false,
    base:'ca473cd198d3a906a9ba9b14402118a4bbcd952a',producer:p.producer,compiler:p.compiler,
    packet_sha256:hash(packet),base_packet_sha256:hash(base),capture_packet_sha256:hash(captured),entry_cache_sha256:hash(cache),
    source_manifest_sha256:manifest.manifest_sha256,source_files_verified:1229,...result},null,2));
}
