import test from 'node:test';
import assert from 'node:assert/strict';
import {appendFileSync,cpSync,mkdtempSync,mkdirSync,readFileSync,rmSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';

const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION??path.dirname(fileURLToPath(import.meta.url));
const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'PRISM_TYPESCRIPT is required');
const url=file=>pathToFileURL(path.join(implementation,file)).href+'?s8-red';
const candidate=await import(url('index.mjs'));
const schema=await import(url('schema.mjs'));
assert.equal(schema.COMPILER_HASH,schema.hash(readFileSync(compiler)));

const V1='prism.semantic-closure/exact-ambient-v1';
const V2='prism.semantic-closure/singleton-wildcard-v2';
const baseConfig={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',
  moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:false,
  noUncheckedSideEffectImports:true},include:['src']};

function fixture({specifier='./file.asset',pattern='*.asset',asset=false},run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-s8-red-'));
  const put=(file,value)=>{const target=path.join(root,file);mkdirSync(path.dirname(target),{recursive:true});
    writeFileSync(target,typeof value==='string'?value:JSON.stringify(value));};
  try {
    put('package.json','{"type":"module"}');put('tsconfig.json',baseConfig);
    put('src/provider.d.ts',`declare module "${pattern}" {export class Client {m():void}}`);
    put('src/app.ts',`import type {Client} from "${specifier}";type View<P>=(p:P)=>void;
      export const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};`);
    if(asset)put(`src/${specifier.slice(2)}`,'asset bytes');
    const packet=candidate.produce({root,compiler,config:'tsconfig.json'});
    assert(!packet.reasons.includes('worker_failed'),'fixture failed before semantic classification');
    const rows=packet.resolutions.filter(row=>row.specifier===specifier);
    assert.equal(rows.length,1,'expected one real source request occurrence');
    const [resolution]=rows;
    assert.equal(resolution.target,null);
    assert.equal(resolution.lookup.status,'unproven');
    assert.equal(resolution.lookup.reason,'non_exact_binding');
    assert.equal(resolution.lookup.wildcard?.status,'observed');
    assert.equal(resolution.lookup.wildcard?.reason,null);
    assert.equal(resolution.lookup.merged_wildcard,null);
    assert.equal(packet.search_provenance.boundary_events.length,0);
    assert.equal(packet.snapshot.outside_lookups,false);
    assert.equal(packet.snapshot.refused_lookup_sha256.length,0);
    assert.equal(packet.diagnostics.length,0);
    assert.equal(packet.config_provenance.status,'observed');
    assert.equal(packet.entry_obligations.complete,true);
    assert.deepEqual(packet.reasons,['unresolved_module']);
    assert.equal(packet.authorizes_runtime_edge,false);
    assert.equal(packet.scope.class_authority,false);
    return run({packet,resolution});
  } finally {rmSync(root,{recursive:true,force:true});}
}

function project(config,files,run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-s8-control-'));
  const put=(file,value)=>{const target=path.join(root,file);mkdirSync(path.dirname(target),{recursive:true});
    writeFileSync(target,typeof value==='string'?value:JSON.stringify(value));};
  try {
    put('package.json','{"type":"module"}');put('tsconfig.json',config);
    for(const [file,value] of Object.entries(files))put(file,value);
    const options={root,compiler,config:'tsconfig.json'},packet=candidate.produce(options);
    assert(!packet.reasons.includes('worker_failed'),'control fixture failed');
    return run({packet,options,put});
  } finally {rmSync(root,{recursive:true,force:true});}
}

test('historical schema18 is always recomputed under exact-ambient-v1',()=>fixture({},({packet})=>{
  const historical=structuredClone(packet);
  historical.schema='prism.callable-observation/18';historical.producer.version='0.19.0';
  historical.producer.sha256='b75dbd6dc5fdb1f083d134fa132a2b89e2091f14f0c69f6f245149d2ad4def2e';
  historical.semantic_closure={policy:V1,complete:false,reasons:['resolution_unproven'],
    rows:[{index:0,disposition:'unproven',reason:'unadmitted_binding'}]};
  assert.deepEqual(schema.parsePacket(JSON.stringify(historical)).semantic_closure,historical.semantic_closure);
  historical.semantic_closure=packet.semantic_closure;
  assert.throws(()=>schema.parsePacket(JSON.stringify(historical)),/invalid_packet/);
}));

