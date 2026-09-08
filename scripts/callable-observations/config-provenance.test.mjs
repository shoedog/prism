// Characterization/negative validation for the pure S4 helper. This is not a
// production RED: the producer/schema/index integration intentionally remains S4-owned.
import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {COMPILER_HASH,canonical,hash} from './schema.mjs';
import {CONFIG_OPTION_NAMES,createConfigCapture,observeConfigProvenance} from './config-provenance.mjs';

const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'PRISM_TYPESCRIPT is required');
const ts=createRequire(import.meta.url)(compiler);
assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
const root='/__prism__/project',rootFile=root+'/tsconfig.json';

function makeFixture(rootConfig,others={},aliases={},caseSensitive=true) {
  const encode=value=>typeof value==='string'?value:JSON.stringify(value);
  const physical=new Map([[rootFile,encode(rootConfig)],[root+'/src/app.ts','export {};'],
    ...Object.entries(others).map(([name,value])=>[root+'/'+name,encode(value)])]);
  const aliasMap=new Map(Object.entries(aliases).map(([from,to])=>[root+'/'+from,root+'/'+to]));
  const findKey=(map,file)=>map.has(file)?file:caseSensitive?null:[...map.keys()].find(key=>key.toLowerCase()===file.toLowerCase())??null;
  const resolve=file=>{let found=path.posix.normalize(file),seen=new Set();for(;;){
    const alias=findKey(aliasMap,found);if(!alias||seen.has(alias))break;seen.add(alias);found=aliasMap.get(alias);
  }return findKey(physical,found)??found;};
  const canonicalId=file=>{const found=resolve(file);return found.startsWith('/__prism__/')?found.slice('/__prism__/'.length):null;};
  const configReadMembership=new Set();
  const host={useCaseSensitiveFileNames:caseSensitive,
    readFile(file){const found=resolve(file),value=physical.get(found);if(value!==undefined&&/\.json$|\/base$/.test(file))configReadMembership.add(canonicalId(file));return value;},
    fileExists:file=>physical.has(resolve(file)),
    readDirectory:()=>[root+'/src/app.ts'],getCurrentDirectory:()=>root,
    directoryExists:dir=>[...physical.keys()].some(file=>file.startsWith(dir+'/'))};
  const read=ts.readConfigFile(rootFile,host.readFile),capture=createConfigCapture();
  const parsed=ts.parseJsonConfigFileContent(read.config,host,root,undefined,rootFile,undefined,undefined,capture);
  const source=ts.parseJsonText(rootFile,physical.get(rootFile));
  const inventory=new Map();
  for(const [file,text] of physical)inventory.set(canonicalId(file),{sha256:hash(text),size:Buffer.byteLength(text)});
  const inputs={ts,source,effectiveOptions:parsed.options,capturedRecords:capture.capturedRecords,
    canonicalId,configReadMembership,inventory,configDiagnostics:[read.error,...parsed.errors].filter(Boolean),caseSensitive};
  return {inputs,parsed,capture,observe(overrides={}){return observeConfigProvenance({...inputs,...overrides});}};
}

const observed=result=>{assert.equal(result.status,'observed');assert.equal(result.reason,null);return result;};
const unproven=(result,reason)=>{assert.deepEqual(result,{status:'unproven',reason,files:[],extends:[],options:[]});};
const option=(result,name)=>result.options.find(item=>item.name===name);

test('bounded recorder is an always-miss, nonthrowing observational cache',()=>{
  const capture=createConfigCapture({cap:2});
  assert.equal(capture.get('anything'),undefined);
  assert.doesNotThrow(()=>capture.set('a',null));
  assert.doesNotThrow(()=>capture.set(Symbol('b'),{extendedResult:{}}));
  assert.doesNotThrow(()=>capture.set({unserializable:1n},()=>{}));
  assert.equal(capture.get('a'),undefined);
  assert.equal(capture.capturedRecords.records.length,2);
  assert.deepEqual(capture.capturedRecords.records[0],{key:'a',value:null});
  assert.equal(typeof capture.capturedRecords.records[1].key,'symbol');
  assert.deepEqual(capture.capturedRecords.records[1].value,{extendedResult:{}});
  assert.deepEqual({...capture.capturedRecords,records:undefined},{records:undefined,writes:3,overflow:true,cap:2});
  assert.throws(()=>createConfigCapture({cap:0}),/invalid_capture_cap/);
});

