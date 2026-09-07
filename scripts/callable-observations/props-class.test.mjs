import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,cpSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
import {produce,validate} from "./index.mjs";
const compiler=process.env.PRISM_TYPESCRIPT,profiles=process.env.PRISM_CALLABLE_PROFILES;
assert(compiler && profiles,"explicit pinned compiler and profiles required");
async function fixture(run){
  const root=mkdtempSync(path.join(tmpdir(),"prism-props-test-"));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const source=(annotation="FC<Props<Client>>",parameter="{client}",body="const callback=()=>client.m()",extra="")=>put("src/app.ts",`import type {FC} from 'react';import Client from './client';import type {Props} from './props';${extra}\nexport const run:${annotation}=(${parameter})=>{${body};return null;};`);
  try {
    cpSync(path.join(profiles,"react19/node_modules"),path.join(root,"node_modules"),{recursive:true});
    put("package.json",'{"type":"module"}');put("tsconfig.json",JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:"ES2022",module:"ESNext",moduleResolution:"Bundler",types:[],libReplacement:false},include:["src"]}));
    put("src/client.ts","export default class Client {m(){}}");put("src/props.ts","export interface Props<T>{client:T}");source();
    await run({root,put,source,options:{root,compiler,config:"tsconfig.json"}});
  } finally {rmSync(root,{recursive:true,force:true});}
}
const entry=p=>{assert(p.observations.length,JSON.stringify(p.reasons));const c=p.observations[0].nested.calls[0];assert(c,"missing nested call");assert(c.props_class,"missing Props/class provenance");return c.props_class;};
test("imported generic Props instantiate the property class with source binder identity",()=>fixture(({options})=>{
  const p=produce(options),c=entry(p);assert.equal(c.status,"observed",JSON.stringify(c));
  assert.equal(c.class_declaration.file,"project/src/client.ts");assert.equal(c.property_declarations[0].file,"project/src/props.ts");
  assert.equal(c.instantiation[0].parameter.file,"project/src/props.ts");assert.equal(c.instantiation[0].argument_declarations[0].file,"project/src/client.ts");
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
}));
test("class constructors do not acquire instance observations from shared declarations",()=>fixture(({source,options})=>{
  source("FC<Props<typeof Client>>","{client}","const callback=()=>client.toString()");
  const c=entry(produce(options));assert.equal(c.status,"unproven");assert.equal(c.class_declaration,null);
}));
test("obsolete schema two rejects before root access",()=>fixture(({options})=>{
  const p=produce(options);p.schema="prism.callable-observation/2";let reads=0;
  assert.equal(validate(JSON.stringify(p),{get root(){reads++;throw Error('old packet');}}).valid,false);assert.equal(reads,0);
}));

test("inline, alias-literal and renamed Props paths remain source-backed",()=>fixture(({put,source,options})=>{
  const cases=[
    ["export interface Props<T>{client:T}","FC<{client:Client}>","{client}","client"],
    ["export type Props<T>={client:T}","FC<Props<Client>>","{client:local}","local"],
    ["import Client from './client';export interface Props {client:Client}","FC<Props>","props","props.client"],
    ["import Client from './client';export interface Props<T=Client>{client:T}","FC<Props>","{client}","client"],
  ];
  for(const [shape,annotation,parameter,receiver] of cases){
    put("src/props.ts",shape);source(annotation,parameter,`const callback=()=>${receiver}.m()`);
    const p=produce(options),c=entry(p);assert.equal(c.status,"observed",JSON.stringify(c));
    assert.equal(c.class_declaration.file,"project/src/client.ts");assert.equal(validate(JSON.stringify(p),options).valid,true);
  }
}));

test("actual contextual parameter chooses Props rather than the first generic argument",()=>fixture(({source,options})=>{
  source("View<string,Props<Client>>","{client}","const callback=()=>client.m()","type View<Decoy,P>=(p:P)=>null;");
  const c=entry(produce(options));assert.equal(c.status,"observed");assert.equal(c.props_type,"Props<Client>");
}));

