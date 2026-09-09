// S9 producer-facing RED: frozen S8 has no schema20/v3 lane.
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,rmSync,writeFileSync} from 'node:fs';
import path from 'node:path';
import {tmpdir} from 'node:os';
import {pathToFileURL} from 'node:url';
import {produce} from './index.mjs';
import {parsePacket,hash,canonical} from './schema.mjs';

const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'PRISM_TYPESCRIPT is required');
const baselineRoot=process.env.PRISM_CALLABLE_BASELINE;
const baseline=baselineRoot?await import(pathToFileURL(path.join(baselineRoot,'index.mjs')).href+'?s9-v3-receipt'):null;
const empty='declare module "*.scss" {}';
const shorthand='declare module "*.scss";';
const app='class Client{m(){}}type View<P>=(p:P)=>void;const run:View<{client:Client}>=({client})=>{const cb=()=>client.m();};';

function fixture({reverse=false,asset=false,compilerOptions={},extraFiles={},appPrefix='',configMutate=null},run) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-s9-red-'));
  const put=(file,text)=>{const target=path.join(root,file);mkdirSync(path.dirname(target),{recursive:true});writeFileSync(target,text);};
  try {
    put('package.json','{"type":"module"}');
    const config={compilerOptions:{strict:true,noEmit:true,types:[],module:'ESNext',moduleResolution:'Bundler',target:'ES2022',skipLibCheck:true,noUncheckedSideEffectImports:true,libReplacement:false,...compilerOptions},include:['src']};
    configMutate?.(config);put('tsconfig.json',JSON.stringify(config));
    put('src/a.d.ts',reverse?shorthand:empty);put('src/b.d.ts',reverse?empty:shorthand);
    put('src/app.ts',appPrefix+'import "./style.scss";'+app);for(const [file,text] of Object.entries(extraFiles))put(file,text);if(asset)put('src/style.scss','.style{}');
    return run({root,options:{root,compiler,config:'tsconfig.json'}});
  } finally {rmSync(root,{recursive:true,force:true});}
}

const legacyProjection=packet=>{const copy=structuredClone(packet);delete copy.schema;delete copy.producer;delete copy.semantic_closure;return copy;};
function assertPromotedPair(packet,reverse) {
  const rows=packet.resolutions.filter(row=>row.specifier==='./style.scss');assert.equal(rows.length,1);const [resolution]=rows;
  const merged=resolution.lookup.merged_wildcard,wildcard=resolution.lookup.wildcard;
  assert.equal(resolution.target,null);assert.equal(resolution.lookup.status,'unproven');assert.equal(resolution.lookup.reason,'ambiguous_binding');
  assert.equal(resolution.lookup.providers.length,0,'no exact provider');assert.equal(resolution.lookup.augmentations.length,0,'no augmentation');
  assert.equal(wildcard.status,'unproven');assert.equal(wildcard.reason,'duplicate_provider');assert.equal(wildcard.providers.length,2);assert.equal(wildcard.matches.length,2);assert.equal(wildcard.augmentations.length,0,'no competing augmentation');
  assert.equal(merged.status,'observed');assert.equal(merged.reason,null);assert.equal(merged.declarations.length,2);
  assert.deepEqual(merged.declarations.map(row=>row.declaration),resolution.lookup.declarations,'exact provider population');
  assert.deepEqual(merged.declarations.map(row=>row.shape),reverse?['shorthand','empty_block']:['empty_block','shorthand']);
  assert.deepEqual(merged.selected,merged.declarations[0],'compiler-selected contributor, not merely an included contributor');
  assert.deepEqual(wildcard.matches.map(row=>row.file),resolution.lookup.declarations.map(row=>row.file),'no competing wildcard pattern');
  return resolution;
}
function assertS8Receipt(options,after) {
  if(!baseline)return;
  const before=baseline.produce(options);
  // Retrospective receipt: S8 is expected to retain this row as unadmitted;
  // this is an explicit before/after comparison, not a new pre-edit RED.
  assert.deepEqual(before.semantic_closure.rows.filter(row=>before.resolutions[row.index]?.specifier==='./style.scss'),[{index:0,disposition:'unproven',reason:'unadmitted_binding'}]);
  assert.deepEqual(legacyProjection(after),legacyProjection(before),'S9 changed a legacy Pair-A/B packet field');
}

