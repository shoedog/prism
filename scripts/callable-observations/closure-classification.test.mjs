// Characterization only: filesystem resolution, checker binding and closure differ.
import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,realpathSync} from 'node:fs';
import {tmpdir} from 'node:os';import path from 'node:path';import {createRequire} from 'node:module';
import {produce,validate} from './index.mjs';import {COMPILER_HASH,hash} from './schema.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);
const app='class Client{m(){}}type View<P>=(p:P)=>void;export const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';
function fixture(run){
  const root=realpathSync(mkdtempSync(path.join(tmpdir(),'prism-closure-classify-')));
  const put=(f,text)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),text);};
  const config={compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'node',baseUrl:'.',types:[],libReplacement:false},include:['src']};
  put('package.json','{"type":"module"}');put('tsconfig.json',JSON.stringify(config));put('src/app.ts',app);
  function inspect(file,specifier,parentKind){
    const parsed=ts.parseJsonConfigFileContent(JSON.parse(readFileSync(path.join(root,'tsconfig.json'))),ts.sys,root);
    assert.deepEqual(parsed.errors,[]);const program=ts.createProgram(parsed.fileNames,parsed.options),checker=program.getTypeChecker();
    const sf=program.getSourceFile(path.join(root,file)),uses=[];
    function visit(n){if(ts.isStringLiteral(n)&&n.text===specifier&&(!parentKind||ts.SyntaxKind[n.parent.kind]===parentKind))uses.push(n);ts.forEachChild(n,visit);}visit(sf);
    assert.equal(uses.length,1);const use=uses[0],symbol=checker.getSymbolAtLocation(use);
    return {defs:symbol?.declarations??[],target:ts.resolveModuleName(specifier,sf.fileName,parsed.options,ts.sys).resolvedModule,
      codes:[...new Set(ts.getPreEmitDiagnostics(program).map(d=>d.code))],options:parsed.options};
  }
  try{return run({root,put,config,inspect,options:{root,compiler,config:'tsconfig.json'}});}finally{rmSync(root,{recursive:true,force:true});}
}
function withheld(p){assert.equal(p.status,'unproven');assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert.equal(p.observations[0].nested.calls[0].props_class.reason,'program_unproven');}

