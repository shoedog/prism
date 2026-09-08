// H1 helper/compiler characterization, not an integrated-producer RED claim.
import test from 'node:test';import assert from 'node:assert/strict';
import {readFileSync}from'node:fs';import{createRequire}from'node:module';
import {COMPILER_HASH,hash}from'./schema.mjs';
import {projectSyntheticAddress}from'./identity-domains.mjs';

const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);

test('synthetic projector preserves lexical case, aliases and absent filenames',()=>{
  for(const directory of['','Config/','alias/','alias-two/deep/']){
    const id=`project/${directory}__inferred type names__.ts`;
    assert.equal(projectSyntheticAddress('/__prism__/'+id),id);
    assert.equal(projectSyntheticAddress(`/__prism__/project/${directory}__lib_node_modules_lookup_lib.es5.d.ts__.ts`),
      `project/${directory}__lib_node_modules_lookup_lib.es5.d.ts__.ts`);
  }
  assert.notEqual(projectSyntheticAddress('/__prism__/project/alias/__inferred type names__.ts'),projectSyntheticAddress('/__prism__/project/real/__inferred type names__.ts'));
});
test('synthetic projector normalizes lexical dots without inventing file identity',()=>{
  assert.equal(projectSyntheticAddress('/__prism__/project/Config/./nested/../__inferred type names__.ts'),'project/Config/__inferred type names__.ts');
});
test('synthetic projector rejects unsafe, refused, outside and non-project domains',()=>{
  for(const value of['/__prism__/project/.git/config','/__prism__/project/virtual:input','/outside/name','/__prism__/compiler/lib.d.ts','/__prism__/project','project/name',null]){
    assert.throws(()=>projectSyntheticAddress(value),/unsupported_input/);
  }
});

for(const config of['tsconfig.json','Config/TSconfig.json','alias/tsconfig.json','alias-two/deep/tsconfig.json'])
test(`pinned Program keeps lexical inferred type/lib coordinates for ${config}`,()=>{
  const root='/__prism__/project',directory=config.includes('/')?config.slice(0,config.lastIndexOf('/')):'',from=directory?`${root}/${directory}`:root;
  const options={configFilePath:`${root}/${config}`,types:['missing'],lib:['lib.es5.d.ts']};
  const callbacks=[];
  const host={useCaseSensitiveFileNames:()=>false,getCanonicalFileName:ts.createGetCanonicalFileName(false),getCurrentDirectory:()=>root,
    getDefaultLibFileName:()=>'/__prism__/compiler/lib.d.ts',getDefaultLibLocation:()=>'/__prism__/compiler',getNewLine:()=> '\n',writeFile:()=>assert.fail('no emit'),
    fileExists:()=>false,readFile:()=>undefined,directoryExists:()=>false,
    getSourceFile:(file,version)=>file===`${root}/app.ts`?ts.createSourceFile(file,'export {};',version,true):undefined,
    resolveTypeReferenceDirectiveReferences:(entries,containing)=>{callbacks.push({channel:'type',from:containing});return entries.map(()=>({resolvedTypeReferenceDirective:undefined}));},
    resolveLibrary:(name,containing,compilerOptions,libFile)=>{callbacks.push({channel:'lib',from:containing,name,libFile});return {resolvedModule:undefined};}};
  ts.createProgram([`${root}/app.ts`],options,host);
  assert.deepEqual(callbacks.map(x=>[x.channel,x.from]),[
    ['type',`${from}/__inferred type names__.ts`],['lib',`${from}/__lib_node_modules_lookup_lib.es5.d.ts__.ts`]]);
  for(const callback of callbacks)assert.equal(projectSyntheticAddress(callback.from),callback.from.slice('/__prism__/'.length));
});
