// Opt-in research ONLY. Serialized observations never select inputs or own edges.
import {readFileSync,realpathSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {settings,readPacket,producerHash} from './index.mjs';
import {canonical,hash,relative,parsePacket,PROFILES,MAX_PACKET_BYTES} from './schema.mjs';

const SCHEMA='prism.project-membership/1';
const reasons=['invalid_options','budget_exceeded','worker_failed','unsupported_input','compiler_mismatch',
  'unstable_snapshot','membership_unavailable','membership_config','membership_identity','native_failed','native_identity'];
const fail=reason=>{throw Error(reason);};
const requireThat=(ok,reason='membership_identity')=>{if(!ok)fail(reason);};
const keys=(v,wanted)=>v&&typeof v==='object'&&!Array.isArray(v)&&canonical(Object.keys(v).sort())===canonical(wanted.split(' ').sort());
const digest=s=>typeof s==='string'&&/^[a-f0-9]{64}$/.test(s);
const integer=n=>Number.isSafeInteger(n)&&n>=0;
const id=s=>relative(s)&&/^(project|compiler)\//.test(s);
const array=a=>Array.isArray(a)&&a.length<=100000;
const unavailable=reason=>({schema:SCHEMA,authorizes_runtime_edge:false,status:'unavailable',reason,payload:null});
const sources=()=>hash(canonical({observer:producerHash(),membership:['membership.mjs','membership-worker.mjs'].map(f=>
  [f,hash(readFileSync(new URL(f,import.meta.url)))])}));
const sortedUnique=rows=>array(rows)&&rows.every((r,i)=>id(r.id)&&(i===0||rows[i-1].id<r.id));

function assemble(first,native,identity) {
  const {packet,selection,program}=first;
  requireThat(packet.compiler.verified&&packet.closure.stable_snapshot&&packet.config_provenance.status==='observed');
  requireThat(!packet.reasons.some(r=>['invalid_config','unsupported_references','unsupported_plugins'].includes(r)));
  requireThat(keys(identity,'producer_sha256 native_sha256')&&Object.values(identity).every(digest));
  const manifest=new Map(packet.snapshot.files.map(r=>[r.id,r]));
  requireThat(manifest.size===packet.snapshot.files.length);
  requireThat(array(selection)&&selection.length===packet.config_provenance.files.length);
  selection.forEach((s,i)=>{
    requireThat(keys(s,'file sha256 fields')&&s.file===packet.config_provenance.files[i].file&&
      s.sha256===manifest.get(s.file)?.sha256&&array(s.fields));
    const names=new Set();
    for(const f of s.fields) {
      requireThat(keys(f,'name values anchor')&&['files','include','exclude'].includes(f.name)&&!names.has(f.name));names.add(f.name);
      requireThat(array(f.values)&&f.values.every(v=>typeof v==='string')&&keys(f.anchor,'file sha256 start_byte end_byte')&&
        f.anchor.file===s.file&&f.anchor.sha256===s.sha256&&integer(f.anchor.start_byte)&&integer(f.anchor.end_byte)&&
        f.anchor.start_byte<=f.anchor.end_byte&&f.anchor.end_byte<=manifest.get(s.file).size);
    }
  });
  requireThat(keys(native,'schema authorizes_runtime_edge files skipped supported type_database_present')&&
    native.schema==='prism.native-membership/1'&&native.authorizes_runtime_edge===false&&typeof native.type_database_present==='boolean');
  requireThat(sortedUnique(native.files)&&sortedUnique(native.supported)&&array(native.skipped));
  const languages=new Set(['Python','JavaScript','TypeScript','Tsx','Go','Rust','Java','C','Cpp','Lua','Terraform','Bash']);
  const L=new Map(native.files.map(f=>{
    requireThat(keys(f,'id sha256 size language parse_errors')&&f.id.startsWith('project/')&&digest(f.sha256)&&
      integer(f.size)&&integer(f.parse_errors)&&languages.has(f.language));
    const m=manifest.get(f.id);requireThat(m&&m.sha256===f.sha256&&m.size===f.size);
    return [f.id,f];
  }));
  const supports=new Map(native.supported.map(f=>{
    requireThat(keys(f,'id language')&&(f.language===null||languages.has(f.language)));return [f.id,f.language];
  }));
  for(const s of native.skipped) {
    requireThat(keys(s,'id reason')&&typeof s.id==='string'&&s.id.startsWith('project/')&&
      (s.id==='project/'||id(s.id.replace(/\/$/,''))));
    requireThat(['Unsupported','Ignored','GoTestdata','Symlink','Hidden','Unreadable','NotUtf8','ParseFailed'].includes(s.reason)||
      keys(s.reason,'TooLarge')&&keys(s.reason.TooLarge,'bytes')&&integer(s.reason.TooLarge.bytes));
  }
  requireThat(sortedUnique(program)&&canonical(program.map(f=>f.id))===canonical(packet.snapshot.program_files)&&
    canonical([...supports.keys()])===canonical(packet.snapshot.program_files));
  const roots=new Set(packet.snapshot.roots),P=new Set(packet.snapshot.program_files);
  requireThat(roots.size===packet.snapshot.roots.length&&[...roots].every(f=>manifest.has(f)));
  const members=program.map(f=>{
    requireThat(keys(f,'id sha256 size declaration json domain root')&&typeof f.declaration==='boolean'&&typeof f.json==='boolean');
    const m=manifest.get(f.id),domain=f.id.startsWith('compiler/')?'compiler':f.id.includes('/node_modules/')?'dependency':'repository';
    requireThat(m&&m.sha256===f.sha256&&m.size===f.size&&f.domain===domain&&f.root===roots.has(f.id));
    return {...f,native_language:supports.get(f.id),native_member:L.has(f.id),native_parse_errors:L.get(f.id)?.parse_errors??null,
      inventory_aliases:packet.snapshot.links.filter(l=>l.target===f.id||f.id.startsWith(l.target+'/')).map(l=>l.id)};
  });
  const failedSkip=s=>['Unreadable','NotUtf8','ParseFailed','Symlink'].includes(s.reason)||typeof s.reason==='object';
  return {packet,selection,program:members,native,
    native_status:native.files.some(f=>f.parse_errors>0)||native.skipped.some(failedSkip)?'incomplete':'observed',
    differences:{roots_not_program:[...roots].filter(f=>!P.has(f)).sort(),program_not_roots:[...P].filter(f=>!roots.has(f)),
      program_not_native:[...P].filter(f=>!L.has(f)),native_not_program:[...L.keys()].filter(f=>!P.has(f))},identity};
}

export function parseMembership(text) {
  requireThat(typeof text==='string'&&Buffer.byteLength(text)<=MAX_PACKET_BYTES);
  const value=JSON.parse(text);
  requireThat(keys(value,'schema authorizes_runtime_edge status reason payload')&&value.schema===SCHEMA&&value.authorizes_runtime_edge===false);
  if(value.status==='unavailable') {requireThat(reasons.includes(value.reason)&&value.payload===null);return value;}
  requireThat(value.status==='observed'&&value.reason===null);
  const p=value.payload;
  requireThat(keys(p,'packet selection program native native_status differences identity')&&array(p.program));
  parsePacket(JSON.stringify(p.packet));
  requireThat(Buffer.byteLength(text)<=PROFILES[p.packet.scope.acquisition_profile].packet_bytes);
  const program=p.program.map(f=>{
    requireThat(keys(f,'id sha256 size declaration json domain root native_language native_member native_parse_errors inventory_aliases'));
    const {native_language,native_member,native_parse_errors,inventory_aliases,...base}=f;return base;
  });
  requireThat(canonical(p)===canonical(assemble({packet:p.packet,selection:p.selection,program},p.native,p.identity)));
  return value;
}

export function observeMembership(input,nativeExecutable) {
  try {
    const options=settings(input),profile=PROFILES[options.profile];
    requireThat(typeof nativeExecutable==='string'&&path.isAbsolute(nativeExecutable),'invalid_options');
    const nativePath=realpathSync(nativeExecutable),deadline=Date.now()+options.limits.timeout_ms;
    const identity={producer_sha256:sources(),native_sha256:hash(readFileSync(nativePath))};
    const env={...process.env};delete env.NODE_OPTIONS;delete env.NODE_PATH;
    function run(executable,args,body,reason) {
      const remaining=deadline-Date.now();if(remaining<=0)fail('budget_exceeded');
      const r=spawnSync(executable,args,{input:body,encoding:'utf8',env,timeout:remaining,maxBuffer:profile.packet_bytes});
      if(r.error)fail(['ETIMEDOUT','ENOBUFS'].includes(r.error.code)?'budget_exceeded':reason);
      requireThat(r.status===0&&r.stderr==='',reason);
      const data=JSON.parse(r.stdout);if(data.unavailable)fail(data.unavailable);return data;
    }
    const args=[`--max-old-space-size=${profile.heap_mb}`,fileURLToPath(new URL('membership-worker.mjs',import.meta.url))];
    const first=run(process.execPath,args,JSON.stringify(options),'worker_failed');
    parsePacket(JSON.stringify(first.packet));
    const ids=JSON.stringify(first.packet.snapshot.program_files);
    requireThat(Buffer.byteLength(ids)<=8*1024*1024,'budget_exceeded');
    const native=run(nativePath,[options.root],ids,'native_failed');
    const second=run(process.execPath,args,JSON.stringify(options),'worker_failed');
    requireThat(canonical(first)===canonical(second),'unstable_snapshot');
    requireThat(identity.native_sha256===hash(readFileSync(nativePath))&&identity.producer_sha256===sources(),'native_identity');
    requireThat(Date.now()<=deadline,'budget_exceeded');
    const result={schema:SCHEMA,authorizes_runtime_edge:false,status:'observed',reason:null,payload:assemble(first,native,identity)};
    requireThat(Buffer.byteLength(JSON.stringify(result))<=profile.packet_bytes,'budget_exceeded');
    return parseMembership(JSON.stringify(result));
  }catch(error){return unavailable(reasons.includes(error.message)?error.message:'worker_failed');}
}

export function validateMembership(text,input,nativeExecutable) {
  try {
    const claimed=parseMembership(text);
    const fresh=observeMembership(input,nativeExecutable);
    const valid=fresh.status==='observed'&&canonical(claimed)===canonical(fresh);
    return {valid,authorizes_runtime_edge:false,reason:valid?'reproduced_observation':'unavailable_or_changed'};
  }catch{return {valid:false,authorizes_runtime_edge:false,reason:'invalid_artifact'};}
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [mode,compiler,root,config,native,profile='default',links='reject']=process.argv.slice(2);
  if(!['observe','validate'].includes(mode)||![7,8,9].includes(process.argv.length))throw Error(
    'usage: membership.mjs observe|validate <typescript.js> <audit-root> <relative-config> <trusted-native-executable> [default|installed] [reject|in-root]');
  const input={compiler,root,config,profile,links};
  const result=mode==='observe'?observeMembership(input,native):validateMembership(readPacket(0,PROFILES[profile]?.packet_bytes),input,native);
  console.log(JSON.stringify(result));
  if(mode==='observe'?result.status!=='observed':!result.valid)process.exitCode=1;
}
