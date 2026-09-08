// S4 integration contract REDs. These must be rerun against the exact frozen
// schema15/D1 baseline before any production edit; helper-only RED is not a
// claim about historical producer behavior.
import test from 'node:test';
import assert from 'node:assert/strict';
import {appendFileSync,cpSync,mkdtempSync,mkdirSync,readFileSync,rmSync,symlinkSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {observeConfigProvenance,CONFIG_OPTION_NAMES} from './config-provenance.mjs';

const implementation=process.env.PRISM_CALLABLE_IMPLEMENTATION??path.dirname(fileURLToPath(import.meta.url));
const baselineImplementation=process.env.PRISM_CALLABLE_BASELINE;
const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'PRISM_TYPESCRIPT is required');
const moduleUrl=(root,file,tag='')=>pathToFileURL(path.join(root,file)).href+tag;
const candidate=await import(moduleUrl(implementation,'index.mjs'));
const baseline=baselineImplementation?await import(moduleUrl(baselineImplementation,'index.mjs','?s4-baseline')):null;
const schema=await import(moduleUrl(implementation,'schema.mjs'));
const ts=createRequire(import.meta.url)(compiler);
assert.equal(schema.COMPILER_HASH,schema.hash(readFileSync(compiler)));

function fixture(run,config={compilerOptions:{types:[],lib:['es5'],target:'ES2022',noEmit:true},files:['src/app.ts']}) {
  const root=mkdtempSync(path.join(tmpdir(),'prism-s4-integration-'));
  const put=(file,value)=>{const target=path.join(root,file);mkdirSync(path.dirname(target),{recursive:true});
    writeFileSync(target,typeof value==='string'?value:JSON.stringify(value));};
  const save=value=>put('tsconfig.json',value);
  try {
    save(config);put('src/app.ts','export {};');
    return run({root,put,save,options:{root,compiler,config:'tsconfig.json'}});
  } finally {rmSync(root,{recursive:true,force:true});}
}

const legacyProjection=packet=>{const value=structuredClone(packet);delete value.schema;delete value.producer;delete value.config_provenance;delete value.entry_obligations;return value;};
function pair(options) {
  const before=baseline?.produce(options),after=candidate.produce(options);
  // Historical parity is asserted only when the caller supplies a frozen base.
  // Normal repository tests always execute the current-behavior assertions below.
  if(before)assert.deepEqual(legacyProjection(after),legacyProjection(before),'S4 changed a pre-existing packet field');
  return {before,after};
}
const provenance=packet=>{assert(Object.hasOwn(packet,'config_provenance'),'missing schema16 config_provenance');return packet.config_provenance;};
const option=(record,name)=>record.options.find(row=>row.name===name);
const full=(record,file)=>record.files.find(anchor=>anchor.file===file);
const unproven=(record,reason)=>assert.deepEqual(record,{status:'unproven',reason,files:[],extends:[],options:[]});

test('direct full worker retains seven-option provenance and reproduces in schema17',()=>fixture(({options})=>{
  const {after}=pair(options),record=provenance(after);
  assert.equal(after.schema,'prism.callable-observation/18');
  assert.equal(after.producer.version,'0.19.0');
  assert.equal(record.status,'observed');
  assert.deepEqual(record.files.map(row=>row.file),['project/tsconfig.json']);
  assert.deepEqual(after.snapshot.config_files,record.files.map(row=>row.file));
  assert.deepEqual(record.extends,[]);
  assert.deepEqual(record.options.map(row=>row.name),CONFIG_OPTION_NAMES);
  for(const name of ['types','lib','target'])assert.equal(option(record,name).present,true);
  for(const name of ['typeRoots','noLib','libReplacement','noResolve']) {
    const row=option(record,name);assert.equal(row.present,false);assert.equal(row.origin,null);
    assert.equal(row.value_sha256,schema.hash(schema.canonical({present:false,value:null})));
  }
  const anchor=record.files[0],snapshot=after.snapshot.files.find(row=>row.id===anchor.file);
  assert.equal(anchor.kind,'SourceFile');assert.equal(anchor.start_utf16,0);assert.equal(anchor.start_byte,0);
  assert.equal(anchor.end_byte,snapshot.size);assert.equal(anchor.sha256,snapshot.sha256);
  assert.equal(candidate.validate(JSON.stringify(after),options).valid,true);
}));

