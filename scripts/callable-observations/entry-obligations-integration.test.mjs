// S5 worker/schema contract REDs. Run first against the exact frozen schema16
// producer. The accepted pure-helper tests are compatibility evidence only.
import test from 'node:test';
import assert from 'node:assert/strict';
import {appendFileSync,cpSync,mkdtempSync,mkdirSync,readFileSync,rmSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';

const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION??path.dirname(fileURLToPath(import.meta.url));
const baselineImplementation=process.env.PRISM_CALLABLE_BASELINE;
const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'PRISM_TYPESCRIPT is required');
const url=(root,file,tag='')=>pathToFileURL(path.join(root,file)).href+tag;
const candidate=await import(url(implementation,'index.mjs'));
const baseline=baselineImplementation?await import(url(baselineImplementation,'index.mjs','?s5-baseline')):null;
const schema=await import(url(implementation,'schema.mjs'));
assert.equal(schema.COMPILER_HASH,schema.hash(readFileSync(compiler)));

function fixture(config,run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-s5-integration-'));
  const put=(file,value)=>{const target=path.join(root,file);mkdirSync(path.dirname(target),{recursive:true});
    writeFileSync(target,typeof value==='string'?value:JSON.stringify(value));};
  try {
    put('tsconfig.json',config);put('src/app.ts','export {};');
    return run({root,put,options:{root,compiler,config:'tsconfig.json'}});
  } finally {rmSync(root,{recursive:true,force:true});}
}

const baseOptions=(compilerOptions={},top={})=>({compilerOptions:{strict:true,noEmit:true,target:'ES2022',
  module:'ESNext',moduleResolution:'Node',libReplacement:false,skipLibCheck:true,...compilerOptions},files:['src/app.ts'],...top});
const oldProjection=packet=>{const q=structuredClone(packet);delete q.schema;delete q.producer;delete q.entry_obligations;return q;};
function pair(options) {
  const before=baseline?.produce(options),after=candidate.produce(options);
  if(before)assert.deepEqual(oldProjection(after),oldProjection(before),'S5 changed a pre-existing packet field');
  assert(!after.reasons.includes('worker_failed'),'worker fixture failed');
  return {before,after};
}
function obligations(packet) {
  assert(Object.hasOwn(packet,'entry_obligations'),'missing schema17 entry_obligations');
  return packet.entry_obligations;
}
const concise=row=>[row.kind,row.origin,row.index,row.disposition,row.reason];
const oldRows=packet=>packet.type_lib_entries.map(row=>[row.kind,row.origin,row.index,row.status,row.reason]);

test('rootless configured type and lib rows are disabled only by no_roots',()=>fixture(
  baseOptions({types:['fixture'],lib:['es5']},{include:['absent'],files:undefined}),({put,options})=>{
    put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');
    const {after}=pair(options);assert.equal(after.snapshot.roots.length,0);
    assert.deepEqual(oldRows(after),[
      ['lib','configured',0,'unproven','unprocessed'],['types','configured',0,'unproven','unprocessed']]);
    assert.equal(after.config_provenance.status,'unproven');
    assert.deepEqual(obligations(after),{complete:false,reasons:['configuration_unproven'],rows:[
      {kind:'lib',origin:'configured',index:0,disposition:'disabled',reason:'no_roots'},
      {kind:'types',origin:'configured',index:0,disposition:'disabled',reason:'no_roots'}]});
  }));

test('configured duplicate types/libs stay ordered selected despite non-disabling options',()=>fixture(
  baseOptions({types:['fixture','fixture'],lib:['es5','es5'],noResolve:true,noCheck:true}),({put,options})=>{
    put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');
    const {after}=pair(options),old=after.type_lib_entries;assert.equal(old.length,4);
    assert(old.every(row=>row.status==='observed'));
    const result=obligations(after);assert.equal(result.complete,true);assert.deepEqual(result.reasons,[]);
    assert.deepEqual(result.rows,old.map(({kind,origin,index})=>({kind,origin,index,disposition:'selected',reason:null})));
  }));

test('automatic names present and absent both retain automatic-discovery uncertainty',()=>{
  for(const present of [true,false])fixture(baseOptions({lib:[]}),({put,options})=>{
    if(present)put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');
    const {after}=pair(options),types=after.type_lib_entries.filter(row=>row.kind==='types');
    assert.equal(types.length,present?1:0);if(present)assert.equal(types[0].status,'observed');
    assert.deepEqual(obligations(after),{complete:false,reasons:['automatic_discovery_unproven'],
      rows:types.map(({kind,origin,index})=>({kind,origin,index,disposition:'selected',reason:null}))});
  });
});

