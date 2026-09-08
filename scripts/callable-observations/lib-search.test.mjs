import test from 'node:test';import assert from 'node:assert/strict';
import {readFileSync}from'node:fs';import{createRequire}from'node:module';
import {COMPILER_HASH,hash}from'./schema.mjs';
import {libFileForReference,libraryNameFromLibFile}from'./lib-search.mjs';

const compiler=process.env.PRISM_TYPESCRIPT;assert(compiler,'pinned compiler required');
assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);

test('pure lib-reference mapping equals every pinned compiler libMap entry',()=>{
  for(const [name,file] of ts.libMap)assert.equal(libFileForReference(name),file,name);
});

test('pure library package-name derivation matches pinned dotted-file algorithm',()=>{
  assert.equal(libraryNameFromLibFile('lib.dom.d.ts'),'@typescript/lib-dom');
  assert.equal(libraryNameFromLibFile('lib.es2015.core.d.ts'),'@typescript/lib-es2015/core');
  assert.equal(libraryNameFromLibFile('lib.esnext.disposable.extra.d.ts'),'@typescript/lib-esnext/disposable-extra');
  for(const invalid of [null,'dom.d.ts','lib..d.ts','lib.dom.ts'])assert.equal(libraryNameFromLibFile(invalid),null);
});