test('direct root records the fixed option population including absence',()=>{
  const fixture=makeFixture({compilerOptions:{types:['node'],lib:['es5'],target:'ES2022',maxNodeModuleJsDepth:2},files:['src/app.ts']});
  const result=observed(fixture.observe());
  assert.deepEqual(result.files.map(anchor=>anchor.file),['project/tsconfig.json']);
  assert.deepEqual(result.extends,[]);
  assert.deepEqual(result.options.map(item=>item.name),CONFIG_OPTION_NAMES);
  assert.equal(option(result,'types').present,true);assert.equal(option(result,'lib').present,true);
  assert.equal(option(result,'target').present,true);assert.equal(option(result,'typeRoots').present,false);
  assert.equal(option(result,'typeRoots').value_sha256,hash(canonical({present:false,value:null})));
  assert.match(option(result,'types').origin.sha256,/^[a-f0-9]{64}$/);
});

test('all seven explicit null resets are unsupported direct and inherited origins',()=>{
  const inherited={types:['node'],lib:['es5'],typeRoots:['./types'],noLib:true,
    libReplacement:true,noResolve:true,target:'ES2022'};
  for(const name of CONFIG_OPTION_NAMES) {
    const direct=makeFixture({compilerOptions:{[name]:null},files:['src/app.ts']});
    assert.deepEqual(direct.parsed.errors.map(error=>error.code),[],name+' direct diagnostics');
    assert.equal(direct.parsed.options[name],undefined,name+' direct reset');
    unproven(direct.observe(),'unsupported_option');
    const reset=makeFixture({extends:'./base.json',compilerOptions:{[name]:null},files:['src/app.ts']},{
      'base.json':{compilerOptions:{[name]:inherited[name]}}});
    assert.deepEqual(reset.parsed.errors.map(error=>error.code),[],name+' inherited diagnostics');
    assert.equal(reset.parsed.options[name],undefined,name+' inherited reset');
    unproven(reset.observe(),'unsupported_option');
  }
});

test('empty arrays, false booleans and target enum zero remain supported',()=>{
  const fixture=makeFixture({compilerOptions:{types:[],lib:[],typeRoots:[],noLib:false,
    libReplacement:false,noResolve:false,target:'ES3'},files:['src/app.ts']});
  const result=observed(fixture.observe());
  for(const name of ['types','lib','typeRoots'])assert.deepEqual(fixture.parsed.options[name],[]);
  for(const name of ['noLib','libReplacement','noResolve'])assert.equal(fixture.parsed.options[name],false);
  assert.equal(fixture.parsed.options.target,0);
  assert.equal(result.options.length,7);
});

test('root-first local chain records overriding and inherited exact origins',()=>{
  const fixture=makeFixture({extends:'./config/base',compilerOptions:{types:['local']},files:['src/app.ts']},{
    'config/base.json':{extends:'./grand.json',compilerOptions:{lib:['es5'],noResolve:true}},
    'config/grand.json':{compilerOptions:{types:['base'],typeRoots:['./types'],target:'ES2022'}}});
  const result=observed(fixture.observe());
  assert.deepEqual(result.files.map(anchor=>anchor.file),['project/tsconfig.json','project/config/base.json','project/config/grand.json']);
  assert.deepEqual(result.extends.map(edge=>[edge.source.file,edge.target.file]),[
    ['project/tsconfig.json','project/config/base.json'],['project/config/base.json','project/config/grand.json']]);
  assert.equal(option(result,'types').origin.file,'project/tsconfig.json');
  assert.equal(option(result,'lib').origin.file,'project/config/base.json');
  assert.equal(option(result,'typeRoots').origin.file,'project/config/grand.json');
  assert.equal(option(result,'noLib').present,false);
});

test('extensionless lookup selects an existing exact file before .json',()=>{
  const fixture=makeFixture({extends:'./base',files:['src/app.ts']},{
    base:{compilerOptions:{types:['exact']}},'base.json':{compilerOptions:{types:['json']}}});
  const result=observed(fixture.observe());
  assert.equal(result.extends[0].target.file,'project/base');
  assert.equal(option(result,'types').origin.file,'project/base');
});

test('AST anchors survive comments, trailing commas, Unicode and escaped keys',()=>{
  const text='{\n // emoji 🧪 changes byte offsets\n "compilerOptions":{"\\u0074ypes":["a","a"],"lib":[],},\n "files":["src/app.ts"],\n}';
  const result=observed(makeFixture(text).observe());
  const types=option(result,'types');
  assert.equal(types.present,true);assert.ok(types.origin.start_byte>types.origin.start_utf16);
  assert.equal(types.value_sha256,hash(canonical({present:true,value:['a','a']})));
  assert.equal(option(result,'lib').value_sha256,hash(canonical({present:true,value:[]})));
});

