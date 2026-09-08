// S7 schema/worker contract REDs. The exact S6-equivalent baseline includes the
// accepted but unwired helper; producer RED occurs at absent field/digest seams.
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
const baseline=baselineImplementation?await import(url(baselineImplementation,'index.mjs','?s7-baseline')):null;
const schema=await import(url(implementation,'schema.mjs'));
assert.equal(schema.COMPILER_HASH,schema.hash(readFileSync(compiler)));

const baseConfig=(options={},top={})=>({compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',
  moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:false,noUncheckedSideEffectImports:true,...options},include:['src'],...top});
function fixture(config,files,run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-s7-integration-'));
  const put=(file,value)=>{const target=path.join(root,file);mkdirSync(path.dirname(target),{recursive:true});
    writeFileSync(target,typeof value==='string'?value:JSON.stringify(value));};
  try {
    put('package.json','{"type":"module"}');put('tsconfig.json',config);
    for(const [file,value] of Object.entries(files))put(file,value);
    return run({root,put,options:{root,compiler,config:'tsconfig.json'}});
  } finally {rmSync(root,{recursive:true,force:true});}
}
const oldProjection=packet=>{const q=structuredClone(packet);delete q.schema;delete q.producer;delete q.semantic_closure;return q;};
function pair(options) {
  const before=baseline?.produce(options),after=candidate.produce(options);
  if(before)assert.deepEqual(oldProjection(after),oldProjection(before),'S7 changed a pre-existing packet field');
  assert(!after.reasons.includes('worker_failed'),'worker fixture failed');return {before,after};
}
function semantic(packet) {
  assert(Object.hasOwn(packet,'semantic_closure'),'missing schema18 semantic_closure');return packet.semantic_closure;
}
const exactFiles=specifier=>({
  'src/app.ts':`import type {Client} from "${specifier}";type View<P>=(p:P)=>void;export const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};`,
  'src/provider.d.ts':`declare module "${specifier}" { export class Client { m(): void } }`,
});
function exactSetup(specifier,extraOptions={}) {
  return fixture(baseConfig(extraOptions),exactFiles(specifier),({options})=>{
    const packet=pair(options).after,[resolution]=packet.resolutions;
    assert.equal(packet.resolutions.length,1);assert.equal(resolution.specifier,specifier);assert.equal(resolution.target,null);
    assert.equal(resolution.lookup.status,'observed');assert.equal(resolution.lookup.reason,null);
    assert.equal(packet.config_provenance.status,'observed');assert.equal(packet.entry_obligations.complete,true);
    return {packet,options,resolution};
  });
}
const exactResult={policy:'prism.semantic-closure/exact-ambient-v1',complete:true,reasons:[],
  rows:[{index:0,disposition:'exact_ambient',reason:null}]};

for(const specifier of ['node:known','virtual:known'])test(`${specifier} exact ambient is semantically complete without baseUrl`,()=>{
  const {packet}=exactSetup(specifier);assert.equal(packet.search_provenance.boundary_events.length,0);
  assert.deepEqual(packet.reasons,['unresolved_module']);assert.equal(packet.status,'unproven');
  assert.equal(packet.authorizes_runtime_edge,false);assert.equal(packet.scope.class_authority,false);
  assert.equal(packet.observations[0].nested.calls[0].props_class.reason,'program_unproven');
  assert.deepEqual(semantic(packet),exactResult);
});

test('filesystem-selected local module is complete without reclassifying old closure',()=>fixture(baseConfig(),{
  'src/app.ts':'import {Client} from "./client";type View<P>=(p:P)=>void;export const run:View<{client:Client}>=({client})=>{client.m();};',
  'src/client.ts':'export class Client {m(){}}',
},({options})=>{
  const packet=pair(options).after,[resolution]=packet.resolutions;assert.equal(resolution.target,'project/src/client.ts');
  assert(packet.snapshot.program_files.includes(resolution.target));assert.equal(packet.search_provenance.boundary_events.length,0);
  assert.deepEqual(semantic(packet),{policy:'prism.semantic-closure/exact-ambient-v1',complete:true,reasons:[],
    rows:[{index:0,disposition:'filesystem_selected',reason:null}]});
}));

