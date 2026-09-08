import test from'node:test';import assert from'node:assert/strict';
import{mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,symlinkSync,cpSync}from'node:fs';
import{tmpdir}from'node:os';import path from'node:path';import{pathToFileURL}from'node:url';

const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const url=file=>implementation?pathToFileURL(path.join(implementation,file)):new URL(file,import.meta.url);
const{produce,validate}=await import(url('index.mjs'));const{parsePacket,hash,COMPILER_HASH}=await import(url('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
function fixture(run){const root=mkdtempSync(path.join(tmpdir(),'prism-lib-search-'));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const link=(target,file)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});symlinkSync(target,path.join(root,file));};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Node',types:[],lib:['dom'],libReplacement:true,skipLibCheck:true},files:['src/app.ts']};
  const save=(file='tsconfig.json')=>put(file,JSON.stringify(config));
  try{save();put('src/app.ts','export {};');return run({root,put,link,config,save,options:{root,compiler,config:'tsconfig.json'}});}finally{rmSync(root,{recursive:true,force:true});}}
const refs=p=>p.type_lib_references.filter(r=>r.kind==='lib'),entries=p=>p.type_lib_entries.filter(r=>r.kind==='lib');
function observed(row,target){assert(row);assert.equal(row.status,'observed');assert.equal(row.target,target);}
function ledger(p){assert(!p.reasons.includes('worker_failed'),'worker must complete');assert(Array.isArray(p.search_provenance?.lib_searches),'missing lib-search ledger');return p.search_provenance;}
function replacement(put,name='dom'){put(`node_modules/@typescript/lib-${name}/package.json`,JSON.stringify({name:`@typescript/lib-${name}`,version:'1.0.0',types:'index.d.ts'}));put(`node_modules/@typescript/lib-${name}/index.d.ts`,`interface Replacement${name.replace(/\W/g,'')} {}`);}

test('library replacement success retains resolver target, selected file and configured beneficiary',()=>fixture(({put,options})=>{
  replacement(put);const p=produce(options),target='project/node_modules/@typescript/lib-dom/index.d.ts';observed(entries(p)[0],target);const s=ledger(p),[r]=s.lib_searches;
  assert.deepEqual(r,{id:0,name:'@typescript/lib-dom',from:'project/__lib_node_modules_lookup_lib.dom.d.ts__.ts',lib_file:'lib.dom.d.ts',target,selected:target,
    beneficiaries:[{origin:'configured',request:null,index:0}]});
}));

test('replacement miss retains null resolver target and selected builtin fallback',()=>fixture(({options})=>{
  const p=produce(options);observed(entries(p)[0],'compiler/lib.dom.d.ts');const [r]=ledger(p).lib_searches;
  assert.equal(r.name,'@typescript/lib-dom');assert.equal(r.target,null);assert.equal(r.selected,'compiler/lib.dom.d.ts');assert.deepEqual(r.beneficiaries,[{origin:'configured',request:null,index:0}]);
}));

test('missing builtin fallback remains selected with no invented beneficiary',()=>fixture(({options})=>{
  const copy=mkdtempSync(path.join(tmpdir(),'prism-lib-compiler-'));try{
    cpSync(path.dirname(compiler),copy,{recursive:true,filter:file=>path.basename(file)!=='lib.dom.d.ts'});
    const selected='compiler/lib.dom.d.ts',p=produce({...options,compiler:path.join(copy,path.basename(compiler))}),[entry]=entries(p);assert.equal(entry.reason,'target_not_in_program');
    const [r]=ledger(p).lib_searches;assert.equal(r.target,null);assert.equal(r.selected,selected);assert.deepEqual(r.beneficiaries,[]);
  }finally{rmSync(copy,{recursive:true,force:true});}
}));

