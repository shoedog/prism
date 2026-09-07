import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,cpSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
// Optional explicit predecessor module supports same-environment behavioral controls.
const {produce,validate}=await import(process.env.PRISM_OBSERVER_MODULE??"./index.mjs");
const compiler=process.env.PRISM_TYPESCRIPT,profiles=process.env.PRISM_CALLABLE_PROFILES;
assert(compiler && profiles,"explicit pinned compiler and profiles required");
async function fixture(run) {
  const root=mkdtempSync(path.join(tmpdir(),"prism-nested-test-"));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const source=(body,parameter="{client}")=>put("src/app.ts",`import type {FC} from 'react';class Client {m(){}}\nexport const run:FC<{client:Client}>=(${parameter})=>{${body};return null;};`);
  try {
    cpSync(path.join(profiles,"react19/node_modules"),path.join(root,"node_modules"),{recursive:true});
    put("package.json",'{"type":"module"}');
    put("tsconfig.json",JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:"ES2022",module:"ESNext",moduleResolution:"Bundler",types:[],libReplacement:false},include:["src"]}));
    source("const callback=()=>client.m()");
    await run({source,put,options:{root,compiler,config:"tsconfig.json"}});
  } finally {rmSync(root,{recursive:true,force:true});}
}
const nested=p=>{assert(p.observations.length,JSON.stringify(p.reasons));assert(p.observations[0].nested,"missing nested observations");return p.observations[0].nested;};

test("nested capture retains the outer parameter binding and callback anchors",()=>fixture(({options})=>{
  const p=produce(options),n=nested(p);assert.equal(n.calls.length,1);
  assert.equal(n.calls[0].binding.status,"linked");
  assert.equal(n.calls[0].binding.parameter.kind,"Parameter");
  assert.equal(n.calls[0].binding.declarations[0].kind,"BindingElement");
  assert.equal(n.calls[0].functions.length,1);assert.deepEqual(n.barriers,[]);
  assert.deepEqual(p.observations[0].calls,[]);assert.equal(p.authorizes_runtime_edge,false);
}));
test("nested captures with direct writes are explicitly unproven",()=>fixture(({source,options})=>{
  source("const callback=()=>client.m();client=new Client()");
  const b=nested(produce(options)).calls[0].binding;
  assert.equal(b.status,"unproven");assert.equal(b.reason,"write_barrier");assert.equal(b.writes.length,1);
}));
test("schema one rejects before caller-root access after nested schema transition",()=>fixture(({options})=>{
  const p=produce(options);p.schema="prism.callable-observation/1";
  let reads=0;assert.equal(validate(JSON.stringify(p),{get root(){reads++;throw Error("old packet");}}).valid,false);
  assert.equal(reads,0);
}));

test("renamed and property-chain captures retain parameter identity across callback depths",()=>fixture(({source,options})=>{
  for(const [parameter,body,kind,depth] of [
    ["{client:local}","const callback=()=>local.m()","BindingElement",1],
    ["props","const callback=function(){return ()=>props.client.m()}","Parameter",2],
    ["{client}:any","const callback=async()=>client.m()","BindingElement",1],
  ]) {
    source(body,parameter);const p=produce(options),c=nested(p).calls[0];
    assert.equal(c.binding.status,"linked");assert.equal(c.binding.declarations[0].kind,kind);
    assert.equal(c.functions.length,depth);assert.equal(validate(JSON.stringify(p),options).valid,true);
    assert.equal(p.scope.class_authority,false);
  }
}));

test("shadow and foreign bindings never link by spelling or receiver type",()=>fixture(({source,options})=>{
  const cases=["const callback=(client:Client)=>client.m()",
    "const callback=()=>{let client=new Client();client.m()}",
    "const other=client;const callback=()=>other.m()",
    "const callback=()=>{try{}catch(client){client.m()}}",
    "const callback=()=>{for(const client of [new Client()])client.m()}"];
  const outcomes=cases.map(body=>{source(body);return nested(produce(options)).calls[0].binding.reason;});
  assert.deepEqual(outcomes,cases.map(()=>"other_binding"));
}));

test("duplicate outer bindings remain unproven under compiler error recovery",()=>fixture(({source,options})=>{
  const cases=["var client;const callback=()=>client.m()",
    "if(false){var client=new Client()}const callback=()=>client.m()",
    "let client;const callback=()=>client.m()"];
  const outcomes=cases.map(body=>{source(body);return nested(produce(options)).calls[0].binding.status;});
  source("const callback=()=>client.m()","{client},client:any");outcomes.push(nested(produce(options)).calls[0].binding.status);
  assert.deepEqual(outcomes,["unproven","unproven","unproven","unproven"]);
}));

test("direct assignment target population keeps whole-body write barriers",()=>fixture(({source,options})=>{
  const cases=["client=new Client()","client ||= new Client()","client &&= new Client()","client ??= new Client()",
    "client += 1","client++","--client","delete client.m","client.m=()=>{}","client['m']=()=>{}",
    "({client}={client:new Client()})","({x:client}={x:new Client()})","[client]=[new Client()]",
    "[...client]=[]","({...client}={})","({client=new Client()}={})",
    "for(client of [new Client()]){}","for(client in {}){}",
    "const mutate=()=>{client=new Client()}","const mutate=(unused=(client=new Client()))=>{}",
    "(client as any)=new Client()"];
  const outcomes=cases.map(write=>{source(`const callback=()=>client.m();${write}`);const b=nested(produce(options)).calls[0].binding;return {write,reason:b.reason,writes:b.writes.length>0};});
  assert.deepEqual(outcomes,cases.map(write=>({write,reason:"write_barrier",writes:true})));
}));

