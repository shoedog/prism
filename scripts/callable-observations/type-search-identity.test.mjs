import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,symlinkSync}from'node:fs';
import {tmpdir}from'node:os';import path from'node:path';
import{produce,validate}from'./index.mjs';import{parsePacket,hash,COMPILER_HASH}from'./schema.mjs';

const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-type-identity-'));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const link=(target,file)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});symlinkSync(target,path.join(root,file));};
  try{return run({root,put,link,options:config=>({root,compiler,config,links:'in-root'})});}finally{rmSync(root,{recursive:true,force:true});}}
const config=overrides=>JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:['missing'],libReplacement:false,skipLibCheck:true,...overrides?.compilerOptions},files:overrides?.files??['app.ts']});
function typeSearch(p,origin='configured'){
  assert(!p.reasons.includes('worker_failed'),'worker must complete');const s=p.search_provenance,b=s.type_batches.filter(x=>x.origin===origin),r=s.type_requests.filter(x=>x.origin===origin);
  assert(b.length);assert(r.length);return{s,b,r,q:s.type_searches.filter(x=>r.some(y=>y.execution===x.id))};
}

for(const layout of ['config-file-alias','directory-alias','multi-hop-directory-alias','mixed-case'])test(`${layout} preserves the selected lexical synthetic address`,()=>fixture(({put,link,options})=>{
  let selected,expected;
  if(layout==='mixed-case'){
    selected='Config/TSconfig.json';expected='project/Config/__inferred type names__.ts';put(selected,config());put('Config/app.ts','export {};');
  }else{
    put('real/tsconfig.json',config(layout==='config-file-alias'?{files:['../real/app.ts']}:undefined));put('real/app.ts','export {};');
    if(layout==='config-file-alias'){link('../real/tsconfig.json','alias/tsconfig.json');selected='alias/tsconfig.json';expected='project/alias/__inferred type names__.ts';}
    if(layout==='directory-alias'){link('real','alias');selected='alias/tsconfig.json';expected='project/alias/__inferred type names__.ts';}
    if(layout==='multi-hop-directory-alias'){link('real','alias-one');link('alias-one','alias-two');selected='alias-two/tsconfig.json';expected='project/alias-two/__inferred type names__.ts';}
  }
  const opts=options(selected),p=produce(opts),{b,r,q}=typeSearch(p);
  assert.equal(p.scope.config,'project/'+selected);assert(b.every(x=>x.from===expected));assert(r.every(x=>x.from===expected));assert(q.every(x=>x.from===expected));
  assert.equal(validate(JSON.stringify(p),opts).valid,true);
}));

test('alias and real config selections retain distinct epochs and reject synthetic-from substitution',()=>fixture(({put,link,options})=>{
  put('real/tsconfig.json',config());put('real/app.ts','export {};');link('real','alias');
  const aliasOptions=options('alias/tsconfig.json'),realOptions=options('real/tsconfig.json'),alias=produce(aliasOptions),real=produce(realOptions);
  const a=typeSearch(alias),r=typeSearch(real),aliasFrom='project/alias/__inferred type names__.ts',realFrom='project/real/__inferred type names__.ts';
  assert(a.b.every(x=>x.from===aliasFrom));assert(r.b.every(x=>x.from===realFrom));assert.notEqual(alias.snapshot.options_sha256,real.snapshot.options_sha256);
  assert.equal(validate(JSON.stringify(alias),realOptions).valid,false);
  const substituted=structuredClone(alias);for(const batch of substituted.search_provenance.type_batches)if(batch.origin!=='source')batch.from=realFrom;
  for(const request of substituted.search_provenance.type_requests)if(request.origin!=='source')request.from=realFrom;
  for(const search of substituted.search_provenance.type_searches)search.from=realFrom;
  assert.throws(()=>parsePacket(JSON.stringify(substituted)),/invalid_packet/);
}));

for(const origin of ['configured','automatic'])test(`${origin} alias entry batches retain numeric indices 0 through 11`,()=>fixture(({put,link,options})=>{
  const indices=Array.from({length:12},(_,index)=>index),compilerOptions={};
  if(origin==='configured')compilerOptions.types=indices.map(()=> 'missing');
  else {compilerOptions.types=undefined;compilerOptions.typeRoots=indices.map(index=>`./types-${index}`);for(const index of indices)put(`real/types-${index}/missing/index.d.ts`,`interface T${index} {}`);}
  put('real/tsconfig.json',config({compilerOptions}));put('real/app.ts','export {};');link('real','alias');
  const opts=options('alias/tsconfig.json'),p=produce(opts),{b,r,q}=typeSearch(p,origin),from='project/alias/__inferred type names__.ts';
  assert.equal(b.length,1);assert.deepEqual(b[0],{id:0,origin,from,size:12});assert.deepEqual(r.map(x=>x.index),indices);assert(r.every(x=>x.from===from));assert(q.every(x=>x.from===from));
  assert.equal(validate(JSON.stringify(p),opts).valid,true);
}));

test('source, selected type target and module aliases remain canonical file identities',()=>fixture(({put,link,options})=>{
  put('real/tsconfig.json',config({compilerOptions:{types:[]},files:['app.d.ts']}));
  put('real/app.d.ts','/// <reference types="../type-alias/dep.d.ts" />\nimport {x} from "./dep";export {x};');put('real/dep.d.ts','export const x:number;');
  put('type-real/dep.d.ts','interface TypeAlias {}');link('real','alias');link('type-real','type-alias');
  const opts=options('alias/tsconfig.json'),p=produce(opts),{b,r,q}=typeSearch(p,'source'),module=p.search_provenance.module_requests.find(x=>x.specifier==='./dep');
  assert(module);assert.equal(module.from,'project/real/app.d.ts');assert.equal(module.target,'project/real/dep.d.ts');
  assert.equal(b[0].from,'project/real/app.d.ts');assert.equal(r[0].from,'project/real/app.d.ts');assert.equal(r[0].request.file,'project/real/app.d.ts');assert.equal(q[0].target,'project/type-real/dep.d.ts');
  assert.equal(validate(JSON.stringify(p),opts).valid,true);
}));
