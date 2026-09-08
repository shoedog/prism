// Explicit cache-double edge controls, not real-Program behavioral RED evidence.
import test from 'node:test';import assert from 'node:assert/strict';
import {observeEntries} from './entries.mjs';import {REFERENCE_LIMIT} from './schema.mjs';
function setup(kind='types'){
  const target={fileName:'target.d.ts',path:'target-path'},sources=[target],options=kind==='types'?{types:['fixture'],lib:[]}:{types:[],lib:['lib.es5.d.ts']};
  const reasons=[kind==='types'?{kind:8,typeReference:'fixture'}:{kind:6,index:0}],includes=new Map([[target.path,reasons]]);
  const names=kind==='types'?['fixture']:[],resolution={resolvedTypeReferenceDirective:{resolvedFileName:target.fileName}};
  const program={getCompilerOptions:()=>options,getSourceFiles:()=>sources,getSourceFile:()=>target,getRootFileNames:()=>['source.ts'],getFileIncludeReasons:()=>includes,
    getAutomaticTypeDirectiveNames:()=>names,getAutomaticTypeDirectiveResolutions:()=>({get:(name,mode)=>{assert.equal(name,'fixture');assert.equal(mode,undefined);return resolution;}}),resolvedLibReferences:new Map([['lib.es5.d.ts',{actual:target.fileName}]])};
  const ts={getDefaultLibFileName:()=> 'lib.d.ts'},toId=f=>'project/'+f;
  return {target,sources,options,reasons,includes,names,resolution,program,ts,toId,run:()=>observeEntries(ts,program,toId)};
}
for(const kind of ['types','lib'])test(`${kind} exact cache and inclusion satisfy helper contract`,()=>{const x=setup(kind);assert.equal(x.run()[0].status,'observed');});
for(const kind of ['types','lib'])test(`${kind} cannot borrow another entry inclusion`,()=>{
  const x=setup(kind);x.reasons[0]=kind==='types'?{kind:8,typeReference:'other'}:{kind:6,index:1};assert.equal(x.run()[0].reason,'missing_inclusion');
  x.reasons[0]=kind==='types'?{kind:5,typeReference:'fixture'}:{kind:7,index:0};assert.equal(x.run()[0].reason,'missing_inclusion');
});
for(const kind of ['types','lib'])test(`${kind} absent cache cannot be repaired from inclusion alone`,()=>{
  const x=setup(kind);if(kind==='types')x.program.getAutomaticTypeDirectiveResolutions=()=>undefined;else x.program.resolvedLibReferences=undefined;assert.equal(x.run()[0].reason,'unprocessed');
});
test('type name/index mismatch cannot borrow a cache entry',()=>{const x=setup();x.names[0]='other';assert.equal(x.run()[0].reason,'unprocessed');});
for(const kind of ['types','lib'])test(`${kind} same-field different object and unsafe identity do not prove membership`,()=>{
  let x=setup(kind);x.program.getSourceFile=()=>({...x.target});assert.equal(x.run()[0].reason,'target_not_in_program');
  x=setup(kind);assert.equal(observeEntries(x.ts,x.program,()=>null)[0].reason,'target_not_in_program');
});
test('configured lib missing actual path remains unresolved',()=>{const x=setup('lib');x.program.resolvedLibReferences.set('lib.es5.d.ts',{});assert.equal(x.run()[0].reason,'unresolved');});
test('default selection requires exactly one indexless lib inclusion',()=>{
  const x=setup('lib');delete x.options.lib;x.reasons[0]={kind:6};assert.equal(x.run()[0].status,'observed');
  x.reasons[0]={kind:6,index:0};assert.equal(x.run()[0].reason,'unprocessed');
  x.reasons[0]={kind:6};x.includes.set('other-path',[{kind:6}]);assert.throws(x.run,/unsupported_input/);
});
test('default cached inclusion with absent SourceFile remains unproven',()=>{const x=setup('lib');delete x.options.lib;x.reasons[0]={kind:6};x.program.getSourceFile=()=>undefined;assert.equal(x.run()[0].reason,'target_not_in_program');});
test('entry ledger overflow aborts rather than truncating configured or automatic names',()=>{
  for(const automatic of [false,true]){const x=setup();x.program.getRootFileNames=()=>[];
    if(automatic){delete x.options.types;x.program.getAutomaticTypeDirectiveNames=()=>Array(REFERENCE_LIMIT+1).fill('fixture');}
    else x.options.types=Array(REFERENCE_LIMIT+1).fill('fixture');assert.throws(x.run,/budget_exceeded/);}
});