test('observed default library is a selected obligation without duplicating transitive libs',()=>fixture(
  baseOptions({types:[]}),({options})=>{
    const {after}=pair(options),entries=after.type_lib_entries;assert.equal(entries.length,1);
    assert.deepEqual(oldRows(after),[['lib','default',0,'observed',null]]);
    assert.deepEqual(obligations(after),{complete:true,reasons:[],rows:[
      {kind:'lib',origin:'default',index:0,disposition:'selected',reason:null}]});
  }));

test('proven noLib disables an unprocessed default even with source-backed builtin membership',()=>fixture(
  baseOptions({types:[],noLib:true}),({put,options})=>{
    put('src/app.ts','/// <reference path="../../compiler/lib.es2022.full.d.ts" />\nexport {};');
    const {after}=pair(options),[entry]=after.type_lib_entries;assert.equal(entry.reason,'unprocessed');
    assert(after.snapshot.program_files.includes('compiler/lib.es2022.full.d.ts'));
    const noLib=after.config_provenance.options.find(row=>row.name==='noLib');
    assert.equal(noLib.value_sha256,schema.hash(schema.canonical({present:true,value:true})));
    assert.deepEqual(obligations(after),{complete:true,reasons:[],rows:[
      {kind:'lib',origin:'default',index:0,disposition:'disabled',reason:'no_lib'}]});
  }));

test('source no-default-lib suppression stays unproven and never borrows membership as intent',()=>fixture(
  baseOptions({types:[]}),({put,options})=>{
    put('src/app.ts','/// <reference no-default-lib="true" />\n/// <reference path="../../compiler/lib.es2022.full.d.ts" />\nexport {};');
    const {after}=pair(options),[entry]=after.type_lib_entries;assert.equal(entry.reason,'unprocessed');
    assert(after.snapshot.program_files.includes('compiler/lib.es2022.full.d.ts'));
    assert.deepEqual(obligations(after),{complete:false,reasons:['unproven_entry'],rows:[
      {kind:'lib',origin:'default',index:0,disposition:'unproven',reason:'suppression_unproven'}]});
  }));

test('explicit empty types and libs prove an empty entry population',()=>fixture(
  baseOptions({types:[],lib:[]}),({options})=>{
    const {after}=pair(options);assert.deepEqual(after.type_lib_entries,[]);
    assert.deepEqual(obligations(after),{complete:true,reasons:[],rows:[]});
  }));

test('missing configured type retains unresolved rather than becoming disabled',()=>fixture(
  baseOptions({types:['missing'],lib:[]}),({options})=>{
    const {after}=pair(options),[entry]=after.type_lib_entries;assert.equal(entry.reason,'unresolved');
    assert(after.diagnostics.some(row=>row.code===2688));
    assert.deepEqual(obligations(after),{complete:false,reasons:['unproven_entry'],rows:[
      {kind:'types',origin:'configured',index:0,disposition:'unproven',reason:'unresolved'}]});
  }));

test('configured library target and inclusion failures retain their old reason labels',()=>{
  fixture(baseOptions({types:[],lib:['dom']}),({options})=>{
    const copied=mkdtempSync(path.join(tmpdir(),'prism-s5-compiler-'));
    try {
      cpSync(path.dirname(compiler),copied,{recursive:true,filter:file=>path.basename(file)!=='lib.dom.d.ts'});
      const selected={...options,compiler:path.join(copied,path.basename(compiler))};
      const {after}=pair(selected),[entry]=after.type_lib_entries;assert.equal(entry.reason,'target_not_in_program');
      assert.equal(obligations(after).rows[0].reason,'target_not_in_program');
    } finally {rmSync(copied,{recursive:true,force:true});}
  });
  fixture(baseOptions({types:[],lib:['es5']}),({put,options})=>{
    put('src/app.ts','/// <reference no-default-lib="true" />\n/// <reference lib="es5" />\nexport {};');
    const {after}=pair(options),[entry]=after.type_lib_entries;assert.equal(entry.reason,'missing_inclusion');
    assert.equal(obligations(after).rows[0].reason,'missing_inclusion');
  });
});

