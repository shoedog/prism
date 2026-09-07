// Characterization of independent closure obligations. No closure policy change.
import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,rmSync,readFileSync} from 'node:fs';import {tmpdir} from 'node:os';import path from 'node:path';import {createRequire} from 'node:module';
import {produce,validate} from './index.mjs';import {hash,COMPILER_HASH} from './schema.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);
const app='class Client{m(){}}type View<P>=(p:P)=>void;const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-closure-policy-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:true,noUncheckedSideEffectImports:true},include:['src']};
  const save=()=>put('tsconfig.json',JSON.stringify(config));
  try{put('package.json','{"type":"module"}');save();put('src/a.d.ts','declare module "*.scss" {}');put('src/b.d.ts','declare module "*.scss";');put('src/app.ts','import "./style.scss";'+app);return run({put,config,save,options:{root,compiler,config:'tsconfig.json'}});}
  finally{rmSync(root,{recursive:true,force:true});}}
const observed=r=>r.lookup.status==='observed'||r.lookup.wildcard?.status==='observed'||r.lookup.merged_wildcard?.status==='observed';
function withheld(p){assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);assert.equal(p.status,'unproven');assert.equal(p.closure.resolution,false);
  assert(p.observations.some(o=>o.nested.calls.some(c=>c.props_class.reason==='program_unproven')));}