test('only direct unique compilerOptions properties count as origins',()=>{
  const fixture=makeFixture({compilerOptions:{},watchOptions:{types:['nested']},files:['src/app.ts']});
  const result=observed(fixture.observe({configDiagnostics:[]}));
  assert.equal(option(result,'types').present,false);
});

test('decoded duplicate properties fail closed, including escaped spellings',()=>{
  const duplicate='{ "compilerOptions":{"types":[],"\\u0074ypes":["node"]},"files":["src/app.ts"] }';
  unproven(makeFixture(duplicate).observe({configDiagnostics:[]}),'duplicate_property');
  const rootDuplicate='{ "compilerOptions":{},"\\u0063ompilerOptions":{},"files":["src/app.ts"] }';
  unproven(makeFixture(rootDuplicate).observe({configDiagnostics:[]}),'duplicate_property');
  const duplicateBeforeUnsupported='{ "extends":"./a","\\u0065xtends":[],"files":["src/app.ts"] }';
  unproven(makeFixture(duplicateBeforeUnsupported,{'a':{}}).observe({configDiagnostics:[]}),'duplicate_property');
});

test('unsupported extends forms and configDir options fail closed',()=>{
  for(const value of [['./base.json'], '/__prism__/project/base.json','pkg/base','.\\base.json']) {
    const fixture=makeFixture({extends:value,files:['src/app.ts']},{'base.json':{compilerOptions:{}}});
    const result=fixture.observe({configDiagnostics:[]});
    unproven(result,'unsupported_extends');
  }
  const configDir=makeFixture({compilerOptions:{typeRoots:['${configDir}/types']},files:['src/app.ts']});
  unproven(configDir.observe(),'unsupported_option');
});

test('compiler diagnostics, missing targets and cycles never emit partial proof',()=>{
  const malformed=makeFixture('{"compilerOptions":');
  unproven(malformed.observe(),'invalid_config');
  const missing=makeFixture({extends:'./missing.json',files:['src/app.ts']});
  unproven(missing.observe({configDiagnostics:[]}),'missing_target');
  const cycle=makeFixture({extends:'./base.json',files:['src/app.ts']},{'base.json':{extends:'./tsconfig.json'}});
  unproven(cycle.observe(),'invalid_config');
  unproven(observeConfigProvenance({}),'unavailable');
  unproven(missing.observe({capturedRecords:{records:[],writes:-1,overflow:false,cap:33},configDiagnostics:[]}),'unavailable');
});

test('effective-value drift and captured-record omission or addition fail closed',()=>{
  const fixture=makeFixture({extends:'./base.json',files:['src/app.ts']},{'base.json':{compilerOptions:{types:['node']}}});
  unproven(fixture.observe({effectiveOptions:{...fixture.inputs.effectiveOptions,types:['other']}}),'value_mismatch');
  const records=fixture.capture.capturedRecords.records;
  unproven(fixture.observe({capturedRecords:{...fixture.capture.capturedRecords,records:[]}}),'selection_mismatch');
  unproven(fixture.observe({capturedRecords:{...fixture.capture.capturedRecords,records:[...records,records[0]],writes:2}}),'selection_mismatch');
  unproven(fixture.observe({capturedRecords:{...fixture.capture.capturedRecords,
    records:[{...records[0],key:root+'/other.json'}]}}),'selection_mismatch');
});

test('32 files are admitted; 33 files and real recorder overflow fail closed',()=>{
  const chainFixture=count=>{
    const others={};
    for(let index=0;index<count;index++)others[`c${index}.json`]=index===count-1?{compilerOptions:{types:[]}}:{extends:`./c${index+1}.json`};
    return makeFixture({extends:'./c0.json',files:['src/app.ts']},others);
  };
  assert.equal(observed(chainFixture(31).observe()).files.length,32);
  unproven(chainFixture(32).observe(),'chain_limit');
  const overflow=chainFixture(34);
  assert.deepEqual({writes:overflow.capture.capturedRecords.writes,retained:overflow.capture.capturedRecords.records.length,
    overflow:overflow.capture.capturedRecords.overflow},{writes:34,retained:33,overflow:true});
  unproven(overflow.observe(),'chain_limit');
});

