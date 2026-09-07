import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, cpSync, rmSync, symlinkSync} from "node:fs";
import {spawn,spawnSync} from "node:child_process";
import {once} from "node:events";
import {createRequire} from "node:module";
import {tmpdir} from "node:os";
import path from "node:path";
import {produce, validate, producerHash} from "./index.mjs";
import {parsePacket,PACKET_BYTES,hash} from "./schema.mjs";

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

test("provenance retains each explicit renamed import and barrel hop",()=>fixture(({put,options})=>{
  put("src/barrel.ts","export type {FC as Component} from 'react';");
  put("src/app.ts","import type {Component as View} from './barrel';import Client from './client';export const run: View<{client:Client}>=({client})=>{client.m();return null;};");
  const p=produce(options),chain=p.observations[0].provenance;
  assert(chain,"configured observation lacks declaration/alias provenance");
  assert.equal(chain.status,"traced",JSON.stringify(chain));
  assert.deepEqual(chain.hops[0].aliases.map(a=>a.declarations[0].kind),["ImportSpecifier","ExportSpecifier"]);
  assert.deepEqual(chain.hops[0].aliases.map(a=>a.declarations[0].file),["project/src/app.ts","project/src/barrel.ts"]);
  assert.equal(chain.terminal.kind,"InterfaceDeclaration");
  assert.equal(chain.terminal.file,"project/node_modules/@types/react/index.d.ts");
}));

test("qualified generic aliases retain namespace binding, argument and binder source scope",()=>fixture(({put,options})=>{
  put("src/shape.ts","import * as R from 'react';export type View<P> = R.FC<P>;");
  put("src/app.ts","import type {View} from './shape';import Client from './client';export const run: View<{client:Client}>=({client})=>{client.m();return null;};");
  const p=produce(options),chain=p.observations[0].provenance;
  assert(chain,"configured observation lacks scoped generic provenance");
  assert.equal(chain.status,"traced",JSON.stringify(chain));
  assert.equal(chain.hops[0].type_arguments[0].file,"project/src/app.ts");
  assert.equal(chain.hops[0].type_parameters[0].file,"project/src/shape.ts");
  assert.equal(chain.hops[1].qualifiers[0].aliases[0].declarations[0].kind,"NamespaceImport");
  assert.deepEqual(chain.hops[1].qualifiers[0].aliases.map(a=>a.declarations[0].kind),["NamespaceImport","ExportAssignment"]);
  assert.equal(chain.hops[1].qualifiers[0].aliases[0].module_bindings[0].kind,"ModuleDeclaration");
  assert.equal(chain.hops[1].qualifiers[0].use.file,"project/src/shape.ts");
  assert.equal(p.authorizes_runtime_edge,false);
}));

test("schema zero is rejected before root access after provenance schema transition",()=>fixture(({options})=>{
  const p=produce(options);p.schema="prism.callable-observation/0";
  let consulted=0;const forbidden={get root(){consulted++;throw Error("old schema must reject pre-I/O");}};
  assert.equal(validate(JSON.stringify(p),forbidden).valid,false);
  assert.equal(consulted,0);
}));

test("provenance default imports, namespace gateways and inline callable terminals",()=>fixture(({put,options})=>{
  put("src/barrel.ts","export type {FC as default} from 'react';");
  const cases=[
    ["import View from './barrel';","View<{client:Client}>","InterfaceDeclaration"],
    ["import R from 'react';","R.FC<{client:Client}>","InterfaceDeclaration"],
    ["","(p:{client:Client})=>null","FunctionType"],
    ["type View = {(p:{client:Client}):null};","View","TypeLiteral"],
    ["interface View {(p:{client:Client}):null}","View","InterfaceDeclaration"],
  ];
  for(const [imports,type,terminal] of cases){
    put("src/app.ts",`import Client from './client';${imports}export const run: ${type}=({client})=>{client.m();return null;};`);
    const p=produce(options),chain=p.observations[0].provenance;
    assert.equal(chain.status,"traced",JSON.stringify(chain));assert.equal(chain.terminal.kind,terminal);
    assert.equal(validate(JSON.stringify(p),options).valid,true);
  }
}));

