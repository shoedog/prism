import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from 'node:fs';import path from 'node:path';import {tmpdir} from 'node:os';
import {produce,validate} from './index.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
const empty='declare module "*.scss" {}',short='declare module "*.scss";';
const app='class Client{m(){}}type View<P>=(p:P)=>void;const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-merged-wildcard-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,types:[],module:'ESNext',moduleResolution:'Bundler',target:'ES2022',skipLibCheck:true,noUncheckedSideEffectImports:true,libReplacement:false},include:['src']};
  const source=s=>put('src/app.ts',s+app);
  try{put('tsconfig.json',JSON.stringify(config));put('src/a.d.ts',empty);put('src/b.d.ts',short);source('import "./style.scss";');return run({put,source,config,options:{root,compiler,config:'tsconfig.json'}});}
  finally{rmSync(root,{recursive:true,force:true});}}
function resolution(p,s='./style.scss',context='import'){const rs=p.resolutions.filter(r=>r.specifier===s&&r.lookup.context===context);assert.equal(rs.length,1,JSON.stringify(p.reasons));assert(Object.hasOwn(rs[0].lookup,'merged_wildcard'),'new independent lane required');return rs[0];}
function withheld(p){assert.equal(p.status,'unproven');assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  for(const k of ['dependencies','augmentation','resolution'])assert.equal(p.closure[k],false);
  assert(p.observations.some(o=>o.nested.calls.some(c=>c.props_class.reason==='program_unproven')));}
