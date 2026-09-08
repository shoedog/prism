import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { auditSearchBoundaries, classifyPhase } from "./audit-search-boundaries.mjs";
import { instrumentBoundaryWorker } from "./capture-search-boundaries.mjs";
import { COMPILER_HASH, LIMITS, canonical, hash } from "../../../scripts/callable-observations/schema.mjs";

const originalWorker=fileURLToPath(new URL("../../../scripts/callable-observations/worker.mjs",import.meta.url));
const zero="0".repeat(64),one="1".repeat(64);
const anchor=file=>({file,sha256:zero,kind:"StringLiteral",start_utf16:0,end_utf16:0,start_byte:0,end_byte:0});
function packet(producer=zero) {
  const files=["project/a.ts","project/b.ts"].map(id=>({id,sha256:zero,size:0}));
  const lookup=request=>({status:"unproven",reason:"filesystem_target",context:"import",request,declarations:[],providers:[],augmentations:[],wildcard:null,merged_wildcard:null});
  return {schema:"prism.callable-observation/12",authorizes_runtime_edge:false,producer:{version:"0.13.0",sha256:producer},
    compiler:{version:"5.9.3",sha256:COMPILER_HASH,verified:true,library_sha256:zero},
    scope:{config:"project/tsconfig.json",acquisition_profile:"default",link_policy:"in-root",callable_scope:"direct-annotated-function",class_authority:false,case_sensitive:true},
    status:"unproven",reasons:["outside_lookup","unsupported_lookup"],closure:{stable_snapshot:true,dependencies:false,references:true,augmentation:false,resolution:false},limits:{...LIMITS},
    snapshot:{sha256:hash(canonical({files,directories:[],links:[]})),files,directories:[],links:[],roots:[],config_files:[],program_files:files.map(f=>f.id),reads:[],failed_lookups:[],refused_lookup_sha256:[hash("/__prism__/project/bad:name")],outside_lookups:true,options_sha256:zero},
    resolutions:[{from:"project/a.ts",specifier:"./b",target:"project/b.ts",lookup:lookup(anchor("project/a.ts"))},
      {from:"project/b.ts",specifier:"./a",target:"project/a.ts",lookup:lookup(anchor("project/b.ts"))}],
    diagnostics:[],observations:[],type_lib_references:[],type_lib_entries:[]};
}
const modules=[
  {index:0,from:"/__prism__/project/a.ts",specifier:"./b",request:anchor("project/a.ts"),mode:null,target:"project/b.ts"},
  {index:1,from:"/__prism__/project/b.ts",specifier:"./a",request:anchor("project/b.ts"),mode:null,target:"project/a.ts"},
];
const EXPECTED={modules:2,events:6};
const event=(index,kind,input,owner,stack)=>({index,kind,input,normalized:path.posix.normalize(input).replace(/\/+$/,""),operation:"fileExists",owner,stack});
function trace() {
  return {schema:"prism.search-boundary-capture/1",modules:structuredClone(modules),events:[
    event(0,"outside","/shared",modules[0],["at getPeerDependenciesOfPackageJsonInfo"]),
    event(1,"outside","/shared",modules[1],["at loadModuleFromNearestNodeModulesDirectoryWorker"]),
    event(2,"refused","/__prism__/project/bad:name",modules[0],["at resolveModuleName"]),
    event(3,"outside","/source",null,["at primaryLookup","at actualResolveTypeReferenceDirectiveNamesWorker","at processTypeReferenceDirectives","at Object.createProgram"]),
    event(4,"outside","/entry",null,["at primaryLookup","at actualResolveTypeReferenceDirectiveNamesWorker","at Object.createProgram"]),
    event(5,"outside","/lib",null,["at loadModuleFromNearestNodeModulesDirectoryWorker","at actualResolveLibrary","at actualResolveTypeReferenceDirectiveNamesWorker","at processTypeReferenceDirectives"]),
  ]};
}
function fixture(run) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),"prism-boundary-audit-")),copy=path.join(root,"copy");fs.mkdirSync(copy);
  const state={normal:packet(),captured:packet(one),trace:trace(),receipt:null};
  try {
    run({state,write(){
      const normalFile=path.join(root,"normal.json"),packetFile=path.join(copy,"packet.json"),captureFile=path.join(copy,"boundaries.json"),workerFile=path.join(copy,"worker.mjs");
      const normalBytes=JSON.stringify(state.normal),packetBytes=JSON.stringify(state.captured),captureBytes=JSON.stringify(state.trace),workerBytes="instrumented worker";
      fs.writeFileSync(normalFile,normalBytes);fs.writeFileSync(packetFile,packetBytes);fs.writeFileSync(captureFile,captureBytes);fs.writeFileSync(workerFile,workerBytes);
      state.receipt={copy,packet:packetFile,capture:captureFile,worker_before_sha256:hash(fs.readFileSync(originalWorker)),worker_after_sha256:hash(workerBytes),packet_sha256:hash(packetBytes),capture_sha256:hash(captureBytes)};
      const receiptFile=path.join(root,"receipt.json");fs.writeFileSync(receiptFile,JSON.stringify(state.receipt));
      return {normalFile,receiptFile};
    }});
  } finally {fs.rmSync(root,{recursive:true,force:true});}
}
const runAudit=files=>auditSearchBoundaries(files.normalFile,files.receiptFile,EXPECTED);