for(const selected of [
  {label:'absent asset',input:{}},
  {label:'present asset',input:{asset:true}},
  {label:'query specifier',input:{specifier:'./file.asset?raw',pattern:'*.asset?raw'}},
])test(`S8 promotes an observed singleton wildcard with ${selected.label}`,()=>fixture(selected.input,({packet})=>{
  assert.deepEqual(packet.semantic_closure.rows,[{index:0,disposition:'singleton_wildcard',reason:null}]);
  assert.equal(packet.semantic_closure.policy,V2);
  assert.equal(packet.semantic_closure.complete,true);
  assert.deepEqual(packet.semantic_closure.reasons,[]);
}));

test('exact ambient binding is not rewritten as singleton wildcard',()=>project(baseConfig,{
  'src/provider.d.ts':'declare module "node:known" {export interface Client {m():void}}',
  'src/app.ts':'import type {Client} from "node:known";export type Seen=Client;',
},({packet})=>{
  assert.deepEqual(packet.semantic_closure.rows,[{index:0,disposition:'exact_ambient',reason:null}]);
  assert.equal(packet.resolutions[0].lookup.wildcard,null);
}));

for(const selected of [
  {label:'duplicate provider',extra:{'src/second.d.ts':'declare module "*.asset" {export interface Client {x:number}}'},reason:'duplicate_provider'},
  {label:'augmentation',extra:{'src/augment.d.ts':'export {};declare module "*.asset" {interface Client {x:number}}'},reason:'augmentation'},
  {label:'competing selected pattern',extra:{'src/second.d.ts':'declare module "./file.*" {export interface Client {x:number}}'},reason:'competing_pattern'},
])test(`${selected.label} remains unresolved`,()=>project(baseConfig,{
  'src/provider.d.ts':'declare module "*.asset" {export interface Client {m():void}}',
  'src/app.ts':'import type {Client} from "./file.asset";export type Seen=Client;',...selected.extra,
},({packet})=>{
  const [resolution]=packet.resolutions;assert.equal(resolution.lookup.wildcard.status,'unproven');
  assert.equal(resolution.lookup.wildcard.reason,selected.reason);
  assert.deepEqual(packet.semantic_closure.rows,packet.resolutions.map((_,index)=>
    ({index,disposition:'unproven',reason:'unresolved_module'})));
  assert(packet.semantic_closure.reasons.includes('resolution_unproven'));
}));

test('unsupported dynamic-import context remains unresolved',()=>project(baseConfig,{
  'src/provider.d.ts':'declare module "*.asset" {export const value:string}',
  'src/app.ts':'export const lazy=import("./file.asset");',
},({packet})=>{
  const [resolution]=packet.resolutions;assert.equal(resolution.lookup.context,'dynamic_import');
  assert.equal(resolution.lookup.wildcard.status,'unproven');assert.equal(resolution.lookup.wildcard.reason,'unsupported_request');
  assert.deepEqual(packet.semantic_closure.rows,[{index:0,disposition:'unproven',reason:'unresolved_module'}]);
}));

test('merged side-effect pair remains unadmitted',()=>project(baseConfig,{
  'src/a.d.ts':'declare module "*.asset" {}','src/b.d.ts':'declare module "*.asset";',
  'src/app.ts':'import "./file.asset";',
},({packet})=>{
  const [resolution]=packet.resolutions;assert.equal(resolution.lookup.wildcard.reason,'duplicate_provider');
  assert.equal(resolution.lookup.merged_wildcard.status,'observed');
  assert.deepEqual(packet.semantic_closure.rows,[{index:0,disposition:'unproven',reason:'unadmitted_binding'}]);
}));

test('one unrelated unresolved request keeps the aggregate unproven',()=>project(baseConfig,{
  'src/provider.d.ts':'declare module "*.asset" {export interface Client {m():void}}',
  'src/app.ts':'import type {Client} from "./file.asset";import type {} from "node:missing";export type Seen=Client;',
},({packet})=>{
  const bySpecifier=new Map(packet.resolutions.map((row,index)=>[row.specifier,packet.semantic_closure.rows[index]]));
  assert.equal(packet.resolutions.find(row=>row.specifier==='./file.asset').lookup.wildcard.status,'observed');
  assert.deepEqual(bySpecifier.get('node:missing'),{index:1,disposition:'unproven',reason:'unresolved_module'});
  assert(packet.semantic_closure.reasons.includes('resolution_unproven'));
}));