for(const exists of [false,true])test(`all requests source-observed still do not admit closure: asset=${exists}`,()=>fixture(({put,options})=>{
  if(exists)put('src/style.scss','.style {}');const p=produce(options);assert.equal(p.resolutions.length,1);assert(p.resolutions.every(observed));
  assert.deepEqual(p.diagnostics,[]);assert.equal(p.resolutions[0].target,null);assert.equal(p.snapshot.files.some(f=>f.id==='project/src/style.scss'),exists);withheld(p);
}));
test('a source-observed pair cannot erase a skipped declaration dependency',()=>fixture(({put,options})=>{
  put('src/peer.d.ts','import {Missing} from "missing-peer";export type T=Missing;');const p=produce(options);
  assert(p.resolutions.some(observed));const gap=p.resolutions.find(r=>r.specifier==='missing-peer');assert(gap);assert.equal(gap.lookup.declarations.length,0);assert.deepEqual(p.diagnostics,[]);withheld(p);
}));
test('exact ambient binding and opaque unsafe-path probes coexist',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.baseUrl='.';save();put('src/a.d.ts','declare module "virtual:input" {export const value:number;}');put('src/b.d.ts','');put('src/app.ts','import {value} from "virtual:input";'+app);
  const p=produce(options);assert.equal(p.resolutions[0].lookup.status,'observed');assert(p.snapshot.refused_lookup_sha256.length>0);assert(p.reasons.includes('unsupported_lookup'));withheld(p);
}));
test('observed bindings do not traverse or discharge project references',()=>fixture(({put,config,save,options})=>{
  config.references=[{path:'./other'}];save();put('other/tsconfig.json','{"compilerOptions":{"composite":true},"files":["index.ts"]}');put('other/index.ts','export class Outside {}');
  const p=produce(options);assert(p.resolutions.every(observed));assert(p.reasons.includes('unsupported_references'));assert.equal(p.closure.references,false);assert(!p.snapshot.program_files.includes('project/other/index.ts'));withheld(p);
}));
test('observed bindings do not authorize executing configured plugins',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.plugins=[{name:'never-run'}];save();put('node_modules/never-run/package.json','{"name":"never-run","main":"index.js"}');put('node_modules/never-run/index.js','throw Error("PLUGIN_EXECUTED");');
  const p=produce(options);assert(p.resolutions.every(observed));assert(p.reasons.includes('unsupported_plugins'));assert(!p.reasons.includes('worker_failed'));withheld(p);
}));
test('all filesystem requests resolved does not erase compiler diagnostics',()=>fixture(({put,options})=>{
  put('src/dep.ts','export const value=1;');put('src/app.ts','import {value} from "./dep";const wrong:string=value;'+app);
  const p=produce(options);assert.equal(p.resolutions.length,1);assert(p.resolutions.every(r=>r.target!==null));assert(p.diagnostics.some(d=>d.code===2322));withheld(p);
}));
test('triple-slash path obligations are not module-literal request occurrences',()=>fixture(({put,config,save,options})=>{
  put('src/reference.d.ts','/// <reference path="./absent.d.ts" />\ninterface Other {}');
  for(const skip of [false,true]){config.compilerOptions.skipLibCheck=skip;save();const p=produce(options);
    assert(p.resolutions.every(observed));assert(!p.resolutions.some(r=>r.specifier==='./absent.d.ts'));assert(p.snapshot.failed_lookups.includes('project/src/absent.d.ts'));
    assert.equal(p.diagnostics.some(d=>d.code===6053),!skip);withheld(p);
  }
}));
test('a missing type-reference directive is distinct from module source coverage',()=>fixture(({put,config,save,options})=>{
  put('src/reference.d.ts','/// <reference types="missing-types" />\ninterface Other {}');
  for(const skip of [false,true]){config.compilerOptions.skipLibCheck=skip;save();const p=produce(options);
    assert(p.resolutions.every(observed));assert(!p.resolutions.some(r=>r.specifier==='missing-types'));assert.equal(p.diagnostics.some(d=>d.code===2688),!skip);withheld(p);
  }
}));
test('KNOWN WRONG: skipped absent path can falsely report a complete Program',()=>fixture(({put,config,save,options})=>{
  put('src/app.ts',app);put('src/reference.d.ts','/// <reference path="./absent.d.ts" />\ninterface Other {}');
  for(const skip of [false,true]){config.compilerOptions.skipLibCheck=skip;save();const p=produce(options);
    assert.equal(p.resolutions.length,0);assert(p.snapshot.program_files.includes('project/src/reference.d.ts'));assert(p.snapshot.failed_lookups.includes('project/src/absent.d.ts'));
    assert.equal(p.status,skip?'observed':'unproven');assert.equal(p.closure.resolution,skip);assert.equal(p.observations[0].nested.calls[0].props_class.status,skip?'observed':'unproven');
    assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  }
}));
test('source binding does not prove an ambient class has a runtime implementation',()=>fixture(({put,options})=>{
  put('src/a.d.ts','declare module "virtual-client" {export class Client{m():void}}');put('src/b.d.ts','');
  put('src/app.ts','import type {Client} from "virtual-client";type View<P>=(p:P)=>void;const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};');
  const p=produce(options);assert.equal(p.resolutions[0].lookup.status,'observed');const c=p.observations[0].nested.calls[0].props_class;assert.equal(c.class_declaration.file,'project/src/a.d.ts');withheld(p);
}));
test('closure promotion is rejected before root I/O and stale snapshots still reject',()=>fixture(({put,options})=>{
  const p=produce(options);assert(p.resolutions.every(observed));let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const change of [q=>{q.reasons=[];q.status='observed';for(const k of Object.keys(q.closure))q.closure[k]=true;},q=>q.closure.dependencies=true,q=>q.closure.augmentation=true]){
    const q=structuredClone(p);change(q);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);
  }assert.equal(reads,0);put('excluded/new.d.ts','declare module "*.scss";');assert.equal(validate(JSON.stringify(p),options).valid,false);
}));
test('a normalized outside-probe digest cannot identify its initiating request',()=>{
  const byRequest=[];for(const specifier of ['missing-a','missing-b']){
    const paths=[];const host={fileExists:()=>false,readFile:()=>undefined,directoryExists:f=>{paths.push(path.posix.normalize(f));return false;},getCurrentDirectory:()=>'/__prism__/project'};
    ts.resolveModuleName(specifier,'/__prism__/project/src/app.ts',{moduleResolution:ts.ModuleResolutionKind.Node10},host);
    assert(paths.includes('/node_modules'));byRequest.push(paths.map(hash));
  }assert(byRequest.every(xs=>xs.includes(hash('/node_modules'))));
});
