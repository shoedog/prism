import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, cpSync, rmSync, symlinkSync} from "node:fs";
import {spawn,spawnSync} from "node:child_process";
import {once} from "node:events";
import {createRequire} from "node:module";
import {tmpdir} from "node:os";
import path from "node:path";
import {produce, validate} from "./index.mjs";
import {parsePacket,PACKET_BYTES} from "./schema.mjs";

const compiler=process.env.PRISM_TYPESCRIPT, profiles=process.env.PRISM_CALLABLE_PROFILES;
assert(compiler && profiles,"explicit pinned compiler and profiles required; no silent skip");
async function fixture(run,profile="react19") {
  const root=mkdtempSync(path.join(tmpdir(),"prism-configured-test-"));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const config={compilerOptions:{strict:true,noEmit:true,target:"ES2022",module:"ESNext",
    moduleResolution:"Bundler",types:[],libReplacement:false,skipLibCheck:false},include:["src"]};
  try {
    cpSync(path.join(profiles,profile,"node_modules"),path.join(root,"node_modules"),{recursive:true});
    put("tsconfig.json",JSON.stringify(config));
    put("package.json",JSON.stringify({type:"module"}));
    put("src/client.ts","export default class Client { m() {} }");
    put("src/app.ts","import type {FC} from 'react'; import Client from './client';\nconst marker='🦊';\r\nexport const run: FC<{client:Client}> = ({client}) => {client.m();return null;};");
    put("excluded.ts","this is deliberately not valid TypeScript !!!");
    await run({root,put,config,options:{root,compiler,config:"tsconfig.json"}});
  } finally {rmSync(root,{recursive:true,force:true});}
}

test("configured include membership replaces a syntax-only empty census",()=>fixture(({options})=>{
  const packet=produce(options);
  assert.deepEqual(packet.snapshot.roots,["project/src/app.ts","project/src/client.ts"]);
  assert.equal(packet.status,"observed",JSON.stringify({reasons:packet.reasons,diagnostics:packet.diagnostics,outside:packet.snapshot.outside_lookups}));
  assert.equal(packet.authorizes_runtime_edge,false);
}));
test("installed declarations supply anchored contextual observations, not class authority",()=>fixture(({options})=>{
  const packet=produce(options);
  assert.equal(packet.observations.length,1);
  const observation=packet.observations[0];
  assert.equal(observation.signatures[0].file,"project/node_modules/@types/react/index.d.ts");
  assert.equal(observation.calls[0].declarations[0].file,"project/src/client.ts");
  assert(observation.calls[0].call.start_byte>observation.calls[0].call.start_utf16);
  assert.equal(packet.authorizes_runtime_edge,false);
}));
test("an asserted authority flag is rejected, never accepted by the validator",()=>fixture(({options})=>{
  const packet=produce(options);packet.authorizes_runtime_edge=true;
  assert.equal(validate(JSON.stringify(packet),options).valid,false);
}));

test("valid packets reproduce without absolute roots and remain non-authorizing",()=>fixture(({root,options})=>{
  const packet=produce(options),text=JSON.stringify(packet);
  assert(!text.includes(root));assert(!text.includes(compiler));
  assert.equal(validate(text,options).valid,true);
  assert.equal(validate(text,options).authorizes_runtime_edge,false);
  assert.deepEqual(parsePacket(text),packet);
}));

test("closed schema rejects malformed and unsafe packets before consulting caller roots",()=>fixture(({options})=>{
  const base=produce(options);
  const mutations=[p=>p.schema="future",p=>p.extra=true,p=>p.snapshot.extra=true,
    p=>p.scope.class_authority=true,p=>p.snapshot.files[0].id="project/../escape",
    p=>p.snapshot.files[0].id="/absolute",p=>p.snapshot.files[0].id="project\\escape",
    p=>p.observations[0].implementation.start_byte=-1,p=>p.compiler.sha256="0",
    p=>p.status="exact",p=>p.observations[0].signatures="bogus"];
  let consulted=0;const forbidden={get root(){consulted++;throw Error("must not consult root");}};
  for(const mutate of mutations){const p=structuredClone(base);mutate(p);assert.equal(validate(JSON.stringify(p),forbidden).valid,false);}
  for(const text of ["{", "null", " ".repeat(PACKET_BYTES+1)])assert.equal(validate(text,forbidden).valid,false);
  assert.equal(consulted,0);
}));

