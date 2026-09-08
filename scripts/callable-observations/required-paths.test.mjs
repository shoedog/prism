import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,symlinkSync} from 'node:fs';
import {tmpdir} from 'node:os';import path from 'node:path';import {pathToFileURL} from 'node:url';
// The same tests run against the exact base without copying or editing that base.
const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const moduleURL=f=>implementation?pathToFileURL(path.join(implementation,f)):new URL(f,import.meta.url);
const {produce,validate}=await import(moduleURL('index.mjs'));
const {parsePacket,hash,COMPILER_HASH}=await import(moduleURL('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler);assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
const app='class Client{m(){}}type View<P>=(p:P)=>void;const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-required-path-test-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:true},include:['src']};
  const save=()=>put('tsconfig.json',JSON.stringify(config));
  try{save();put('src/app.ts',app);return run({put,save,config,root,options:{root,compiler,config:'tsconfig.json'}});}
  finally{rmSync(root,{recursive:true,force:true});}}
function withheld(p){assert(!p.reasons.includes('worker_failed'));assert.equal(p.status,'unproven');assert(p.reasons.includes('unproven_path_reference'));
  for(const k of ['dependencies','references','augmentation','resolution'])assert.equal(p.closure[k],false,k);
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert.equal(p.observations[0].nested.calls[0].props_class.reason,'program_unproven');}
function complete(p){assert.deepEqual(p.reasons,[]);assert.equal(p.status,'observed');assert(Object.values(p.closure).every(Boolean));
  assert.equal(p.observations[0].nested.calls[0].props_class.status,'observed');assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);}
for(const flags of [{skipLibCheck:true},{skipLibCheck:false},{skipLibCheck:false,noCheck:true}])
  test(`missing required declaration path refuses independently of diagnostics: ${JSON.stringify(flags)}`,()=>fixture(({put,save,config,options})=>{
    Object.assign(config.compilerOptions,flags);save();put('src/ref.d.ts','/// <reference path="./absent.d.ts" />\ninterface Other {}');
    const p=produce(options);assert.equal(p.resolutions.length,0);assert(p.snapshot.failed_lookups.includes('project/src/absent.d.ts'));
    assert.equal(p.diagnostics.some(d=>d.code===6053),!flags.skipLibCheck&&!flags.noCheck);withheld(p);
  }));
