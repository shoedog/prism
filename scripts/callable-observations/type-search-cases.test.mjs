import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';import path from 'node:path';import {pathToFileURL} from 'node:url';

const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const url=f=>implementation?pathToFileURL(path.join(implementation,f)):new URL(f,import.meta.url);
const {produce,validate}=await import(url('index.mjs'));
const {parsePacket,hash,COMPILER_HASH}=await import(url('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);

function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-type-search-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:true},include:['src']};
  const save=()=>put('tsconfig.json',JSON.stringify(config));
  try{save();put('src/app.ts','export {};');return run({put,config,save,options:{root,compiler,config:'tsconfig.json'}});}finally{rmSync(root,{recursive:true,force:true});}}
function provenance(p){assert(!p.reasons.includes('worker_failed'),'worker must complete');
  assert(Array.isArray(p.search_provenance?.type_requests),'missing type-request ledger');
  assert(Array.isArray(p.search_provenance?.type_searches),'missing type-search ledger');return p.search_provenance;}
function batches(p){const s=provenance(p);assert(Array.isArray(s.type_batches),'missing type-batch ledger');return s;}
const types=p=>p.type_lib_references.filter(r=>r.kind==='types');
const entries=p=>p.type_lib_entries.filter(r=>r.kind==='types');
function packageType(put,name='fixture',file='index.d.ts'){put(`node_modules/@types/${name}/package.json`,JSON.stringify({name:`@types/${name}`,version:'1.0.0',types:file}));put(`node_modules/@types/${name}/${file}`,'interface Fixture {}');}

test('source duplicate callback occurrences retain exact anchors and share one batch execution',()=>fixture(({put,options})=>{
  const text='// 🐕é\n/// <reference types="fixture" />\n/// <reference types="fixture" />';put('src/ref.d.ts',text);packageType(put);
  const p=produce(options);assert.equal(types(p).length,2,'existing source ledger must resolve before S3 assertion');const s=provenance(p);
  assert.equal(s.type_requests.length,2);assert.equal(s.type_searches.length,1);assert.deepEqual(s.type_requests.map(r=>r.origin),['source','source']);
  assert.deepEqual(s.type_requests.map(r=>r.index),[0,1]);assert.deepEqual(s.type_requests.map(r=>r.execution),[0,0]);
  assert.notEqual(s.type_requests[0].request.start_byte,s.type_requests[1].request.start_byte);assert(s.type_requests[0].request.start_byte>s.type_requests[0].request.start_utf16);
  for(const r of s.type_requests){assert.equal(r.from,'project/src/ref.d.ts');assert.equal(r.name,'fixture');assert.equal(r.mode,null);assert.equal(r.request.file,r.from);}
  assert.equal(s.type_searches[0].target,'project/node_modules/@types/fixture/index.d.ts');
}));

test('configured duplicate entries retain indices and share the derived-entry execution',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.types=['fixture','fixture'];save();packageType(put);const p=produce(options);assert.equal(entries(p).length,2);const s=provenance(p);
  assert.equal(s.type_requests.length,2);assert.equal(s.type_searches.length,1);assert.deepEqual(s.type_requests.map(r=>r.origin),['configured','configured']);
  assert.deepEqual(s.type_requests.map(r=>r.index),[0,1]);assert.deepEqual(s.type_requests.map(r=>r.execution),[0,0]);
  for(const r of s.type_requests){assert.equal(r.from,'project/__inferred type names__.ts');assert.equal(r.request,null);assert.equal(r.mode,null);}
}));

test('configured duplicate callback retains one exact batch and request membership',()=>fixture(({config,save,options})=>{
  config.compilerOptions.types=['missing','missing'];save();const p=produce(options),s=batches(p);
  assert.deepEqual(s.type_batches,[{id:0,origin:'configured',from:'project/__inferred type names__.ts',size:2}]);
  assert.deepEqual(s.type_requests.map(r=>r.batch),[0,0]);
}));

test('automatic duplicates across typeRoots retain enumeration occurrences without owning discovery',()=>fixture(({put,config,save,options})=>{
  delete config.compilerOptions.types;config.compilerOptions.typeRoots=['./types-a','./types-b'];save();
  for(const dir of ['types-a','types-b'])put(`${dir}/fixture/index.d.ts`,'interface Fixture {}');
  const p=produce(options);assert.equal(entries(p).length,2);const s=provenance(p),rr=s.type_requests;
  assert.deepEqual(rr.map(r=>[r.origin,r.index,r.execution]),[['automatic',0,0],['automatic',1,0]]);assert.equal(s.type_searches.length,1);
  assert.equal(s.type_searches[0].target,'project/types-a/fixture/index.d.ts');
}));

