// Opt-in research tooling. No Prism runtime imports or proof consumer.
import {spawnSync} from "node:child_process";
import {readFileSync,readSync} from "node:fs";
import {fileURLToPath} from "node:url";
import path from "node:path";
import {SCHEMA,COMPILER_HASH,LIMITS,PACKET_BYTES,PROFILES,relative,hash,canonical,parsePacket} from "./schema.mjs";
import {classifySemanticClosureV3} from "./semantic-closure-v3.mjs";
export function producerHash() {
  return hash(Buffer.concat(["schema.mjs","index.mjs","worker.mjs","inventory.mjs","provenance.mjs","nested.mjs","props-class.mjs","exact-ambient.mjs","wildcard.mjs","merged-wildcard.mjs","required-paths.mjs","type-lib.mjs","entries.mjs","search-provenance.mjs","identity-domains.mjs","lib-search.mjs","config-provenance.mjs","entry-obligations.mjs","semantic-closure.mjs","semantic-closure-v2.mjs","semantic-closure-v3.mjs"].map(f=>readFileSync(new URL(f,import.meta.url)))));
}
export function settings(options) {
  if(!options || typeof options.root!=="string" || typeof options.compiler!=="string"
      || !relative(options.config) || Object.keys(options).some(k=>!["root","compiler","config","limits","profile","links"].includes(k))) throw Error("invalid_options");
  const profile=options.profile===undefined?"default":options.profile;
  if(typeof profile!=="string" || !Object.hasOwn(PROFILES,profile))throw Error("invalid_options");
  const links=options.links===undefined?"reject":options.links;
  if(!["reject","in-root"].includes(links))throw Error("invalid_options");
  const ceiling=PROFILES[profile].limits,limits={...ceiling,...options.limits};
  if(Object.keys(limits).some(k=>!Object.hasOwn(LIMITS,k) || !Number.isSafeInteger(limits[k])
      || limits[k]<1 || limits[k]>ceiling[k])) throw Error("invalid_limits");
  return {root:path.resolve(options.root),compiler:path.resolve(options.compiler),config:options.config,profile,links,limits};
}
export function emptyPacket(options,reason) {
  const zero=hash("");
  const semantic_closure=classifySemanticClosureV3({compilerVerified:false,stableSnapshot:false,
    configObserved:false,entryComplete:false,noResolve:false,diagnosticCount:0,globalReasons:[reason],
    outside:false,refusedCount:0,boundaryCount:0,programFiles:[],resolutions:[]});
  return {schema:SCHEMA,authorizes_runtime_edge:false,producer:{version:"0.21.0",sha256:producerHash()},
    compiler:{version:"5.9.3",sha256:COMPILER_HASH,verified:false,library_sha256:zero},
    scope:{config:"project/"+options.config,acquisition_profile:options.profile,link_policy:options.links,callable_scope:"direct-annotated-function",class_authority:false,case_sensitive:null},
    status:"unproven",reasons:[reason],limits:options.limits,
    closure:{stable_snapshot:false,dependencies:false,references:false,augmentation:false,resolution:false},
    snapshot:{sha256:zero,files:[],directories:[],links:[],roots:[],config_files:[],program_files:[],reads:[],
      failed_lookups:[],refused_lookup_sha256:[],outside_lookups:false,options_sha256:zero},
    diagnostics:[],resolutions:[],observations:[],type_lib_references:[],type_lib_entries:[],search_provenance:{module_requests:[],type_batches:[],type_requests:[],type_searches:[],lib_searches:[],boundary_events:[]},
    config_provenance:{status:"unproven",reason:"unavailable",files:[],extends:[],options:[]},
    entry_obligations:{complete:false,reasons:["configuration_unproven"],rows:[]},semantic_closure};
}
export function produce(input) {
  const options=settings(input);
  const profile=PROFILES[options.profile];
  const child=spawnSync(process.execPath,[`--max-old-space-size=${profile.heap_mb}`,fileURLToPath(new URL("worker.mjs",import.meta.url))],
    {input:JSON.stringify(options),encoding:"utf8",timeout:options.limits.timeout_ms,maxBuffer:profile.packet_bytes});
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
export function readPacket(fd,cap=PACKET_BYTES) {
  if(!Object.values(PROFILES).some(p=>p.packet_bytes===cap))throw Error("invalid_options");
  const chunks=[];let size=0;
  for(;;) {
    const chunk=Buffer.alloc(Math.min(65536,cap+1-size));
    const n=readSync(fd,chunk,0,chunk.length,null);if(!n)break;
    size+=n;if(size>cap)throw Error("invalid_packet");
    chunks.push(chunk.subarray(0,n));
  }
  return Buffer.concat(chunks).toString("utf8");
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [mode,compiler,root,config,profile="default",links="reject"]=process.argv.slice(2);
  if(!["produce","validate"].includes(mode) || ![6,7,8].includes(process.argv.length)) throw Error("usage: index.mjs produce|validate <typescript.js> <project-root> <relative-config> [default|installed] [reject|in-root]; validate reads JSON on stdin");
  let result;
  try {const options=settings({compiler,root,config,profile,links});result=mode==="produce"?produce(options):validate(readPacket(0,PROFILES[profile].packet_bytes),options);}
  catch {result={valid:false,authorizes_runtime_edge:false,reason:"invalid_packet_or_options"};process.exitCode=1;}
  console.log(JSON.stringify(result,null,2));
  if(mode==="validate" && !result.valid) process.exitCode=1;
}
