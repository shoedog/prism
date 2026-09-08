// Audit-only: node audit-closure-policy-proof.mjs <compiler> <root> <packet> <instrumented-packet> <reference-cache>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
import {parsePacket,hash,canonical,COMPILER_HASH} from '../../../scripts/callable-observations/schema.mjs';
const [compiler,root,packetFile,instrumentedFile,cacheFile]=process.argv.slice(2);
assert(compiler&&root&&packetFile&&instrumentedFile&&cacheFile);assert.equal(hash(fs.readFileSync(compiler)),COMPILER_HASH);
const ts=createRequire(import.meta.url)(compiler),bytes=fs.readFileSync(packetFile),p=parsePacket(bytes.toString());
assert.equal(hash(bytes),'19871f6f359422afd029c22a0f1109da6d363d5b5b98e4a49ca429f13629b78a');
assert.equal(hash(fs.readFileSync(cacheFile)),'e9f39176f6902eb340a79e92b737da58a6f46006723c8e0b2a946f83e0a309f3','fixed compiler-cache capture digest');
const instrumented=JSON.parse(fs.readFileSync(instrumentedFile)),project=q=>{q=structuredClone(q);delete q.producer;return q;};
assert.deepEqual(project(instrumented),project(p),'cache capture must preserve every field except copied producer digest');
const inventory=new Map(p.snapshot.files.map(f=>[f.id,f])),program=new Set(p.snapshot.program_files),sources=new Map(),directives=[];
function physical(id){assert(/^(project|compiler)\//.test(id));assert(!id.split('/').includes('..'));return id.startsWith('project/')?path.join(root,id.slice(8)):path.join(path.dirname(compiler),id.slice(9));}
for(const id of program){const data=fs.readFileSync(physical(id));assert.equal(hash(data),inventory.get(id).sha256);
  const sf=ts.createSourceFile(id,data.toString('utf8'),ts.ScriptTarget.Latest,true);sources.set(id,sf);
  for(const [kind,refs] of [['path',sf.referencedFiles],['types',sf.typeReferenceDirectives],['lib',sf.libReferenceDirectives]])for(const ref of refs){
    assert.equal(sf.text.slice(ref.pos,ref.end),ref.fileName);
    const row={from:id,sha256:hash(data),kind,name:ref.fileName,start_utf16:ref.pos,end_utf16:ref.end,start_byte:Buffer.byteLength(sf.text.slice(0,ref.pos)),end_byte:Buffer.byteLength(sf.text.slice(0,ref.end))};
    if(kind==='path'){row.lexical_candidate=path.posix.normalize(path.posix.join(path.posix.dirname(id),ref.fileName));row.candidate_in_inventory=inventory.has(row.lexical_candidate);row.candidate_in_program=program.has(row.lexical_candidate);}
    directives.push(row);
  }
}
const normalize=f=>{assert(f.startsWith('/__prism__/'));const id=f.slice('/__prism__/'.length);assert(inventory.has(id));assert(program.has(id));return id;};
const cache=JSON.parse(fs.readFileSync(cacheFile)).map(r=>({...r,from:normalize(r.from),target:r.target===null?null:normalize(r.target)}));
const key=r=>canonical([r.from,r.kind,r.name,r.start_utf16,r.end_utf16]);
assert.deepEqual(cache.map(key).sort(),directives.filter(r=>r.kind!=='path').map(key).sort(),'complete original-source/cache occurrence multiset');
const byKey=new Map(cache.map(r=>[key(r),r]));assert.equal(byKey.size,cache.length);
for(const d of directives)if(d.kind!=='path')d.compiler_cache_target=byKey.get(key(d)).target;
const count=xs=>xs.reduce((o,x)=>(o[x]=(o[x]??0)+1,o),{});
assert.equal(program.size,1445);assert.deepEqual(count(directives.map(r=>r.kind)),{lib:112,path:78,types:27});
assert(directives.filter(r=>r.kind==='path').every(r=>r.candidate_in_inventory&&r.candidate_in_program));
const gaps=directives.filter(r=>r.compiler_cache_target===null);assert.equal(gaps.length,1);assert.equal(gaps[0].name,'react-scripts');assert.equal(gaps[0].from,'project/packages/excalidraw/react-app-env.d.ts');
const nulls=p.resolutions.filter(r=>r.target===null),partition=count(nulls.map(r=>r.lookup.status==='observed'?'exact':r.lookup.wildcard?.status==='observed'?'singleton':r.lookup.merged_wildcard?.status==='observed'?'merged':r.lookup.declarations.length===0?'no_source':r.lookup.context==='require'?'require':'augmentation_name'));
assert.deepEqual(partition,{merged:84,exact:272,singleton:231,no_source:14,require:1,augmentation_name:1});
// Recheck every null-target request's original source bytes and coordinates.
for(const r of nulls){const a=r.lookup.request,sf=sources.get(a.file);assert(sf);assert.equal(inventory.get(a.file).sha256,a.sha256);
  assert.equal(Buffer.byteLength(sf.text.slice(0,a.start_utf16)),a.start_byte);assert.equal(Buffer.byteLength(sf.text.slice(0,a.end_utf16)),a.end_byte);
  assert.equal(sf.text.slice(a.start_utf16+1,a.end_utf16-1),r.specifier);
}
const configBytes=fs.readFileSync(physical(p.scope.config));assert.equal(hash(configBytes),inventory.get(p.scope.config).sha256);
const config=ts.parseConfigFileTextToJson('tsconfig.json',configBytes.toString('utf8'));assert(!config.error);assert.equal(config.config.compilerOptions.skipLibCheck,true);
assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);assert.equal(p.status,'unproven');assert.equal(p.diagnostics.length,0);
console.log(JSON.stringify({schema:'prism.closure-policy-proof-audit/1',audit_only:true,closure_admission:false,authorizes_runtime_edge:false,base:'52e72bed7d0685cf0a0901d053563e067dc64387',compiler_sha256:COMPILER_HASH,packet_sha256:hash(bytes),producer:p.producer,instrumented_packet_sha256:hash(fs.readFileSync(instrumentedFile)),reference_cache_sha256:hash(fs.readFileSync(cacheFile)),non_producer_packet_equal:true,program_sources_checked:program.size,null_request_coordinates_checked:nulls.length,null_partition:partition,config_sha256:inventory.get(p.scope.config).sha256,skipLibCheck:true,directive_counts:count(directives.map(r=>r.kind)),path_candidate_checks_only:true,type_cache_resolved:26,lib_cache_resolved:112,additional_reference_gaps:gaps,refusal_digests:p.snapshot.refused_lookup_sha256.length,outside_lookups:p.snapshot.outside_lookups,diagnostics:p.diagnostics.length,directives},null,2));