test("conflicting anchor ranges and manifest hashes fail before root access",()=>fixture(({options})=>{
  const base=produce(options);let consulted=0;
  const forbidden={get root(){consulted++;throw Error("pre-I/O contract");}};
  for(const mutate of [p=>p.observations[0].implementation.end_byte=0,
    p=>p.observations[0].annotation.sha256="0".repeat(64)]) {
    const p=structuredClone(base);mutate(p);assert.equal(validate(JSON.stringify(p),forbidden).valid,false);
  }
  assert.equal(consulted,0);
}));

test("explicitly requested excluded metadata cannot silently disappear from program closure",()=>fixture(({put,config,options})=>{
  put(".git/hidden.d.ts","import 'react';declare module 'react' {interface FunctionComponent<P>{extra?:string;}}");
  put("tsconfig.json",JSON.stringify({...config,include:["src",".git"]}));
  const p=produce(options);assert.equal(p.status,"unproven",JSON.stringify(p.reasons));
}));

test("well-shaped forged anchors, options, membership and closure cannot survive recomputation",()=>fixture(({options})=>{
  const base=produce(options);
  const mutations=[p=>p.observations[0].calls[0].call.start_byte++,
    p=>p.snapshot.options_sha256="0".repeat(64),p=>p.snapshot.files.pop(),
    p=>p.snapshot.failed_lookups=[],p=>p.closure.augmentation=false,
    p=>p.observations[0].calls[0].declarations=[]];
  assert(base.snapshot.failed_lookups.length>0);
  for(const mutate of mutations){const p=structuredClone(base);mutate(p);assert.equal(validate(JSON.stringify(p),options).valid,false);}
}));

test("local extends and paths use actual config; config and directory additions invalidate packets",()=>fixture(({put,config,options})=>{
  put("config/base.json",JSON.stringify({compilerOptions:config.compilerOptions}));
  put("tsconfig.json",JSON.stringify({extends:"./config/base.json",include:["src"],compilerOptions:{baseUrl:".",paths:{"@client":["src/client.ts"]}}}));
  put("src/app.ts","import type {FC as Component} from 'react';import Client from '@client';export const run: Component<{client:Client}>=({client})=>{client.m();return null;};");
  const before=produce(options);assert.equal(before.status,"observed");
  assert(before.snapshot.config_files.includes("project/config/base.json"));
  assert(before.resolutions.some(r=>r.specifier==="@client" && r.target==="project/src/client.ts"));
  put("src/added.ts","export const added=1;");
  const after=produce(options);assert(after.snapshot.roots.includes("project/src/added.ts"));
  assert.equal(validate(JSON.stringify(before),options).valid,false);
  put("config/base.json",JSON.stringify({compilerOptions:{...config.compilerOptions,strict:false}}));
  assert.equal(validate(JSON.stringify(after),options).valid,false);
}));

test("missing dependency removal/restoration replaces the epoch without stale observations",()=>fixture(({root,options})=>{
  const file=path.join(root,"node_modules/@types/react/index.d.ts"),bytes=readFileSync(file);
  const before=produce(options);rmSync(file);
  const absent=produce(options);assert.equal(absent.status,"unproven");
  assert(absent.diagnostics.some(d=>d.code===2307));
  assert.equal(validate(JSON.stringify(before),options).valid,false);
  writeFileSync(file,bytes);
  assert.equal(validate(JSON.stringify(absent),options).valid,false);
  assert.equal(validate(JSON.stringify(before),options).valid,true);
}));