test("class and Props imported through renamed barrels retain defining files",()=>fixture(({put,source,options})=>{
  put("src/actual.ts","export default class Client{m(){}}");
  put("src/client.ts","export {default} from './actual';");
  put("src/shape.ts","export interface Original<T>{client:T}");
  put("src/props.ts","export type {Original as Props} from './shape';");
  const c=entry(produce(options));assert.equal(c.status,"observed");
  assert.equal(c.props_declarations[0].file,"project/src/shape.ts");assert.equal(c.class_declaration.file,"project/src/actual.ts");
  source("FC<Props<Client>>","{client}","const callback=()=>client.m()","namespace Decoy{export class Client{wrong(){}}}");
  assert.equal(entry(produce(options)).class_declaration.file,"project/src/actual.ts");
}));

test("unsupported Props shapes and augmentation never produce class observations",()=>fixture(({put,source,options})=>{
  const cases=[
    "export interface Props<T>{client:T}export interface Props<T>{extra?:string}",
    "interface Base<T>{client:T}export interface Props<T> extends Base<T>{}",
    "export type Props<T>={client:T}|{other:T}",
    "export type Props<T>={client:T}&{extra?:string}",
    "export type Props<T>={[K in 'client']:T}",
    "export interface Props<T>{client?:T}",
    "export interface Props<T>{get client():T}",
    "export interface Props<T>{client:T;[key:string]:T}",
    "export interface Props<T>{['client']:T}",
    "export interface Props<T>{client:T extends object?T:never}",
    "type Wrapped<T>=T extends object?T:never;export interface Props<T>{client:Wrapped<T>}",
    "export type Props<T>={client:T};export type Props<T>={client:T};",
  ];
  source();const outcomes=cases.map(shape=>{put("src/props.ts",shape);return entry(produce(options)).status;});
  assert.deepEqual(outcomes,cases.map(()=>"unproven"));
}));

test("module augmentation changes Props and invalidates the previous observation",()=>fixture(({put,options})=>{
  const a=produce(options);assert.equal(entry(a).status,"observed");
  put("src/augment.ts","import './props';declare module './props'{interface Props<T>{extra?:string}}");
  const b=produce(options);assert.equal(entry(b).status,"unproven");assert.equal(validate(JSON.stringify(a),options).valid,false);
}));

test("non-instance, generic, inherited and merged class population stays unproven",()=>fixture(({put,options})=>{
  const cases=[
    "export default interface Client{m():void}",
    "export default class Client<T=string>{m(){}}",
    "class Base{m(){}}export default class Client extends Base{}",
    "class Client{m(){}}interface Client{extra?:string}export default Client;",
    "class Client{m(){}}namespace Client{export const x=1}export default Client;",
    "type Client=any;export default Client;",
    "type Client={m():void};export default Client;",
    "class A{m(){}}class B{m(){}}type Client=A|B;export default Client;",
  ];
  const outcomes=cases.map(declaration=>{put("src/client.ts",declaration);return entry(produce(options)).status;});
  assert.deepEqual(outcomes,cases.map(()=>"unproven"));
}));

test("binding, explicit parameter, deeper paths and callable barriers remain prerequisites",()=>fixture(({source,options})=>{
  const cases=[
    ["FC<Props<Client>>","{client}:any","const callback=()=>client.m()","explicit_parameter"],
    ["FC<Props<Client>>","{client}","const callback=()=>client.m();client=new Client()","binding_unproven"],
    ["FC<Props<Client>>","{client}","const callback=(client:Client)=>client.m()","binding_unproven"],
    ["FC<Props<Client>>","{client}","const callback=()=>client.extra.m()","unsupported_path"],
    ["FC<Props<Client>> & {x?:string}","{client}","const callback=()=>client.m()","callable_unproven"],
  ];
  const outcomes=cases.map(([a,p,b])=>{source(a,p,b);return entry(produce(options)).reason;});
  assert.deepEqual(outcomes,cases.map(c=>c[3]));
}));

