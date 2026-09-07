import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,rmSync,symlinkSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
import {snapshot} from "./inventory.mjs";
import {LIMITS} from "./schema.mjs";
function fixture(run) {
  const root=mkdtempSync(path.join(tmpdir(),"prism-links-")),project=path.join(root,"project");
  const put=(id,text)=>{mkdirSync(path.dirname(path.join(root,id)),{recursive:true});writeFileSync(path.join(root,id),text);};
  const link=(id,target)=>{mkdirSync(path.dirname(path.join(root,id)),{recursive:true});symlinkSync(target,path.join(root,id));};
  put("project/pkg/a.ts","export class A{}");put("compiler/typescript.js","compiler");
  try{return run({root,project,put,link,options:{root:project,compiler:path.join(root,"compiler/typescript.js"),links:"in-root",limits:LIMITS}});}
  finally{rmSync(root,{recursive:true,force:true});}
}
test("file and directory link chains share physical inventory without subtree expansion",()=>fixture(({link,options})=>{
  link("project/node_modules/pkg","../pkg");link("project/a.ts","pkg/a.ts");link("project/b.ts","a.ts");
  const s=snapshot(options);
  assert.equal(s.resolve("project/node_modules/pkg/a.ts"),"project/pkg/a.ts");
  assert.equal(s.resolve("project/b.ts"),"project/pkg/a.ts");
  assert.equal(s.files.size,2);assert.equal(s.links.length,3);
  assert.equal(s.resolve("project/node_modules/pkg/missing.ts"),"project/pkg/missing.ts");
  assert.equal(s.read(s.resolve("project/b.ts")).toString(),"export class A{}");
  assert.throws(()=>snapshot({...options,links:"reject"}),/unsupported_input/);
}));
test("same-content link retargeting changes epoch identity",()=>fixture(({link,put,project,options})=>{
  put("project/pkg/b.ts","export class A{}");link("project/a.ts","pkg/a.ts");const a=snapshot(options);
  rmSync(path.join(project,"a.ts"));link("project/a.ts","pkg/b.ts");const b=snapshot(options);
  assert.notEqual(a.digest,b.digest);assert.notEqual(a.links[0].target,b.links[0].target);
}));
for(const [label,target] of [["missing","missing"],["outside","../../escape"],["cross-root","../compiler/typescript.js"],["metadata",".git/config"],["absolute","/etc/passwd"],["embedded parent","pkg/../pkg/a.ts"]]) {
  test(`refuses ${label} link without following it`,()=>fixture(({link,options})=>{
    link("project/link",target);assert.throws(()=>snapshot(options),/unsupported_input/);
  }));
}
test("cycles and traversal exhaustion fail closed",()=>fixture(({link,project,options})=>{
  link("project/a","b");link("project/b","a");assert.throws(()=>snapshot(options),/unsupported_input/);
  rmSync(path.join(project,"b"));link("project/b","pkg/a.ts");
  assert.throws(()=>snapshot({...options,limits:{...LIMITS,link_steps:1}}),/budget_exceeded/);
}));
test("links count toward the entry cap and missing target restoration changes acquisition",()=>fixture(({link,put,options})=>{
  link("project/a","pkg/a.ts");const s=snapshot(options);
  assert.throws(()=>snapshot({...options,limits:{...LIMITS,files:s.files.size+s.dirs.length}}),/budget_exceeded/);
  link("project/b","later");assert.throws(()=>snapshot(options),/unsupported_input/);
  put("project/later","later");assert.equal(snapshot(options).links.length,2);
}));
test("BOM-prefixed link targets are literal filenames, not UTF-8 text headers",()=>fixture(({link,put,options})=>{
  put("project/a.ts","wrong");put("project/\uFEFFa.ts","right");link("project/alias","\uFEFFa.ts");
  const s=snapshot(options);assert.equal(s.resolve("project/alias"),"project/\uFEFFa.ts");
  assert.equal(s.read(s.resolve("project/alias")).toString(),"right");
}));
