// Opt-in research tooling. No Prism runtime imports or proof consumer.
import {spawnSync} from "node:child_process";
import {readFileSync,readSync} from "node:fs";
import {fileURLToPath} from "node:url";
import path from "node:path";
import {SCHEMA,COMPILER_HASH,LIMITS,PACKET_BYTES,relative,hash,canonical,parsePacket} from "./schema.mjs";
export function producerHash() {
  return hash(Buffer.concat(["schema.mjs","index.mjs","worker.mjs"].map(f=>readFileSync(new URL(f,import.meta.url)))));
}
export function settings(options) {
  if(!options || typeof options.root!=="string" || typeof options.compiler!=="string"
      || !relative(options.config) || Object.keys(options).some(k=>!["root","compiler","config","limits"].includes(k))) throw Error("invalid_options");
  const limits={...LIMITS,...options.limits};
  if(Object.keys(limits).some(k=>!Object.hasOwn(LIMITS,k) || !Number.isSafeInteger(limits[k])
      || limits[k]<1 || limits[k]>LIMITS[k])) throw Error("invalid_limits");
  return {root:path.resolve(options.root),compiler:path.resolve(options.compiler),config:options.config,limits};
}
export function emptyPacket(options,reason) {
  const zero=hash("");
  return {schema:SCHEMA,authorizes_runtime_edge:false,producer:{version:"0.1.0",sha256:producerHash()},
    compiler:{version:"5.9.3",sha256:COMPILER_HASH,verified:false,library_sha256:zero},
    scope:{config:"project/"+options.config,callable_scope:"direct-annotated-function",class_authority:false,case_sensitive:null},
    status:"unproven",reasons:[reason],limits:options.limits,
    closure:{stable_snapshot:false,dependencies:false,references:false,augmentation:false,resolution:false},
    snapshot:{sha256:zero,files:[],directories:[],roots:[],config_files:[],program_files:[],reads:[],
      failed_lookups:[],outside_lookups:false,options_sha256:zero},
    diagnostics:[],resolutions:[],observations:[]};
}
export function produce(input) {
  const options=settings(input);
  const child=spawnSync(process.execPath,["--max-old-space-size=512",fileURLToPath(new URL("worker.mjs",import.meta.url))],
    {input:JSON.stringify(options),encoding:"utf8",timeout:options.limits.timeout_ms,maxBuffer:PACKET_BYTES});
  if(child.error || child.status!==0) return emptyPacket(options,
    ["ETIMEDOUT","ENOBUFS"].includes(child.error?.code)?"budget_exceeded":"worker_failed");
  try {return parsePacket(child.stdout);} catch {return emptyPacket(options,"worker_failed");}
}
export function validate(text,options) {
  // Schema validation is pre-I/O with respect to the audited roots. Packet paths
  // are never opened. Equality checks every field against independently recomputed evidence.
  try {
    const packet=parsePacket(text);
    const fresh=produce(options);
    const valid=canonical(packet)===canonical(fresh);
    return {valid,authorizes_runtime_edge:false,packet_status:packet.status,reason:valid?"reproduced_observation":"stale_or_tampered"};
  } catch {return {valid:false,authorizes_runtime_edge:false,reason:"invalid_packet_or_options"};}
}
export function readPacket(fd) {
  const chunks=[];let size=0;
  for(;;) {
    const chunk=Buffer.alloc(Math.min(65536,PACKET_BYTES+1-size));
    const n=readSync(fd,chunk,0,chunk.length,null);if(!n)break;
    size+=n;if(size>PACKET_BYTES)throw Error("invalid_packet");
    chunks.push(chunk.subarray(0,n));
  }
  return Buffer.concat(chunks).toString("utf8");
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [mode,compiler,root,config]=process.argv.slice(2);
  if(!["produce","validate"].includes(mode) || process.argv.length!==6) throw Error("usage: index.mjs produce|validate <typescript.js> <project-root> <relative-config>; validate reads JSON on stdin");
  let result;
  try {result=mode==="produce"?produce({compiler,root,config}):validate(readPacket(0),{compiler,root,config});}
  catch {result={valid:false,authorizes_runtime_edge:false,reason:"invalid_packet_or_options"};process.exitCode=1;}
  console.log(JSON.stringify(result,null,2));
  if(mode==="validate" && !result.valid) process.exitCode=1;
}