test('source import and require modes are separate executions with exact selected targets',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';save();
  put('src/ref.d.ts','/// <reference types="fixture" resolution-mode="import" />\n/// <reference types="fixture" resolution-mode="require" />');
  put('node_modules/fixture/package.json',JSON.stringify({name:'fixture',version:'1.0.0',exports:{'.':{import:'./import.d.mts',require:'./require.d.cts'}}}));
  put('node_modules/fixture/import.d.mts','export interface Imported {}');put('node_modules/fixture/require.d.cts','export interface Required {}');
  const p=produce(options),s=provenance(p);assert.deepEqual(s.type_requests.map(r=>r.mode),['import','require']);assert.deepEqual(s.type_requests.map(r=>r.execution),[0,1]);
  assert.deepEqual(s.type_searches.map(r=>r.target),['project/node_modules/fixture/import.d.mts','project/node_modules/fixture/require.d.cts']);
}));

test('same missing name in two source files has two executions and a zero-event global cache hit',()=>fixture(({put,options})=>{
  put('src/a.d.ts','/// <reference types="missing-shared" />');put('src/b.d.ts','/// <reference types="missing-shared" />');
  const p=produce(options);assert.equal(types(p).filter(r=>r.name==='missing-shared').length,2);const s=provenance(p),rr=s.type_requests.filter(r=>r.name==='missing-shared');
  assert.equal(rr.length,2);assert.notEqual(rr[0].execution,rr[1].execution);assert.equal(s.type_searches.filter(r=>r.name==='missing-shared').length,2);
  const owned=id=>s.boundary_events.filter(e=>e.owner?.channel==='type'&&e.owner.id===id);
  assert(owned(rr[0].execution).length>0);assert.equal(owned(rr[1].execution).length,0);
}));

test('an imported package revisited as a later root retains both callback batches',()=>fixture(({put,config,save,options})=>{
  config.files=['src/app.ts','node_modules/pkg/index.d.ts'];delete config.include;save();
  put('src/app.ts','import {x} from "pkg";export const y=x;');
  put('node_modules/pkg/package.json','{"name":"pkg","version":"1.0.0","types":"index.d.ts"}');
  put('node_modules/pkg/index.d.ts','/// <reference types="missing-revisit" />\nexport const x:number;');
  const p=produce(options),s=batches(p),from='project/node_modules/pkg/index.d.ts';assert(!p.reasons.includes('unsupported_input'));
  assert.deepEqual(s.type_batches.filter(b=>b.from===from),[
    {id:0,origin:'source',from,size:1},{id:1,origin:'source',from,size:1}]);
  assert.deepEqual(s.type_requests.map(r=>[r.batch,r.index,r.execution]),[[0,0,0],[1,0,1]]);
  assert.equal(types(p).filter(r=>r.request.file===from).length,1);assert.equal(validate(JSON.stringify(p),options).valid,true);
  assert.equal(s.boundary_events.filter(e=>e.owner?.channel==='type'&&e.owner.id===1).length,0);
  const omitted=structuredClone(p);omitted.search_provenance.type_batches.pop();omitted.search_provenance.type_requests.pop();omitted.search_provenance.type_searches.pop();
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(omitted)));assert.equal(validate(JSON.stringify(omitted),options).reason,'stale_or_tampered');
  const borrowed=structuredClone(p);borrowed.search_provenance.type_requests[1].execution=0;borrowed.search_provenance.type_searches.pop();
  assert.throws(()=>parsePacket(JSON.stringify(borrowed)),/invalid_packet/);
}));

test('unresolved configured entry remains a real attributed search',()=>fixture(({config,save,options})=>{
  config.compilerOptions.types=['react-scripts'];save();const p=produce(options);assert.equal(entries(p)[0].reason,'unresolved');const s=provenance(p),r=s.type_requests[0],q=s.type_searches[0];
  assert.equal(r.origin,'configured');assert.equal(r.name,'react-scripts');assert.equal(q.target,null);assert(s.boundary_events.some(e=>e.owner?.channel==='type'&&e.owner.id===q.id));
}));

test('source, execution and genuine boundary-owner substitutions cannot survive proof checks',()=>fixture(({put,options})=>{
  put('src/a.d.ts','/// <reference types="missing-a" />');put('src/b.d.ts','/// <reference types="missing-b" />');const p=produce(options),s=provenance(p);
  assert.equal(validate(JSON.stringify(p),options).valid,true);const source=s.type_requests.filter(r=>r.origin==='source');assert.equal(source.length,2);
  const ownerA=s.boundary_events.findIndex(e=>e.owner?.channel==='type'&&e.owner.id===source[0].execution),ownerB=s.boundary_events.findIndex(e=>e.owner?.channel==='type'&&e.owner.id===source[1].execution);assert(ownerA>=0&&ownerB>=0);
  for(const alter of [q=>{const r=q.search_provenance.type_requests;[r[0].request,r[1].request]=[r[1].request,r[0].request];},
    q=>{const r=q.search_provenance.type_requests;[r[0].execution,r[1].execution]=[r[1].execution,r[0].execution];}]){const q=structuredClone(p);alter(q);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);}
  const q=structuredClone(p);q.search_provenance.boundary_events[ownerA].owner=structuredClone(s.boundary_events[ownerB].owner);
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(q)));assert.equal(validate(JSON.stringify(q),options).reason,'stale_or_tampered');
}));