test("incomplete program preserves candidate anchors but cannot report observed",()=>fixture(({put,options})=>{
  put("src/unresolved.ts","import {missing} from './absent';export const x=missing;");
  const p=produce(options),c=entry(p);assert.equal(p.status,"unproven");assert.equal(c.status,"unproven");
  assert.equal(c.reason,"program_unproven");assert.equal(c.class_declaration.file,"project/src/client.ts");
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));

test("type-argument limits and new nested anchors are checked before audited I/O",()=>fixture(({put,source,options})=>{
  put("src/props.ts","export interface Props<T,U>{client:T;other?:U}");source("FC<Props<Client,string>>");
  const p=produce(options);assert.equal(entry(p).status,"observed");
  assert.equal(entry(produce({...options,limits:{props_type_args:1}})).reason,"type_argument_limit");
  let reads=0;const forbidden={get root(){reads++;throw Error("pre-I/O");}};
  for(const change of [c=>c.class_declaration.sha256="0".repeat(64),c=>c.instantiation[0].parameter.end_byte=0,
    c=>c.instantiation[0].argument_declarations[0].file="project/../escape",c=>c.reason="unsupported_class"]){
    const q=structuredClone(p);change(entry(q));assert.equal(validate(JSON.stringify(q),forbidden).valid,false);
  }
  assert.equal(reads,0);
  const q=structuredClone(p);entry(q).class_declaration=entry(q).props_declarations[0];
  assert.equal(validate(JSON.stringify(q),options).valid,false);
}));

test("defining-class A to B and missing/restoration replace identical consumer evidence",()=>fixture(({put,options})=>{
  put("src/a.ts","export default class Client{m(){}}");put("src/b.ts","export default class Client{m(){}}");
  put("src/client.ts","export {default} from './a';");const a=produce(options);assert.equal(entry(a).class_declaration.file,"project/src/a.ts");
  put("src/client.ts","export {default} from './b';");const b=produce(options);assert.equal(entry(b).class_declaration.file,"project/src/b.ts");
  assert.equal(validate(JSON.stringify(a),options).valid,false);
  put("src/client.ts","export {default} from './missing';");assert.equal(entry(produce(options)).status,"unproven");
  put("src/client.ts","export {default} from './a';");assert.equal(validate(JSON.stringify(a),options).valid,true);
}));

test("a non-class anchor in the class field is impossible before root I/O",()=>fixture(({options})=>{
  const p=produce(options);assert.equal(entry(p).status,"observed");entry(p).class_declaration=entry(p).props_declarations[0];
  let reads=0;assert.equal(validate(JSON.stringify(p),{get root(){reads++;throw Error('pre-I/O');}}).valid,false);assert.equal(reads,0);
}));

test("overloaded contextual signatures and unresolved type parameters remain unproven",()=>fixture(({source,options})=>{
  source("View","{client}","const callback=()=>client.m()","interface View{(p:Props<Client>):null;(p:Props<Client>,x?:string):null}");
  assert.equal(entry(produce(options)).reason,"signature_ambiguity");
  source("View","{client}","const callback=()=>client.m()","type View=<T extends Client>(p:Props<T>)=>null;");
  assert.equal(entry(produce(options)).status,"unproven");
}));

test("anonymous default classes and renamed direct property types retain source identity",()=>fixture(({put,source,options})=>{
  put("src/client.ts","export default class {m(){}}");
  put("src/props.ts","import type Renamed from './client';export interface Props {readonly client:Renamed;other?:string}");
  source("FC<Props>");const c=entry(produce(options));assert.equal(c.status,"observed");assert.equal(c.class_declaration.file,"project/src/client.ts");
}));

test("a forged program completion cannot promote an incomplete class observation",()=>fixture(({put,options})=>{
  put("src/missing.ts","import 'not-installed';");const p=produce(options),c=entry(p);
  assert.equal(c.reason,"program_unproven");c.status="observed";c.reason=null;
  let reads=0;assert.equal(validate(JSON.stringify(p),{get root(){reads++;throw Error('pre-I/O');}}).valid,false);assert.equal(reads,0);
}));