test('schema10 through schema16 history remains readable without invented obligations',()=>fixture(
  baseOptions({types:[],lib:['es5']}),({options})=>{
    const current=pair(options).after;
    const p16=structuredClone(current);p16.schema='prism.callable-observation/16';p16.producer.version='0.17.0';delete p16.entry_obligations;delete p16.semantic_closure;
    const p15=structuredClone(p16);p15.schema='prism.callable-observation/15';p15.producer.version='0.16.0';delete p15.config_provenance;
    const p14=structuredClone(p15);p14.schema='prism.callable-observation/14';p14.producer.version='0.15.0';delete p14.search_provenance.lib_searches;
    for(const event of p14.search_provenance.boundary_events)if(event.owner?.channel==='lib')event.owner=null;
    const p13=structuredClone(p14);p13.schema='prism.callable-observation/13';p13.producer.version='0.14.0';
    delete p13.search_provenance.type_batches;delete p13.search_provenance.type_requests;delete p13.search_provenance.type_searches;
    for(const event of p13.search_provenance.boundary_events)if(event.owner?.channel==='type')event.owner=null;
    const p12=structuredClone(p13);p12.schema='prism.callable-observation/12';p12.producer.version='0.13.0';delete p12.search_provenance;
    const p11=structuredClone(p12);p11.schema='prism.callable-observation/11';p11.producer.version='0.12.0';delete p11.type_lib_entries;
    const p10=structuredClone(p11);p10.schema='prism.callable-observation/10';p10.producer.version='0.11.1';delete p10.type_lib_references;
    for(const packet of [p10,p11,p12,p13,p14,p15,p16]) {
      const parsed=schema.parsePacket(JSON.stringify(packet));assert.equal(Object.hasOwn(parsed,'entry_obligations'),false);
    }
  }));

test('schema17 parser recomputes rows, reasons and completeness before root I/O',()=>fixture(
  baseOptions({types:['fixture'],lib:['es5']}),({put,options})=>{
    put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');
    const packet=pair(options).after,result=obligations(packet);assert.equal(result.rows.length,2);
    const mutations=[
      q=>delete q.entry_obligations,
      q=>q.entry_obligations.complete=false,
      q=>q.entry_obligations.reasons.push('unproven_entry'),
      q=>q.entry_obligations.rows.pop(),
      q=>q.entry_obligations.rows.reverse(),
      q=>Object.assign(q.entry_obligations.rows[0],{disposition:'disabled',reason:'no_lib'}),
    ];
    for(const mutate of mutations) {
      const forged=structuredClone(packet);mutate(forged);assert.throws(()=>schema.parsePacket(JSON.stringify(forged)),/invalid_packet/);
      let reads=0;assert.equal(candidate.validate(JSON.stringify(forged),{get root(){reads++;throw Error('forbidden');}}).valid,false);assert.equal(reads,0);
    }
  }));

test('same-genuine config value substitution parses but fails full reproduction',()=>fixture(
  baseOptions({types:['fixture'],lib:['es5']}),({put,options})=>{
    put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');
    const packet=pair(options).after;obligations(packet);
    const rows=packet.config_provenance.options,types=rows.find(row=>row.name==='types'),lib=rows.find(row=>row.name==='lib');
    assert.notEqual(types.value_sha256,lib.value_sha256);
    const forged=structuredClone(packet),forgedRows=forged.config_provenance.options;
    [forgedRows.find(row=>row.name==='types').value_sha256,forgedRows.find(row=>row.name==='lib').value_sha256]=
      [lib.value_sha256,types.value_sha256];
    assert.doesNotThrow(()=>schema.parsePacket(JSON.stringify(forged)));
    assert.equal(candidate.validate(JSON.stringify(forged),options).reason,'stale_or_tampered');
  }));

test('producer digest includes the accepted entry-obligations helper bytes',async()=>{
  const root=mkdtempSync(path.join(tmpdir(),'prism-s5-digest-'));
  try {
    const copied=path.join(root,'implementation');cpSync(implementation,copied,{recursive:true});
    appendFileSync(path.join(copied,'entry-obligations.mjs'),'\n// digest mutation\n');
    const changed=await import(url(copied,'index.mjs','?s5-digest'));
    assert.notEqual(changed.producerHash(),candidate.producerHash());
  } finally {rmSync(root,{recursive:true,force:true});}
});
