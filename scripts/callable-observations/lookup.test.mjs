import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
import {produce,validate} from "./index.mjs";
import {hash,relative} from "./schema.mjs";
const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,"PRISM_TYPESCRIPT must name the pinned compiler");
async function fixture(run){
  const root=mkdtempSync(path.join(tmpdir(),"prism-lookup-test-"));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const source=extra=>put("src/app.ts",`${extra}\nclass Client{m(){}}interface Props{client:Client}type View<P>=(props:P)=>void;export const run:View<Props>=({client})=>{const callback=()=>client.m();};`);
  try{
    put("package.json",'{"type":"module"}');
    put("tsconfig.json",JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:"ES2022",module:"ESNext",moduleResolution:"Bundler",baseUrl:".",types:[],libReplacement:false},include:["src"]}));
    source("");await run({root,put,source,options:{root,compiler,config:"tsconfig.json"}});
  }finally{rmSync(root,{recursive:true,force:true});}
}
const candidate=p=>p.observations[0]?.nested.calls[0]?.props_class;
for(const specifier of ["node:url","virtual:pwa-register"]){
  test(`refused ${specifier} lookup preserves partial observations and blocks closure`,()=>fixture(({source,options})=>{
    source(`import '${specifier}';`);const p=produce(options);
    assert.equal(p.observations.length,1,JSON.stringify(p.reasons));
    assert.equal(p.status,"unproven");assert(p.reasons.includes("unsupported_lookup"));
    assert(p.snapshot.refused_lookup_sha256.includes(hash(`/__prism__/project/${specifier}`)));
    assert(p.snapshot.failed_lookups.every(relative));assert.equal(p.closure.dependencies,false);
    assert.equal(candidate(p).reason,"program_unproven");assert(candidate(p).class_declaration);
    assert.equal(validate(JSON.stringify(p),options).valid,true);
  }));
}
test("ordinary missing imports retain observations without refusal evidence",()=>fixture(({source,options})=>{
  source("import './missing';");const p=produce(options);assert.equal(p.observations.length,1);
  assert(p.reasons.includes("unresolved_module"));assert(!p.reasons.includes("worker_failed"));
  assert.deepEqual(p.snapshot.refused_lookup_sha256,[]);
}));
test("closed safe program keeps observed class and empty refusal evidence",()=>fixture(({options})=>{
  const p=produce(options);assert.equal(p.status,"observed",JSON.stringify(p.reasons));
  assert.equal(candidate(p).status,"observed");assert.deepEqual(p.snapshot.refused_lookup_sha256,[]);
}));
test("obsolete schema three rejects before audited-root access",()=>fixture(({options})=>{
  const p=produce(options);p.schema="prism.callable-observation/3";let reads=0;
  assert.equal(validate(JSON.stringify(p),{get root(){reads++;throw Error('forbidden');}}).valid,false);assert.equal(reads,0);
}));

test("ambient declarations cannot mask refused-lookup closure",()=>fixture(({put,source,options})=>{
  put("src/ambient.d.ts","declare module 'virtual:pwa-register' {export const value:number;}");
  source("import {value} from 'virtual:pwa-register';export const result=value;");
  const p=produce(options);assert.equal(p.observations.length,1);assert.deepEqual(p.diagnostics,[]);
  assert(p.reasons.includes("unsupported_lookup"));assert.equal(p.closure.dependencies,false);
  assert.equal(p.closure.augmentation,false);assert.equal(p.closure.resolution,false);
  assert.equal(candidate(p).reason,"program_unproven");
}));

test("refusals are sorted unique opaque digests and cannot inhabit path fields",()=>fixture(({source,options})=>{
  source("import 'node:url';import 'virtual:pwa-register';import 'node:url';");const p=produce(options);
  const entries=p.snapshot.refused_lookup_sha256;assert(entries.length>1);
  assert.deepEqual(entries,[...new Set(entries)].sort());
  let reads=0;const forbidden={get root(){reads++;throw Error('forbidden');}};
  const mutations=[
    q=>q.snapshot.refused_lookup_sha256.push(entries[0]),
    q=>q.snapshot.refused_lookup_sha256.reverse(),
    q=>q.snapshot.refused_lookup_sha256[0]="project/node:url",
    q=>q.snapshot.refused_lookup_sha256[0]="A".repeat(64),
    q=>q.snapshot.refused_lookup_sha256=[],
    q=>q.reasons=q.reasons.filter(r=>r!=="unsupported_lookup"),
    q=>q.snapshot.failed_lookups.push("project/node:url"),
    ...["dependencies","augmentation","resolution"].map(k=>q=>q.closure[k]=true),
  ];
  for(const mutate of mutations){const q=structuredClone(p);mutate(q);assert.equal(validate(JSON.stringify(q),forbidden).valid,false);}
  assert.equal(reads,0);
}));