for(const target of ['./dep.d.ts','./dep','../excluded/dep.d.ts'])test(`compiler-selected path succeeds: ${target}`,()=>fixture(({put,options})=>{
  put('src/ref.d.ts',`/// <reference path="${target}" />\ninterface Other {}`);
  const id=target.includes('excluded')?'excluded/dep.d.ts':'src/dep.d.ts';put(id,'interface Dep {}');
  const p=produce(options);complete(p);assert(p.snapshot.program_files.includes('project/'+id));
  if(target==='./dep')assert(p.snapshot.failed_lookups.includes('project/src/dep.ts'));
}));
test('excluded inventory presence cannot discharge noResolve path',()=>fixture(({put,save,config,options})=>{
  config.compilerOptions.noResolve=true;save();put('src/ref.d.ts','/// <reference path="../excluded/dep.d.ts" />');put('excluded/dep.d.ts','interface Dep {}');
  const p=produce(options);assert(p.snapshot.files.some(f=>f.id==='project/excluded/dep.d.ts'));assert(!p.snapshot.program_files.includes('project/excluded/dep.d.ts'));withheld(p);
}));
test('incidental Program membership cannot discharge noResolve path',()=>fixture(({put,save,config,options})=>{
  config.compilerOptions.noResolve=true;save();put('src/ref.d.ts','/// <reference path="./dep.d.ts" />');put('src/dep.d.ts','interface Dep {}');
  const p=produce(options);assert(p.snapshot.program_files.includes('project/src/dep.d.ts'));assert.deepEqual(p.diagnostics,[]);withheld(p);
}));
for(const target of ['./absent','./present.txt','./ref.d.ts'])test(`missing extensionless, unsupported extension, or self path refuses: ${target}`,()=>fixture(({put,options})=>{
  put('src/ref.d.ts',`/// <reference path="${target}" />`);put('src/present.txt','interface Dep {}');const p=produce(options);assert.deepEqual(p.diagnostics,[]);withheld(p);
}));
test('imported declaration transitive path is required',()=>fixture(({put,options})=>{
  put('src/app.ts','import "./dep";'+app);put('src/dep.d.ts','/// <reference path="../excluded/first.d.ts" />\nexport {};');
  put('excluded/first.d.ts','/// <reference path="./absent.d.ts" />');const p=produce(options);assert(p.resolutions.every(r=>r.target!==null));assert.deepEqual(p.diagnostics,[]);withheld(p);
}));
test('outside path is refused without publishing absolute spelling',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference path="/not-an-admitted-input/absent.d.ts" />');const p=produce(options);withheld(p);assert(p.reasons.includes('outside_lookup'));
  assert(!JSON.stringify(p).includes('/not-an-admitted-input'));
}));
test('in-root symlink target keeps canonical source ownership',()=>fixture(({put,root,options})=>{
  put('src/ref.d.ts','/// <reference path="../alias/dep.d.ts" />');put('excluded/dep.d.ts','interface Dep {}');symlinkSync('excluded',path.join(root,'alias'));
  const p=produce({...options,links:'in-root'});complete(p);assert(p.snapshot.program_files.includes('project/excluded/dep.d.ts'));
}));
for(const present of [false,true])test(`redirect original path is not discharged by inherited AST or incidental target: present=${present}`,()=>fixture(({put,options})=>{
  for(const name of ['a','b']){
    put(`node_modules/${name}/package.json`,JSON.stringify({name,version:'1.0.0',types:'index.d.ts'}));put(`node_modules/${name}/index.d.ts`,'import "shared";export {};');
    put(`node_modules/${name}/node_modules/shared/package.json`,JSON.stringify({name:'shared',version:'1.0.0',types:'index.d.ts'}));
    put(`node_modules/${name}/node_modules/shared/index.d.ts`,name==='a'?'export {};':'/// <reference path="../../../../../src/dep.d.ts" />\nexport {};');
  }
  if(present)put('src/dep.d.ts','interface Dep {}');put('src/app.ts','import "a";import "b";'+app);
  const p=produce(options);assert(p.snapshot.program_files.includes('project/node_modules/b/node_modules/shared/index.d.ts'));assert.equal(p.snapshot.program_files.includes('project/src/dep.d.ts'),present);
  assert.deepEqual(p.diagnostics,[]);withheld(p);
}));
test('ordinary failed module candidates do not withhold closure',()=>fixture(({put,options})=>{
  put('src/app.ts','import "./dep";'+app);put('src/dep.d.ts','export {};');const p=produce(options);assert(p.snapshot.failed_lookups.includes('project/src/dep.ts'));complete(p);
}));
test('absent path becoming present invalidates prior packet and removes only its refusal',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference path="./dep.d.ts" />');const p=produce(options);withheld(p);assert.equal(validate(JSON.stringify(p),options).valid,true);
  put('src/dep.d.ts','interface Dep {}');assert.equal(validate(JSON.stringify(p),options).valid,false);complete(produce(options));
}));
test('path refusal closure promotions fail schema validation before root I/O',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference path="./absent.d.ts" />');const p=produce(options);let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  // An independent valid refusal packet makes the parser RED meaningful on base.
  const q=structuredClone(p);q.reasons=['unproven_path_reference'];q.status='unproven';
  for(const k of ['dependencies','references','augmentation','resolution'])q.closure[k]=false;
  for(const o of q.observations)for(const c of o.nested.calls){c.props_class.status='unproven';c.props_class.reason='program_unproven';}
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(q)));
  for(const k of ['dependencies','references','augmentation','resolution']){
    const r=structuredClone(q);r.closure[k]=true;assert.throws(()=>parsePacket(JSON.stringify(r)),/invalid_packet/);assert.equal(validate(JSON.stringify(r),forbidden).valid,false);
  }
  const old=structuredClone(q);old.producer.version='0.11.0';assert.throws(()=>parsePacket(JSON.stringify(old)),/invalid_packet/);
  assert.equal(validate(JSON.stringify(old),forbidden).valid,false);
  assert.equal(reads,0);
  // Removing the reason and forging all bits can be well-shaped, but cannot
  // manufacture the occurrence evidence that full recomputation requires.
  const erased=structuredClone(p);erased.reasons=[];erased.status='observed';for(const k of Object.keys(erased.closure))erased.closure[k]=true;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(erased)));assert.equal(validate(JSON.stringify(erased),options).valid,false);
}));
test('historical schema10 packet remains readable but cannot validate as current',()=>fixture(({options})=>{
  const p=produce(options);complete(p);const old=structuredClone(p);old.schema='prism.callable-observation/10';old.producer.version='0.11.0';delete old.type_lib_references;delete old.type_lib_entries;delete old.search_provenance;delete old.config_provenance;delete old.entry_obligations;
  assert.doesNotThrow(()=>parsePacket(JSON.stringify(old)));assert.equal(validate(JSON.stringify(old),options).valid,false);
}));
test('same-byte wrong-file substitution does not discharge a required path',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference path="../excluded/dep.d.ts" />');put('src/dep.d.ts','interface Dep {}');const p=produce(options);
  assert(p.snapshot.program_files.includes('project/src/dep.d.ts'));assert.deepEqual(p.diagnostics,[]);withheld(p);
}));
test('BOM Unicode original source and repeated directive indices remain valid',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','\uFEFF// \u{1f600}\n/// <reference path="./dep.d.ts" />\n/// <reference path="./dep.d.ts" />');put('src/dep.d.ts','interface Dep {}');complete(produce(options));
}));
test('a later missing directive cannot be hidden by an earlier successful index',()=>fixture(({put,options})=>{
  put('src/ref.d.ts','/// <reference path="./dep.d.ts" />\n/// <reference path="./absent.d.ts" />');put('src/dep.d.ts','interface Dep {}');withheld(produce(options));
}));
test('noResolve without path directives does not acquire a new refusal',()=>fixture(({save,config,options})=>{
  config.compilerOptions.noResolve=true;save();complete(produce(options));
}));
