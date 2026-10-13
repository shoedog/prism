'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('fs'),os=require('os'),path=require('path');
const {census}=require(process.env.E5_CENSUS_WORKER||'./census-e5.cjs');
const tsPath=path.join(os.homedir(),'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js');

function row(callee,caller,{facts=false,module=false,binding={Unproven:'unbound'}}={}) {
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'e5-step1-oracle-'));
 try {
  fs.writeFileSync(path.join(root,'callee.js'),callee);
  fs.writeFileSync(path.join(root,'caller.js'),caller);
  const start=caller.indexOf('f(3)'),end=start+4;assert.ok(start>=0);
  const site={record_kind:'call_site',caller:{file:'caller.js',name:'caller',start_line:1,end_line:1},source_span:{file:'caller.js',line:1,start_byte:Buffer.byteLength(caller.slice(0,start)),end_byte:Buffer.byteLength(caller.slice(0,end))},callee_text:'f',origin:'Source',resolved_targets:[{kind:'free_single',confidence:'exact',function_id:{file:'callee.js',name:'f',start_line:1,end_line:1}}]};
  const sites=path.join(root,'sites.jsonl');fs.writeFileSync(sites,JSON.stringify(site)+'\n');
  const options={};
  if(facts) {
   options.factsFile=path.join(root,'facts.jsonl');
   fs.writeFileSync(options.factsFile,JSON.stringify({caller:site.caller,span:[site.source_span.start_byte,site.source_span.end_byte],name:'f',origin:'Source',target:site.resolved_targets[0].function_id,local_binding:binding,jsx:false,qualifier:null,forms:[{form:'function_declaration',name_explicit:true,esm:module,kind:'function_declaration',start_byte:module?7:0,end_byte:Buffer.byteLength(callee)}]})+'\n');
  }
  const result=census(tsPath,root,sites,options);assert.equal(result.rows.length,1);
  return result.rows[0];
 } finally {fs.rmSync(root,{recursive:true});}
}

test('R-local prices scalar require costs and every-require exemption with binding controls',()=>{
 const scalar='module.exports = function f(x){ return x; };';
 const cases=[
  ['literal scalar require',scalar,'const f = require("./callee"); function caller(){ return f(3); }','local_declaration',true,false,true],
  ['dynamic scalar require',scalar,'const f = require(key); function caller(){ return f(3); }','local_declaration',true,false,false],
  ['wrapped require initializer',scalar,'const f = wrap(require("./callee")); function caller(){ return f(3); }','local_declaration',true,false,false],
  ['member require initializer',scalar,'const f = require("./callee").f; function caller(){ return f(3); }','local_declaration',true,false,false],
  ['ordinary local alias',scalar,'const f = other.cb; function caller(){ return f(3); }','local_declaration',true,true,false],
  ['local parameter',scalar,'function caller(f){ return f(3); }','local_declaration',true,true,false],
  ['local binding element',scalar,'const { f } = other; function caller(){ return f(3); }','local_declaration',true,true,false],
  ['local function',scalar,'function caller(){ function f(x){ return x; } return f(3); }','local_declaration',true,true,false],
  ['local class',scalar,'function caller(){ class f {} return f(3); }','local_declaration',true,true,false],
  ['unbound name',scalar,'function caller(){ return f(3); }','none',false,false,null],
  ['named import','export function f(x){ return x; }','import { f } from "./callee"; function caller(){ return f(3); }','imported',false,false,null],
  ['destructured require','exports.f = function f(x){ return x; };','const { f } = require("./callee"); function caller(){ return f(3); }','require_destructuring',false,false,null],
 ];
 const failures=[];
 for(const [label,callee,caller,axis,local,exempted,literal] of cases) {
  const r=row(callee,caller);
  try {
   assert.equal(r.binding,axis);
   assert.equal(r.rules['R-local'],local);
   assert.equal(r.rules['R-local-no-require'],exempted);
   if(literal!==null) {
    assert.equal(r.binding_evidence.literal_scalar_require,literal);
    assert.equal(r.binding_evidence.require_initialized,!exempted);
   }
   if(label==='literal scalar require')assert.equal(r.verdict,'CORRECT');
  } catch(error) {failures.push(label+': '+error.message);}
 }
 assert.deepEqual(failures,[], 'complete predicate/binding/require population');
});

test('R-union census covers each arm, their overlap and script/import negative controls',()=>{
 const script='function f(x){ return x; }',module='export function f(x){ return x; }';
 const cases=[
  ['a only',module,'function caller(){ return f(3); }',true,false,true,{Unproven:'unbound'}],
  ['b only',script,'export function caller(){ return f(3); }',false,true,true,{Unproven:'unbound'}],
  ['both',module,'export function caller(){ return f(3); }',true,true,true,{Unproven:'unbound'}],
  ['script negative',script,'function caller(){ return f(3); }',false,false,false,{Unproven:'unbound'}],
  ['import negative',module,'import { f } from "./callee"; function caller(){ return f(3); }',false,false,false,{Unproven:'import'}],
 ];
 const failures=[];
 for(const [label,callee,caller,a,b,union,binding] of cases) {
  const r=row(callee,caller,{facts:true,module:callee===module,binding});
  try {assert.deepEqual([r.rules['R-recommended'],r.rules['R-bound'],r.rules['R-union']],[a,b,union]);}
  catch(error) {failures.push(label+': '+error.message);}
 }
 assert.deepEqual(failures,[], 'both arms and preserving controls');
});