for(const reverse of [false,true])for(const exists of [false,true])test(`bounded pair: reverse=${reverse}, asset=${exists}`,()=>fixture(({put,options})=>{
  if(reverse){put('src/a.d.ts',short);put('src/b.d.ts',empty);}if(exists)put('src/style.scss','.style {}');
  const p=produce(options),r=resolution(p),m=r.lookup.merged_wildcard;assert.equal(m.status,'observed');assert.equal(m.reason,null);
  assert.equal(r.lookup.reason,'ambiguous_binding');assert.equal(r.lookup.wildcard.reason,'duplicate_provider');assert.equal(r.target,null);
  assert.deepEqual(m.declarations.map(d=>d.declaration),r.lookup.declarations);assert.deepEqual(m.declarations.map(d=>d.shape),reverse?['shorthand','empty_block']:['empty_block','shorthand']);
  assert.deepEqual(m.selected,m.declarations[0]);assert.equal(p.snapshot.files.some(f=>f.id==='project/src/style.scss'),exists);withheld(p);
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
for(const [name,code,context] of [
  ['default','import value from "./style.scss";','import'],['named','import {value} from "./style.scss";','import'],
  ['namespace','import * as value from "./style.scss";','import'],['type','import type value from "./style.scss";','import'],
  ['export','export * from "./style.scss";','export'],['import_type','type T=import("./style.scss");','import_type'],
  ['equals','import value=require("./style.scss");','import_equals'],['dynamic','const value=import("./style.scss");','dynamic_import'],
  ['attributes','import "./style.scss" with {type:"css"};','import'],
])test(`unsupported context: ${name}`,()=>fixture(({source,options})=>{source(code);const p=produce(options),m=resolution(p,'./style.scss',context).lookup.merged_wildcard;assert.equal(m.status,'unproven');assert.equal(m.reason,'unsupported_request');withheld(p);}));
test('require is not a side-effect import',()=>fixture(({put,config,options})=>{
  config.compilerOptions.allowJs=true;put('tsconfig.json',JSON.stringify(config));put('src/app.ts',app);put('src/legacy.js','require("./style.scss");');
  assert.equal(resolution(produce(options),'./style.scss','require').lookup.merged_wildcard.reason,'unsupported_request');
}));
for(const [name,a,b,extra,reason] of [
  ['two empty',empty,empty,null,'unsupported_provider'],['two shorthand',short,short,null,'unsupported_provider'],
  ['typed','declare module "*.scss" {export const value:string;}',short,null,'unsupported_provider'],
  ['third',empty,short,short,'provider_count'],['nested',empty,short,'declare namespace Bad {module "*.scss" {}}','provider_count'],
])test(`provider barrier: ${name}`,()=>fixture(({put,options})=>{put('src/a.d.ts',a);put('src/b.d.ts',b);if(extra)put('src/c.d.ts',extra);
  const p=produce(options),m=resolution(p).lookup.merged_wildcard;assert.equal(m.status,'unproven');assert.equal(m.reason,reason);withheld(p);
}));
test('non-declaration-file contributor is refused',()=>fixture(({put,options})=>{put('src/b.d.ts','');put('src/b.ts',short);assert.equal(resolution(produce(options)).lookup.merged_wildcard.reason,'unsupported_provider');}));
for(const name of ['*.scss','./style.scss'])test(`augmentation ${name} cannot hide behind duplicate priority`,()=>fixture(({put,options})=>{
  put('src/augment.d.ts',`export {};declare module "${name}" {interface Added{x:number}}`);const p=produce(options),r=resolution(p);
  assert.equal(r.lookup.merged_wildcard.status,'unproven');assert.equal(r.lookup.merged_wildcard.reason,'augmentation');withheld(p);
}));
test('competing shorter pattern is still a barrier to a merged selected pattern',()=>fixture(({put,options})=>{
  put('src/a.d.ts','declare module "./style.*" {}');put('src/b.d.ts','declare module "./style.*";');put('src/c.d.ts',short);
  const r=resolution(produce(options));assert.equal(r.lookup.wildcard.pattern,'./style.*');assert.equal(r.lookup.wildcard.reason,'duplicate_provider');assert.equal(r.lookup.merged_wildcard.reason,'competing_pattern');
}));
test('tied-prefix competing pattern cannot admit a merged selected binding',()=>fixture(({put,options})=>{
  put('src/c.d.ts','declare module "*style.scss";');const r=resolution(produce(options));assert.equal(r.lookup.merged_wildcard.reason,'competing_pattern');
}));
test('excluded provider stays outside the Program census',()=>fixture(({put,options})=>{
  put('src/b.d.ts','');put('excluded/b.d.ts',short);const r=resolution(produce(options));assert.equal(r.lookup.merged_wildcard,null);assert.equal(r.lookup.wildcard.status,'observed');
}));
test('exact provider takes the existing exact lane',()=>fixture(({put,source,options})=>{
  put('src/exact.d.ts','declare module "exact.scss" {}');source('import "exact.scss";');const r=resolution(produce(options),'exact.scss');assert.equal(r.lookup.status,'observed');assert.equal(r.lookup.merged_wildcard,null);
}));
test('source/body/context and Program membership edits invalidate the pair epoch',()=>fixture(({put,source,options})=>{
  const p=produce(options);assert.equal(resolution(p).lookup.merged_wildcard.status,'observed');
  for(const mutate of [()=>put('src/a.d.ts',short),()=>source('import * as value from "./style.scss";'),()=>put('src/c.d.ts',short)]){
    mutate();assert.equal(validate(JSON.stringify(p),options).valid,false);put('src/a.d.ts',empty);source('import "./style.scss";');put('src/c.d.ts','');
  }
}));
test('nested receiver write barriers survive the new observation',()=>fixture(({source,options})=>{
  source('import "./style.scss";type F=(client:Client)=>void;const written:F=client=>{client=new Client();const cb=()=>client.m();};');const p=produce(options);
  assert.equal(resolution(p).lookup.merged_wildcard.status,'observed');assert(p.observations.some(o=>o.nested.calls.some(c=>c.binding.reason==='write_barrier'&&c.props_class.status==='unproven')));
}));
test('malformed pair records reject before audited root access',()=>fixture(({options})=>{
  const p=produce(options);assert.equal(resolution(p).lookup.merged_wildcard.status,'observed');let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const change of [q=>q.schema='prism.callable-observation/9',q=>resolution(q).lookup.merged_wildcard.asset_exists=true,
    q=>resolution(q).lookup.merged_wildcard.selected=null,q=>resolution(q).lookup.merged_wildcard.declarations=[],
    q=>resolution(q).lookup.merged_wildcard.selected.shape='typed',q=>resolution(q).lookup.merged_wildcard.reason='provider_count',
    q=>resolution(q).lookup.merged_wildcard.selected.declaration.file='project/missing',q=>q.closure.resolution=true]){
    const q=structuredClone(p);change(q);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);
  }assert.equal(reads,0);
}));
test('two genuine contributor substitution cannot forge selected value identity',()=>fixture(({options})=>{
  const p=produce(options),m=resolution(p).lookup.merged_wildcard;assert.equal(m.status,'observed');m.selected=structuredClone(m.declarations[1]);
  assert.equal(validate(JSON.stringify(p),options).reason,'stale_or_tampered');
}));
test('omission and reordered contributor provenance cannot survive recomputation',()=>fixture(({options})=>{
  const p=produce(options);assert.equal(resolution(p).lookup.merged_wildcard.status,'observed');
  for(const change of [q=>resolution(q).lookup.merged_wildcard=null,q=>resolution(q).lookup.merged_wildcard.declarations.reverse()]){
    const q=structuredClone(p);change(q);assert.equal(validate(JSON.stringify(q),options).valid,false);
  }
}));
for(const changed of [false,true])test(`redirected augmentation census uses actual source bytes: changed=${changed}`,()=>fixture(({put,source,options})=>{
  for(const name of ['x','y']){
    put(`node_modules/${name}/package.json`,JSON.stringify({name,version:'1.0.0',types:'index.d.ts'}));put(`node_modules/${name}/index.d.ts`,'import "shared";export {};');
    put(`node_modules/${name}/node_modules/shared/package.json`,JSON.stringify({name:'shared',version:'1.0.0',types:'index.d.ts'}));
    put(`node_modules/${name}/node_modules/shared/index.d.ts`,`export {};declare module "*.scss" {interface Added{x:number${changed&&name==='y'?';extra:string':''}}}`);
  }
  source('import "x";import "y";import "./style.scss";');const p=produce(options),r=resolution(p);assert.equal(r.lookup.merged_wildcard.reason,'augmentation');
  assert.equal(r.lookup.wildcard.augmentations.length,2);assert.equal(r.lookup.wildcard.augmentations[0].sha256!==r.lookup.wildcard.augmentations[1].sha256,changed);
}));
