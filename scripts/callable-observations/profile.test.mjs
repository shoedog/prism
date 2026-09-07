import test from "node:test";
import assert from "node:assert/strict";
import {settings,emptyPacket,validate,produce,readPacket} from "./index.mjs";
import {parsePacket,LIMITS,PACKET_BYTES,MAX_PACKET_BYTES} from "./schema.mjs";
import {snapshot} from "./inventory.mjs";
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
const input={root:"/unused",compiler:"/unused/compiler/typescript.js",config:"tsconfig.json"};
test("installed profile is explicit, bounded and recorded in independently selected settings",()=>{
  const defaults=settings(input),installed=settings({...input,profile:"installed"});
  assert.equal(defaults.profile,"default");assert.equal(defaults.limits.bytes,128*1024*1024);
  assert.equal(installed.limits.bytes,1024*1024*1024);assert.equal(installed.limits.files,100000);
  assert.equal(installed.limits.read_bytes,32*1024*1024);
  assert.equal(emptyPacket(installed,"budget_exceeded").scope.acquisition_profile,"installed");
  assert.equal(parsePacket(JSON.stringify(emptyPacket(installed,"budget_exceeded"))).limits.files,100000);
});
test("unknown profile, over-profile limits and forged profiles reject before root access",()=>{
  assert.throws(()=>settings({...input,profile:"unbounded"}),/invalid_options/);
  for(const profile of ["default","installed"]) {
    assert.throws(()=>settings({...input,profile,limits:{bytes:2**40}}),/invalid_limits/);
    const packet=emptyPacket(settings({...input,profile}),"budget_exceeded");packet.scope.acquisition_profile="forged";
    let reads=0;assert.equal(validate(JSON.stringify(packet),{get root(){reads++;throw Error();}}).valid,false);assert.equal(reads,0);
  }
});
test("hash budget and materialized-read budget are independent",()=>{
  const root=mkdtempSync(path.join(tmpdir(),"prism-profile-"));
  try {
    mkdirSync(path.join(root,"project"));mkdirSync(path.join(root,"compiler"));
    writeFileSync(path.join(root,"project/large"),Buffer.alloc(100));
    const s=snapshot({root:path.join(root,"project"),compiler:path.join(root,"compiler/typescript.js"),limits:{...LIMITS,read_bytes:10}});
    assert.equal(s.files.get("project/large").size,100);
    assert.throws(()=>s.read("project/large"),/budget_exceeded/);
  } finally {rmSync(root,{recursive:true,force:true});}
});
test("packets cannot silently select a larger profile during validation",()=>{
  const packet=produce({...input,profile:"installed"});
  assert.equal(validate(JSON.stringify(packet),input).valid,false);
  assert.equal(packet.authorizes_runtime_edge,false);
});
test("profile-specific packet caps and types are strict before I/O",()=>{
  for(const profile of [null,["default"],{},"__proto__"])assert.throws(()=>settings({...input,profile}),/invalid_options/);
  const p=emptyPacket(settings(input),"budget_exceeded"),text=JSON.stringify(p);
  assert.throws(()=>parsePacket(text+" ".repeat(PACKET_BYTES)),/invalid_packet/);
  p.scope.acquisition_profile="installed";
  assert.throws(()=>parsePacket(JSON.stringify(p)),/invalid_packet/);
  p.limits.read_bytes=32*1024*1024;
  assert.equal(parsePacket(JSON.stringify(p)+" ".repeat(PACKET_BYTES)).scope.acquisition_profile,"installed");
  assert.throws(()=>parsePacket(" ".repeat(MAX_PACKET_BYTES+1)),/invalid_packet/);
  p.scope.acquisition_profile=["default"];assert.throws(()=>parsePacket(JSON.stringify(p)),/invalid_packet/);
  assert.throws(()=>readPacket(-1,Infinity),/invalid_options/);
});