test('exact ambient source symbol is not a filesystem target or closed Program',()=>fixture(({put,inspect,options})=>{
  put('src/ambient.d.ts','declare module "builtin-like" {export const value:number;}');
  put('src/app.ts','import {value} from "builtin-like";'+app);const raw=inspect('src/app.ts','builtin-like');
  assert.equal(raw.target,undefined);assert.deepEqual(raw.codes,[]);assert.equal(raw.defs.length,1);
  assert.equal(raw.defs[0].name.text,'builtin-like');withheld(produce(options));
}));
test('node prefix without an ambient provider supplies no source authority',()=>fixture(({put,inspect,options})=>{
  put('src/app.ts','import {value} from "node:unknown";'+app);const raw=inspect('src/app.ts','node:unknown');
  assert.equal(raw.target,undefined);assert.equal(raw.defs.length,0);assert(raw.codes.includes(2307));withheld(produce(options));
}));
test('exact virtual ambient module still preserves refused lookup evidence',()=>fixture(({put,inspect,options})=>{
  put('src/ambient.d.ts','declare module "virtual:input" {export const value:number;}');
  put('src/app.ts','import {value} from "virtual:input";'+app);const raw=inspect('src/app.ts','virtual:input');
  assert.equal(raw.target,undefined);assert.equal(raw.defs.length,1);assert.deepEqual(raw.codes,[]);
  const p=produce(options);withheld(p);assert(p.reasons.includes('unsupported_lookup'));assert(p.snapshot.refused_lookup_sha256.length);
}));
for(const exists of [false,true])test(`wildcard source binding does not certify an asset exists: exists=${exists}`,()=>fixture(({put,inspect,options})=>{
  put('src/ambient.d.ts','declare module "*.woff2" {const url:string;export default url;}');
  if(exists)put('src/font.woff2','fixture asset');put('src/app.ts','import url from "./font.woff2";'+app);
  const raw=inspect('src/app.ts','./font.woff2');assert.equal(raw.target,undefined);assert.deepEqual(raw.codes,[]);assert.equal(raw.defs[0].name.text,'*.woff2');
  const p=produce(options);withheld(p);assert.equal(p.snapshot.files.some(f=>f.id==='project/src/font.woff2'),exists);
}));
test('duplicate wildcard declaration population is visible despite clean diagnostics',()=>fixture(({put,inspect,options})=>{
  put('src/a.d.ts','declare module "*.scss" {export const value:string;}');put('src/b.d.ts','declare module "*.scss";');
  put('src/app.ts','import {value} from "./styles.scss";'+app);const raw=inspect('src/app.ts','./styles.scss');
  assert.equal(raw.target,undefined);assert.deepEqual(raw.codes,[]);assert.equal(raw.defs.length,2);withheld(produce(options));
}));
test('augmentation declaration-name symbol does not prove its missing import target',()=>fixture(({put,config,inspect,options})=>{
  config.compilerOptions.skipLibCheck=true;put('tsconfig.json',JSON.stringify(config));
  put('src/augment.d.ts','import {Thing} from "missing-package";declare module "missing-package" {interface Extra {x:number}}');
  const imported=inspect('src/augment.d.ts','missing-package','ImportDeclaration');
  const augmentation=inspect('src/augment.d.ts','missing-package','ModuleDeclaration');
  assert.equal(imported.defs.length,0);assert.equal(augmentation.defs.length,1);assert.equal(augmentation.target,undefined);
  assert.deepEqual(imported.codes,[]);withheld(produce(options));
}));
test('skipLibCheck suppresses missing declaration-import diagnostics, not missing source',()=>fixture(({put,config,inspect,options})=>{
  put('src/dependency.d.ts','import {Thing} from "missing-peer";export type Value=Thing;');
  for(const skip of [false,true]){
    config.compilerOptions.skipLibCheck=skip;put('tsconfig.json',JSON.stringify(config));
    const raw=inspect('src/dependency.d.ts','missing-peer');assert.equal(raw.defs.length,0);assert.equal(raw.target,undefined);
    assert.equal(raw.codes.includes(2307),!skip);withheld(produce(options));
  }
}));
test('unchecked missing side-effect imports can have zero diagnostics without source authority',()=>fixture(({put,config,inspect,options})=>{
  put('src/app.ts','import "./absent.css";'+app);
  for(const check of [false,true]){
    config.compilerOptions.noUncheckedSideEffectImports=check;put('tsconfig.json',JSON.stringify(config));
    const raw=inspect('src/app.ts','./absent.css');assert.equal(raw.defs.length,0);assert.equal(raw.target,undefined);
    assert.equal(raw.codes.includes(2307),check);withheld(produce(options));
  }
}));
test('exports-only declaration subpath is mode-sensitive; actual Node10 remains unchanged',()=>fixture(({put,inspect,options})=>{
  put('node_modules/pkg/package.json',JSON.stringify({name:'pkg',version:'1.0.0',exports:{'./hidden':{types:'./dist/hidden.d.ts'}}}));
  put('node_modules/pkg/dist/hidden.d.ts','export interface Thing {x:number;}');
  put('src/app.ts','import type {Thing} from "pkg/hidden";'+app);const raw=inspect('src/app.ts','pkg/hidden');
  assert.equal(raw.target,undefined);assert.equal(raw.defs.length,0);assert(raw.codes.includes(2307));
  const alternative=ts.resolveModuleName('pkg/hidden',path.join(options.root,'src/app.ts'),{...raw.options,moduleResolution:ts.ModuleResolutionKind.Bundler},ts.sys);
  assert(alternative.resolvedModule.resolvedFileName.endsWith('/dist/hidden.d.ts'));
  assert.equal(raw.options.moduleResolution,ts.ModuleResolutionKind.Node10);withheld(produce(options));
}));
test('excluded ambient provider is not admitted by spelling; membership changes replace the epoch',()=>fixture(({put,options})=>{
  put('outside/ambient.d.ts','declare module "virtual:input" {export const value:number;}');
  put('src/app.ts','import {value} from "virtual:input";'+app);const old=produce(options);assert(old.diagnostics.some(d=>d.code===2307));
  put('src/ambient.d.ts','declare module "virtual:input" {export const value:number;}');
  const fresh=produce(options);assert.deepEqual(fresh.diagnostics,[]);withheld(fresh);assert.equal(validate(JSON.stringify(old),options).valid,false);
}));
test('synthetic JSX runtime requests have no source span and must not be anchored as literals',()=>fixture(({put,config,root})=>{
  config.compilerOptions.jsx='react-jsx';put('tsconfig.json',JSON.stringify(config));put('src/view.tsx','export const view=<div/>;');
  const parsed=ts.parseJsonConfigFileContent(config,ts.sys,root),host=ts.createCompilerHost(parsed.options),requests=[];
  host.resolveModuleNameLiterals=(xs,from)=>xs.map(l=>{requests.push(l);return ts.resolveModuleName(l.text,from,parsed.options,host);});
  const program=ts.createProgram(parsed.fileNames,parsed.options,host);ts.getPreEmitDiagnostics(program);
  const runtime=requests.find(l=>l.text==='react/jsx-runtime'&&l.pos<0);assert(runtime);
  assert.throws(()=>runtime.getStart(),/real position/);
}));
test('ancestor search is distinguishable from an explicit outside-relative request',()=>{
  const calls=[];const host={fileExists:f=>{calls.push(f);return false;},directoryExists:f=>{calls.push(f);return false;},readFile:()=>undefined,getCurrentDirectory:()=>'/__prism__/project'};
  const options={moduleResolution:ts.ModuleResolutionKind.Node10};
  ts.resolveModuleName('absent','/__prism__/project/src/app.ts',options,host);
  assert(calls.includes('/__prism__/node_modules'));assert(calls.includes('/node_modules'));
  calls.length=0;ts.resolveModuleName('../../../escape','/__prism__/project/src/app.ts',options,host);
  assert(calls.some(f=>f==='/'||f.startsWith('/escape')));assert(!calls.includes('/__prism__/node_modules'));
});
test('workspace canonical package peer-ID probes are metadata, not the import target',()=>{
  const files=new Map([
    ['/__prism__/project/pkg/package.json',JSON.stringify({name:'pkg',version:'1.0.0',types:'index.d.ts',peerDependencies:{peer:'*'}})],
    ['/__prism__/project/pkg/index.d.ts','export interface Thing {}'],
  ]),probes=[];
  const host={fileExists:f=>files.has(f),readFile:f=>files.get(f),realpath:f=>f,directoryExists:f=>{probes.push(f);return [...files.keys()].some(k=>k.startsWith(f.replace(/\/+$/,'')+'/'));}};
  const result=ts.resolveModuleName('..','/__prism__/project/pkg/src/app.ts',{moduleResolution:ts.ModuleResolutionKind.Node10},host);
  assert.equal(result.resolvedModule.resolvedFileName,'/__prism__/project/pkg/index.d.ts');
  assert(probes.includes('/__prism__//peer'));
});
