// Diagnostic audit only; receipt consistency is not authenticated provenance.
// node audit-search-boundaries.mjs <normal-packet.json> <capture-location.json>
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { COMPILER_HASH, canonical, hash, parsePacket } from "../../../scripts/callable-observations/schema.mjs";

const CAPTURE_SCHEMA="prism.search-boundary-capture/1";
const PHASES=["module_refused","module","source_types","entry_types","lib","unknown"];
const MECHANISMS=["peer_metadata","type_root","ancestor_module","unknown"];
export const S1_EXPECTED_COUNTS={modules:6893,events:2075,kinds:{refused:1269,outside:806},
  phases:{module_refused:1269,module:448,source_types:26,entry_types:4,lib:328},
  mechanisms:{peer_metadata:6,type_root:28,ancestor_module:772,unknown:1269},
  phase_mechanism:{module_refused:{unknown:1269},module:{peer_metadata:6,ancestor_module:442},
    source_types:{type_root:24,ancestor_module:2},entry_types:{type_root:4},lib:{ancestor_module:328}},distinct_refused:264};
const worker=fileURLToPath(new URL("../../../scripts/callable-observations/worker.mjs",import.meta.url));
const own=(value,keys,label)=>{
  assert(value && typeof value==="object" && !Array.isArray(value),`${label} must be an object`);
  assert.deepEqual(Object.keys(value).sort(),[...keys].sort(),`${label} schema`);
};
const integer=value=>Number.isSafeInteger(value)&&value>=0;
const digest=value=>typeof value==="string"&&/^[a-f0-9]{64}$/.test(value);
const stackHas=(event,name)=>event.stack.some(frame=>frame.includes(name));

export function classifyPhase(event) {
  if(event.owner!==null)return event.kind==="refused"?"module_refused":"module";
  const marker=event.stack.find(frame=>frame.includes("actualResolveTypeReferenceDirectiveNamesWorker")
    ||frame.includes("actualResolveLibrary"));
  if(!marker)return "unknown";
  if(marker.includes("actualResolveLibrary"))return "lib";
  const markerIndex=event.stack.indexOf(marker);
  const context=event.stack.slice(markerIndex+1).find(frame=>frame.includes("processTypeReferenceDirectives")
    ||frame.includes("Object.createProgram"));
  if(context?.includes("processTypeReferenceDirectives"))return "source_types";
  if(context?.includes("Object.createProgram"))return "entry_types";
  return "unknown";
}

function classifyMechanism(event) {
  if(stackHas(event,"getPeerDependenciesOfPackageJsonInfo"))return "peer_metadata";
  if(stackHas(event,"primaryLookup"))return "type_root";
  if(stackHas(event,"loadModuleFromNearestNodeModulesDirectoryWorker"))return "ancestor_module";
  return "unknown";
}

function validateCapture(trace) {
  own(trace,["schema","events","modules"],"capture");
  assert.equal(trace.schema,CAPTURE_SCHEMA,"capture schema");
  assert(Array.isArray(trace.modules)&&Array.isArray(trace.events),"capture arrays");
  assert(trace.modules.length>0&&trace.modules.length<=100000,"capture module count");
  assert(trace.events.length>0&&trace.events.length<=100000,"capture event count");
  for(const [index,module] of trace.modules.entries()) {
    own(module,["index","from","specifier","request","mode","target"],`module ${index}`);
    assert.equal(module.index,index,"ordered unique module indices");
    assert.equal(path.posix.normalize(module.from),module.from,"normalized module source");
    assert(module.from.startsWith("/__prism__/project/"),"module source outside virtual project");
    assert.equal(typeof module.specifier,"string","module specifier");
    assert(module.request===null||module.request&&typeof module.request==="object","module request");
    assert([null,1,99].includes(module.mode),"module mode");
    assert(module.target===null||typeof module.target==="string"&&path.posix.normalize(module.target)===module.target,"module target");
  }
  for(const [index,event] of trace.events.entries()) {
    own(event,["index","kind","input","normalized","operation","owner","stack"],`event ${index}`);
    assert.equal(event.index,index,"ordered unique event indices");
    assert(["refused","outside"].includes(event.kind),"event kind");
    assert.equal(typeof event.input,"string","event input");
    assert.equal(event.normalized,path.posix.normalize(event.input).replace(/\/+$/,""),"normalized event path");
    assert(["readFile","entries","fileExists","directoryExists","realpath","identity"].includes(event.operation),"event operation");
    assert(Array.isArray(event.stack)&&event.stack.every(frame=>typeof frame==="string"),"event stack");
    if(event.owner!==null) {
      assert(integer(event.owner?.index)&&event.owner.index<trace.modules.length,"event owner index");
      assert.deepEqual(event.owner,trace.modules[event.owner.index],"event owner catalog equality");
    }
  }
}