test('full worker admits all seven bounded normalized value shapes',()=>fixture(({save,options})=>{
  save({compilerOptions:{types:[],lib:[],typeRoots:[],noLib:false,libReplacement:false,
    noResolve:false,target:'ES3',noEmit:true},files:['src/app.ts']});
  const record=provenance(pair(options).after);assert.equal(record.status,'observed');
  assert.deepEqual(record.options.map(row=>[row.name,row.present]),CONFIG_OPTION_NAMES.map(name=>[name,true]));
  for(const row of record.options)assert.equal(row.origin.kind,'PropertyAssignment');
}));

test('empty worker outcomes initialize unavailable without changing legacy refusal',()=>fixture(({options})=>{
  const selected={...options,compiler:path.join(options.root,'missing-typescript.js')};
  const packet=pair(selected).after;assert.deepEqual(packet.reasons,['unsupported_input']);
  unproven(provenance(packet),'unavailable');
}));

test('three-file full worker preserves root-first nearest origins and read parity',()=>fixture(({put,save,options})=>{
  save({extends:'./config/base',compilerOptions:{types:[]},files:['src/app.ts']});
  put('config/base.json',{extends:'./grand.json',compilerOptions:{lib:['es5'],noResolve:false}});
  put('config/grand.json',{compilerOptions:{typeRoots:['./types'],target:'ES3'}});
  put('config/types/.keep','');
  const {after}=pair(options),record=provenance(after);
  assert.equal(record.status,'observed');
  assert.deepEqual(record.files.map(row=>row.file),[
    'project/tsconfig.json','project/config/base.json','project/config/grand.json']);
  assert.deepEqual(after.snapshot.config_files,record.files.map(row=>row.file).toSorted());
  assert.deepEqual(record.extends.map(edge=>[edge.source.file,edge.target.file]),[
    ['project/tsconfig.json','project/config/base.json'],['project/config/base.json','project/config/grand.json']]);
  assert.equal(option(record,'types').origin.file,'project/tsconfig.json');
  assert.equal(option(record,'lib').origin.file,'project/config/base.json');
  assert.equal(option(record,'noResolve').origin.file,'project/config/base.json');
  assert.equal(option(record,'typeRoots').origin.file,'project/config/grand.json');
  assert.equal(option(record,'target').origin.file,'project/config/grand.json');
  assert.equal(candidate.validate(JSON.stringify(after),options).valid,true);
}));

test('full worker honors extensionless exact-file precedence over .json',()=>fixture(({put,save,options})=>{
  save({extends:'./base',files:['src/app.ts']});
  put('base',{compilerOptions:{types:[]}});put('base.json',{compilerOptions:{types:['json']}});
  const record=provenance(pair(options).after);assert.equal(record.status,'observed');
  assert.deepEqual(record.files.map(row=>row.file),['project/tsconfig.json','project/base']);
  assert.equal(option(record,'types').origin.file,'project/base');
}));

test('unsupported full-worker config forms retain legacy results and fail observation closed',()=>{
  const cases=[
    ['array',{extends:['./base.json'],files:['src/app.ts']},{'base.json':{compilerOptions:{}}},'unsupported_extends'],
    ['package',{extends:'package/config',files:['src/app.ts']},{},'invalid_config'],
    ['absolute',{extends:'/outside/config.json',files:['src/app.ts']},{},'invalid_config'],
    ['missing',{extends:'./missing.json',files:['src/app.ts']},{},'invalid_config'],
    ['cycle',{extends:'./base.json',files:['src/app.ts']},{'base.json':{extends:'./tsconfig.json'}},'invalid_config'],
    ['configDir',{compilerOptions:{typeRoots:['${configDir}/types']},files:['src/app.ts']},{},'unsupported_option'],
  ];
  for(const [name,config,files,reason] of cases)fixture(({put,save,options})=>{
    save(config);for(const [file,value] of Object.entries(files))put(file,value);
    unproven(provenance(pair(options).after),reason,name);
  });
  fixture(({save,options})=>{
    save('{"compilerOptions":{"types":[],"\\u0074ypes":["node"]},"files":["src/app.ts"]}');
    unproven(provenance(pair(options).after),'duplicate_property');
  });
});