test("A to B and back changes declaration origin and invalidates same-span evidence",()=>fixture(({put,options})=>{
  const app=name=>`import type {FC} from 'react';import Client from './${name}';export const run: FC<{client:Client}>=({client})=>{client.m();return null;};`;
  put("src/otherx.ts","export default class Other {m(){}}");
  put("src/app.ts",app("client"));const a=produce(options);
  put("src/app.ts",app("otherx"));const b=produce(options);
  assert.equal(a.observations[0].calls[0].call.start_byte,b.observations[0].calls[0].call.start_byte);
  assert.equal(b.observations[0].calls[0].declarations[0].file,"project/src/otherx.ts");
  assert.equal(validate(JSON.stringify(a),options).valid,false);
  put("src/app.ts",app("client"));assert.equal(validate(JSON.stringify(a),options).valid,true);
  assert.equal(validate(JSON.stringify(b),options).valid,false);
}));

test("augmentation membership is inventoried; clean overload diagnostics are not ownership",()=>fixture(({put,options,root})=>{
  const before=produce(options);
  put("src/other.ts","export default class Other {m(){}}");
  put("src/augment.d.ts","import 'react';import Other from './other';declare module 'react' {interface FunctionComponent<P>{(props:{client:Other}):ReactNode;}}");
  const merged=produce(options);assert.deepEqual(merged.diagnostics,[]);
  assert.equal(merged.observations[0].signatures.length,2);
  assert(merged.observations[0].callable_declarations.some(d=>d.file==="project/src/augment.d.ts"));
  assert.equal(merged.observations[0].calls[0].declarations.length,2);
  assert.equal(merged.authorizes_runtime_edge,false);
  assert.equal(validate(JSON.stringify(before),options).valid,false);
  rmSync(path.join(root,"src/augment.d.ts"));assert.equal(validate(JSON.stringify(merged),options).valid,false);
}));

test("explicit any remains explicit while assertion-only producers are not invented",()=>fixture(({put,options})=>{
  put("src/app.ts","import type {FC} from 'react';import Client from './client';export const run: FC<{client:Client}>=({client}:any)=>{client.m();return null;};");
  const p=produce(options);assert.equal(p.observations[0].explicit_parameter,true);
  assert.equal(p.observations[0].calls[0].receiver_type,"any");
  assert.deepEqual(p.observations[0].calls[0].declarations,[]);
  put("src/app.ts","import type {FC} from 'react';import Client from './client';function run({client}:any){client.m();return null;} export const cast=run as FC<{client:Client}>;");
  assert.deepEqual(produce(options).observations,[]);
}));

test("unsupported references and plugins are reported without executing project code",()=>fixture(({put,config,options})=>{
  put("ref/tsconfig.json","{}");
  put("plugin.js","throw new Error('project plugin must never run');");
  put("tsconfig.json",JSON.stringify({...config,references:[{path:"./ref"}],compilerOptions:{...config.compilerOptions,plugins:[{name:"./plugin.js"}]}}));
  const p=produce(options);assert.equal(p.status,"unproven");
  assert(p.reasons.includes("unsupported_references"));assert(p.reasons.includes("unsupported_plugins"));
  assert.equal(p.closure.references,false);
}));

test("absent package boundary causes outside lookup, not synthetic closure",()=>fixture(({root,options})=>{
  rmSync(path.join(root,"package.json"));const p=produce(options);
  assert(p.reasons.includes("outside_lookup"));assert.equal(p.status,"unproven");
}));

test("symlinks, invalid compiler and snapshot/time/observation budgets fail closed",()=>fixture(({root,put,options})=>{
  symlinkSync("src",path.join(root,"linked"));assert.deepEqual(produce(options).reasons,["unsupported_input"]);
  rmSync(path.join(root,"linked"));
  put("bad/typescript.js","throw Error('not an authorized compiler');");
  assert.deepEqual(produce({...options,compiler:path.join(root,"bad/typescript.js")}).reasons,["compiler_mismatch"]);
  for(const limits of [{bytes:1},{files:1},{depth:1},{timeout_ms:1}])assert.deepEqual(produce({...options,limits}).reasons,["budget_exceeded"]);
  put("src/second.ts","export const f: (p:string)=>void = p=>{p.trim();};");
  assert.deepEqual(produce({...options,limits:{observations:1}}).reasons,["budget_exceeded"]);
  assert.throws(()=>produce({...options,config:"../tsconfig.json"}),/invalid_options/);
  assert.throws(()=>produce({...options,limits:{bytes:Infinity}}),/invalid_limits/);
}));