function verifyReceipt(receipt) {
  own(receipt,["copy","packet","capture","worker_before_sha256","worker_after_sha256","packet_sha256","capture_sha256"],"receipt");
  for(const key of ["worker_before_sha256","worker_after_sha256","packet_sha256","capture_sha256"])
    assert(digest(receipt[key]),`${key} digest`);
  const copy=path.resolve(receipt.copy),instrumented=path.join(copy,"worker.mjs");
  assert.equal(path.resolve(receipt.packet),path.join(copy,"packet.json"),"receipt packet location");
  assert.equal(path.resolve(receipt.capture),path.join(copy,"boundaries.json"),"receipt capture location");
  assert.equal(hash(fs.readFileSync(worker)),receipt.worker_before_sha256,"worker before hash mismatch");
  assert.equal(hash(fs.readFileSync(instrumented)),receipt.worker_after_sha256,"worker after hash mismatch");
  const packetBytes=fs.readFileSync(receipt.packet),captureBytes=fs.readFileSync(receipt.capture);
  assert.equal(hash(packetBytes),receipt.packet_sha256,"packet receipt hash mismatch");
  assert.equal(hash(captureBytes),receipt.capture_sha256,"capture receipt hash mismatch");
  return {packetBytes,captureBytes};
}

const multiset=values=>values.reduce((map,value)=>map.set(value,(map.get(value)??0)+1),new Map());
function assertSameMultiset(actual,expected) {
  assert.deepEqual([...multiset(actual)].sort(),[...multiset(expected)].sort(),"module resolution multiset");
}
const withoutProducer=packet=>{const copy=structuredClone(packet);delete copy.producer;return copy;};
const increment=(record,key)=>record[key]=(record[key]??0)+1;

export function auditSearchBoundaries(normalPacketFile,receiptFile,expected=S1_EXPECTED_COUNTS) {
  const normalBytes=fs.readFileSync(normalPacketFile),receipt=JSON.parse(fs.readFileSync(receiptFile,"utf8"));
  const {packetBytes,captureBytes}=verifyReceipt(receipt);
  const normal=parsePacket(normalBytes.toString()),captured=parsePacket(packetBytes.toString());
  assert.equal(normal.compiler.sha256,COMPILER_HASH,"normal compiler hash");
  assert.equal(captured.compiler.sha256,COMPILER_HASH,"capture compiler hash");
  assert.deepEqual(withoutProducer(captured),withoutProducer(normal),"captured packet differs outside producer");
  const trace=JSON.parse(captureBytes.toString());validateCapture(trace);
  const moduleKeys=trace.modules.map(module=>canonical({from:module.from.slice("/__prism__/".length),specifier:module.specifier,target:module.target,request:module.request}));
  const resolutionKeys=normal.resolutions.map(({from,specifier,target,lookup})=>canonical({from,specifier,target,request:lookup.request}));
  assertSameMultiset(moduleKeys,resolutionKeys);
  const refused=[...new Set(trace.events.filter(event=>event.kind==="refused").map(event=>hash(event.normalized)))].sort();
  assert.deepEqual(refused,normal.snapshot.refused_lookup_sha256,"refused path hash set");
  assert.equal(trace.events.some(event=>event.kind==="outside"),normal.snapshot.outside_lookups,"outside lookup boolean");
  const phases={},mechanisms={},phase_mechanism={};
  for(const event of trace.events) {
    const phase=classifyPhase(event),mechanism=classifyMechanism(event);
    assert.notEqual(phase,"unknown",`unknown event phase at index ${event.index}`);
    if(event.kind==="outside")assert.notEqual(mechanism,"unknown",`unknown outside mechanism at index ${event.index}`);
    increment(phases,phase);increment(mechanisms,mechanism);
    increment(phase_mechanism[phase]??={},mechanism);
  }
  const ordered=(source,keys)=>Object.fromEntries(keys.filter(key=>source[key]).map(key=>[key,source[key]]));
  const paths=[...new Set(trace.events.map(event=>event.normalized))].sort(),owners=new Map();
  for(const event of trace.events)if(event.owner!==null){const set=owners.get(event.normalized)??new Set();set.add(event.owner.index);owners.set(event.normalized,set);}
  const shared=new Set([...owners].filter(([,set])=>set.size>1).map(([name])=>name));
  const counts={modules:trace.modules.length,events:trace.events.length,
    kinds:{refused:trace.events.filter(e=>e.kind==="refused").length,outside:trace.events.filter(e=>e.kind==="outside").length},
    phases:ordered(phases,PHASES),mechanisms:ordered(mechanisms,MECHANISMS),
    phase_mechanism:Object.fromEntries(PHASES.filter(p=>phase_mechanism[p]).map(p=>[p,ordered(phase_mechanism[p],MECHANISMS)])),
    distinct_refused:refused.length,distinct_paths:paths.length,shared_owner_paths:shared.size,
    shared_owner_events:trace.events.filter(event=>event.owner!==null&&shared.has(event.normalized)).length};
  for(const [name,value] of Object.entries(expected))assert.deepEqual(counts[name],value,`fixed ${name} count`);
  return {schema:"prism.search-boundary-audit/1",diagnostic_only:true,authorizes_runtime_edge:false,admission:false,
    custody:{receipt_consistent:true,authenticated_provenance:false},compiler_sha256:COMPILER_HASH,
    counts,path_set_sha256:hash(JSON.stringify(paths))};
}
export const audit=auditSearchBoundaries;

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length,4,"usage: audit-search-boundaries.mjs <normal-packet.json> <capture-location.json>");
  console.log(JSON.stringify(auditSearchBoundaries(process.argv[2],process.argv[3])));
}
