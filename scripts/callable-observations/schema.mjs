import {createHash} from "node:crypto";
export const SCHEMA="prism.callable-observation/1";
export const COMPILER_HASH="3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675";
export const LIMITS={files:20000,bytes:128*1024*1024,depth:64,timeout_ms:30000,observations:2000,provenance_steps:32};
export const PACKET_BYTES=8*1024*1024;
export const hash=b=>createHash("sha256").update(b).digest("hex");
export const canonical=x=>JSON.stringify(sort(x));
function sort(x) {
  if(Array.isArray(x)) return x.map(sort);
  if(x && typeof x==="object") return Object.fromEntries(Object.keys(x).sort().map(k=>[k,sort(x[k])]));
  return x;
}
export const relative=s=>typeof s==="string" && s.length>0 && s.length<=4096
  && !/[\\:\x00-\x1f]/.test(s) && s.split("/").every(p=>p && p!=="." && p!=="..");
const id=s=>relative(s) && /^(project|compiler)(\/|$)/.test(s);
const str=s=>typeof s==="string" && s.length<=65536;
const integer=n=>Number.isSafeInteger(n) && n>=0;
const boolean=x=>typeof x==="boolean";
const digest=s=>typeof s==="string" && /^[a-f0-9]{64}$/.test(s);
const literal=v=>x=>x===v;
const array=rule=>x=>Array.isArray(x) && x.length<=100000 && x.every(rule);
const nullable=rule=>x=>x===null || rule(x);
const object=shape=>x=>x!==null && typeof x==="object" && !Array.isArray(x)
  && Object.keys(x).length===Object.keys(shape).length
  && Object.entries(shape).every(([k,v])=>Object.hasOwn(x,k) && v(x[k]));
const anchor=object({file:id,sha256:digest,kind:str,start_utf16:integer,end_utf16:integer,start_byte:integer,end_byte:integer});
const alias=object({declarations:array(anchor),target:array(anchor),module:nullable(anchor),
  module_declarations:array(anchor),module_exports:array(anchor),module_bindings:array(anchor)});
const provenance=object({status:x=>["traced","unproven"].includes(x),
  reason:nullable(x=>["step_limit","cycle","ambiguous_declaration","unresolved_symbol",
    "unsupported_declaration","unsupported_type","unsupported_heritage","unsupported_export_star"].includes(x)),
  steps_used:integer,terminal:nullable(anchor),hops:array(object({reference:anchor,type_arguments:array(anchor),
    type_parameters:array(anchor),declarations:array(anchor),aliases:array(alias),
    qualifiers:array(object({use:anchor,aliases:array(alias),declarations:array(anchor)}))}))});
const reasons=array(x=>[
  "budget_exceeded","unsupported_input","compiler_mismatch","unstable_snapshot",
  "compiler_diagnostics","unsupported_references","unsupported_plugins",
  "outside_lookup","unresolved_module","invalid_config","worker_failed",
].includes(x));
const packet=object({
  schema:literal(SCHEMA),authorizes_runtime_edge:literal(false),
  producer:object({version:literal("0.2.0"),sha256:digest}),
  compiler:object({version:literal("5.9.3"),sha256:literal(COMPILER_HASH),verified:boolean,library_sha256:digest}),
  scope:object({config:id,callable_scope:literal("direct-annotated-function"),class_authority:literal(false),case_sensitive:nullable(boolean)}),
  status:x=>["observed","unproven"].includes(x),reasons,
  closure:object({stable_snapshot:boolean,dependencies:boolean,references:boolean,augmentation:boolean,resolution:boolean}),
  limits:object(Object.fromEntries(Object.keys(LIMITS).map(k=>[k,integer]))),
  snapshot:object({sha256:digest,files:array(object({id,sha256:digest,size:integer})),directories:array(id),
    roots:array(id),config_files:array(id),program_files:array(id),reads:array(id),failed_lookups:array(id),
    outside_lookups:boolean,options_sha256:digest}),
  resolutions:array(object({from:id,specifier:str,target:nullable(id)})),
  diagnostics:array(object({code:integer,file:nullable(id),start:nullable(integer)})),
  observations:array(object({annotation:anchor,implementation:anchor,parameter:nullable(anchor),
    explicit_parameter:boolean,signatures:array(anchor),callable_declarations:array(anchor),provenance,
    calls:array(object({call:anchor,receiver:anchor,receiver_type:str,declarations:array(anchor)}))})),
});
export function parsePacket(text) {
  if(typeof text!=="string" || Buffer.byteLength(text)>PACKET_BYTES) throw Error("invalid_packet");
  const value=JSON.parse(text);
  if(!packet(value)) throw Error("invalid_packet");
  const files=new Map(value.snapshot.files.map(f=>[f.id,f]));
  if(files.size!==value.snapshot.files.length) throw Error("invalid_packet");
  if(Object.values(value.limits).some(n=>n<1) || Object.entries(value.limits).some(([k,v])=>v>LIMITS[k])) throw Error("invalid_packet");
  if(!value.scope.config.startsWith("project/")) throw Error("invalid_packet");
  if(value.status==="observed" && (value.reasons.length || !value.compiler.verified || Object.values(value.closure).some(x=>!x))) throw Error("invalid_packet");
  if(value.status==="unproven" && !value.reasons.length) throw Error("invalid_packet");
  if(value.snapshot.files.length && value.snapshot.sha256!==hash(canonical({files:value.snapshot.files,directories:value.snapshot.directories}))) throw Error("invalid_packet");
  for(const name of ["roots","program_files","config_files","reads"]) {
    if(value.snapshot[name].some(id=>!files.has(id))) throw Error("invalid_packet");
  }
  function checkAnchor(a) {
    if(!a)return;
    const f=files.get(a.file);
    if(!f || f.sha256!==a.sha256 || a.start_utf16>a.end_utf16 || a.start_byte>a.end_byte
      || a.end_byte>f.size || a.end_utf16>a.end_byte) throw Error("invalid_packet");
  }
  for(const o of value.observations) {
    [o.annotation,o.implementation,o.parameter,...o.signatures,...o.callable_declarations].forEach(checkAnchor);
    for(const c of o.calls)[c.call,c.receiver,...c.declarations].forEach(checkAnchor);
    const p=o.provenance;
    if(p.steps_used>value.limits.provenance_steps || (p.status==="traced" ? p.reason!==null || !p.terminal : !p.reason || p.terminal!==null))throw Error("invalid_packet");
    checkAnchor(p.terminal);
    const checkAlias=a=>[a.module,...a.declarations,...a.target,...a.module_declarations,...a.module_exports,...a.module_bindings].forEach(checkAnchor);
    for(const h of p.hops) {
      [h.reference,...h.type_arguments,...h.type_parameters,...h.declarations].forEach(checkAnchor);
      h.aliases.forEach(checkAlias);
      for(const q of h.qualifiers){[q.use,...q.declarations].forEach(checkAnchor);q.aliases.forEach(checkAlias);}
    }
  }
  return value;
}
