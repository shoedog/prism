import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,cpSync,symlinkSync} from 'node:fs';
import {tmpdir} from 'node:os';import path from 'node:path';import {pathToFileURL} from 'node:url';
const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const url=f=>implementation?pathToFileURL(path.join(implementation,f)):new URL(f,import.meta.url);
const {produce,validate}=await import(url('index.mjs'));const {hash,COMPILER_HASH,parsePacket}=await import(url('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler);assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-entry-test-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],libReplacement:false,skipLibCheck:true},include:['src']};
  const save=()=>put('tsconfig.json',JSON.stringify(config));
  try{save();put('src/app.ts','export {};');return run({root,put,config,save,options:{root,compiler,config:'tsconfig.json'}});}finally{rmSync(root,{recursive:true,force:true});}}
function entries(p,kind){assert(!p.reasons.includes('worker_failed'));assert(Array.isArray(p.type_lib_entries),'entry observation channel must exist');return p.type_lib_entries.filter(r=>r.kind===kind);}
function observed(p,r,target){assert(r);assert.equal(r.status,'observed');assert.equal(r.reason,null);assert.equal(r.target,target);assert.equal(r.inclusion,true);assert.equal(r.mode,null);assert(p.snapshot.program_files.includes(target));}
function unproven(r,reason){assert.equal(r.status,'unproven');assert.equal(r.reason,reason);assert.equal(r.target,null);assert.equal(r.inclusion,false);}
for(const automatic of [false,true])test(`actual ${automatic?'automatic':'configured'} type entries are separate from source directives`,()=>fixture(({put,config,save,options})=>{
  if(automatic)delete config.compilerOptions.types;else config.compilerOptions.types=['fixture'];save();
  put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options),rr=entries(p,'types');assert.equal(rr.length,1);assert.equal(rr[0].origin,automatic?'automatic':'configured');assert.equal(rr[0].name,'fixture');assert.equal(rr[0].index,0);observed(p,rr[0],'project/node_modules/@types/fixture/index.d.ts');assert.equal(p.type_lib_references.filter(r=>r.kind==='types').length,0);
}));
test('explicit empty types and libs do not discover installed packages or default libs',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.lib=[];save();put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options);assert.deepEqual(entries(p,'types'),[]);assert.deepEqual(entries(p,'lib'),[]);
}));
test('duplicate configured types retain occurrence indices despite name-keyed cache',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.types=['fixture','fixture'];save();put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options),rr=entries(p,'types');assert.deepEqual(rr.map(r=>r.index),[0,1]);for(const r of rr)observed(p,r,'project/node_modules/@types/fixture/index.d.ts');
}));
test('automatic census respects typeRoots, hidden packages and typings:null',()=>fixture(({put,config,save,options})=>{
  delete config.compilerOptions.types;config.compilerOptions.typeRoots=['./types'];save();
  for(const name of ['keep','.hidden','disabled'])put(`types/${name}/index.d.ts`,'interface Fixture {}');
  put('types/disabled/package.json','{"typings":null}');put('node_modules/@types/outside/index.d.ts','interface Other {}');
  const p=produce(options),rr=entries(p,'types');assert.deepEqual(rr.map(r=>r.name),['keep']);observed(p,rr[0],'project/types/keep/index.d.ts');
}));
for(const automatic of [false,true])test(`missing ${automatic?'automatic':'configured'} provider retains unresolved cache result`,()=>fixture(({put,config,save,options})=>{
  if(automatic){delete config.compilerOptions.types;put('node_modules/@types/missing/package.json','{"name":"@types/missing"}');}else config.compilerOptions.types=['missing'];save();
  const p=produce(options),[r]=entries(p,'types');assert.equal(r.name,'missing');unproven(r,'unresolved');assert(p.diagnostics.some(d=>d.code===2688));
}));
test('type entry mode remains null even in a module package with conditional exports',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.module='NodeNext';config.compilerOptions.moduleResolution='NodeNext';config.compilerOptions.types=['fixture'];save();put('package.json','{"type":"module"}');
  put('node_modules/fixture/package.json','{"name":"fixture","version":"1.0.0","exports":{".":{"import":"./import.d.mts","require":"./require.d.cts"}}}');
  put('node_modules/fixture/import.d.mts','export {};');put('node_modules/fixture/require.d.cts','export {};');const p=produce(options);observed(p,entries(p,'types')[0],'project/node_modules/fixture/require.d.cts');
}));
test('noResolve does not disable configured type entry processing',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.noResolve=true;config.compilerOptions.types=['fixture'];save();put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options);observed(p,entries(p,'types')[0],'project/node_modules/@types/fixture/index.d.ts');
}));
test('effective inherited options are observed without inventing config declaration spans',()=>fixture(({put,config,save,options})=>{
  delete config.compilerOptions.types;config.extends='./base.json';save();put('base.json','{"compilerOptions":{"types":["fixture"],"lib":["es5"]}}');put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');
  const p=produce(options);observed(p,entries(p,'types')[0],'project/node_modules/@types/fixture/index.d.ts');assert(p.snapshot.config_files.includes('project/base.json'));assert.equal(entries(p,'lib')[0].name,'lib.es5.d.ts');assert(!Object.hasOwn(entries(p,'types')[0],'request'));
}));
test('configured libs retain normalized names, order and repeated option indices',()=>fixture(({config,save,options})=>{
  config.compilerOptions.lib=['es5','dom','es5'];save();const p=produce(options),rr=entries(p,'lib');assert.deepEqual(rr.map(r=>[r.origin,r.index,r.name]),[['configured',0,'lib.es5.d.ts'],['configured',1,'lib.dom.d.ts'],['configured',2,'lib.es5.d.ts']]);for(const r of rr)observed(p,r,'compiler/'+r.name);
}));
test('default lib entry is the indexless default target, not each transitive lib',()=>fixture(({options})=>{
  const p=produce(options),rr=entries(p,'lib');assert.equal(rr.length,1);assert.equal(rr[0].origin,'default');assert.equal(rr[0].index,0);assert.equal(rr[0].name,'lib.es2022.full.d.ts');observed(p,rr[0],'compiler/lib.es2022.full.d.ts');assert(p.type_lib_references.some(r=>r.kind==='lib'));
}));
for(const replacement of [false,true])test(`configured lib replacement=${replacement} uses actual cached target`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.lib=['dom'];config.compilerOptions.libReplacement=true;save();if(replacement){put('node_modules/@typescript/lib-dom/package.json','{"name":"@typescript/lib-dom","version":"1.0.0","types":"index.d.ts"}');put('node_modules/@typescript/lib-dom/index.d.ts','interface Fixture {}');}
  const p=produce(options);observed(p,entries(p,'lib')[0],replacement?'project/node_modules/@typescript/lib-dom/index.d.ts':'compiler/lib.dom.d.ts');
}));
for(const configured of [false,true])test(`noLib leaves ${configured?'configured':'default'} entry unprocessed despite incidental membership`,()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.noLib=true;if(configured)config.compilerOptions.lib=['es5'];save();put('src/app.ts',`/// <reference path="../../compiler/${configured?'lib.es5.d.ts':'lib.es2022.full.d.ts'}" />`);
  const p=produce(options),[r]=entries(p,'lib');assert(p.snapshot.program_files.includes('compiler/'+r.name));unproven(r,'unprocessed');
}));
test('no-default-lib source suppresses implicit entry without claiming cause or borrowing builtin presence',()=>fixture(({put,options})=>{
  put('src/app.ts','/// <reference no-default-lib="true" />\n/// <reference path="../../compiler/lib.es2022.full.d.ts" />');const p=produce(options),[r]=entries(p,'lib');assert(p.snapshot.program_files.includes('compiler/'+r.name));unproven(r,'unprocessed');
}));
test('empty root population does not borrow configured type provider inventory',()=>fixture(({put,config,save,options})=>{
  config.include=['absent'];config.compilerOptions.types=['fixture'];save();put('node_modules/@types/fixture/index.d.ts','interface Fixture {}');const p=produce(options);assert.equal(p.snapshot.roots.length,0);unproven(entries(p,'types')[0],'unprocessed');unproven(entries(p,'lib')[0],'unprocessed');
}));
test('absent configured builtin is not observed from its cached filename',()=>fixture(({config,save,options})=>{
  const copy=mkdtempSync(path.join(tmpdir(),'prism-entry-compiler-'));try{cpSync(path.dirname(compiler),copy,{recursive:true,filter:f=>path.basename(f)!=='lib.dom.d.ts'});config.compilerOptions.lib=['dom'];save();const p=produce({...options,compiler:path.join(copy,path.basename(compiler))});unproven(entries(p,'lib')[0],'target_not_in_program');}finally{rmSync(copy,{recursive:true,force:true});}
}));
test('canonical in-root type entry target uses actual canonical Program identity',()=>fixture(({root,put,config,save,options})=>{
  config.compilerOptions.types=['./alias/dep'];save();put('excluded/dep.d.ts','interface Fixture {}');symlinkSync('excluded',path.join(root,'alias'));const p=produce({...options,links:'in-root'});observed(p,entries(p,'types')[0],'project/excluded/dep.d.ts');
}));
test('entry omission, origin/index and two genuine same-byte target substitutions fail reproduction',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.types=['one','two'];save();for(const name of ['one','two'])put(`node_modules/@types/${name}/index.d.ts`,'interface Shared {}');const p=produce(options),rr=entries(p,'types');assert.equal(rr.length,2);assert.equal(validate(JSON.stringify(p),options).valid,true);
  for(const alter of [q=>q.type_lib_entries.pop(),q=>q.type_lib_entries.find(r=>r.kind==='types').origin='automatic',q=>q.type_lib_entries.find(r=>r.kind==='types').index=99,q=>q.type_lib_entries.find(r=>r.kind==='types').target=rr[1].target]){const q=structuredClone(p);alter(q);assert.equal(validate(JSON.stringify(q),options).valid,false);}
  config.compilerOptions.types=['two'];save();assert.equal(validate(JSON.stringify(p),options).valid,false);
}));
test('illegal kind/origin/mode, duplicate and target/inclusion forgeries reject before root I/O',()=>fixture(({options})=>{
  const p=produce(options);entries(p,'lib');let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const alter of [q=>q.type_lib_entries[0].origin='automatic',q=>q.type_lib_entries[0].mode='import',q=>q.type_lib_entries.push(q.type_lib_entries[0]),q=>q.type_lib_entries[0].target='project/absent.d.ts',q=>q.type_lib_entries[0].inclusion=false]){const q=structuredClone(p);alter(q);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);}assert.equal(reads,0);
}));
test('historical schema11 remains readable without invented entries but is not current-valid',()=>fixture(({options})=>{
  const p=produce(options);entries(p,'lib');const old=structuredClone(p);old.schema='prism.callable-observation/11';old.producer.version='0.12.0';delete old.type_lib_entries;delete old.search_provenance;delete old.config_provenance;delete old.entry_obligations;assert.doesNotThrow(()=>parsePacket(JSON.stringify(old)));assert.equal(Object.hasOwn(parsePacket(JSON.stringify(old)),'type_lib_entries'),false);assert.equal(validate(JSON.stringify(old),options).valid,false);
}));
test('automatic duplicates across typeRoots preserve enumeration occurrences and selected target',()=>fixture(({put,config,save,options})=>{
  delete config.compilerOptions.types;config.compilerOptions.typeRoots=['./types-a','./types-b'];save();for(const dir of ['types-a','types-b'])put(`${dir}/fixture/index.d.ts`,'interface Fixture {}');
  const p=produce(options),rr=entries(p,'types');assert.deepEqual(rr.map(r=>[r.origin,r.index,r.name]),[['automatic',0,'fixture'],['automatic',1,'fixture']]);for(const r of rr)observed(p,r,'project/types-a/fixture/index.d.ts');
}));
test('configured lib cannot borrow a transitive source-lib cache when entry traversal was suppressed',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.lib=['es5'];save();put('src/app.ts','/// <reference no-default-lib="true" />\n/// <reference lib="es5" />');const p=produce(options);assert(p.snapshot.program_files.includes('compiler/lib.es5.d.ts'));assert(p.type_lib_references.some(r=>r.target==='compiler/lib.es5.d.ts'));unproven(entries(p,'lib')[0],'missing_inclusion');
}));
test('missing default builtin retains failure instead of inventing an observed fallback',()=>fixture(({options})=>{
  const copy=mkdtempSync(path.join(tmpdir(),'prism-entry-default-compiler-'));try{cpSync(path.dirname(compiler),copy,{recursive:true,filter:f=>path.basename(f)!=='lib.es2022.full.d.ts'});const p=produce({...options,compiler:path.join(copy,path.basename(compiler))});const [r]=entries(p,'lib');assert.equal(r.origin,'default');assert.equal(r.status,'unproven');assert.equal(r.target,null);assert.equal(r.inclusion,false);assert(p.diagnostics.some(d=>d.code===6053));}finally{rmSync(copy,{recursive:true,force:true});}
}));
test('package selection epoch changes reject prior entries despite same-byte provider files',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.types=['fixture'];save();for(const name of ['one','two'])put(`node_modules/fixture/${name}.d.ts`,'interface Shared {}');
  put('node_modules/fixture/package.json','{"name":"fixture","version":"1.0.0","types":"one.d.ts"}');const p=produce(options);observed(p,entries(p,'types')[0],'project/node_modules/fixture/one.d.ts');
  put('node_modules/fixture/package.json','{"name":"fixture","version":"1.0.0","types":"two.d.ts"}');const q=produce(options);observed(q,entries(q,'types')[0],'project/node_modules/fixture/two.d.ts');assert.equal(validate(JSON.stringify(p),options).valid,false);
}));
