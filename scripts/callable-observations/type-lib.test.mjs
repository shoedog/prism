import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,symlinkSync,cpSync} from 'node:fs';import {tmpdir} from 'node:os';import path from 'node:path';import {pathToFileURL} from 'node:url';
const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const url=f=>implementation?pathToFileURL(path.join(implementation,f)):new URL(f,import.meta.url);
const {produce,validate}=await import(url('index.mjs'));const {parsePacket,hash,COMPILER_HASH}=await import(url('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler);assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
const app='class Client{m(){}}type View<P>=(p:P)=>void;const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-type-lib-test-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:true},include:['src']},save=()=>put('tsconfig.json',JSON.stringify(config));
  try{save();put('src/app.ts',app);return run({put,config,save,root,options:{root,compiler,config:'tsconfig.json'}});}finally{rmSync(root,{recursive:true,force:true});}}
function refs(p){assert(!p.reasons.includes('worker_failed'));assert(Array.isArray(p.type_lib_references),'source type/lib observation channel must exist');return p.type_lib_references;}
const at=(p,file='project/src/ref.d.ts')=>refs(p).filter(r=>r.request.file===file);
function refused(p,r,reason){assert.equal(r.status,'unproven');assert.equal(r.reason,reason);assert.equal(r.target,null);assert.equal(r.inclusion,false);
  assert(p.reasons.includes('unproven_type_lib_reference'));assert.equal(p.status,'unproven');for(const k of ['dependencies','references','augmentation','resolution'])assert.equal(p.closure[k],false);
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);assert.equal(p.observations[0].nested.calls[0].props_class.reason,'program_unproven');}
function observed(p,r,target){assert.equal(r.status,'observed');assert.equal(r.reason,null);assert.equal(r.target,target);assert.equal(r.inclusion,true);assert(p.snapshot.program_files.includes(target));}
for(const kind of ['types','lib'])for(const skip of [false,true])test(`missing ${kind} is a source occurrence independent of skipLibCheck=${skip}`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.skipLibCheck=skip;save();put('src/ref.d.ts',`/// <reference ${kind}="missing-fixture" />`);const p=produce(options);assert.equal(p.status,'unproven');const [r]=at(p);assert.equal(at(p).length,1);assert.equal(r.kind,kind);assert.equal(r.name,'missing-fixture');assert.equal(r.mode,null);assert.equal(r.index,0);refused(p,r,'unresolved');
  assert.equal(p.diagnostics.some(d=>d.code===(kind==='types'?2688:2726)),!skip);assert.equal(p.resolutions.length,0);
}));
test('noCheck cannot turn an unresolved lib directive into complete closure',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.noCheck=true;config.compilerOptions.skipLibCheck=false;save();put('src/ref.d.ts','/// <reference lib="missing-fixture" />');const p=produce(options);assert.deepEqual(p.diagnostics,[]);assert.equal(p.status,'unproven');refused(p,at(p)[0],'unresolved');
}));
test('type target is actually selected and included, not merely inventoried',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference types="fixture" />');put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options);observed(p,at(p)[0],'project/node_modules/@types/fixture/index.d.ts');assert.equal(p.status,'unproven');assert.deepEqual(p.reasons,['outside_lookup']);
}));
test('BOM Unicode and repeated occurrences retain source spans and indices',()=>fixture(({put,options})=>{
  const text='\uFEFF// 😀\n/// <reference lib="es2015" />\n/// <reference lib="es2015" />';put('src/ref.d.ts',text);const p=produce(options),rr=at(p);assert.equal(rr.length,2);
  for(const [index,r] of rr.entries()){observed(p,r,'compiler/lib.es2015.d.ts');assert.equal(r.index,index);assert.equal(r.request.sha256,hash(Buffer.from(text)));assert.equal(text.slice(r.request.start_utf16,r.request.end_utf16),r.name);assert.equal(Buffer.byteLength(text.slice(0,r.request.start_utf16)),r.request.start_byte);}
}));
for(const [kind,flag] of [['types','noResolve'],['lib','noLib']])test(`${flag} records unprocessed ${kind} despite incidental target membership`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions[flag]=true;save();put('src/ref.d.ts',`/// <reference ${kind}="${kind==='types'?'./dep.d.ts':'es5'}" />`);put('src/dep.d.ts','interface Dep {}');
  if(kind==='lib')put('src/incidental.d.ts','/// <reference path="../../compiler/lib.es5.d.ts" />');
  const p=produce(options);assert(p.snapshot.program_files.includes(kind==='types'?'project/src/dep.d.ts':'compiler/lib.es5.d.ts'));refused(p,at(p)[0],'unprocessed');
}));
for(const mode of ['import','require'])test(`explicit ${mode} type mode selects conditional export`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';save();
  put('src/ref.d.ts',`/// <reference types="fixture" resolution-mode="${mode}" />`);
  put('node_modules/fixture/package.json',JSON.stringify({name:'fixture',version:'1.0.0',exports:{'.':{import:'./import.d.mts',require:'./require.d.cts'}}}));
  put('node_modules/fixture/import.d.mts','export interface Imported {}');put('node_modules/fixture/require.d.cts','export interface Required {}');
  const p=produce(options),r=at(p)[0];assert.equal(r.mode,mode);observed(p,r,`project/node_modules/fixture/${mode}.d.${mode==='import'?'mts':'cts'}`);
}));
test('lib replacement uses compiler actual target instead of builtin-name inference',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.libReplacement=true;save();put('src/ref.d.ts','/// <reference lib="dom" />');
  put('node_modules/@typescript/lib-dom/package.json','{"name":"@typescript/lib-dom","version":"1.0.0","types":"index.d.ts"}');put('node_modules/@typescript/lib-dom/index.d.ts','interface TestDom {}');
  const p=produce(options);observed(p,at(p)[0],'project/node_modules/@typescript/lib-dom/index.d.ts');
}));
test('failed optional lib replacement search can still observe the builtin target',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.libReplacement=true;save();put('src/ref.d.ts','/// <reference lib="dom" />');const p=produce(options);observed(p,at(p)[0],'compiler/lib.dom.d.ts');assert(!p.reasons.includes('unproven_type_lib_reference'));
}));
for(const kind of ['types','lib'])test(`redirect ${kind} census uses original bytes and does not borrow another source cache`,()=>fixture(({put,options})=>{
  for(const name of ['a','b']){put(`node_modules/${name}/package.json`,JSON.stringify({name,version:'1.0.0',types:'index.d.ts'}));put(`node_modules/${name}/index.d.ts`,'import "shared";export {};');
    put(`node_modules/${name}/node_modules/shared/package.json`,'{"name":"shared","version":"1.0.0","types":"index.d.ts"}');
    put(`node_modules/${name}/node_modules/shared/index.d.ts`,name==='a'?'export {};':`/// <reference ${kind}="missing-original" />\nexport {};`);}
  put('src/app.ts','import "a";import "b";'+app);const p=produce(options),rr=at(p,'project/node_modules/b/node_modules/shared/index.d.ts');assert.equal(rr.length,1);assert.equal(rr[0].name,'missing-original');refused(p,rr[0],'unprocessed');
}));
test('imported declaration transitive type directives are enumerated',()=>fixture(({put,options})=>{
  put('src/app.ts','import "./dep";'+app);put('src/dep.d.ts','/// <reference types="missing-transitive" />\nexport {};');const p=produce(options);assert(p.resolutions.every(r=>r.target!==null));refused(p,at(p,'project/src/dep.d.ts')[0],'unresolved');
}));
for(const automatic of [false,true])test(`configured/automatic type entries are not invented as source directives: automatic=${automatic}`,()=>fixture(({put,config,save,options})=>{
  if(automatic)delete config.compilerOptions.types;else config.compilerOptions.types=['fixture'];save();put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options);assert(p.snapshot.program_files.includes('project/node_modules/@types/fixture/index.d.ts'));assert.equal(refs(p).filter(r=>r.kind==='types').length,0);
}));
test('canonical in-root symlink type target retains source identity',()=>fixture(({put,root,options})=>{
  put('src/ref.d.ts','/// <reference types="../alias/dep.d.ts" />');put('excluded/dep.d.ts','interface Dep {}');symlinkSync('excluded',path.join(root,'alias'));
  const p=produce({...options,links:'in-root'});observed(p,at(p)[0],'project/excluded/dep.d.ts');
}));
test('removal, same-byte target substitution, and mode forgery fail recomputation',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference types="./one.d.ts" />\n/// <reference types="./two.d.ts" />');for(const name of ['one','two'])put(`src/${name}.d.ts`,'interface Shared {}');
  const p=produce(options),rr=at(p);assert.equal(rr.length,2);for(const r of rr)assert.equal(r.status,'observed');assert.equal(validate(JSON.stringify(p),options).valid,true);
  for(const alter of [q=>q.type_lib_references.splice(q.type_lib_references.findIndex(r=>r.request.file==='project/src/ref.d.ts'),1),q=>q.type_lib_references.find(r=>r.request.file==='project/src/ref.d.ts').target=rr[1].target,q=>q.type_lib_references.find(r=>r.request.file==='project/src/ref.d.ts').mode='import']){
    const q=structuredClone(p);alter(q);assert.equal(validate(JSON.stringify(q),options).valid,false);}
  put('src/two.d.ts','interface Changed {}');assert.equal(validate(JSON.stringify(p),options).valid,false);
}));
test('refusal, membership, duplicate, and anchor forgeries reject before root I/O',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference lib="missing-fixture" />');const p=produce(options);refs(p);let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const alter of [q=>q.closure.references=true,q=>q.reasons=q.reasons.filter(r=>r!=='unproven_type_lib_reference'),q=>q.type_lib_references.push(q.type_lib_references[0]),q=>q.type_lib_references[0].request.file='project/not-in-program.d.ts',q=>q.type_lib_references[0].request.end_utf16++]){
    const q=structuredClone(p);alter(q);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);}
  assert.equal(reads,0);
}));
for(const type of ['module','commonjs'])test(`implicit type mode follows actual package format: ${type}`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';save();put('package.json',JSON.stringify({type}));put('src/ref.d.ts','/// <reference types="fixture" />');
  put('node_modules/fixture/package.json',JSON.stringify({name:'fixture',version:'1.0.0',exports:{'.':{import:'./import.d.mts',require:'./require.d.cts'}}}));
  put('node_modules/fixture/import.d.mts','export interface Imported {}');put('node_modules/fixture/require.d.cts','export interface Required {}');
  const p=produce(options),r=at(p)[0],mode=type==='module'?'import':'require';assert.equal(r.mode,mode);observed(p,r,`project/node_modules/fixture/${mode}.d.${mode==='import'?'mts':'cts'}`);
}));
test('mixed modes for the same type name retain distinct selected targets',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';save();put('src/ref.d.ts','/// <reference types="fixture" resolution-mode="import" />\n/// <reference types="fixture" resolution-mode="require" />');
  put('node_modules/fixture/package.json',JSON.stringify({name:'fixture',version:'1.0.0',exports:{'.':{import:'./import.d.mts',require:'./require.d.cts'}}}));
  put('node_modules/fixture/import.d.mts','export interface Imported {}');put('node_modules/fixture/require.d.cts','export interface Required {}');
  const p=produce(options),rr=at(p);assert.equal(rr.length,2);assert.deepEqual(rr.map(r=>r.mode),['import','require']);assert(rr.every(r=>r.status==='observed'));assert.notEqual(rr[0].target,rr[1].target);
}));
test('cached builtin filename without its SourceFile is not an observed target',()=>fixture(({put,options})=>{
  const copy=mkdtempSync(path.join(tmpdir(),'prism-type-lib-compiler-'));
  try{cpSync(path.dirname(compiler),copy,{recursive:true,filter:f=>path.basename(f)!=='lib.dom.d.ts'});put('src/ref.d.ts','/// <reference lib="dom" />');
    const p=produce({...options,compiler:path.join(copy,path.basename(compiler))});assert(!p.snapshot.program_files.includes('compiler/lib.dom.d.ts'));refused(p,at(p)[0],'target_not_in_program');
  }finally{rmSync(copy,{recursive:true,force:true});}
}));
test('legacy schema10 is readable without invented observations and never current-valid',()=>fixture(({options})=>{
  const p=produce(options);refs(p);const old=structuredClone(p);old.schema='prism.callable-observation/10';old.producer.version='0.11.1';delete old.type_lib_references;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(old)));assert.equal(Object.hasOwn(parsePacket(JSON.stringify(old)),'type_lib_references'),false);assert.equal(validate(JSON.stringify(old),options).valid,false);
  old.reasons.push('unproven_type_lib_reference');old.status='unproven';for(const k of ['dependencies','references','augmentation','resolution'])old.closure[k]=false;
  assert.throws(()=>parsePacket(JSON.stringify(old)),/invalid_packet/);
}));
