import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from 'node:fs';
import path from 'node:path';
import {tmpdir} from 'node:os';
import {produce,validate} from './index.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
const app='class Client{m(){}}type View<P>=(p:P)=>void;export const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
const ambient='declare module "virtual:input" {export const value:number;export interface Thing {x:number}}';
function fixture(run){
  const root=mkdtempSync(path.join(tmpdir(),'prism-exact-ambient-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Bundler',baseUrl:'.',types:[],libReplacement:false,skipLibCheck:true},include:['src']};
  const source=s=>put('src/app.ts',s+app);
  try{put('package.json','{"type":"module"}');put('tsconfig.json',JSON.stringify(config));put('src/ambient.d.ts',ambient);
    source('import {value} from "virtual:input";');return run({put,source,config,options:{root,compiler,config:'tsconfig.json'}});
  }finally{rmSync(root,{recursive:true,force:true});}
}
function lookup(p,specifier='virtual:input',context){
  const matches=p.resolutions.filter(r=>r.specifier===specifier && (!context||r.lookup?.context===context));
  assert.equal(matches.length,1,JSON.stringify(p.reasons));assert(matches[0].lookup,'source-backed lookup record required');return matches[0].lookup;
}
function barriers(p){
  assert.equal(p.status,'unproven');assert(p.reasons.includes('unresolved_module'));assert(p.reasons.includes('unsupported_lookup'));
  assert.equal(p.closure.dependencies,false);assert.equal(p.closure.augmentation,false);assert.equal(p.closure.resolution,false);
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert.equal(p.observations[0].nested.calls[0].props_class.reason,'program_unproven');
}
for(const [context,source] of [
  ['import','import {value} from "virtual:input";'],
  ['export','export {value} from "virtual:input";'],
  ['import_type','type T=import("virtual:input").Thing;'],
  ['import_equals','import input = require("virtual:input");'],
])test(`singleton exact ambient ${context} is observed without closure admission`,()=>fixture(({source:write,options})=>{
  write(source);const p=produce(options),l=lookup(p);assert.equal(l.status,'observed');assert.equal(l.reason,null);assert.equal(l.context,context);
  assert.equal(l.request.kind,'StringLiteral');assert.equal(l.request.file,'project/src/app.ts');
  assert.equal(l.declarations.length,1);assert.deepEqual(l.declarations,l.providers);assert.deepEqual(l.augmentations,[]);
  assert.equal(l.providers[0].file,'project/src/ambient.d.ts');assert.equal(l.providers[0].kind,'ModuleDeclaration');
  assert.equal(p.resolutions.find(r=>r.specifier==='virtual:input').target,null);barriers(p);assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
test('same-name duplicate providers poison a clean-diagnostics binding',()=>fixture(({put,options})=>{
  put('src/duplicate.d.ts','declare module "virtual:input" {export const other:number;}');
  const p=produce(options),l=lookup(p);assert.equal(l.status,'unproven');assert.equal(l.reason,'duplicate_provider');assert.equal(l.providers.length,2);barriers(p);
}));
test('same-name augmentation poisons singleton provider and declaration names stay separate',()=>fixture(({put,options})=>{
  put('src/augment.d.ts','export {};declare module "virtual:input" {interface Added {x:number}}');
  const p=produce(options),l=lookup(p,'virtual:input','import');assert.equal(l.status,'unproven');assert.equal(l.reason,'augmentation');
  assert.equal(l.providers.length,1);assert.equal(l.augmentations.length,1);
  const name=lookup(p,'virtual:input','augmentation_name');assert.equal(name.status,'unproven');assert.equal(name.reason,'augmentation_request');barriers(p);
}));
test('an excluded provider is not source binding authority',()=>fixture(({put,options})=>{
  put('src/ambient.d.ts','');put('excluded/provider.d.ts',ambient);const l=lookup(produce(options));
  assert.equal(l.status,'unproven');assert.equal(l.reason,'unresolved_symbol');assert.deepEqual(l.providers,[]);assert.deepEqual(l.declarations,[]);
}));
test('wildcard binding retains candidate anchor without exact authority',()=>fixture(({put,options})=>{
  put('src/ambient.d.ts','declare module "virtual:*" {export const value:number;}');const l=lookup(produce(options));
  assert.equal(l.status,'unproven');assert.equal(l.reason,'non_exact_binding');assert.equal(l.declarations.length,1);assert.deepEqual(l.providers,[]);
}));
test('non-declaration-file provider is outside this bounded slice',()=>fixture(({put,options})=>{
  put('src/ambient.d.ts','');put('src/provider.ts',ambient);const l=lookup(produce(options));
  assert.equal(l.status,'unproven');assert.equal(l.reason,'unsupported_provider');assert.equal(l.providers.length,1);
}));
test('shorthand ambient provider lacks supported declaration body',()=>fixture(({put,options})=>{
  put('src/ambient.d.ts','declare module "virtual:input";');const l=lookup(produce(options));
  assert.equal(l.status,'unproven');assert.equal(l.reason,'unsupported_provider');assert.equal(l.providers.length,1);
}));
test('filesystem target stays separate even with an exact ambient candidate',()=>fixture(({put,config,options})=>{
  config.compilerOptions.paths={'virtual:input':['src/value.ts']};put('tsconfig.json',JSON.stringify(config));put('src/value.ts','export const value=1;');
  const p=produce(options),l=lookup(p);assert.equal(l.status,'unproven');assert.equal(l.reason,'filesystem_target');
  assert.equal(p.resolutions.find(r=>r.specifier==='virtual:input').target,'project/src/value.ts');
}));
test('synthetic JSX runtime request has null source anchor',()=>fixture(({put,config,options})=>{
  config.compilerOptions.jsx='react-jsx';put('tsconfig.json',JSON.stringify(config));put('src/view.tsx','export const view=<div/>;');
  const requests=produce(options).resolutions.filter(r=>r.specifier==='react/jsx-runtime');
  assert.deepEqual(requests.map(r=>r.from).sort(),['project/src/app.ts','project/src/view.tsx']);
  for(const {lookup:l} of requests){assert(l);assert.equal(l.status,'unproven');assert.equal(l.reason,'synthetic_request');assert.equal(l.context,'synthetic');assert.equal(l.request,null);}
}));
test('dynamic imports remain explicitly unsupported request contexts',()=>fixture(({source,options})=>{
  source('const mod=import("virtual:input");');const l=lookup(produce(options));assert.equal(l.context,'dynamic_import');assert.equal(l.reason,'unsupported_request');assert.equal(l.status,'unproven');
}));
test('duplicate insertion and restoration invalidates old observations without positive cache',()=>fixture(({put,options})=>{
  const a=produce(options);assert.equal(lookup(a).status,'observed');
  put('src/ambient.d.ts',ambient+'\ndeclare module "virtual:input" {export const other:number;}');
  const b=produce(options);assert.equal(lookup(b).reason,'duplicate_provider');assert.equal(validate(JSON.stringify(a),options).valid,false);
  put('src/ambient.d.ts',ambient);assert.equal(validate(JSON.stringify(a),options).valid,true);
}));
test('invalid observation shapes and previous schema reject before root access',()=>fixture(({options})=>{
  const p=produce(options);lookup(p);let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const mutate of [
    q=>q.schema='prism.callable-observation/7',
    q=>lookup(q).request=null,q=>lookup(q).request.file='project/absent.ts',
    q=>lookup(q).providers=[],q=>lookup(q).providers.push(lookup(q).providers[0]),
    q=>lookup(q).augmentations=lookup(q).providers,q=>lookup(q).reason='unresolved_symbol',
    q=>lookup(q).context='augmentation_name',q=>lookup(q).providers[0].sha256='0'.repeat(64),
    q=>lookup(q).extra=true,q=>q.resolutions.find(r=>r.specifier==='virtual:input').target='project/src/app.ts',
  ]){const q=structuredClone(p);mutate(q);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);}
  assert.equal(reads,0);
}));
test('structurally valid forged binding evidence fails independent recomputation',()=>fixture(({options})=>{
  const p=produce(options),l=lookup(p);l.request.start_utf16++;l.request.start_byte++;p.search_provenance.module_requests.find(r=>r.specifier===p.resolutions.find(r=>r.lookup===l).specifier).request=l.request;
  assert.equal(validate(JSON.stringify(p),options).reason,'stale_or_tampered');
}));
test('positive lookup observations cannot clear the existing unresolved-module barrier pre-I/O',()=>fixture(({options})=>{
  const p=produce(options);assert.equal(lookup(p).status,'observed');let reads=0;
  for(const mutate of [q=>q.reasons=q.reasons.filter(r=>r!=='unresolved_module'),
    ...['dependencies','augmentation','resolution'].map(k=>q=>q.closure[k]=true)]){
    const q=structuredClone(p);mutate(q);
    assert.equal(validate(JSON.stringify(q),{get root(){reads++;throw Error('forbidden');}}).valid,false);
  }
  assert.equal(reads,0);
}));
test('same-name nested declaration cannot evade the independent provider census',()=>fixture(({put,options})=>{
  put('src/invalid.d.ts','declare namespace Invalid {module "virtual:input" {export const other:number;}}');
  const l=lookup(produce(options));assert.equal(l.status,'unproven');assert.equal(l.reason,'duplicate_provider');assert.equal(l.providers.length,2);
}));
test('merged wildcard candidates remain unproven and retain both anchors',()=>fixture(({put,options})=>{
  put('src/ambient.d.ts','declare module "virtual:*" {export const value:number;}');
  put('src/second.d.ts','declare module "virtual:*" {export const other:number;}');
  const l=lookup(produce(options));assert.equal(l.status,'unproven');assert.equal(l.reason,'ambiguous_binding');assert.equal(l.declarations.length,2);assert.deepEqual(l.providers,[]);
}));
test('missing import and its orphan augmentation name keep separate binding evidence',()=>fixture(({put,source,options})=>{
  source('');put('src/ambient.d.ts','import {Thing} from "virtual:input";declare module "virtual:input" {interface Added{x:number}}');
  const p=produce(options),use=lookup(p,'virtual:input','import'),name=lookup(p,'virtual:input','augmentation_name');
  assert.equal(use.status,'unproven');assert.equal(use.reason,'augmentation');assert.deepEqual(use.declarations,[]);assert.equal(use.augmentations.length,1);
  assert.equal(name.reason,'augmentation_request');assert.equal(name.declarations.length,1);
}));
test('require syntax is labeled but does not expand supported request authority',()=>fixture(({put,config,options})=>{
  config.compilerOptions.allowJs=true;put('tsconfig.json',JSON.stringify(config));put('src/legacy.js','const input=require("virtual:input");');
  const l=lookup(produce(options),'virtual:input','require');assert.equal(l.status,'unproven');assert.equal(l.reason,'unsupported_request');assert(l.request);
}));
test('an excluded same-name provider is not part of the configured Program census',()=>fixture(({put,options})=>{
  put('excluded/duplicate.d.ts',ambient);const p=produce(options),l=lookup(p);assert.equal(l.status,'observed');assert.equal(l.providers.length,1);
  assert(p.snapshot.files.some(f=>f.id==='project/excluded/duplicate.d.ts'));assert(!p.snapshot.program_files.includes('project/excluded/duplicate.d.ts'));
}));
test('exact-ambient observations never bypass nested receiver writes',()=>fixture(({source,options})=>{
  source('import {value} from "virtual:input";type F=(client:Client)=>void;const written:F=client=>{client=new Client();const cb=()=>client.m();};');
  const p=produce(options);assert.equal(lookup(p).status,'observed');
  const written=p.observations.find(o=>o.nested.calls.some(c=>c.binding.reason==='write_barrier'));
  assert(written);assert.equal(written.nested.calls[0].props_class.status,'unproven');
}));
test('source-backed JSDoc import type retains its actual literal anchor',()=>fixture(({put,config,options})=>{
  config.compilerOptions.allowJs=true;put('tsconfig.json',JSON.stringify(config));
  put('src/doc.js','/** @type {import("virtual:input").Thing} */\nexport const thing={x:1};');
  const p=produce(options),l=lookup(p,'virtual:input','import_type');assert.equal(l.status,'observed');
  assert.equal(l.request.file,'project/src/doc.js');assert.equal(l.request.kind,'StringLiteral');assert.equal(l.providers.length,1);
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
for(const changed of [false,true])test(`package-ID redirects retain original census bytes: changed=${changed}`,()=>fixture(({put,source,options})=>{
  for(const name of ['a','b']){
    put(`node_modules/${name}/package.json`,JSON.stringify({name,version:'1.0.0',types:'index.d.ts'}));
    put(`node_modules/${name}/index.d.ts`,'import "shared";export {};');
    put(`node_modules/${name}/node_modules/shared/package.json`,JSON.stringify({name:'shared',version:'1.0.0',types:'index.d.ts'}));
    put(`node_modules/${name}/node_modules/shared/index.d.ts`,
      `export {};declare module "virtual:input" {interface Added{x:number${changed&&name==='b'?';extra:string':''}}}`);
  }
  source('import "a";import "b";import {value} from "virtual:input";');
  const p=produce(options),l=lookup(p,'virtual:input','import');
  assert.equal(l.status,'unproven');assert.equal(l.reason,'augmentation');assert.equal(l.augmentations.length,2);
  assert.deepEqual(l.augmentations.map(a=>a.file).sort(),[
    'project/node_modules/a/node_modules/shared/index.d.ts','project/node_modules/b/node_modules/shared/index.d.ts']);
  assert.equal(l.augmentations[0].sha256!==l.augmentations[1].sha256,changed);
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