test('shared source and duplicate configured lib demands share one search with every positive beneficiary',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.lib=['dom','dom'];save();put('src/app.ts','/// <reference lib="dom" />\n/// <reference lib="dom" />\nexport {};');
  const p=produce(options),source=refs(p).filter(r=>r.name==='dom'),configured=entries(p).filter(r=>r.name==='lib.dom.d.ts');assert.equal(source.length,2);assert.equal(configured.length,2);
  for(const row of [...source,...configured])observed(row,'compiler/lib.dom.d.ts');const [r]=ledger(p).lib_searches;assert.equal(ledger(p).lib_searches.length,1);
  assert.deepEqual(r.beneficiaries.map(b=>[b.origin,b.index]),[['source',0],['source',1],['configured',0],['configured',1]]);
  assert.notDeepEqual(r.beneficiaries[0].request,r.beneficiaries[1].request);
}));

test('aliased source lib anchor is canonical while library lookup address stays lexical',()=>fixture(({put,link,options})=>{
  put('real/tsconfig.json',JSON.stringify({compilerOptions:{types:[],lib:[],libReplacement:true,skipLibCheck:true},files:['ref.d.ts']}));
  put('real/ref.d.ts','/// <reference lib="dom" />');link('real','alias');replacement(put);
  const selected={...options,config:'alias/tsconfig.json',links:'in-root'},p=produce(selected),[row]=refs(p);observed(row,'project/node_modules/@typescript/lib-dom/index.d.ts');
  const [r]=ledger(p).lib_searches;assert.equal(r.from,'project/alias/__lib_node_modules_lookup_lib.dom.d.ts__.ts');assert.equal(r.selected,'project/node_modules/@typescript/lib-dom/index.d.ts');
  assert.equal(r.beneficiaries[0].request.file,'project/real/ref.d.ts');assert.equal(validate(JSON.stringify(p),selected).valid,true);
}));

test('libReplacement false retains positive rows without fabricating a search',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.libReplacement=false;save();put('src/app.ts','/// <reference lib="dom" />\nexport {};');const p=produce(options);
  for(const row of [...refs(p).filter(r=>r.name==='dom'),...entries(p)])observed(row,'compiler/lib.dom.d.ts');assert.deepEqual(ledger(p).lib_searches,[]);
}));

test('default selection has no fabricated search while transitive source lib searches remain visible',()=>fixture(({config,save,options})=>{
  delete config.compilerOptions.lib;save();const p=produce(options),searches=ledger(p).lib_searches;
  assert(entries(p).some(r=>r.origin==='default'&&r.status==='observed'));assert(searches.length>0);assert(!searches.some(r=>r.lib_file==='lib.es2022.full.d.ts'));
  assert(searches.every(r=>r.beneficiaries.every(b=>b.origin==='source')));
}));

test('noLib suppresses configured library processing and creates no search',()=>fixture(({config,save,options})=>{
  config.compilerOptions.noLib=true;save();const p=produce(options),[row]=entries(p);assert.equal(row.reason,'unprocessed');assert.deepEqual(ledger(p).lib_searches,[]);
}));

test('no-default-lib excludes configured beneficiary despite a source-backed cached search',()=>fixture(({put,options})=>{
  put('src/app.ts','/// <reference no-default-lib="true" />\n/// <reference lib="dom" />\nexport {};');const p=produce(options),source=refs(p).find(r=>r.name==='dom'),[entry]=entries(p);
  observed(source,'compiler/lib.dom.d.ts');assert.equal(entry.reason,'missing_inclusion');const [r]=ledger(p).lib_searches;
  assert.deepEqual(r.beneficiaries.map(b=>b.origin),['source']);assert.equal(r.beneficiaries[0].request.file,'project/src/app.ts');
}));

