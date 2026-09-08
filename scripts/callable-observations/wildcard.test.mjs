import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from 'node:fs';import path from 'node:path';import {tmpdir} from 'node:os';
import {produce,validate} from './index.mjs';
import {hash,canonical} from './schema.mjs';
import {classifySemanticClosureV3} from './semantic-closure-v3.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
const app='class Client{m(){}}type View<P>=(p:P)=>void;export const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
const provider='declare module "*.asset" {export const value:string;export interface Thing{x:number}}';
function fixture(run){
  const root=mkdtempSync(path.join(tmpdir(),'prism-wildcard-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Bundler',baseUrl:'.',types:[],libReplacement:false,skipLibCheck:true},include:['src']};
  const source=s=>put('src/app.ts',s+app);
  try{put('package.json','{"type":"module"}');put('tsconfig.json',JSON.stringify(config));put('src/provider.d.ts',provider);
    source('import {value} from "./file.asset";');return run({put,source,config,options:{root,compiler,config:'tsconfig.json'}});
  }finally{rmSync(root,{recursive:true,force:true});}
}
function resolution(p,specifier='./file.asset',context='import'){
  const rs=p.resolutions.filter(r=>r.specifier===specifier&&r.lookup.context===context);assert.equal(rs.length,1,JSON.stringify(p.reasons));
  assert(Object.hasOwn(rs[0].lookup,'wildcard'),'wildcard observation field required');return rs[0];
}
function withheld(p){assert.equal(p.status,'unproven');assert(p.reasons.includes('unresolved_module'));
  for(const key of ['dependencies','augmentation','resolution'])assert.equal(p.closure[key],false);
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert.equal(p.observations.at(-1).nested.calls[0].props_class.reason,'program_unproven');}
function refreshSemantic(p){const observed=p.config_provenance.status==='observed',option=observed&&p.config_provenance.options.find(r=>r.name==='noResolve');
  p.semantic_closure=classifySemanticClosureV3({compilerVerified:p.compiler.verified,stableSnapshot:p.closure.stable_snapshot,
    configObserved:observed,entryComplete:p.entry_obligations.complete,noResolve:option?.present===true&&option.value_sha256===hash(canonical({present:true,value:true})),
    diagnosticCount:p.diagnostics.length,globalReasons:p.reasons,outside:p.snapshot.outside_lookups,refusedCount:p.snapshot.refused_lookup_sha256.length,
    boundaryCount:p.search_provenance.boundary_events.length,programFiles:p.snapshot.program_files,resolutions:p.resolutions});}