test('orphan type owners and invalid source or entry links reject before root I/O',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.types=['missing-entry'];save();put('src/ref.d.ts','/// <reference types="missing-source" />');
  const p=produce(options),s=provenance(p),forbidden={get root(){throw Error('must not read root');}};assert.equal(s.type_requests.length,2);
  const mutations=[q=>q.search_provenance.boundary_events.find(e=>e.owner?.channel==='type').owner.id=999,
    q=>q.search_provenance.type_batches.find(b=>b.origin==='source').size=99,
    q=>q.search_provenance.type_batches.find(b=>b.origin==='configured').from='project/not-inferred.ts',
    q=>q.search_provenance.type_requests.find(r=>r.origin==='source').batch=99,
    q=>q.search_provenance.type_requests.find(r=>r.origin==='source').index=99,
    q=>q.search_provenance.type_requests.find(r=>r.origin==='configured').index=99,
    q=>q.search_provenance.type_requests.find(r=>r.origin==='configured').from='project/not-inferred.ts'];
  for(const mutate of mutations){const q=structuredClone(p);mutate(q);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);}
}));

for(const origin of ['source','configured','automatic'])test(`omitting one duplicate ${origin} callback occurrence is structurally rejected`,()=>fixture(({put,config,save,options})=>{
  if(origin==='source')put('src/ref.d.ts','/// <reference types="missing" />\n/// <reference types="missing" />');
  if(origin==='configured'){config.compilerOptions.types=['missing','missing'];save();}
  if(origin==='automatic'){
    delete config.compilerOptions.types;config.compilerOptions.typeRoots=['./types-a','./types-b'];save();
    put('types-a/missing/index.d.ts','interface A {}');put('types-b/missing/index.d.ts','interface B {}');
  }
  const p=produce(options),s=provenance(p),rr=s.type_requests.filter(r=>r.origin===origin);assert.equal(rr.length,2);
  const q=structuredClone(p),remove=q.search_provenance.type_requests.findLastIndex(r=>r.origin===origin);
  q.search_provenance.type_requests.splice(remove,1);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);
}));

for(const origin of ['source','configured','automatic'])test(`double-digit ${origin} callback indices retain numeric order and index-10 integrity`,()=>fixture(({put,config,save,options})=>{
  const count=12,expected=Array.from({length:count},(_,index)=>index);
  if(origin==='source')put('src/ref.d.ts',expected.map(()=> '/// <reference types="missing" />').join('\n'));
  if(origin==='configured'){config.compilerOptions.types=expected.map(()=> 'missing');save();}
  if(origin==='automatic'){
    delete config.compilerOptions.types;config.compilerOptions.typeRoots=expected.map(index=>`./types-${index}`);save();
    for(const index of expected)put(`types-${index}/missing/index.d.ts`,`interface T${index} {}`);
  }
  const p=produce(options),s=batches(p),batch=s.type_batches.find(b=>b.origin===origin),rr=s.type_requests.filter(r=>r.origin===origin);
  assert(batch);assert.equal(batch.size,count);assert.deepEqual(rr.map(r=>r.index),expected);assert(rr.every(r=>r.batch===batch.id));
  const omitted=structuredClone(p),remove=omitted.search_provenance.type_requests.findIndex(r=>r.origin===origin&&r.index===10);
  omitted.search_provenance.type_requests.splice(remove,1);omitted.search_provenance.type_requests.forEach((r,index)=>r.id=index);
  assert.throws(()=>parsePacket(JSON.stringify(omitted)),/invalid_packet/);
  const swapped=structuredClone(p),requests=swapped.search_provenance.type_requests;
  const ten=requests.findIndex(r=>r.origin===origin&&r.index===10),eleven=requests.findIndex(r=>r.origin===origin&&r.index===11);
  [requests[ten],requests[eleven]]=[requests[eleven],requests[ten]];requests.forEach((r,index)=>r.id=index);
  assert.throws(()=>parsePacket(JSON.stringify(swapped)),/invalid_packet/);
}));

