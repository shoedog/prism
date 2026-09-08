// Helper contract/edge controls with explicit cache doubles. Integration RED is
// separately captured with real pinned Programs in type-lib.test.mjs.
import test from 'node:test';import assert from 'node:assert/strict';import {observeTypeLib} from './type-lib.mjs';import {REFERENCE_LIMIT} from './schema.mjs';
function setup(){const text='/// <reference types="fixture" />',ref={fileName:'fixture',pos:22,end:29};
  const sf={fileName:'source.d.ts',path:'source-path',text,typeReferenceDirectives:[ref],libReferenceDirectives:[]},target={fileName:'target.d.ts',path:'target-path',text:'',typeReferenceDirectives:[],libReferenceDirectives:[]};
  const reasons=[{kind:5,file:sf.path,index:0}],sources=[sf,target];
  const program={getSourceFiles:()=>sources,getFileIncludeReasons:()=>new Map([[target.path,reasons]]),getCompilerOptions:()=>({}),getDefaultResolutionModeForFile:()=>undefined,
    getResolvedTypeReferenceDirectiveFromTypeReferenceDirective:()=>({resolvedTypeReferenceDirective:{resolvedFileName:target.fileName}}),getSourceFile:()=>target};
  const ts={ModuleKind:{CommonJS:1,ESNext:99}},toId=f=>'project/'+f,read=()=>text;return {sf,target,reasons,sources,program,ts,toId,read,run:()=>observeTypeLib(ts,program,toId,read)};}
test('exact cache target and occurrence inclusion satisfy helper contract',()=>{const x=setup();assert.equal(x.run()[0].status,'observed');});
for(const [field,value] of [['kind',7],['file','other-source'],['index',1]])test(`another occurrence cannot lend ${field} inclusion authority`,()=>{
  const x=setup();x.reasons[0][field]=value;assert.equal(x.run()[0].reason,'missing_inclusion');assert.equal(x.run()[0].target,null);
});
test('a same-field different SourceFile object is not Program membership',()=>{
  const x=setup();x.program.getSourceFile=()=>({...x.target});assert.equal(x.run()[0].reason,'target_not_in_program');
});
test('original-source text, owner and directive-name mismatch refuse',()=>{
  let x=setup();x.sf.text+=' ';assert.throws(x.run,/unsupported_input/);
  x=setup();x.sf.typeReferenceDirectives[0].fileName='changed';assert.throws(x.run,/unsupported_input/);
  x=setup();x.sf.redirectInfo={unredirected:{...x.sf,fileName:'different.d.ts'}};assert.throws(x.run,/unsupported_input/);
});
test('unknown mode and unsafe selected identity refuse',()=>{
  let x=setup();x.program.getDefaultResolutionModeForFile=()=>12;assert.throws(x.run,/unsupported_input/);
  x=setup();assert.equal(observeTypeLib(x.ts,x.program,f=>f===x.target.fileName?null:'project/'+f,x.read)[0].reason,'target_not_in_program');
});
test('source ledger overflow aborts instead of returning a truncated prefix',()=>{
  const x=setup();x.program.getCompilerOptions=()=>({noResolve:true});x.sf.typeReferenceDirectives=Array(REFERENCE_LIMIT+1).fill(x.sf.typeReferenceDirectives[0]);assert.throws(x.run,/budget_exceeded/);
});