for(const specifier of ['known','@scope/known'])test(`${specifier} exact row cannot waive outside lookup barriers`,()=>{
  const {packet}=exactSetup(specifier);assert.equal(packet.snapshot.outside_lookups,true);
  assert(packet.search_provenance.boundary_events.some(row=>row.kind==='outside'));
  assert.deepEqual(semantic(packet),{policy:'prism.semantic-closure/exact-ambient-v1',complete:false,
    reasons:['boundary_encounter'],rows:[{index:0,disposition:'exact_ambient',reason:null}]});
});

test('refused exact binding remains globally blocked regardless of owner',()=>fixture(baseConfig({baseUrl:'.'}),exactFiles('virtual:input'),({options})=>{
  const packet=pair(options).after,[resolution]=packet.resolutions;assert.equal(resolution.lookup.status,'observed');
  assert(packet.snapshot.refused_lookup_sha256.length>0);assert(packet.search_provenance.boundary_events.some(row=>row.kind==='refused'));
  const result=semantic(packet);assert.equal(result.complete,false);assert.deepEqual(result.reasons,['boundary_encounter']);
  assert.deepEqual(result.rows,[{index:0,disposition:'exact_ambient',reason:null}]);
}));

test('unobserved config and incomplete S5 entries remain independent barriers',()=>fixture(baseConfig({types:null}),exactFiles('node:known'),({options})=>{
  const packet=pair(options).after;assert.equal(packet.config_provenance.status,'unproven');assert.equal(packet.entry_obligations.complete,false);
  const result=semantic(packet);assert.equal(result.complete,false);
  assert(result.reasons.includes('config_unproven'));assert(result.reasons.includes('entry_obligations_incomplete'));
}));

test('automatic discovery uncertainty blocks even with zero automatic names',()=>fixture(baseConfig(),exactFiles('node:known'),({put,options})=>{
  const config=baseConfig();delete config.compilerOptions.types;put('tsconfig.json',config);
  const packet=pair(options).after;assert.equal(packet.type_lib_entries.filter(row=>row.kind==='types').length,0);
  assert.deepEqual(packet.entry_obligations.reasons,['automatic_discovery_unproven']);
  assert(semantic(packet).reasons.includes('entry_obligations_incomplete'));
}));

test('explicit noResolve blocks an otherwise exact eligible occurrence',()=>{
  const {packet}=exactSetup('node:known',{noResolve:true});
  const noResolve=packet.config_provenance.options.find(row=>row.name==='noResolve');
  assert.equal(noResolve.value_sha256,schema.hash(schema.canonical({present:true,value:true})));
  assert.deepEqual(semantic(packet),{policy:'prism.semantic-closure/exact-ambient-v1',complete:false,reasons:['no_resolve'],
    rows:[{index:0,disposition:'exact_ambient',reason:null}]});
});

for(const [label,directive,oldReason,newReason] of [
  ['path','/// <reference path="./absent.d.ts" />','unproven_path_reference','required_path_unproven'],
  ['types','/// <reference types="missing-types" />','unproven_type_lib_reference','type_lib_unproven'],
  ['lib','/// <reference lib="missing-lib" />','unproven_type_lib_reference','type_lib_unproven'],
])test(`missing source ${label} obligation blocks exact ambient coverage`,()=>fixture(baseConfig(),{
  ...exactFiles('node:known'),'src/app.ts':`${directive}\n${exactFiles('node:known')['src/app.ts']}`,
},({options})=>{
  const packet=pair(options).after;assert(packet.reasons.includes(oldReason));assert.equal(packet.resolutions[0].lookup.status,'observed');
  const result=semantic(packet);assert.equal(result.complete,false);assert(result.reasons.includes(newReason));
}));

test('compiler diagnostics block exact ambient coverage without changing the row',()=>fixture(baseConfig(),{
  ...exactFiles('node:known'),'src/app.ts':exactFiles('node:known')['src/app.ts']+'\nconst wrong:string=1;',
},({options})=>{
  const packet=pair(options).after;assert(packet.diagnostics.length>0);assert(packet.reasons.includes('compiler_diagnostics'));
  const result=semantic(packet);assert(result.reasons.includes('compiler_diagnostics'));
  assert.deepEqual(result.rows,[{index:0,disposition:'exact_ambient',reason:null}]);
}));