for(const exists of [false,true])test(`singleton wildcard binding is independent of asset presence=${exists}`,()=>fixture(({put,options})=>{
  if(exists)put('src/file.asset','asset bytes');const p=produce(options),r=resolution(p),w=r.lookup.wildcard;
  assert.equal(w.status,'observed');assert.equal(w.reason,null);assert.equal(w.pattern,'*.asset');
  assert.deepEqual(w.providers,r.lookup.declarations);assert.deepEqual(w.matches,w.providers);assert.deepEqual(w.augmentations,[]);
  assert.equal(w.providers[0].file,'project/src/provider.d.ts');assert.equal(r.target,null);assert.equal(r.lookup.reason,'non_exact_binding');
  assert.equal(p.snapshot.files.some(f=>f.id==='project/src/file.asset'),exists);withheld(p);assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
for(const [context,code] of [['export','export {value} from "./file.asset";'],['import_type','type T=import("./file.asset").Thing;'],['import_equals','import v = require("./file.asset");']])
  test(`supported ${context} wildcard context`,()=>fixture(({source,options})=>{source(code);const p=produce(options);assert.equal(resolution(p,'./file.asset',context).lookup.wildcard.status,'observed');withheld(p);}));
test('merged wildcard providers retain both anchors without promotion',()=>fixture(({put,options})=>{
  put('src/second.d.ts',provider);const w=resolution(produce(options)).lookup.wildcard;
  assert.equal(w.status,'unproven');assert.equal(w.reason,'duplicate_provider');assert.equal(w.providers.length,2);assert.equal(w.matches.length,2);
}));
test('competing matching pattern refuses even the compiler-selected longest prefix',()=>fixture(({put,options})=>{
  put('src/more.d.ts','declare module "./file.*" {export const value:string;}');const w=resolution(produce(options)).lookup.wildcard;
  assert.equal(w.status,'unproven');assert.equal(w.reason,'competing_pattern');assert.equal(w.pattern,'./file.*');assert.equal(w.matches.length,2);
}));
test('an unrelated nonmatching pattern does not poison singleton evidence',()=>fixture(({put,options})=>{
  put('src/other.d.ts','declare module "*.other" {export const value:string;}');const w=resolution(produce(options)).lookup.wildcard;assert.equal(w.status,'observed');assert.equal(w.matches.length,1);
}));
for(const name of ['*.asset','./file.asset'])test(`augmentation ${name} retains a barrier`,()=>fixture(({put,options})=>{
  put('src/augment.d.ts',`export {};declare module "${name}" {interface Added{x:number}}`);
  const w=resolution(produce(options)).lookup.wildcard;assert(w);assert.equal(w.status,'unproven');assert.equal(w.reason,'augmentation');assert(w.augmentations.length);
}));
test('excluded provider cannot supply wildcard authority',()=>fixture(({put,options})=>{
  put('src/provider.d.ts','');put('excluded/provider.d.ts',provider);const r=resolution(produce(options));assert.equal(r.lookup.wildcard,null);assert.equal(r.lookup.reason,'unresolved_symbol');
}));
for(const shorthand of [false,true])test(`unsupported provider stays unproven: shorthand=${shorthand}`,()=>fixture(({put,options})=>{
  put('src/provider.d.ts',shorthand?'declare module "*.asset";':'');if(!shorthand)put('src/provider.ts',provider);
  const w=resolution(produce(options)).lookup.wildcard;assert.equal(w.status,'unproven');assert.equal(w.reason,'unsupported_provider');
}));
for(const [pattern,specifier] of [['pre*suf','presuf'],['*','anything'],['./*?raw','./asset?raw']])test(`single-star matching handles ${pattern}`,()=>fixture(({put,source,options})=>{
  put('src/provider.d.ts',`declare module "${pattern}" {export const value:string;}`);source(`import {value} from "${specifier}";`);
  assert.equal(resolution(produce(options),specifier).lookup.wildcard.status,'observed');
}));
for(const [pattern,specifier] of [['ab*bc','abc'],['*.ASSET','./file.asset'],['*.*.asset','./file.asset']])test(`nonmatching/invalid pattern ${pattern} supplies no binding`,()=>fixture(({put,source,options})=>{
  put('src/provider.d.ts',`declare module "${pattern}" {export const value:string;}`);source(`import {value} from "${specifier}";`);
  assert.equal(resolution(produce(options),specifier).lookup.wildcard,null);
}));
test('exact ambient binding retains its existing lane and no wildcard observation',()=>fixture(({put,source,options})=>{
  put('src/exact.d.ts','declare module "exact.asset" {export const value:string;}');source('import {value} from "exact.asset";');
  const r=resolution(produce(options),'exact.asset');assert.equal(r.lookup.status,'observed');assert.equal(r.lookup.wildcard,null);
}));
test('dynamic import wildcard candidate is not a supported positive context',()=>fixture(({source,options})=>{
  source('const mod=import("./file.asset");');const w=resolution(produce(options),'./file.asset','dynamic_import').lookup.wildcard;
  assert.equal(w.status,'unproven');assert.equal(w.reason,'unsupported_request');
}));
test('adding and restoring duplicate provider invalidates the epoch',()=>fixture(({put,options})=>{
  const old=produce(options);assert.equal(resolution(old).lookup.wildcard.status,'observed');
  put('src/provider.d.ts',provider+'\n'+provider);assert.equal(resolution(produce(options)).lookup.wildcard.reason,'duplicate_provider');assert.equal(validate(JSON.stringify(old),options).valid,false);
  put('src/provider.d.ts',provider);assert.equal(validate(JSON.stringify(old),options).valid,true);
}));
test('malformed wildcard evidence rejects before audited-root access',()=>fixture(({options})=>{
  const p=produce(options);assert(resolution(p).lookup.wildcard);let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  for(const change of [q=>q.schema='prism.callable-observation/8',q=>q.resolutions[0].lookup.wildcard.asset_exists=true,
    q=>resolution(q).lookup.wildcard.providers=[],q=>resolution(q).lookup.wildcard.matches=[],
    q=>resolution(q).lookup.wildcard.augmentations=resolution(q).lookup.wildcard.providers,
    q=>resolution(q).lookup.wildcard.pattern='*.other',q=>resolution(q).lookup.wildcard.pattern='**',
    q=>resolution(q).lookup.wildcard.reason='duplicate_provider',q=>resolution(q).lookup.wildcard.providers[0].file='project/absent',
    q=>q.reasons=q.reasons.filter(r=>r!=='unresolved_module'),q=>q.closure.resolution=true]){
    const forged=structuredClone(p);change(forged);assert.equal(validate(JSON.stringify(forged),forbidden).valid,false);
  }assert.equal(reads,0);
}));
test('well-shaped forged wildcard disposition is rejected by full recomputation',()=>fixture(({options})=>{
  const p=produce(options),w=resolution(p).lookup.wildcard;assert(w);w.status='unproven';w.reason='binding_mismatch';
  refreshSemantic(p);
  assert.equal(validate(JSON.stringify(p),options).reason,'stale_or_tampered');
}));
test('JSDoc import types retain actual wildcard use anchors',()=>fixture(({put,config,options})=>{
  config.compilerOptions.allowJs=true;put('tsconfig.json',JSON.stringify(config));put('src/doc.js','/** @type {import("./file.asset").Thing} */\nexport const thing={x:1};');
  const p=produce(options),r=resolution(p,'./file.asset','import_type');assert.equal(r.lookup.wildcard.status,'observed');assert.equal(r.lookup.request.file,'project/src/doc.js');
}));
test('nested receiver writes stay blocked with observed wildcard bindings',()=>fixture(({source,options})=>{
  source('import {value} from "./file.asset";type F=(client:Client)=>void;const written:F=client=>{client=new Client();const cb=()=>client.m();};');
  const p=produce(options);assert.equal(resolution(p).lookup.wildcard.status,'observed');
  assert(p.observations.some(o=>o.nested.calls.some(c=>c.binding.reason==='write_barrier'&&c.props_class.status==='unproven')));
}));
test('same-pattern invalid nested provider cannot evade the independent census',()=>fixture(({put,options})=>{
  put('src/nested.d.ts','declare namespace Invalid {module "*.asset" {export const value:string;}}');
  const w=resolution(produce(options)).lookup.wildcard;assert.equal(w.status,'unproven');assert.equal(w.reason,'duplicate_provider');assert.equal(w.providers.length,2);
}));
for(const changed of [false,true])test(`redirected wildcard augmentation census retains source files: changed=${changed}`,()=>fixture(({put,source,options})=>{
  for(const name of ['a','b']){
    put(`node_modules/${name}/package.json`,JSON.stringify({name,version:'1.0.0',types:'index.d.ts'}));
    put(`node_modules/${name}/index.d.ts`,'import "shared";export {};');
    put(`node_modules/${name}/node_modules/shared/package.json`,JSON.stringify({name:'shared',version:'1.0.0',types:'index.d.ts'}));
    put(`node_modules/${name}/node_modules/shared/index.d.ts`,
      `export {};declare module "*.asset" {interface Added{x:number${changed&&name==='b'?';extra:string':''}}}`);
  }
  source('import "a";import "b";import {value} from "./file.asset";');
  const p=produce(options),w=resolution(p).lookup.wildcard;assert.equal(w.reason,'augmentation');assert.equal(w.status,'unproven');
  assert.deepEqual(w.augmentations.map(a=>a.file).sort(),['project/node_modules/a/node_modules/shared/index.d.ts','project/node_modules/b/node_modules/shared/index.d.ts']);
  assert.equal(w.augmentations[0].sha256!==w.augmentations[1].sha256,changed);assert.equal(validate(JSON.stringify(p),options).valid,true);
}));
test('removing the wildcard observation cannot survive recomputation',()=>fixture(({options})=>{
  const p=produce(options);assert.equal(resolution(p).lookup.wildcard.status,'observed');resolution(p).lookup.wildcard=null;
  refreshSemantic(p);
  assert.equal(validate(JSON.stringify(p),options).reason,'stale_or_tampered');
}));
test('require wildcard candidates remain outside the supported request contexts',()=>fixture(({put,config,options})=>{
  config.compilerOptions.allowJs=true;put('tsconfig.json',JSON.stringify(config));put('src/legacy.js','const v=require("./file.asset");');
  const w=resolution(produce(options),'./file.asset','require').lookup.wildcard;assert.equal(w.status,'unproven');assert.equal(w.reason,'unsupported_request');
}));