test('outside search boundary remains an independent global barrier',()=>project(baseConfig,{
  'src/provider.d.ts':'declare module "*.asset" {export interface Client {m():void}}',
  'src/app.ts':'import type {Client} from "./file.asset";import type {} from "plain-missing";export type Seen=Client;',
},({packet})=>{
  assert(packet.search_provenance.boundary_events.some(row=>row.kind==='outside'));
  assert(packet.semantic_closure.reasons.includes('boundary_encounter'));
}));

test('refused search boundary remains an independent global barrier',()=>project({
  ...baseConfig,compilerOptions:{...baseConfig.compilerOptions,baseUrl:'.'},
},{
  'src/provider.d.ts':'declare module "*.asset" {export interface Client {m():void}}\ndeclare module "virtual:known" {}',
  'src/app.ts':'import type {Client} from "./file.asset";import "virtual:known";export type Seen=Client;',
},({packet})=>{
  assert(packet.search_provenance.boundary_events.some(row=>row.kind==='refused'));
  assert(packet.semantic_closure.reasons.includes('boundary_encounter'));
}));

test('filesystem target outside Program remains row-unproven',()=>project({
  ...baseConfig,compilerOptions:{...baseConfig.compilerOptions,baseUrl:'.',paths:{'virtual:input':['src/value.js']}},
},{
  'src/app.ts':'import {value} from "virtual:input";export {value};','src/value.js':'export const value=1;',
},({packet})=>{
  const [resolution]=packet.resolutions;assert.equal(resolution.target,'project/src/value.js');
  assert(!packet.snapshot.program_files.includes(resolution.target));
  assert.deepEqual(packet.semantic_closure.rows,[{index:0,disposition:'unproven',reason:'target_not_in_program'}]);
}));

test('current packet is schema19 producer 0.20.0 with fixed v2 policy',()=>fixture({},({packet})=>{
  assert.equal(packet.schema,'prism.callable-observation/19');assert.equal(packet.producer.version,'0.20.0');
  assert.equal(packet.semantic_closure.policy,V2);assert.doesNotThrow(()=>schema.parsePacket(JSON.stringify(packet)));
}));

test('schema19 rejects same-genuine semantic row substitutions before root I/O',()=>project(baseConfig,{
  'src/provider.d.ts':'declare module "*.asset" {export interface Client {m():void}}',
  'src/client.ts':'export interface Local {x:number}',
  'src/app.ts':'import type {Client} from "./file.asset";import type {Local} from "./client";export type Seen=Client|Local;',
},({packet})=>{
  assert.deepEqual(new Set(packet.semantic_closure.rows.map(row=>row.disposition)),new Set(['singleton_wildcard','filesystem_selected']));
  const forged=structuredClone(packet),rows=forged.semantic_closure.rows;
  [rows[0].disposition,rows[1].disposition]=[rows[1].disposition,rows[0].disposition];
  assert.throws(()=>schema.parsePacket(JSON.stringify(forged)),/invalid_packet/);
  let reads=0;assert.equal(candidate.validate(JSON.stringify(forged),{get root(){reads++;throw Error('forbidden');}}).valid,false);
  assert.equal(reads,0);
}));

test('singleton provider source changes fail full reproduction',()=>fixture({},({packet})=>project(baseConfig,{
  'src/provider.d.ts':'declare module "*.asset" {export interface Client {m():void}}',
  'src/app.ts':'import type {Client} from "./file.asset";export type Seen=Client;',
},({options,put})=>{
  const fresh=candidate.produce(options);assert.equal(fresh.semantic_closure.rows[0].disposition,'singleton_wildcard');
  put('src/provider.d.ts','declare module "*.other" {export interface Client {m():void}}');
  assert.equal(candidate.validate(JSON.stringify(fresh),options).reason,'stale_or_tampered');
  assert.equal(packet.semantic_closure.rows[0].disposition,'singleton_wildcard');
})));

test('producer digest includes v2 helper while v1 bytes remain frozen',async()=>{
  assert.equal(schema.hash(readFileSync(path.join(implementation,'semantic-closure.mjs'))),
    '1ac30091a62bc03e267f3ff55c906a5e26db06dfc937752dfcabf6bbeb20e4b4');
  const root=mkdtempSync(path.join(tmpdir(),'prism-s8-digest-'));
  try {
    const copied=path.join(root,'implementation');cpSync(implementation,copied,{recursive:true});
    appendFileSync(path.join(copied,'semantic-closure-v2.mjs'),'\n// digest mutation\n');
    const changed=await import(pathToFileURL(path.join(copied,'index.mjs')).href+'?s8-digest');
    assert.notEqual(changed.producerHash(),candidate.producerHash());
  } finally {rmSync(root,{recursive:true,force:true});}
});