for(const layout of ['config-file-alias','directory-alias','multi-hop-directory-alias','mixed-case'])test(`${layout} preserves lexical library lookup coordinates`,()=>fixture(({root,put,link,options})=>{
  let selected,expected;if(layout==='mixed-case'){
    selected='Config/TSconfig.json';expected='project/Config/__lib_node_modules_lookup_lib.dom.d.ts__.ts';put(selected,JSON.stringify({compilerOptions:{types:[],lib:['dom'],libReplacement:true,skipLibCheck:true},files:['app.ts']}));put('Config/app.ts','export {};');
  }else{
    put('real/tsconfig.json',JSON.stringify({compilerOptions:{types:[],lib:['dom'],libReplacement:true,skipLibCheck:true},files:layout==='config-file-alias'?['../real/app.ts']:['app.ts']}));put('real/app.ts','export {};');
    if(layout==='config-file-alias'){link('../real/tsconfig.json','alias/tsconfig.json');selected='alias/tsconfig.json';expected='project/alias/__lib_node_modules_lookup_lib.dom.d.ts__.ts';}
    if(layout==='directory-alias'){link('real','alias');selected='alias/tsconfig.json';expected='project/alias/__lib_node_modules_lookup_lib.dom.d.ts__.ts';}
    if(layout==='multi-hop-directory-alias'){link('real','alias-one');link('alias-one','alias-two');selected='alias-two/tsconfig.json';expected='project/alias-two/__lib_node_modules_lookup_lib.dom.d.ts__.ts';}
  }
  replacement(put);const selectedOptions={...options,config:selected,links:'in-root'},p=produce(selectedOptions),[r]=ledger(p).lib_searches;
  assert.equal(r.from,expected);assert.equal(r.selected,'project/node_modules/@typescript/lib-dom/index.d.ts');assert.equal(validate(JSON.stringify(p),selectedOptions).valid,true);assert(root);
}));

test('lib schema rejects orphan owners, wrong identities and incomplete or substituted beneficiaries before root I/O',()=>fixture(({put,config,save,options})=>{
  config.compilerOptions.lib=['dom','dom'];save();put('src/app.ts','/// <reference lib="dom" />\n/// <reference lib="dom" />\nexport {};');const p=produce(options),s=ledger(p),forbidden={get root(){throw Error('must not read root');}};
  const libEvent=s.boundary_events.findIndex(e=>e.owner?.channel==='lib');assert(libEvent>=0);assert.equal(s.lib_searches[0].beneficiaries.length,4);
  const mutations=[q=>q.search_provenance.boundary_events[libEvent].owner.id=999,q=>q.search_provenance.lib_searches[0].from='project/wrong.ts',
    q=>q.search_provenance.lib_searches[0].name='@typescript/lib-wrong',q=>q.search_provenance.lib_searches[0].lib_file='lib.es5.d.ts',
    q=>q.search_provenance.lib_searches[0].beneficiaries.pop(),q=>q.search_provenance.lib_searches[0].beneficiaries.push(structuredClone(q.search_provenance.lib_searches[0].beneficiaries[0])),
    q=>{const b=q.search_provenance.lib_searches[0].beneficiaries;[b[0].request,b[1].request]=[b[1].request,b[0].request];}];
  for(const mutate of mutations){const q=structuredClone(p);mutate(q);assert.throws(()=>parsePacket(JSON.stringify(q)),/invalid_packet/);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);}
}));

test('two genuine missing selected fallbacks cannot be substituted under reproduction',()=>fixture(({config,save,options})=>{
  config.compilerOptions.lib=['dom','es5'];save();const copy=mkdtempSync(path.join(tmpdir(),'prism-lib-compiler-'));try{
    cpSync(path.dirname(compiler),copy,{recursive:true,filter:file=>!['lib.dom.d.ts','lib.es5.d.ts'].includes(path.basename(file))});
    const selected={...options,compiler:path.join(copy,path.basename(compiler))},p=produce(selected),s=ledger(p);assert.equal(s.lib_searches.length,2);assert(s.lib_searches.every(r=>r.target===null&&!r.beneficiaries.length));
    const q=structuredClone(p);[q.search_provenance.lib_searches[0].selected,q.search_provenance.lib_searches[1].selected]=[q.search_provenance.lib_searches[1].selected,q.search_provenance.lib_searches[0].selected];
    assert.doesNotThrow(()=>parsePacket(JSON.stringify(q)));assert.equal(validate(JSON.stringify(q),selected).reason,'stale_or_tampered');
  }finally{rmSync(copy,{recursive:true,force:true});}
}));
