// PRISM_TYPESCRIPT=/absolute/path/to/typescript.js node --test audit-callable-source.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, symlinkSync, rmSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
const script = fileURLToPath(new URL("./audit-callable-source.mjs",import.meta.url));
const compiler = process.env.PRISM_TYPESCRIPT;
assert(compiler,"PRISM_TYPESCRIPT must name the pinned compiler; do not silently skip");
function fixture(run) {
  const root = mkdtempSync(path.join(tmpdir(),"prism-callable-census-"));
  const git = (...args) => execFileSync("git",["-C",root,"-c","core.hooksPath=/dev/null","-c","commit.gpgsign=false","-c","user.name=Fixture","-c","user.email=fixture@example.invalid",...args],{stdio:"pipe"});
  const commit = () => {git("add",".");git("commit","-qm","fixture");};
  const audit = () => JSON.parse(execFileSync(process.execPath,[script,compiler,root],{encoding:"utf8"}));
  try {git("init","-q");run({root,commit,audit});}
  finally {rmSync(root,{recursive:true,force:true});}
}
test("distinguishes implementation annotations from consumer assertions, including renamed imports",()=>fixture(({root,commit,audit})=>{
  writeFileSync(path.join(root,"app.ts"),"import type {FC as Component} from 'react';\nconst direct: Component<Props> = ({client}) => client.m();\nfunction run({client}) {}\nconst cast = run as Component<Props>;\n");
  commit();const result=audit();
  assert.deepEqual(result.references.map(x=>x.producer),["direct_function_annotation","assertion"]);
  assert.equal(result.compiler_program_checked,false);
  assert.equal(result.status,"");
}));
test("configuration symlinks are refused like source symlinks",()=>fixture(({root,commit,audit})=>{
  writeFileSync(path.join(root,"target.json"),"{}");
  symlinkSync("target.json",path.join(root,"tsconfig.json"));
  commit();const result=audit();
  assert(result.refused.includes("tsconfig.json"),JSON.stringify(result));
  assert(!result.configs.some(x=>x.file==="tsconfig.json"));
}));
test("source changes alter snapshot identity and retain dirty custody",()=>fixture(({root,commit,audit})=>{
  writeFileSync(path.join(root,"app.ts"),"export const a = 1;");commit();
  const before=audit();writeFileSync(path.join(root,"app.ts"),"export const a = 2;");
  const after=audit();assert.notEqual(before.snapshot_sha256,after.snapshot_sha256);
  assert(after.status.includes("app.ts"));
}));