test("removed or forged refusal evidence fails full recomputation",()=>fixture(({source,options})=>{
  source("import 'node:url';");const p=produce(options);assert.equal(p.observations.length,1);
  const removed=structuredClone(p);removed.snapshot.refused_lookup_sha256=[];
  removed.reasons=removed.reasons.filter(r=>r!=="unsupported_lookup");
  assert.equal(validate(JSON.stringify(removed),options).reason,"stale_or_tampered");
  const forged=structuredClone(p);forged.snapshot.refused_lookup_sha256=["0".repeat(64)];
  assert.equal(validate(JSON.stringify(forged),options).reason,"stale_or_tampered");
}));

test("changing and restoring imports replaces refusal evidence",()=>fixture(({source,options})=>{
  source("import 'node:url';");const a=produce(options);assert.equal(a.observations.length,1);
  source("import 'virtual:pwa-register';");const b=produce(options);
  assert.notDeepEqual(a.snapshot.refused_lookup_sha256,b.snapshot.refused_lookup_sha256);
  assert.equal(validate(JSON.stringify(a),options).valid,false);
  source("");const clean=produce(options);assert.equal(clean.status,"observed");
  assert.deepEqual(clean.snapshot.refused_lookup_sha256,[]);assert.equal(candidate(clean).status,"observed");
  source("import 'node:url';");assert.equal(validate(JSON.stringify(a),options).valid,true);
}));

test("actual colon filenames still fail acquisition rather than becoming lookup evidence",()=>fixture(({put,options})=>{
  put("src/virtual:input.ts","export const value=1;");const p=produce(options);
  assert.deepEqual(p.reasons,["unsupported_input"]);assert.deepEqual(p.snapshot.refused_lookup_sha256,[]);
  assert.equal(p.observations.length,0);
}));

test("outside and git lookup boundaries remain distinct from refused IDs",()=>fixture(({source,options})=>{
  source("import '../../../outside';");const p=produce(options);
  assert(p.reasons.includes("outside_lookup"));assert.equal(p.snapshot.outside_lookups,true);
  assert.deepEqual(p.snapshot.refused_lookup_sha256,[]);
  source("import '../.git/secret';");assert.deepEqual(produce(options).reasons,["unsupported_input"]);
}));

test("control-character and overlength probes cannot corrupt path arrays",()=>fixture(({source,options})=>{
  for(const specifier of ["virtual\u0001input","v".repeat(4100)]){
    source(`import ${JSON.stringify(specifier)};`);const p=produce(options);
    assert.equal(p.observations.length,1,JSON.stringify(p.reasons));assert(p.reasons.includes("unsupported_lookup"));
    assert(p.snapshot.failed_lookups.every(relative));assert.equal(validate(JSON.stringify(p),options).valid,true);
  }
}));

for(const refusedFirst of [false,true]){
  test(`safe paths mapping does not blacklist virtual specifiers; refused fallback=${refusedFirst}`,()=>fixture(({put,source,options})=>{
    put("src/value.ts","export const value=1;");
    put("tsconfig.json",JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:"ES2022",module:"ESNext",moduleResolution:"Bundler",baseUrl:".",types:[],libReplacement:false,
      paths:{"virtual:input":refusedFirst?["virtual:absent","src/value.ts"]:["src/value.ts"]}},include:["src"]}));
    source("import {value} from 'virtual:input';export const result=value;");const p=produce(options);
    assert.deepEqual(p.diagnostics,[]);assert(!p.reasons.includes("unresolved_module"));
    assert(p.resolutions.some(r=>r.specifier==="virtual:input" && r.target==="project/src/value.ts"));
    assert.equal(p.status,refusedFirst?"unproven":"observed");
    assert.equal(p.closure.dependencies,!refusedFirst);assert.equal(p.reasons.includes("unsupported_lookup"),refusedFirst);
    assert.equal(candidate(p).status,refusedFirst?"unproven":"observed");assert.equal(validate(JSON.stringify(p),options).valid,true);
  }));
}