for(const [label,config,files,oldReason,newReason] of [
  ['project reference',baseConfig({}, {references:[{path:'./referenced'}]}),{'referenced/tsconfig.json':baseConfig({composite:true},{files:[]})},'unsupported_references','unsupported_project_references'],
  ['plugin',baseConfig({plugins:[{name:'fixture-plugin'}]}),{},'unsupported_plugins','unsupported_plugins'],
])test(`${label} remains a strict global barrier`,()=>fixture(config,{...exactFiles('node:known'),...files},({options})=>{
  const packet=pair(options).after;assert(packet.reasons.includes(oldReason));
  const result=semantic(packet);assert.equal(result.complete,false);assert(result.reasons.includes(newReason));
}));

test('filesystem target outside Program is row-unproven even when inventoried',()=>fixture(baseConfig({baseUrl:'.',paths:{'virtual:input':['src/value.js']}}),{
  'src/app.ts':'import {value} from "virtual:input";export {value};','src/value.js':'export const value=1;',
},({options})=>{
  const packet=pair(options).after,[resolution]=packet.resolutions;assert.equal(resolution.target,'project/src/value.js');
  assert(packet.snapshot.files.some(row=>row.id===resolution.target));assert(!packet.snapshot.program_files.includes(resolution.target));
  const result=semantic(packet);assert.equal(result.complete,false);assert(result.reasons.includes('resolution_unproven'));
  assert.deepEqual(result.rows,[{index:0,disposition:'unproven',reason:'target_not_in_program'}]);
}));

test('exact provider cannot discharge an unrelated eventless missing occurrence',()=>fixture(baseConfig(),{
  ...exactFiles('node:known'),'src/app.ts':exactFiles('node:known')['src/app.ts']+'\nimport type {} from "node:missing";',
},({options})=>{
  const packet=pair(options).after;assert.equal(packet.search_provenance.boundary_events.length,0);assert.equal(packet.resolutions.length,2);
  assert(packet.resolutions.some(row=>row.lookup.status==='observed'));assert(packet.resolutions.some(row=>row.lookup.reason==='unresolved_symbol'));
  const result=semantic(packet);assert.equal(result.complete,false);assert(result.reasons.includes('resolution_unproven'));
  assert.deepEqual(result.rows.map(row=>[row.disposition,row.reason]),[['exact_ambient',null],['unproven','unresolved_module']]);
}));

for(const [label,extraFiles,source,count] of [
  ['duplicate',{'src/second.d.ts':'declare module "node:known" {export class Other {}}'},exactFiles('node:known')['src/app.ts'],1],
  ['augmentation',{'src/augment.d.ts':'export {};declare module "node:known" {interface Client {x:number}}'},exactFiles('node:known')['src/app.ts'],2],
  ['unsupported context',{},'const lazy=import("node:known");',1],
])test(`${label} exact evidence remains unresolved`,()=>fixture(baseConfig(),{
  ...exactFiles('node:known'),...extraFiles,'src/app.ts':source,
},({options})=>{
  const packet=pair(options).after,[resolution]=packet.resolutions;assert.equal(resolution.target,null);assert.equal(resolution.lookup.status,'unproven');
  const result=semantic(packet);assert.equal(result.complete,false);assert(result.reasons.includes('resolution_unproven'));
  assert.equal(packet.resolutions.length,count);assert.deepEqual(result.rows,
    Array.from({length:count},(_,index)=>({index,disposition:'unproven',reason:'unresolved_module'})));
}));

test('empty/refusal packet retains every strict prerequisite barrier',()=>fixture(baseConfig(),exactFiles('node:known'),({options})=>{
  const selected={...options,compiler:path.join(options.root,'missing-typescript.js')},packet=pair(selected).after;
  assert.equal(packet.compiler.verified,false);assert.equal(packet.closure.stable_snapshot,false);
  assert.equal(packet.config_provenance.status,'unproven');assert.equal(packet.entry_obligations.complete,false);
  assert.deepEqual(semantic(packet),{policy:'prism.semantic-closure/exact-ambient-v1',complete:false,
    reasons:['compiler_unverified','unstable_snapshot','config_unproven','entry_obligations_incomplete','global_refusal'],rows:[]});
}));