for(const reverse of [false,true])for(const asset of [false,true])test(`S9 pair source guard reverse=${reverse} asset=${asset}`,()=>fixture({reverse,asset},({options})=>{
  const packet=produce(options),resolution=assertPromotedPair(packet,reverse);
  assert.deepEqual(packet.semantic_closure.rows.filter(row=>packet.resolutions[row.index]===resolution),[{index:packet.resolutions.indexOf(resolution),disposition:'merged_side_effect',reason:null}]);
  assertS8Receipt(options,packet);
  assert.equal(packet.snapshot.files.some(file=>file.id==='project/src/style.scss'),asset);
  assert.equal(packet.schema,'prism.callable-observation/20');
  assert.equal(packet.producer.version,'0.21.0');
  assert.deepEqual(packet.semantic_closure,{policy:'prism.semantic-closure/merged-side-effect-v3',complete:true,reasons:[],rows:[{index:0,disposition:'merged_side_effect',reason:null}]});
}));

for(const [label,setup,reason,assertEvidence] of [
  ['unrelated null request',{appPrefix:'import type {} from "node:missing";'},'resolution_unproven',packet=>{
    const index=packet.resolutions.findIndex(row=>row.specifier==='node:missing'&&row.target===null&&row.lookup.status==='unproven');
    assert(index>=0,'captured unrelated null request required');assert.deepEqual(packet.semantic_closure.rows[index],{index,disposition:'unproven',reason:'unresolved_module'});
  }],
  ['outside encounter',{appPrefix:'import "outside-package";'},'boundary_encounter',packet=>{
    assert.equal(packet.snapshot.outside_lookups,true);assert(packet.search_provenance.boundary_events.some(event=>event.kind==='outside'));
  }],
  ['refused encounter',{compilerOptions:{baseUrl:'.'},appPrefix:'import "virtual:input";'},'boundary_encounter',packet=>{
    assert(packet.snapshot.refused_lookup_sha256.length>0);assert(packet.search_provenance.boundary_events.some(event=>event.kind==='refused'));
  }],
  ['noResolve',{compilerOptions:{noResolve:true}},'no_resolve',packet=>{
    const option=packet.config_provenance.options.find(row=>row.name==='noResolve');assert(option?.present);assert.equal(option.value_sha256,hash(canonical({present:true,value:true})));
  }],
  ['path obligation',{appPrefix:'/// <reference path="./absent.d.ts" />\n'},'required_path_unproven',packet=>assert(packet.reasons.includes('unproven_path_reference'))],
  ['types obligation',{appPrefix:'/// <reference types="missing-types" />\n'},'type_lib_unproven',packet=>{
    assert(packet.reasons.includes('unproven_type_lib_reference'));assert(packet.type_lib_references.some(row=>row.kind==='types'&&row.name==='missing-types'&&row.status==='unproven'&&row.reason==='unresolved'));
  }],
  ['lib obligation',{appPrefix:'/// <reference lib="missing-lib" />\n'},'type_lib_unproven',packet=>{
    assert(packet.reasons.includes('unproven_type_lib_reference'));assert(packet.type_lib_references.some(row=>row.kind==='lib'&&row.name==='missing-lib'&&row.status==='unproven'&&row.reason==='unresolved'));
  }],
  ['compiler diagnostics',{appPrefix:'const wrong:string=1;\n'},'compiler_diagnostics',packet=>assert(packet.diagnostics.length>0)],
  ['unproven config',{compilerOptions:{types:null}},'config_unproven',packet=>assert.equal(packet.config_provenance.status,'unproven')],
  ['incomplete entries',{configMutate:config=>delete config.compilerOptions.types},'entry_obligations_incomplete',packet=>{
    assert.equal(packet.entry_obligations.complete,false);assert(packet.entry_obligations.reasons.includes('automatic_discovery_unproven'));
  }],
])test(`Pair A preserves inherited ${label} barrier and S8 legacy projection receipt`,()=>fixture(setup,({options})=>{
  const packet=produce(options),resolution=assertPromotedPair(packet,false),semantic=packet.semantic_closure;
  assert.deepEqual(semantic.rows.filter(row=>packet.resolutions[row.index]===resolution),[{index:packet.resolutions.indexOf(resolution),disposition:'merged_side_effect',reason:null}]);
  assertEvidence(packet);assert.equal(semantic.complete,false);assert(semantic.reasons.includes(reason),`${label} semantic barrier required`);
  assertS8Receipt(options,packet);
}));

test('schema20 fixes v3 policy and recomputes the selected merged row before root I/O',()=>fixture({reverse:false,asset:false},({options})=>{
  const packet=produce(options);
  for(const mutate of [
    q=>q.semantic_closure.policy='prism.semantic-closure/singleton-wildcard-v2',
    q=>q.semantic_closure.rows[0].disposition='singleton_wildcard',
    q=>q.semantic_closure.rows[0].reason='unadmitted_binding',
    q=>q.semantic_closure.rows[0].index=1,
  ]) {const forged=structuredClone(packet);mutate(forged);assert.throws(()=>parsePacket(JSON.stringify(forged)),/invalid_packet/);}
}));