test("same-spelled FC and generic parameters retain defining identity without substitution",()=>fixture(({put,options})=>{
  put("src/app.ts","import Client from './client';type FC<T>=(p:{client:Client})=>null;export const run: FC<string>=({client}:any)=>{client.m();return null;};");
  const p=produce(options),o=p.observations[0];assert.equal(o.provenance.status,"traced");
  assert.equal(o.provenance.terminal.file,"project/src/app.ts");
  assert.equal(o.provenance.hops[0].type_arguments[0].kind,"StringKeyword");
  assert.equal(o.explicit_parameter,true);assert.equal(o.calls[0].receiver_type,"any");
  assert.equal(p.scope.class_authority,false);
}));

test("provenance negative population is explicitly bounded",()=>fixture(({put,options})=>{
  const cases=[
    ["type View=Other;type Other=View;","View","cycle"],
    ["type View<T>=T;","View<(p:{client:Client})=>null>","unsupported_declaration"],
    ["type View<T>=T extends string?never:(p:{client:Client})=>null;","View<Client>","unsupported_type"],
    ["type View=((p:{client:Client})=>null)&{x?:string};","View","unsupported_type"],
    ["type View=(p:{client:Client})=>null;type View=(p:any)=>null;","View","ambiguous_declaration"],
    ["interface View {(p:{client:Client}):null} interface View {x?:string}","View","ambiguous_declaration"],
    ["interface Base {(p:{client:Client}):null} interface View extends Base {}","View","unsupported_heritage"],
    ["interface View {m():void}","View","unsupported_type"],
    ["type View={x:string};","View","unsupported_type"],
    ["","Missing","unresolved_symbol"],
  ];
  const outcomes=[];
  for(const [declaration,type,reason] of cases) {
    put("src/app.ts",`import Client from './client';${declaration}export const run: ${type}=({client}:any)=>{client.m();return null;};`);
    const p=produce(options);assert(p.observations.length,JSON.stringify(p.reasons));
    const chain=p.observations[0].provenance;
    outcomes.push({declaration,status:chain.status,reason:chain.reason,terminal:chain.terminal===null,authority:p.authorizes_runtime_edge});
  }
  assert.deepEqual(outcomes,cases.map(([declaration,,reason])=>({declaration,status:"unproven",reason,terminal:true,authority:false})));
}));

test("star barrels and imported export-equals gateways do not become complete chains",()=>fixture(({put,options})=>{
  const cases=[
    ["export * from 'react';","unsupported_export_star"],
    ["export type {FC} from 'react';export * as extras from 'react';","unsupported_export_star"],
    ["import * as R from 'react';export = R;","unsupported_declaration"],
  ];
  put("src/app.ts","import type {FC} from './barrel';import Client from './client';export const run: FC<{client:Client}>=({client})=>{client.m();return null;};");
  for(const [source,reason] of cases) {
    put("src/barrel.ts",source);const p=produce(options),chain=p.observations[0].provenance;
    assert.equal(chain.status,"unproven",source);assert.equal(chain.reason,reason,source);
    assert.equal(p.authorizes_runtime_edge,false);
  }
}));

test("missing imports and namespace merges remain partial observations",()=>fixture(({put,options})=>{
  put("src/app.ts","import type {FC} from './missing';import Client from './client';export const run: FC<{client:Client}>=({client}:any)=>{client.m();return null;};");
  let p=produce(options);assert.equal(p.observations[0].provenance.reason,"unresolved_symbol");
  put("src/app.ts","import Client from './client';namespace R {export type FC<T>=(p:T)=>null} namespace R {export type Other=string} export const run: R.FC<{client:Client}>=({client})=>{client.m();return null;};");
  p=produce(options);assert.equal(p.observations[0].provenance.reason,"ambiguous_declaration");
  assert.equal(p.observations[0].provenance.hops[0].qualifiers[0].declarations.length,2);
}));

test("provenance budgets and corrupted nested anchors fail closed",()=>fixture(({options})=>{
  const p=produce(options);assert.equal(p.schema,"prism.callable-observation/1");
  assert.equal(p.observations[0].provenance.status,"traced");
  const limited=produce({...options,limits:{provenance_steps:1}});
  assert.equal(limited.observations[0].provenance.reason,"step_limit");
  assert.equal(limited.observations[0].provenance.steps_used,1);
  let consulted=0;const forbidden={get root(){consulted++;throw Error("pre-I/O");}};
  for(const change of [q=>q.hops[0].aliases[0].module.sha256="0".repeat(64),
    q=>q.hops[0].aliases[0].module_bindings[0].sha256="0".repeat(64),
    q=>q.hops[0].type_arguments[0].end_byte=0,q=>q.terminal.file="project/../escape",
    q=>q.steps_used=33,q=>q.hops[0].extra="forged"]){
    const forged=structuredClone(p);change(forged.observations[0].provenance);
    assert.equal(validate(JSON.stringify(forged),forbidden).valid,false);
  }
  assert.equal(consulted,0);
  const forged=structuredClone(p);forged.observations[0].provenance.hops[0].aliases=[];
  assert.equal(validate(JSON.stringify(forged),options).valid,false);
}));