test("concurrent monotonically changing input prevents stable-snapshot closure",()=>fixture(async({root,options})=>{
  const file=path.join(root,"changing.txt");
  const child=spawn(process.execPath,["-e","const fs=require('fs');fs.writeFileSync(process.argv[1],'x');setInterval(()=>fs.appendFileSync(process.argv[1],'x'),2);process.stdout.write('ready');",file],{stdio:["ignore","pipe","pipe"]});
  try {await once(child.stdout,"data");const p=produce(options);assert(p.reasons.includes("unstable_snapshot"),JSON.stringify(p.reasons));assert.equal(p.closure.stable_snapshot,false);}
  finally {child.kill();await once(child,"close");}
}));

test("CLI emits and validates packets through stdout/stdin without project writes",()=>fixture(({root,options})=>{
  const args=["scripts/callable-observations/index.mjs","produce",compiler,root,options.config];
  const made=spawnSync(process.execPath,args,{encoding:"utf8",maxBuffer:PACKET_BYTES});
  assert.equal(made.status,0,made.stderr);const p=parsePacket(made.stdout);assert.equal(p.status,"observed");
  args[1]="validate";const checked=spawnSync(process.execPath,args,{input:made.stdout,encoding:"utf8"});
  assert.equal(checked.status,0,checked.stderr);assert.equal(JSON.parse(checked.stdout).valid,true);
}));

test("invalid UTF-8 compiler input is refused and inherited lib/type lookups stay unproven",()=>fixture(({put,config,options})=>{
  put("src/bad.ts",Buffer.from([0xff]));assert.deepEqual(produce(options).reasons,["unsupported_input"]);
  put("src/bad.ts","export {}; ");
  delete config.compilerOptions.types;delete config.compilerOptions.libReplacement;
  put("tsconfig.json",JSON.stringify(config));assert.equal(produce(options).status,"unproven");
}));

test("directory import probes with trailing slashes retain valid manifest IDs",()=>fixture(({root,put,options})=>{
  put("src/widgets/index.ts","export {}; ");
  put("src/app.ts","import './widgets/';\n"+readFileSync(path.join(root,"src/app.ts"),"utf8"));
  const p=produce(options);assert.equal(p.status,"observed",JSON.stringify(p.reasons));
  assert(p.snapshot.failed_lookups.every(id=>!id.endsWith("/")));
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));

test("CLI rejects oversized stdin before project access",()=>{
  const r=spawnSync(process.execPath,["scripts/callable-observations/index.mjs","validate",compiler,"/nonexistent-project","tsconfig.json"],
    {input:" ".repeat(PACKET_BYTES+1),encoding:"utf8"});
  assert.equal(r.status,1,r.stderr);assert.equal(JSON.parse(r.stdout).reason,"invalid_packet_or_options");
});

test("React18 installed declarations also produce non-authorizing configured observations",()=>fixture(({options})=>{
  const p=produce(options);assert.equal(p.status,"observed");
  assert.equal(p.observations[0].signatures.length,1);
  assert.equal(p.observations[0].calls[0].declarations[0].file,"project/src/client.ts");
  assert.equal(validate(JSON.stringify(p),options).valid,true);
},"react18"));

test("compiler host case policy cannot select a declaration shadow ahead of a source file",()=>fixture(({root,put,options})=>{
  const ts=createRequire(import.meta.url)(compiler);
  put("src/thing.ts","export default class Source {m(){}}");
  put("src/THING.d.ts","export default class Shadow {m():void;}");
  put("src/app.ts","import type {FC} from 'react';import Client from './THING';export const run: FC<{client:Client}>=({client})=>{client.m();return null;};");
  const live=ts.resolveModuleName("./THING",path.join(root,"src/app.ts"),{moduleResolution:ts.ModuleResolutionKind.Bundler},ts.sys).resolvedModule;
  assert(live);
  const expected=ts.sys.useCaseSensitiveFileNames?"project/src/THING.d.ts":"project/src/thing.ts";
  assert.equal(live.extension,ts.sys.useCaseSensitiveFileNames?".d.ts":".ts");
  const p=produce(options);assert.equal(p.observations[0].calls[0].declarations[0].file,expected);
}));
