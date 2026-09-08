import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {pathToFileURL} from 'node:url';

const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION;
const moduleURL=file=>implementation?pathToFileURL(path.join(implementation,file)):new URL(file,import.meta.url);
const {produce,validate}=await import(moduleURL('index.mjs'));
const {parsePacket}=await import(moduleURL('schema.mjs'));
const compiler=process.env.PRISM_TYPESCRIPT,profiles=process.env.PRISM_CALLABLE_PROFILES;
assert(compiler&&profiles,'pinned compiler and profiles are required');

function fixture(run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-search-provenance-'));
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  const options={root,compiler,config:'tsconfig.json'};
  try {
    put('tsconfig.json',JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Bundler',types:[],libReplacement:false},include:['src']}));
    put('package.json','{"type":"module"}');
    put('src/dep.ts','export const value=1;');
    put('src/app.ts',"import {value} from './dep'; export const run=value;");
    return run({put,options});
  } finally {rmSync(root,{recursive:true,force:true});}
}

test('module lookup callbacks publish a request ledger and reproduce it',()=>fixture(({options})=>{
  const packet=produce(options);
  // This is intentional RED against S1: the Program completes, but has no ledger.
  assert(Array.isArray(packet.search_provenance?.module_requests),'missing module-request ledger');
  assert(Array.isArray(packet.search_provenance?.boundary_events),'missing boundary-event ledger');
  assert(packet.search_provenance.module_requests.length>0,'missing actual module lookup callback');
  assert.equal(validate(JSON.stringify(packet),options).valid,true);
}));

test('ledger ids and resolution membership are structurally closed before root I/O',()=>fixture(({options})=>{
  const packet=produce(options),forbidden={get root(){throw Error('must not read root');}};
  const mutations=[p=>p.search_provenance.module_requests[0].id=1,
    p=>p.search_provenance.module_requests[0].target='project/not-selected.ts',
    p=>p.search_provenance.boundary_events.push({id:0,kind:'outside',operation:'invalid',probe_sha256:'0'.repeat(64),owner:null})];
  for(const mutate of mutations) {
    const forged=structuredClone(packet);mutate(forged);
    assert.throws(()=>parsePacket(JSON.stringify(forged)),/invalid_packet/);
    assert.equal(validate(JSON.stringify(forged),forbidden).valid,false);
  }
}));
