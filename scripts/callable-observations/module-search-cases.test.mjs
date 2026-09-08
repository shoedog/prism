import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';import path from 'node:path';import {pathToFileURL} from 'node:url';
const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const url=f=>implementation?pathToFileURL(path.join(implementation,f)):new URL(f,import.meta.url);
const {produce,validate}=await import(url('index.mjs'));
const {hash,COMPILER_HASH,parsePacket,canonical}=await import(url('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler);assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-module-search-cases-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:true},include:['src']};
  const save=()=>put('tsconfig.json',JSON.stringify(config));
  try{save();put('src/app.ts','export {};');return run({put,config,save,options:{root,compiler,config:'tsconfig.json'}});}finally{rmSync(root,{recursive:true,force:true});}}
function ledger(p){assert(!p.reasons.includes('worker_failed'),'worker must complete');assert(p.search_provenance,'missing module-search provenance');return p.search_provenance;}
const key=r=>canonical({from:r.from,specifier:r.specifier,target:r.target,request:r.request});
function sameRequests(p){const s=ledger(p);assert.deepEqual(s.module_requests.map(key).sort(),p.resolutions.map(r=>key({...r,request:r.lookup.request})).sort());return s;}
function shared(put){put('src/a.ts','import "missing-a";');put('src/b.ts','import "missing-b";');}

test('different module requests retain shared outside paths and owner occurrences',()=>fixture(({put,options})=>{
  shared(put);const p=produce(options),s=sameRequests(p),outside=s.boundary_events.filter(e=>e.kind==='outside');
  assert.equal(s.module_requests.length,2);assert(outside.length>0);
  const byPath=new Map();for(const e of outside){assert.equal(e.owner.channel,'module');const ids=byPath.get(e.probe_sha256)??new Set();ids.add(e.owner.id);byPath.set(e.probe_sha256,ids);}
  assert([...byPath.values()].some(ids=>ids.size===2));assert(p.reasons.includes('outside_lookup'));assert(p.reasons.includes('unresolved_module'));assert.equal(p.closure.dependencies,false);
}));
test('duplicate same-source literals retain different anchors and repeated probes',()=>fixture(({put,options})=>{
  put('src/app.ts','import "missing"; import "missing";');const p=produce(options),s=sameRequests(p);
  assert.equal(s.module_requests.length,2);assert.notEqual(s.module_requests[0].request.start_byte,s.module_requests[1].request.start_byte);
  const owned=id=>s.boundary_events.filter(e=>e.owner?.id===id).map(e=>e.probe_sha256);
  assert(owned(0).length>0);assert.deepEqual(owned(0),owned(1));
}));
test('virtual refusals have actual module owners and reproduce the legacy digest set',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.baseUrl='.';save();put('src/app.ts','import "virtual:first"; import "virtual:second";');
  const p=produce(options),s=sameRequests(p),refused=s.boundary_events.filter(e=>e.kind==='refused');
  assert(refused.length>0);assert(refused.every(e=>e.owner?.channel==='module'));
  assert.deepEqual([...new Set(refused.map(e=>e.probe_sha256))].sort(),p.snapshot.refused_lookup_sha256);
  assert.equal(new Set(refused.map(e=>e.owner.id)).size,2);assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
}));
test('synthetic JSX runtime request has no invented source literal anchor',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.jsx='react-jsx';save();put('src/view.tsx','export const view=<div/>;');
  const p=produce(options),s=sameRequests(p),r=s.module_requests.find(r=>r.specifier==='react/jsx-runtime'&&r.from==='project/src/view.tsx');
  assert(r);assert.equal(r.request,null);assert.equal(r.from,'project/src/view.tsx');assert(s.boundary_events.some(e=>e.owner?.id===r.id));
}));
for(const channel of ['source_type','configured_type','lib'])test(`${channel} boundary encounters retain the intended owner channel`,()=>fixture(({put,config,save,options})=>{
  if(channel==='source_type')put('src/types.d.ts','/// <reference types="missing-types" />');
  if(channel==='configured_type')config.compilerOptions.types=['missing-types'];
  if(channel==='lib'){config.compilerOptions.lib=['es5'];config.compilerOptions.libReplacement=true;}
  save();const p=produce(options),s=sameRequests(p);assert.equal(s.module_requests.length,0);
  assert(s.boundary_events.length>0);
  if(channel==='lib'){const ids=new Set(s.lib_searches.map(r=>r.id));assert.deepEqual(s.lib_searches.map(r=>r.lib_file).sort(),['lib.decorators.d.ts','lib.decorators.legacy.d.ts','lib.es5.d.ts']);assert(s.boundary_events.every(e=>e.owner?.channel==='lib'&&ids.has(e.owner.id)));}
  else {assert.equal(s.type_requests.length,1);assert(s.boundary_events.every(e=>e.owner?.channel==='type'&&e.owner.id===s.type_requests[0].execution));}
  assert.equal(p.closure.dependencies,false);
}));
test('a resolved local package retains its module-owned outside peer metadata probe',()=>fixture(({put,options})=>{
  put('src/app.ts','import {value} from "./dep";export {value};');
  put('src/dep/package.json','{"name":"fixture","version":"1.0.0","types":"index.d.ts","peerDependencies":{"peer":"*"}}');
  put('src/dep/index.d.ts','export const value:number;');const p=produce(options),s=sameRequests(p);
  assert.equal(s.module_requests.length,1);assert.equal(s.module_requests[0].target,'project/src/dep/index.d.ts');
  assert(s.boundary_events.some(e=>e.kind==='outside'&&e.owner?.id===0));assert(!p.reasons.includes('unresolved_module'));assert.equal(p.closure.dependencies,false);
}));
test('non-ASCII source retains exact UTF16 and byte request coordinates',()=>fixture(({put,options})=>{
  const source='/* 🐕 é */ import "./dep";';put('src/app.ts',source);put('src/dep.ts','export {};');
  const p=produce(options),s=sameRequests(p),a=s.module_requests[0].request;
  assert.equal(a.sha256,hash(source));assert(a.start_byte>a.start_utf16);
  assert.equal(source.slice(a.start_utf16,a.end_utf16),'"./dep"');assert.equal(Buffer.from(source).subarray(a.start_byte,a.end_byte).toString(),'"./dep"');
}));
test('orphan owners, omitted refusal families and invalid fields reject before root I/O',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.baseUrl='.';save();put('src/app.ts','import "virtual:first"; import "missing";');
  const p=produce(options),s=ledger(p);assert(s.boundary_events.some(e=>e.kind==='refused'));assert(s.boundary_events.some(e=>e.kind==='outside'));
  let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const change of [q=>q.search_provenance.boundary_events[0].owner.id=999,
    q=>{q.search_provenance.boundary_events=q.search_provenance.boundary_events.filter(e=>e.kind!=='refused');q.search_provenance.boundary_events.forEach((e,i)=>e.id=i);},
    q=>q.snapshot.outside_lookups=false,q=>q.search_provenance.boundary_events[0].operation='stat',
    q=>q.search_provenance.module_requests[0].mode='both',q=>q.search_provenance.module_requests[0].request.end_byte=99999]){
    const q=structuredClone(p);change(q);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);
  }assert.equal(reads,0);
}));
test('two genuine owners, digests and repeated event omission fail full reproduction',()=>fixture(({put,options})=>{
  shared(put);const p=produce(options),s=ledger(p),events=s.boundary_events;
  assert.equal(validate(JSON.stringify(p),options).valid,true);
  const first=events.findIndex(e=>e.owner?.id===0),other=events.findIndex(e=>e.owner?.id===1);
  const distinct=events.findIndex(e=>e.probe_sha256!==events[first].probe_sha256);assert(first>=0&&other>=0&&distinct>=0);
  for(const change of [q=>q.search_provenance.boundary_events[first].owner=structuredClone(events[other].owner),
    q=>{const e=q.search_provenance.boundary_events;[e[first].probe_sha256,e[distinct].probe_sha256]=[e[distinct].probe_sha256,e[first].probe_sha256];},
    q=>{q.search_provenance.boundary_events.splice(first,1);q.search_provenance.boundary_events.forEach((e,i)=>e.id=i);}]){
    const q=structuredClone(p);change(q);assert.doesNotThrow(()=>parsePacket(JSON.stringify(q)));assert.equal(validate(JSON.stringify(q),options).reason,'stale_or_tampered');
  }
}));
test('source and configuration epochs invalidate prior module provenance',()=>fixture(({put,config,save,options})=>{
  shared(put);const p=produce(options);ledger(p);put('src/a.ts','/* changed */ import "missing-a";');assert.equal(validate(JSON.stringify(p),options).valid,false);
  const q=produce(options);ledger(q);config.compilerOptions.moduleResolution='Bundler';save();assert.equal(validate(JSON.stringify(q),options).valid,false);
}));
test('historical schema12 missing type-reference refusal remains readable, never current-valid',()=>fixture(({put,options})=>{
  put('src/types.d.ts','/// <reference types="missing-types" />');const p=produce(options);ledger(p);assert(p.reasons.includes('unproven_type_lib_reference'));
  const old=structuredClone(p);old.schema='prism.callable-observation/12';old.producer.version='0.13.0';delete old.search_provenance;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(old)));assert.equal(validate(JSON.stringify(old),options).valid,false);
}));
for(const [extension,mode] of [['mts','import'],['cts','require']])test(`NodeNext ${extension} request retains actual ${mode} resolution mode`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';save();
  put(`src/view.${extension}`,'import {value} from "fixture";export {value};');
  put('node_modules/fixture/package.json','{"name":"fixture","version":"1.0.0","exports":{".":{"import":"./import.d.mts","require":"./require.d.cts"}}}');
  for(const [branch,ext] of [['import','mts'],['require','cts']])put(`node_modules/fixture/${branch}.d.${ext}`,'export const value:number;');
  const p=produce(options),s=sameRequests(p),r=s.module_requests.find(r=>r.specifier==='fixture');
  assert(r);assert.equal(r.mode,mode);assert.equal(r.target,`project/node_modules/fixture/${mode}.d.${extension}`);
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