test('all seven direct and inherited null resets preserve legacy output and fail observation closed',()=>{
  const inherited={types:['node'],lib:['es5'],typeRoots:['./types'],noLib:true,
    libReplacement:true,noResolve:true,target:'ES2022'};
  for(const name of CONFIG_OPTION_NAMES)fixture(({put,save,options})=>{
    save({compilerOptions:{[name]:null,noEmit:true},files:['src/app.ts']});
    const direct=pair(options).after;assert.deepEqual(direct.diagnostics,[]);unproven(provenance(direct),'unsupported_option');
    put('base.json',{compilerOptions:{[name]:inherited[name]}});
    save({extends:'./base.json',compilerOptions:{[name]:null,noEmit:true},files:['src/app.ts']});
    const reset=pair(options).after;assert.deepEqual(reset.diagnostics,[]);unproven(provenance(reset),'unsupported_option');
  });
});

test('actual measured case policy accepts mixed-case lexical selected config',()=>fixture(({put,save,options})=>{
  put('Base.json',{compilerOptions:{types:[]}});save({extends:'./Base.json',files:['src/app.ts']});
  const {after}=pair(options),record=provenance(after);
  assert.equal(typeof after.scope.case_sensitive,'boolean');assert.equal(record.status,'observed');
  assert.deepEqual(record.files.map(row=>row.file),['project/tsconfig.json','project/Base.json']);
}));

test('in-root config links use canonical anchors and canonical-alias re-entry refuses',()=>fixture(({root,put,save,options})=>{
  put('config/base.json',{compilerOptions:{types:[]}});
  symlinkSync('config/base.json',path.join(root,'alias.json'));
  save({extends:'./alias.json',files:['src/app.ts']});
  const linked={...options,links:'in-root'},positive=pair(linked).after,record=provenance(positive);
  assert.equal(record.status,'observed');
  assert.deepEqual(record.files.map(row=>row.file),['project/tsconfig.json','project/config/base.json']);
  assert.equal(record.extends[0].target.file,'project/config/base.json');
  rmSync(path.join(root,'alias.json'));put('config/base.json',{extends:'./alias-b.json'});
  symlinkSync('config/base.json',path.join(root,'alias-a.json'));
  symlinkSync('config/base.json',path.join(root,'alias-b.json'));
  save({extends:'./alias-a.json',files:['src/app.ts']});
  unproven(provenance(pair(linked).after),'invalid_config');
}));

test('lexical root config link retains canonical first anchor',()=>fixture(({root,put,options})=>{
  put('config/root.json',{compilerOptions:{types:[],noEmit:true},files:['src/app.ts']});
  symlinkSync('config/root.json',path.join(root,'tsconfig-link.json'));
  const selected={...options,config:'tsconfig-link.json',links:'in-root'},record=provenance(pair(selected).after);
  assert.equal(record.status,'observed');
  assert.equal(record.files[0].file,'project/config/root.json');
}));

test('parser rejects invalid config populations before root I/O and retains schema15 history',()=>fixture(({put,save,options})=>{
  save({extends:'./base.json',compilerOptions:{types:[],lib:['es5']},files:['src/app.ts']});
  put('base.json',{compilerOptions:{target:'ES2022'}});
  const {before,after}=pair(options),record=provenance(after);
  if(before)assert.doesNotThrow(()=>schema.parsePacket(JSON.stringify(before)),'supplied frozen schema15 packet must remain readable');
  const mutations=[
    q=>delete q.config_provenance,
    q=>q.config_provenance.files.length=0,
    q=>q.config_provenance.files.push(q.config_provenance.files[0]),
    q=>q.config_provenance.files[0].kind='Other',
    q=>q.config_provenance.files[0].start_byte=1,
    q=>q.config_provenance.extends.length=0,
    q=>q.config_provenance.extends[0].source=q.config_provenance.files[1],
    q=>q.config_provenance.options.pop(),
    q=>q.config_provenance.options.reverse(),
    q=>q.config_provenance.options[0].origin=null,
    q=>q.snapshot.config_files.push('project/src/app.ts'),
    q=>q.config_provenance={status:'unproven',reason:null,files:[],extends:[],options:[]},
    q=>q.config_provenance={status:'unproven',reason:'cycle',files:[record.files[0]],extends:[],options:[]},
  ];
  for(const mutate of mutations) {
    const forged=structuredClone(after);mutate(forged);
    assert.throws(()=>schema.parsePacket(JSON.stringify(forged)),/invalid_packet/);
    let reads=0;assert.equal(candidate.validate(JSON.stringify(forged),{get root(){reads++;throw Error('forbidden');}}).valid,false);
    assert.equal(reads,0);
  }
}));

