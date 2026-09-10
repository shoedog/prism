import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,symlinkSync,chmodSync,existsSync} from 'node:fs';
import path from 'node:path';
import {tmpdir} from 'node:os';
import {deriveMembership} from './membership-worker.mjs';
import {observeMembership,validateMembership,parseMembership} from './membership.mjs';
import {produce} from './index.mjs';
import {spawnSync} from 'node:child_process';
const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'explicit pinned compiler required');
const native=process.env.PRISM_MEMBERSHIP_NATIVE;
assert(native,'explicit freshly built native census required');
const corpus=JSON.parse(readFileSync(new URL('../../docs/eval/receiver-closure/executable-owner-fixtures.json',import.meta.url)));
function fixture(extension,run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-membership-test-'));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  for(const [file,text] of Object.entries(corpus.files))put(file==='src/app.ts'?`src/app.${extension}`:file,text);
  put('package.json','{"type":"module"}');
  put('base.json',JSON.stringify({compilerOptions:corpus.compiler_options}));
  put('tsconfig.json',JSON.stringify({extends:'./base.json',include:['src']}));
  const options={root,compiler,config:'tsconfig.json'};
  try{return run(options,put);}finally{rmSync(root,{recursive:true,force:true});}
}
for(const ext of ['ts','tsx'])test(`genuine direct ${ext} fixture exposes complete Program facets and selection provenance`,()=>fixture(ext,options=>{
  const r=deriveMembership(options);
  assert.equal(r.packet.compiler.verified,true);
  assert.equal(r.packet.closure.stable_snapshot,true);
  assert(Array.isArray(r.program),'new Program facets must be observed, not absent');
  assert.deepEqual(r.program.map(f=>f.id),r.packet.snapshot.program_files);
  assert(r.program.some(f=>f.id===`project/src/app.${ext}`&&f.domain==='repository'&&!f.declaration));
  assert(r.selection.some(f=>f.file==='project/tsconfig.json'&&f.fields.some(p=>p.name==='include')));
}));

const observe=options=>{const r=observeMembership(options,native);assert.equal(r.status,'observed',JSON.stringify(r));
  assert.equal(r.authorizes_runtime_edge,false);return r;};