test('schema18 recomputes exact rows/reasons and rejects same-genuine row swaps before root I/O',()=>fixture(baseConfig(),{
  ...exactFiles('node:known'),'src/client.ts':'export class Local {}',
  'src/app.ts':exactFiles('node:known')['src/app.ts']+'\nimport {Local} from "./client";',
},({options})=>{
  const packet=pair(options).after,result=semantic(packet);assert.equal(result.rows.length,2);
  assert.deepEqual(new Set(result.rows.map(row=>row.disposition)),new Set(['exact_ambient','filesystem_selected']));
  const mutations=[q=>delete q.semantic_closure,q=>q.semantic_closure.complete=false,
    q=>q.semantic_closure.reasons.push('resolution_unproven'),q=>q.semantic_closure.rows.pop(),
    q=>{const rows=q.semantic_closure.rows;[rows[0].disposition,rows[1].disposition]=[rows[1].disposition,rows[0].disposition];},
    q=>q.semantic_closure.rows[0].index=99];
  for(const mutate of mutations) {
    const forged=structuredClone(packet);mutate(forged);assert.throws(()=>schema.parsePacket(JSON.stringify(forged)),/invalid_packet/);
    let reads=0;assert.equal(candidate.validate(JSON.stringify(forged),{get root(){reads++;throw Error('forbidden');}}).valid,false);assert.equal(reads,0);
  }
}));

test('source change leaves a shaped packet readable but fails full reproduction',()=>{
  const files=exactFiles('node:known');fixture(baseConfig(),files,({put,options})=>{
    const packet=pair(options).after;semantic(packet);put('src/provider.d.ts',files['src/provider.d.ts'].replace('Client','Changed'));
    assert.doesNotThrow(()=>schema.parsePacket(JSON.stringify(packet)));
    assert.equal(candidate.validate(JSON.stringify(packet),options).reason,'stale_or_tampered');
  });
});

test('schema10 through schema17 remain readable without invented semantic closure',()=>fixture(baseConfig(),exactFiles('node:known'),({options})=>{
  const current=pair(options).after;
  const p17=structuredClone(current);p17.schema='prism.callable-observation/17';p17.producer.version='0.18.0';delete p17.semantic_closure;
  const p16=structuredClone(p17);p16.schema='prism.callable-observation/16';p16.producer.version='0.17.0';delete p16.entry_obligations;
  const p15=structuredClone(p16);p15.schema='prism.callable-observation/15';p15.producer.version='0.16.0';delete p15.config_provenance;
  const p14=structuredClone(p15);p14.schema='prism.callable-observation/14';p14.producer.version='0.15.0';delete p14.search_provenance.lib_searches;
  for(const event of p14.search_provenance.boundary_events)if(event.owner?.channel==='lib')event.owner=null;
  const p13=structuredClone(p14);p13.schema='prism.callable-observation/13';p13.producer.version='0.14.0';
  delete p13.search_provenance.type_batches;delete p13.search_provenance.type_requests;delete p13.search_provenance.type_searches;
  for(const event of p13.search_provenance.boundary_events)if(event.owner?.channel==='type')event.owner=null;
  const p12=structuredClone(p13);p12.schema='prism.callable-observation/12';p12.producer.version='0.13.0';delete p12.search_provenance;
  const p11=structuredClone(p12);p11.schema='prism.callable-observation/11';p11.producer.version='0.12.0';delete p11.type_lib_entries;
  const p10=structuredClone(p11);p10.schema='prism.callable-observation/10';p10.producer.version='0.11.1';delete p10.type_lib_references;
  for(const packet of [p10,p11,p12,p13,p14,p15,p16,p17]) {
    const parsed=schema.parsePacket(JSON.stringify(packet));assert.equal(Object.hasOwn(parsed,'semantic_closure'),false);
  }
}));

test('producer digest includes semantic-closure helper bytes once wired',async()=>{
  const root=mkdtempSync(path.join(tmpdir(),'prism-s7-digest-'));
  try {
    const copied=path.join(root,'implementation');cpSync(implementation,copied,{recursive:true});
    appendFileSync(path.join(copied,'semantic-closure.mjs'),'\n// digest mutation\n');
    const changed=await import(url(copied,'index.mjs','?s7-digest'));
    assert.notEqual(changed.producerHash(),candidate.producerHash());
  } finally {rmSync(root,{recursive:true,force:true});}
});