for(const origin of ['source','configured','automatic'])test(`a submitted duplicate ${origin} occurrence cannot masquerade as unprocessed`,()=>fixture(({put,config,save,options})=>{
  if(origin==='source')put('src/ref.d.ts','/// <reference types="missing" />\n/// <reference types="missing" />');
  if(origin==='configured'){config.compilerOptions.types=['missing','missing'];save();}
  if(origin==='automatic'){
    delete config.compilerOptions.types;config.compilerOptions.typeRoots=['./types-a','./types-b'];save();
    put('types-a/missing/junk.txt','');put('types-b/missing/junk.txt','');
  }
  const p=produce(options),s=provenance(p),rr=s.type_requests.filter(r=>r.origin===origin);assert.equal(rr.length,2);
  const q=structuredClone(p),remove=q.search_provenance.type_requests.findLastIndex(r=>r.origin===origin),request=q.search_provenance.type_requests[remove];
  const row=origin==='source'?q.type_lib_references.find(r=>r.kind==='types'&&r.request.file===request.from&&r.index===request.index)
    :q.type_lib_entries.find(r=>r.kind==='types'&&r.origin===origin&&r.index===request.index);
  assert.equal(row.reason,'unresolved');row.reason='unprocessed';q.search_provenance.type_requests.splice(remove,1);
  assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);
}));

test('noResolve source directives remain unprocessed without invented callbacks',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.noResolve=true;save();put('src/ref.d.ts','/// <reference types="./dep.d.ts" />');put('src/dep.d.ts','interface Dep {}');
  const p=produce(options),s=provenance(p),[row]=types(p);assert.equal(row.reason,'unprocessed');assert.equal(s.type_requests.filter(r=>r.origin==='source').length,0);
  assert.equal(s.type_batches.filter(b=>b.origin==='source').length,0);
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(p)));
}));

test('redirect-suppressed source directives remain unprocessed without borrowed callbacks',()=>fixture(({put,options})=>{
  for(const name of ['a','b']){
    put(`node_modules/${name}/package.json`,JSON.stringify({name,version:'1.0.0',types:'index.d.ts'}));put(`node_modules/${name}/index.d.ts`,'import "shared";export {};');
    put(`node_modules/${name}/node_modules/shared/package.json`,'{"name":"shared","version":"1.0.0","types":"index.d.ts"}');
    put(`node_modules/${name}/node_modules/shared/index.d.ts`,name==='a'?'export {};':'/// <reference types="missing-original" />\nexport {};');
  }
  put('src/app.ts','import "a";import "b";export {};');const p=produce(options),s=provenance(p);
  const from='project/node_modules/b/node_modules/shared/index.d.ts',rows=types(p).filter(r=>r.request.file===from);
  assert.equal(rows.length,1);assert.equal(rows[0].reason,'unprocessed');assert.equal(s.type_requests.filter(r=>r.from===from).length,0);assert.equal(s.type_batches.filter(b=>b.from===from).length,0);
}));

test('rootless configured type rows remain unprocessed without invented callbacks',()=>fixture(({put,config,save,options})=>{
  config.include=['absent'];config.compilerOptions.types=['fixture'];save();packageType(put);const p=produce(options),s=provenance(p),[row]=entries(p);
  assert.equal(p.snapshot.roots.length,0);assert.equal(row.reason,'unprocessed');assert.equal(s.type_requests.filter(r=>r.origin==='configured').length,0);assert.equal(s.type_batches.filter(b=>b.origin==='configured').length,0);
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(p)));
}));

test('historical schema13 with non-null module mode and unresolved schema12 remain readable',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';save();
  put('src/mode.mts','import "missing-module";');put('src/ref.d.ts','/// <reference types="missing-types" />');const p=produce(options),current=provenance(p);
  assert(current.module_requests.some(r=>r.mode==='import'));
  const old14=structuredClone(p);old14.schema='prism.callable-observation/14';old14.producer.version='0.15.0';delete old14.search_provenance.lib_searches;delete old14.config_provenance;
  for(const e of old14.search_provenance.boundary_events)if(e.owner?.channel==='lib')e.owner=null;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(old14)));const bad14=structuredClone(old14);bad14.search_provenance.type_batches[0].size++;
  assert.throws(()=>parsePacket(JSON.stringify(bad14)),/invalid_packet/);assert.equal(validate(JSON.stringify(old14),options).valid,false);
  const old13=structuredClone(old14);old13.schema='prism.callable-observation/13';old13.producer.version='0.14.0';delete old13.search_provenance.type_batches;delete old13.search_provenance.type_requests;delete old13.search_provenance.type_searches;
  for(const e of old13.search_provenance.boundary_events)if(e.owner?.channel==='type')e.owner=null;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(old13)));assert.equal(validate(JSON.stringify(old13),options).valid,false);
  const old12=structuredClone(old13);old12.schema='prism.callable-observation/12';old12.producer.version='0.13.0';delete old12.search_provenance;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(old12)));assert.equal(validate(JSON.stringify(old12),options).valid,false);
}));