test("barrel A to B and restoration replace provenance even with unchanged consumer bytes",()=>fixture(({root,put,options})=>{
  put("src/a.ts","export type FC<T>=(p:T)=>null;");put("src/b.ts","export type FC<T>=(p:T)=>null;");
  put("src/barrel.ts","export type {FC} from './a';");
  put("src/app.ts","import type {FC} from './barrel';import Client from './client';export const run: FC<{client:Client}>=({client})=>{client.m();return null;};");
  const a=produce(options);assert.equal(a.observations[0].provenance.terminal.file,"project/src/a.ts");
  put("src/barrel.ts","export type {FC} from './b';");const b=produce(options);
  assert.equal(b.observations[0].provenance.terminal.file,"project/src/b.ts");
  assert.equal(validate(JSON.stringify(a),options).valid,false);
  rmSync(path.join(root,"src/barrel.ts"));assert.equal(validate(JSON.stringify(b),options).valid,false);
  put("src/barrel.ts","export type {FC} from './a';");assert.equal(validate(JSON.stringify(a),options).valid,true);
}));

test("duplicate explicit export names are not hidden by checker error recovery",()=>fixture(({put,options})=>{
  put("src/a.ts","export type A=(p:any)=>null;");put("src/b.ts","export type B=(p:any)=>null;");
  put("src/barrel.ts","export type {A as View} from './a';export type {B as View} from './b';");
  put("src/app.ts","import type {View} from './barrel';export const run:View=p=>null;");
  const chain=produce(options).observations[0].provenance;
  assert.equal(chain.status,"unproven");assert.equal(chain.reason,"ambiguous_declaration");
}));

test("direct exported declarations also reject conflicting re-export names",()=>fixture(({put,options})=>{
  put("src/a.ts","export type A=(p:any)=>null;");
  const cases=["export type View=(p:any)=>null;export type {A as View} from './a';",
    "export type {A as View} from './a';export type View=(p:any)=>null;"];
  put("src/app.ts","import type {View} from './barrel';export const run:View=p=>null;");
  const outcomes=cases.map(source=>{put("src/barrel.ts",source);return produce(options).observations[0].provenance.status;});
  assert.deepEqual(outcomes,["unproven","unproven"]);
}));

test("local export aliases stay distinct from local bindings and duplicate imports",()=>fixture(({put,options})=>{
  put("src/barrel.ts","type View=(p:any)=>null;export type {View};");
  put("src/app.ts","import type {View} from './barrel';export const run:View=p=>null;");
  let p=produce(options);assert.equal(p.observations[0].provenance.status,"traced");
  assert.deepEqual(p.observations[0].provenance.hops[0].aliases.map(a=>a.declarations[0].kind),["ImportSpecifier","ExportSpecifier"]);
  put("src/app.ts","import type {View} from './barrel';import type {View} from './barrel';export const run:View=p=>null;");
  p=produce(options);assert.equal(p.observations[0].provenance.reason,"ambiguous_declaration");
}));

test("nested namespace uses and limits retain source identity",()=>fixture(({put,options})=>{
  put("src/barrel.ts","export namespace Outer {export namespace Inner {export type View<P>=(p:P)=>null}};");
  put("src/app.ts","import * as N from './barrel';export const run:N.Outer.Inner.View<string>=p=>null;");
  const p=produce(options),chain=p.observations[0].provenance;
  assert.equal(chain.status,"traced");assert.equal(chain.hops[0].qualifiers.length,3);
  assert.equal(chain.terminal.file,"project/src/barrel.ts");
  assert.equal(produce({...options,limits:{provenance_steps:2}}).observations[0].provenance.reason,"step_limit");
}));

test("producer digest covers the new provenance implementation",()=>{
  const sources=["schema.mjs","index.mjs","worker.mjs","provenance.mjs"].map(f=>readFileSync(new URL(f,import.meta.url)));
  assert.equal(producerHash(),hash(Buffer.concat(sources)));
  assert.notEqual(producerHash(),hash(Buffer.concat(sources.slice(0,3))));
});