test('canonical link target is recorded while alias re-entry is rejected',()=>{
  const positive=makeFixture({extends:'./alias.json',files:['src/app.ts']},{'config/base.json':{compilerOptions:{types:[]}}},{'alias.json':'config/base.json'});
  const result=observed(positive.observe());
  assert.deepEqual(result.files.map(anchor=>anchor.file),['project/tsconfig.json','project/config/base.json']);
  assert.equal(result.extends[0].target.file,'project/config/base.json');

  const loop=makeFixture({extends:'./alias-a.json',files:['src/app.ts']},{'config/base.json':{extends:'./alias-b.json'}},{
    'alias-a.json':'config/base.json','alias-b.json':'config/base.json'});
  unproven(loop.observe({configDiagnostics:[]}),'cycle');
});

test('cache keys follow explicit case policy without borrowing canonical aliases',()=>{
  for(const caseSensitive of [true,false]) {
    const fixture=makeFixture({extends:'./Base.json',files:['src/app.ts']},{
      'Base.json':{compilerOptions:{types:[]}}},{},caseSensitive);
    const record=fixture.capture.capturedRecords.records[0];
    assert.equal(record.key,caseSensitive?root+'/Base.json':root+'/base.json');
    assert.equal(record.value.extendedResult.fileName,root+'/Base.json');
    observed(fixture.observe());
  }
  const alias=makeFixture({extends:'./Base.json',files:['src/app.ts']},{
    'Base.json':{compilerOptions:{types:[]}}},{'alias.json':'Base.json'},false);
  const [record]=alias.capture.capturedRecords.records;
  unproven(alias.observe({capturedRecords:{...alias.capture.capturedRecords,
    records:[{...record,key:root+'/alias.json'}]}}),'selection_mismatch');
  unproven(alias.observe({capturedRecords:{...alias.capture.capturedRecords,
    records:[{...record,key:root+'/wrong.json'}]}}),'selection_mismatch');
});

test('inventory/read membership and source-byte disagreement fail closed',()=>{
  const fixture=makeFixture({compilerOptions:{types:[]},files:['src/app.ts']});
  unproven(fixture.observe({configReadMembership:new Set()}),'selection_mismatch');
  const inventory=new Map(fixture.inputs.inventory),entry={...inventory.get('project/tsconfig.json'),sha256:'0'.repeat(64)};
  inventory.set('project/tsconfig.json',entry);
  unproven(fixture.observe({inventory}),'selection_mismatch');
});

test('recording cache preserves exact compiler result and ordered host transcript',()=>{
  for(const [name,rootConfig,others] of [
    ['direct',{compilerOptions:{types:[],lib:['es5'],target:'ES2022'},files:['src/app.ts']},{}],
    ['inheritance',{extends:'./base.json',compilerOptions:{types:['local']},files:['src/app.ts']},{
      'base.json':{extends:'./grand.json',compilerOptions:{lib:['es5']}},
      'grand.json':{compilerOptions:{typeRoots:['./types'],target:'ES2022'}}}],
  ]) {
    const files=new Map([[rootFile,JSON.stringify(rootConfig)],[root+'/src/app.ts','export {};'],
      ...Object.entries(others).map(([file,value])=>[root+'/'+file,JSON.stringify(value)])]);
    const run=withCapture=>{
      const events=[],host={useCaseSensitiveFileNames:true,
        readFile:file=>{const value=files.get(file);events.push(['readFile',file,value===undefined?null:hash(value)]);return value;},
        fileExists:file=>{const value=files.has(file);events.push(['fileExists',file,value]);return value;},
        readDirectory:(...args)=>{events.push(['readDirectory',args]);return [root+'/src/app.ts'];},
        getCurrentDirectory:()=>{events.push(['getCurrentDirectory']);return root;},
        directoryExists:dir=>{const value=[...files.keys()].some(file=>file.startsWith(dir+'/'));events.push(['directoryExists',dir,value]);return value;}};
      const read=ts.readConfigFile(rootFile,host.readFile),capture=withCapture?createConfigCapture():undefined;
      const parsed=ts.parseJsonConfigFileContent(read.config,host,root,undefined,rootFile,undefined,undefined,capture);
      const output={...parsed,errors:parsed.errors.map(error=>({code:error.code,start:error.start??null,
        file:error.file?.fileName??null,message:error.messageText}))};
      return {events,output};
    };
    const baseline=run(false),candidate=run(true);
    assert.deepEqual(candidate.events,baseline.events,name+' host transcript');
    assert.deepEqual(candidate.output,baseline.output,name+' compiler result');
  }
});