test("writes to distinct shadow bindings and computed read keys do not poison captures",()=>fixture(({source,options})=>{
  source("const callback=()=>client.m();const mutate=(client:Client)=>{client=new Client()};let other;({[client.m()]:other}={})");
  assert.equal(nested(produce(options)).calls[0].binding.status,"linked");
}));

test("unsupported parameters and receiver forms cannot become links",()=>fixture(({source,options})=>{
  const cases=[
    ["{client=new Client()}","client.m()"],["{...client}","client.m()"],
    ["{nested:{client}}","client.m()"],["[client]","client.m()"],
    ["{client}={client:new Client()}","client.m()"],["{['client']:client}","client.m()"],
    ["{client}","this.m()"],["{client}","new Client().m()"],
    ["props","props['client'].m()"],["props","props?.client.m()"],
    ["{client}","client?.m()"],["{client}","client.m?.()"],
    ["{client}","missing.m()"]];
  const outcomes=cases.map(([parameter,call])=>{source(`const callback=()=>${call}`,parameter);return nested(produce(options)).calls[0].binding.status;});
  assert.deepEqual(outcomes,cases.map(()=>"unproven"));
}));

test("unsupported callable scopes and nested budgets remain explicit",()=>fixture(({source,options})=>{
  source("function named(){client.m()}class Other{m(){client.m()}}const object={m(){client.m()}};const callback=()=>()=>client.m()");
  const p=produce(options),n=nested(p);assert.equal(n.calls.length,1);
  assert.deepEqual(n.barriers.map(b=>b.reason),["unsupported_scope","unsupported_scope","unsupported_scope"]);
  const depth=nested(produce({...options,limits:{nested_depth:1}}));
  assert.deepEqual(depth.calls,[]);assert.equal(depth.barriers.at(-1).reason,"depth_limit");
  source("const callback=()=>{client.m();client.m()}");
  const calls=nested(produce({...options,limits:{nested_calls:1}}));
  assert.equal(calls.calls.length,1);assert.equal(calls.barriers[0].reason,"call_limit");
}));

test("nested anchors and impossible statuses reject pre-I/O; plausible forgery rejects recomputation",()=>fixture(({options})=>{
  const p=produce(options);let reads=0;const forbidden={get root(){reads++;throw Error("pre-I/O");}};
  for(const change of [c=>c.binding.use.sha256="0".repeat(64),c=>c.functions[0].end_byte=0,
    c=>c.binding.declarations[0].file="project/../outside",c=>c.binding.reason="write_barrier",c=>c.functions=[]]) {
    const q=structuredClone(p);change(nested(q).calls[0]);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);
  }
  assert.equal(reads,0);
  const q=structuredClone(p);nested(q).calls[0].binding.declarations[0]=q.observations[0].parameter;
  assert.equal(validate(JSON.stringify(q),options).valid,false);
}));

test("write and binding changes replace the packet and restore without stale unions",()=>fixture(({source,options})=>{
  const a=produce(options);assert.equal(nested(a).calls[0].binding.status,"linked");
  source("const callback=()=>client.m();client=new Client()");const b=produce(options);
  assert.equal(nested(b).calls[0].binding.reason,"write_barrier");assert.equal(validate(JSON.stringify(a),options).valid,false);
  const forged=structuredClone(b),binding=nested(forged).calls[0].binding;
  binding.writes=[];binding.status="linked";binding.reason=null;
  assert.equal(validate(JSON.stringify(forged),options).valid,false);
  source("const callback=(client:Client)=>client.m()");assert.equal(validate(JSON.stringify(b),options).valid,false);
  source("const callback=()=>client.m()");assert.equal(validate(JSON.stringify(a),options).valid,true);
}));

test("direct-body inventory excludes class fields and static blocks",()=>fixture(({source,options})=>{
  source("class Other{field=client.m();static {client.m()}};const Other2=class {field=client.m()}");
  const p=produce(options);assert.equal(p.observations.length,1);
  assert.deepEqual(p.observations[0].calls,[],"class initialization is not the annotated implementation's direct body");
}));

test("nested JSX callbacks and named function-expression shadows keep lexical scope",()=>fixture(({put,options})=>{
  put("src/app.ts","export const unused=1;");
  put("src/component.tsx","import type {FC} from 'react';class Client {m(){}}export const run:FC<{client:Client}>=({client})=>{const callback=function client(){client.m()};return <button onClick={()=>client.m()}/>};");
  const n=nested(produce(options));assert.equal(n.calls.length,2);
  assert.equal(n.calls[0].binding.reason,"other_binding");assert.equal(n.calls[1].binding.status,"linked");
}));

test("class effects and property-chain writes retain direct syntactic barriers",()=>fixture(({source,options})=>{
  source("class Other {m(){client=new Client()}}const callback=()=>client.m()");
  assert.equal(nested(produce(options)).calls[0].binding.reason,"write_barrier");
  source("const callback=()=>props.client.m();props['client']=new Client()","props");
  assert.equal(nested(produce(options)).calls[0].binding.reason,"write_barrier");
}));

test("scope-limit barriers cannot be removed by a well-shaped forged packet",()=>fixture(({source,options})=>{
  source("const callback=()=>()=>client.m()");const limited={...options,limits:{nested_depth:1}},p=produce(limited);
  assert.equal(nested(p).barriers[0].reason,"depth_limit");
  nested(p).barriers=[];assert.equal(validate(JSON.stringify(p),limited).valid,false);
}));
