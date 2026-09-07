// Compiler characterization and existing refusal controls, not implementation RED.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,mkdtempSync,mkdirSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createRequire} from 'node:module';
import {COMPILER_HASH,hash} from './schema.mjs';
import {produce,validate} from './index.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);
const empty='declare module "*.scss" {}',shorthand='declare module "*.scss";',typed='declare module "*.scss" {export const value:string;}';
function inspect(declarations,sideEffect=false){
  const files=new Map(declarations.map((s,i)=>[`/fixture/p${i}.d.ts`,s]));
  files.set('/fixture/app.ts',sideEffect?'import "./absent.scss";':'import def, {value, missing} from "./absent.scss";void def;void value;void missing;');
  const options={strict:true,noEmit:true,types:[],module:ts.ModuleKind.ESNext,moduleResolution:ts.ModuleResolutionKind.Node10,noUncheckedSideEffectImports:true,libReplacement:false};
  const host=ts.createCompilerHost(options),original=host.getSourceFile;
  host.getSourceFile=(f,v,...rest)=>files.has(f)?ts.createSourceFile(f,files.get(f),v,true):original(f,v,...rest);
  const p=ts.createProgram([...files.keys()],options,host),c=p.getTypeChecker(),imp=p.getSourceFile('/fixture/app.ts').statements[0],s=c.getSymbolAtLocation(imp.moduleSpecifier);
  const names=sideEffect?[]:[imp.importClause.name,...imp.importClause.namedBindings.elements.map(e=>e.name)];
  return {symbol:s,selected:s?.valueDeclaration?.getSourceFile().fileName,shorthand:!!s?.valueDeclaration&&!s.valueDeclaration.body,
    exports:s?c.getExportsOfModule(s).map(e=>e.name):[],types:names.map(n=>c.typeToString(c.getTypeAtLocation(n))),
    codes:ts.getPreEmitDiagnostics(p).map(d=>d.code)};
}
for(const [first,second] of [[empty,shorthand],[shorthand,empty],[empty,empty],[shorthand,shorthand]]){
  test(`merged source shape and order: ${first} then ${second}`,()=>{
    const r=inspect([first,second]);assert.equal(r.symbol.declarations.length,2);assert.equal(r.selected,'/fixture/p0.d.ts');
    assert.equal(r.shorthand,first===shorthand);assert.deepEqual(r.exports,[]);
    assert.deepEqual(r.codes,first===shorthand?[]:[1192,2305,2305]);assert.deepEqual(r.types,['any','any','any']);
    assert.deepEqual(inspect([first,second],true).codes,[],'side-effect diagnostics do not distinguish either semantics');
  });
}
for(const reverse of [false,true])test(`typed and shorthand merge preserves selected-declaration behavior: reverse=${reverse}`,()=>{
  const r=inspect(reverse?[shorthand,typed]:[typed,shorthand]);assert.equal(r.symbol.declarations.length,2);assert.deepEqual(r.exports,['value']);
  assert.equal(r.selected,'/fixture/p0.d.ts');assert.equal(r.shorthand,reverse);
  assert.deepEqual(r.types,reverse?['any','any','any']:['any','string','any']);assert.deepEqual(r.codes,reverse?[]:[1192,2305]);
  assert.deepEqual(inspect(reverse?[shorthand,typed]:[typed,shorthand],true).codes,[]);
});
test('duplicate typed declarations can diagnose even when two anchors bind',()=>{
  const r=inspect([typed,typed],true);assert.equal(r.symbol.declarations.length,2);assert.deepEqual(r.codes,[2451,2451]);
});
for(const exists of [false,true])test(`merged side-effect binding withholds closure independently of asset inventory=${exists}`,()=>{
  const root=mkdtempSync(path.join(tmpdir(),'prism-merged-proof-'));
  const put=(f,s)=>{mkdirSync(path.dirname(path.join(root,f)),{recursive:true});writeFileSync(path.join(root,f),s);};
  const options={root,compiler,config:'tsconfig.json'};
  try{
    put('tsconfig.json',JSON.stringify({compilerOptions:{strict:true,noEmit:true,types:[],module:'ESNext',moduleResolution:'node',noUncheckedSideEffectImports:true,libReplacement:false},include:['src']}));
    put('src/a.d.ts',empty);put('src/b.d.ts',shorthand);put('src/app.ts','import "./styles.scss";');
    if(exists)put('src/styles.scss','.fixture {}');
    const p=produce(options),r=p.resolutions.find(r=>r.specifier==='./styles.scss');assert(r);
    assert.equal(r.lookup.wildcard.reason,'duplicate_provider');assert.equal(r.lookup.wildcard.providers.length,2);assert.equal(r.target,null);
    assert.deepEqual(p.diagnostics,[]);assert.equal(p.status,'unproven');assert.equal(p.closure.resolution,false);
    assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
    assert.equal(p.snapshot.files.some(f=>f.id==='project/src/styles.scss'),exists);
    put('src/a.d.ts',shorthand);put('src/b.d.ts',empty);
    assert.equal(validate(JSON.stringify(p),options).valid,false,'provider-order/source mutation invalidates the packet');
    assert.equal(produce(options).resolutions.find(r=>r.specifier==='./styles.scss').lookup.wildcard.reason,'duplicate_provider');
  }finally{rmSync(root,{recursive:true,force:true});}
});