test("audits the closed trace and keeps same-path owners event-local",()=>fixture(({write})=>{
  const report=runAudit(write());
  assert.deepEqual(report.counts.phases,{module_refused:1,module:2,source_types:1,entry_types:1,lib:1});
  assert.deepEqual(report.counts.phase_mechanism.module,{peer_metadata:1,ancestor_module:1});
  assert.equal(report.counts.shared_owner_paths,1);assert.equal(report.counts.shared_owner_events,2);
  assert.deepEqual(report.custody,{receipt_consistent:true,authenticated_provenance:false});
  assert.equal(report.diagnostic_only,true);assert.equal(report.authorizes_runtime_edge,false);assert.equal(report.admission,false);
}));
test("rejects packet drift outside producer",()=>fixture(({state,write})=>{
  state.captured.compiler.library_sha256=one;assert.throws(()=>runAudit(write()),/differs outside producer/);
}));
test("rejects a receipt hash mismatch",()=>fixture(({state,write})=>{
  const files=write();state.receipt.packet_sha256=zero;fs.writeFileSync(files.receiptFile,JSON.stringify(state.receipt));
  assert.throws(()=>runAudit(files),/packet receipt hash mismatch/);
}));
test("rejects an owner that is not its indexed catalog entry",()=>fixture(({state,write})=>{
  state.trace.events[0].owner={...state.trace.modules[0],specifier:"swapped"};
  assert.throws(()=>runAudit(write()),/event owner catalog equality/);
}));
test("rejects a changed or omitted module request anchor",()=>fixture(({state,write})=>{
  state.trace.modules[0].request=null;
  for(const entry of state.trace.events)if(entry.owner?.index===0)entry.owner=structuredClone(state.trace.modules[0]);
  assert.throws(()=>runAudit(write()),/module resolution multiset/);
}));
test("rejects capture modes outside the compiler resolution enum",()=>fixture(({state,write})=>{
  state.trace.modules[0].mode=2;
  for(const entry of state.trace.events)if(entry.owner?.index===0)entry.owner=structuredClone(state.trace.modules[0]);
  assert.throws(()=>runAudit(write()),/module mode/);
}));
test("rejects a dropped refused path",()=>fixture(({state,write})=>{
  state.trace.events.splice(2,1);state.trace.events.forEach((entry,index)=>entry.index=index);
  assert.throws(()=>runAudit(write()),/refused path hash set/);
}));
test("rejects an unknown mechanism for an outside event",()=>fixture(({state,write})=>{
  state.trace.events[0].stack=["at unrecognizedResolver"];
  assert.throws(()=>runAudit(write()),/unknown outside mechanism/);
}));
test("rejects zero and bounded-but-truncated event populations",()=>{
  fixture(({state,write})=>{state.trace.events=[];assert.throws(()=>runAudit(write()),/capture event count/);});
  fixture(({state,write})=>{state.trace.events.pop();assert.throws(()=>runAudit(write()),/fixed events count/);});
});
test("uses the nearest resolver frame for nested source and library stacks",()=>{
  const base={kind:"outside",owner:null};
  assert.equal(classifyPhase({...base,stack:["actualResolveLibrary","actualResolveTypeReferenceDirectiveNamesWorker","processTypeReferenceDirectives"]}),"lib");
  assert.equal(classifyPhase({...base,stack:["actualResolveTypeReferenceDirectiveNamesWorker","processTypeReferenceDirectives","Object.createProgram"]}),"source_types");
  assert.equal(classifyPhase({...base,stack:["actualResolveTypeReferenceDirectiveNamesWorker","Object.createProgram"]}),"entry_types");
  assert.equal(classifyPhase({...base,stack:["unrecognized"]}),"unknown");
});
test("instrumentation refuses changed source with a missing unique anchor",()=>{
  assert.throws(()=>instrumentBoundaryWorker("changed worker source"),/unique instrumentation anchor/);
});