for(const ext of ['ts','tsx'])test(`full compiler/native/compiler ${ext} keeps old packet and loaded census`,()=>fixture(ext,options=>{
  const r=observe(options),p=r.payload;
  assert.deepEqual(p.packet,produce(options));assert.equal(p.native_status,'observed');
  assert.deepEqual(p.differences.native_not_program,[]);
  assert(p.program.find(f=>f.id===`project/src/app.${ext}`).native_member);
  assert.equal(validateMembership(JSON.stringify(r),options,native).valid,true);
}));
test('whole native census retains excluded Bash and Terraform outside Program',()=>fixture('ts',(options,put)=>{
  put('outside/run.sh','echo hello\n');put('outside/main.tf','variable "x" { type = string }\n');
  const p=observe(options).payload;
  assert.deepEqual(p.differences.native_not_program,['project/outside/main.tf','project/outside/run.sh']);
  assert(p.native.files.some(f=>f.language==='Terraform'));
}));
test('parent config and sibling import remain inside immutable audit root',()=>fixture('ts',(options,put)=>{
  put('pkg/tsconfig.json',JSON.stringify({extends:'../base.json',include:['src'],compilerOptions:{baseUrl:'..',paths:{sibling:['sibling/value.ts']}}}));
  put('pkg/src/use.ts',"import {value} from 'sibling'; export {value};\n");put('sibling/value.ts','export const value = 1;\n');
  const p=observe({...options,config:'pkg/tsconfig.json'}).payload;
  assert.deepEqual(p.selection.map(s=>s.file),['project/pkg/tsconfig.json','project/base.json']);
  assert(p.program.some(f=>f.id==='project/sibling/value.ts'&&!f.root));
  assert.equal(observeMembership({...options,root:path.join(options.root,'pkg')},native).status,'unavailable');
}));
test('exclude never drops imported source or its changed hash',()=>fixture('ts',(options,put)=>{
  put('tsconfig.json',JSON.stringify({extends:'./base.json',include:['src'],exclude:['src/excluded.ts']}));
  put('src/extra.ts',"import {value} from './excluded'; export {value};\n");put('src/excluded.ts','export const value = 1;');
  const a=observe(options);assert(a.payload.program.some(f=>f.id==='project/src/excluded.ts'&&!f.root));
  put('src/excluded.ts','export const value = 2;');assert.equal(validateMembership(JSON.stringify(a),options,native).valid,false);
}));
test('same-named dependency declarations and JSON/mts stay distinct from native bodies',()=>fixture('ts',(options,put)=>{
  put('src/use.ts',"import type {Client} from 'dep'; import data from './data.json'; import {x} from './other.mjs'; export {data,x}; export type {Client};");
  put('src/data.json','{"x":1}');put('src/other.mts','export const x=1;');
  put('node_modules/dep/package.json','{"name":"dep","types":"index.d.ts"}');
  put('node_modules/dep/index.d.ts','export declare class Client { m(): number; }');
  put('base.json',JSON.stringify({compilerOptions:{...corpus.compiler_options,resolveJsonModule:true,allowSyntheticDefaultImports:true}}));
  const p=observe(options).payload;
  assert(p.program.some(f=>f.id==='project/node_modules/dep/index.d.ts'&&f.declaration&&f.domain==='dependency'&&!f.native_member));
  assert(p.program.some(f=>f.id==='project/src/client.ts'&&!f.declaration&&f.domain==='repository'&&f.native_member));
  for(const name of ['data.json','other.mts'])assert(p.program.some(f=>f.id==='project/src/'+name&&f.native_language===null&&!f.native_member));
  assert(p.program.find(f=>f.id==='project/src/data.json').json);
}));
for(const origin of ['automatic','configured','source'])test(`${origin} type/lib obligations remain incomplete without authority`,()=>fixture('ts',(options,put)=>{
  const compilerOptions={...corpus.compiler_options};
  if(origin==='automatic')delete compilerOptions.types;
  if(origin==='configured')compilerOptions.types=['react-scripts'];
  if(origin==='source')put('src/ref.d.ts','/// <reference types="react-scripts" />\n');
  put('base.json',JSON.stringify({compilerOptions}));
  const p=observe(options).payload;
  assert.equal(p.packet.semantic_closure.complete,false);
  assert(p.program.some(f=>f.domain==='compiler'&&f.declaration));
  if(origin==='source')assert(p.packet.type_lib_references.some(r=>r.name==='react-scripts'&&r.status==='unproven'));
}));
test('ambient/prototype effects remain in dependency/sibling evidence after classification',()=>fixture('ts',(options,put)=>{
  put('src/extra.ts',"import 'dep'; import '../sibling/effect';");
  put('sibling/effect.ts',"import {Client} from '../src/client'; Client.prototype.m=()=>2;\n");
  put('node_modules/dep/package.json','{"types":"index.d.ts"}');put('node_modules/dep/index.d.ts','export {}; declare global { interface Window { extra: number } }');
  const a=observe(options);
  assert(a.payload.program.some(f=>f.id==='project/sibling/effect.ts'));
  assert(a.payload.program.some(f=>f.id==='project/node_modules/dep/index.d.ts'&&f.declaration));
  put('node_modules/dep/index.d.ts','export {}; declare global { interface Window { extra: string } }');
  assert.equal(validateMembership(JSON.stringify(a),options,native).valid,false);
}));
test('in-root link aliases are observed but default rejection and escape/cycle remain',()=>fixture('ts',(options,put)=>{
  symlinkSync('src/client.ts',path.join(options.root,'alias.ts'));
  assert.equal(observeMembership(options,native).status,'unavailable');
  const p=observe({...options,links:'in-root'}).payload;
  assert(p.program.find(f=>f.id==='project/src/client.ts').inventory_aliases.includes('project/alias.ts'));
  assert.equal(p.native_status,'incomplete');
  put('base.json',JSON.stringify({compilerOptions:{...corpus.compiler_options,preserveSymlinks:true}}));
  assert.equal(observeMembership({...options,links:'in-root'},native).status,'unavailable');
  rmSync(path.join(options.root,'alias.ts'));symlinkSync('../outside.ts',path.join(options.root,'alias.ts'));
  assert.equal(observeMembership({...options,links:'in-root'},native).status,'unavailable');
  rmSync(path.join(options.root,'alias.ts'));symlinkSync('alias.ts',path.join(options.root,'alias.ts'));
  assert.equal(observeMembership({...options,links:'in-root'},native).status,'unavailable');
}));
for(const config of [
  '{"extends":"./base.json","include":["src"],"include":["other"]}',
  '{"extends":"./base.json","compilerOptions":{"paths":{"x":["a"],"x":["b"]}},"include":["src"]}',
  '{"extends":"./missing.json","include":["src"]}',
  '{"extends":"./base.json","references":[{"path":"./other"}],"include":["src"]}',
  '{"extends":"./base.json","compilerOptions":{"plugins":[{"name":"untrusted"}]},"include":["src"]}',
  '{"extends":"./base.json","include":["${configDir}/src"]}',
])test(`unsupported/ambiguous config unavailable: ${config}`,()=>fixture('ts',(options,put)=>{
  put('tsconfig.json',config);const r=observeMembership(options,native);assert.equal(r.status,'unavailable');assert.equal(r.payload,null);
}));
test('parse errors and skipped invalid UTF8 are explicit incomplete census',()=>fixture('ts',(options,put)=>{
  put('outside/broken.ts','export const broken = ;\n');put('outside/bytes.ts',Buffer.from([0xff]));
  const p=observe(options).payload;assert.equal(p.native_status,'incomplete');
  assert(p.native.skipped.some(f=>f.reason==='NotUtf8'));
  assert(p.native.files.some(f=>f.parse_errors>0)||p.native.skipped.some(f=>f.reason==='ParseFailed'));
}));
test('resource refusal and unavailable artifacts never validate as empty membership',()=>fixture('ts',options=>{
  for(const limits of [{files:1},{timeout_ms:1}]) {
    const r=observeMembership({...options,limits},native);assert.equal(r.status,'unavailable');assert.equal(r.payload,null);
    assert.equal(validateMembership(JSON.stringify(r),{...options,limits},native).valid,false);
  }
  assert.equal(observeMembership(options,'/nonexistent/native').status,'unavailable');
}));
test('malformed artifacts reject before audited-root access',()=>{
  let reads=0;assert.equal(validateMembership('{"schema":"wrong"}',{get root(){reads++;throw Error();}},native).valid,false);assert.equal(reads,0);
});
test('missing/extra/duplicate/relabeled members and genuine config/epoch substitutions refuse',()=>fixture('ts',(options,put)=>{
  const a=observe(options);
  for(const mutate of [
    r=>r.payload.program.pop(),r=>r.payload.program.push(r.payload.program[0]),
    r=>r.payload.program[0].id='project/not-there.ts',r=>r.payload.program[0].domain='repository',
    r=>r.authorizes_runtime_edge=true,r=>r.payload.program[0].sha256='0'.repeat(64),
  ]) {const altered=structuredClone(a);mutate(altered);assert.throws(()=>parseMembership(JSON.stringify(altered)));}
  put('second.json',JSON.stringify({extends:'./base.json',files:['src/client.ts']}));
  const b=observe({...options,config:'second.json'});
  const c=structuredClone(a);c.payload.selection=b.payload.selection;
  assert.equal(validateMembership(JSON.stringify(c),options,native).valid,false);
  const before=readFileSync(path.join(options.root,'src/client.ts'),'utf8');put('src/client.ts',before+'\n');
  const d=observe(options);const swapped=structuredClone(a);swapped.payload.identity=d.payload.identity;swapped.payload.packet.snapshot=d.payload.packet.snapshot;
  assert.equal(validateMembership(JSON.stringify(swapped),options,native).valid,false);
  put('src/client.ts',before);
  // Identical bytes may reproduce observations, never a live Rust proof epoch.
  assert.equal(validateMembership(JSON.stringify(a),options,native).valid,false,'second config remains an extra audited file');
  rmSync(path.join(options.root,'second.json'));
  assert.equal(validateMembership(JSON.stringify(a),options,native).valid,true,'same bytes reproduce observations, not runtime proof epochs');
}));
test('CLI enforces non-authorizing envelope and strict option selection',()=>fixture('ts',options=>{
  const r=spawnSync(process.execPath,['scripts/callable-observations/membership.mjs','observe',compiler,options.root,options.config,native],{encoding:'utf8'});
  assert.equal(r.status,0,r.stderr);assert.equal(parseMembership(r.stdout).authorizes_runtime_edge,false);
  assert.equal(observeMembership({...options,profile:'unbounded'},native).reason,'invalid_options');
}));
test('Unicode native ordering matches the complete compiler identity census',()=>fixture('ts',(options,put)=>{
  put('src/\ue000.ts','export const x=1;');put('src/\u{10000}.ts','export const y=2;');
  const p=observe(options).payload;
  assert.deepEqual(p.native.files.map(f=>f.id),p.native.files.map(f=>f.id).sort());
  assert.deepEqual(p.native.supported.map(f=>f.id),p.packet.snapshot.program_files);
}));
test('explicit symlink root uses the same canonical identity as the existing observer',()=>fixture('ts',(options,put)=>{
  symlinkSync('src/client.ts',path.join(options.root,'alias.ts'));
  put('tsconfig.json',JSON.stringify({extends:'./base.json',files:['alias.ts']}));
  const input={...options,links:'in-root'},old=produce(input);
  assert(old.compiler.verified&&old.closure.stable_snapshot);
  assert(old.snapshot.program_files.includes('project/src/client.ts'));
  const p=observe(input).payload;
  assert.deepEqual(p.packet,old);assert(p.program.some(f=>f.id==='project/src/client.ts'&&f.root));
}));
test('case-alternate root follows measured compiler filesystem policy',()=>fixture('ts',(options,put)=>{
  put('tsconfig.json',JSON.stringify({extends:'./base.json',files:['src/CLIENT.ts']}));
  const old=produce(options),r=observeMembership(options,native);
  if(old.scope.case_sensitive)assert.equal(r.status,'unavailable');
  else {assert.equal(r.status,'observed',JSON.stringify(r));assert.deepEqual(r.payload.packet,old);
    assert(r.payload.program.some(f=>f.id==='project/src/client.ts'&&f.root));}
}));
test('well-shaped compiler facets and native producer identity cannot be counterfeited',()=>fixture('ts',options=>{
  const a=observe(options);
  for(const mutate of [r=>r.payload.program[0].declaration=!r.payload.program[0].declaration,
    r=>r.payload.identity.native_sha256='0'.repeat(64)]) {
    const altered=structuredClone(a);mutate(altered);parseMembership(JSON.stringify(altered));
    assert.equal(validateMembership(JSON.stringify(altered),options,native).valid,false);
  }
}));
test('native process failure is unavailable, never supplied empty census',()=>fixture('ts',options=>{
  assert.equal(observeMembership(options,process.execPath).reason,'native_failed');
}));
for(const mode of ['root_change','native_change','output_cap'])test(`native sandwich refuses ${mode}`,()=>fixture('ts',options=>{
  const harness=mkdtempSync(path.join(tmpdir(),'prism-membership-driver-'));
  const driver=path.join(harness,'driver.mjs');
  const diagnostic=path.join(harness,'native-result.json');
  const mutation=mode==='root_change'?`fs.writeFileSync(${JSON.stringify(path.join(options.root,'outside.ts'))},'export const changed = 1;');`
    :mode==='native_change'?`fs.appendFileSync(${JSON.stringify(driver)},${JSON.stringify('\n// changed')});`:'';
  const source=`#!/usr/bin/env node\nimport fs from 'node:fs'; import {spawnSync} from 'node:child_process';\n`+
    (mode==='output_cap'?`process.stdout.write(' '.repeat(8*1024*1024+65536));`
      :`const r=spawnSync(${JSON.stringify(native)},[process.argv[2]],{input:fs.readFileSync(0),encoding:'utf8'}); fs.writeFileSync(${JSON.stringify(diagnostic)},JSON.stringify({status:r.status,error:r.error?.message,stderr:r.stderr})); if(r.status!==0)process.exit(1); ${mutation} process.stdout.write(r.stdout);`);
  writeFileSync(driver,source);chmodSync(driver,0o755);
  try {const syntax=spawnSync(process.execPath,['--check',driver],{encoding:'utf8'});assert.equal(syntax.status,0,syntax.stderr);
    const r=observeMembership(options,driver);assert.equal(r.reason,{root_change:'unstable_snapshot',native_change:'native_identity',output_cap:'budget_exceeded'}[mode],existsSync(diagnostic)?readFileSync(diagnostic,'utf8'):'driver did not return a native result');}
  finally {rmSync(harness,{recursive:true,force:true});}
}));