test('same-genuine origin/value substitutions and source changes fail reproduction',()=>fixture(({put,save,options})=>{
  save({compilerOptions:{types:[],lib:['es5'],target:'ES2022'},files:['src/app.ts']});
  const packet=pair(options).after,record=provenance(packet);
  for(const mutate of [
    q=>[option(q.config_provenance,'types').origin,option(q.config_provenance,'lib').origin]=
      [option(q.config_provenance,'lib').origin,option(q.config_provenance,'types').origin],
    q=>[option(q.config_provenance,'types').value_sha256,option(q.config_provenance,'lib').value_sha256]=
      [option(q.config_provenance,'lib').value_sha256,option(q.config_provenance,'types').value_sha256],
  ]) {
    const forged=structuredClone(packet);mutate(forged);assert.doesNotThrow(()=>schema.parsePacket(JSON.stringify(forged)));
    assert.equal(candidate.validate(JSON.stringify(forged),options).valid,false);
  }
  put('tsconfig.json',{compilerOptions:{types:['changed'],lib:['es5'],target:'ES2022'},files:['src/app.ts']});
  assert.equal(candidate.validate(JSON.stringify(packet),options).valid,false);
  assert.equal(record.status,'observed');
}));

test('same-genuine edge target/source swaps and omitted rows are structurally rejected',()=>fixture(({put,save,options})=>{
  save({extends:'./base.json',files:['src/app.ts']});put('base.json',{extends:'./grand.json'});
  put('grand.json',{compilerOptions:{types:[]}});
  const packet=pair(options).after,record=provenance(packet);
  const mutations=[
    q=>q.config_provenance.extends[0].target=structuredClone(full(q.config_provenance,'project/grand.json')),
    q=>q.config_provenance.extends[0].source=structuredClone(q.config_provenance.extends[1].source),
    q=>q.config_provenance.options.splice(2,1),
  ];
  for(const mutate of mutations) {
    const forged=structuredClone(packet);mutate(forged);assert.throws(()=>schema.parsePacket(JSON.stringify(forged)),/invalid_packet/);
  }
  assert.equal(record.status,'observed');
}));

test('producer digest includes the config helper bytes',async()=>{
  const root=mkdtempSync(path.join(tmpdir(),'prism-s4-digest-'));
  try {
    const copied=path.join(root,'implementation');cpSync(implementation,copied,{recursive:true});
    appendFileSync(path.join(copied,'config-provenance.mjs'),'\n// digest mutation\n');
    const changed=await import(moduleUrl(copied,'index.mjs','?s4-digest'));
    assert.notEqual(changed.producerHash(),candidate.producerHash());
  } finally {rmSync(root,{recursive:true,force:true});}
});

test('surplus successful config-read membership is a captured-helper integration RED',()=>{
  const text=JSON.stringify({compilerOptions:{types:[]},files:['src/app.ts']}),extra='{}';
  const source=ts.parseJsonText('/__prism__/project/tsconfig.json',text);
  const inventory=new Map([
    ['project/tsconfig.json',{sha256:schema.hash(text),size:Buffer.byteLength(text)}],
    ['project/surplus.json',{sha256:schema.hash(extra),size:Buffer.byteLength(extra)}],
  ]);
  const result=observeConfigProvenance({ts,source,effectiveOptions:ts.convertCompilerOptionsFromJson({types:[]},'/__prism__/project').options,
    capturedRecords:{records:[],writes:0,overflow:false,cap:33},caseSensitive:true,
    canonicalId:file=>file.slice('/__prism__/'.length),configReadMembership:new Set(inventory.keys()),inventory,configDiagnostics:[]});
  unproven(result,'selection_mismatch');
});
