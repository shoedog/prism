import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,rmSync,symlinkSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
import {snapshot} from "./inventory.mjs";
import {LIMITS,hash,canonical} from "./schema.mjs";
function fixture(run) {
  const root=mkdtempSync(path.join(tmpdir(),"prism-inventory-"));
  const project=path.join(root,"project"),compiler=path.join(root,"compiler");
  mkdirSync(project);mkdirSync(compiler);
  const put=(id,bytes)=>{const f=path.join(root,id);mkdirSync(path.dirname(f),{recursive:true});writeFileSync(f,bytes);};
  put("project/src/a.ts","export const a='🦊';");put("compiler/typescript.js","compiler fixture");
  try{return run({root,project,put,options:{root:project,compiler:path.join(compiler,"typescript.js"),limits:LIMITS}});}
  finally{rmSync(root,{recursive:true,force:true});}
}
test("inventory retains hashes and sizes, not eager content buffers",()=>fixture(({put,options})=>{
  const bytes=Buffer.alloc(200000,171);put("project/unused.bin",bytes);
  const s=snapshot(options);
  assert.equal(Buffer.isBuffer(s.files.get("project/unused.bin")),false,"inventory must not retain content");
  assert.deepEqual(s.files.get("project/unused.bin"),{id:"project/unused.bin",sha256:hash(bytes),size:bytes.length});
  assert(s.read("project/unused.bin").equals(bytes));
  assert.equal(s.read("project/missing"),undefined);
  assert.equal(s.digest,hash(canonical({files:s.manifest,directories:s.dirs,links:s.links})));
}));
test("same-size writes cannot be served as captured bytes",()=>fixture(({put,options})=>{
  put("project/a","AAAA");const s=snapshot(options);put("project/a","BBBB");
  assert.throws(()=>s.read("project/a"),/unstable_snapshot/);
}));
test("removal and file-to-link replacement cannot reuse captured content",()=>fixture(({project,options})=>{
  const s=snapshot(options);rmSync(path.join(project,"src/a.ts"));
  assert.throws(()=>s.read("project/src/a.ts"),/unstable_snapshot/);
  symlinkSync("../../compiler/typescript.js",path.join(project,"src/a.ts"));
  assert.throws(()=>s.read("project/src/a.ts"),/unstable_snapshot/);
}));
test("unused content and directory membership remain part of every epoch",()=>fixture(({put,options})=>{
  const a=snapshot(options);put("project/unused","first");const b=snapshot(options);
  put("project/unused","other");const c=snapshot(options);
  assert.notEqual(a.digest,b.digest);assert.notEqual(b.digest,c.digest);
}));
test("file, byte, depth and symlink barriers remain bounded",()=>fixture(({project,options})=>{
  for(const limits of [{files:1},{bytes:1},{depth:1}])assert.throws(()=>snapshot({...options,limits:{...LIMITS,...limits}}),/budget_exceeded/);
  symlinkSync("src",path.join(project,"link"));assert.throws(()=>snapshot(options),/unsupported_input/);
}));
