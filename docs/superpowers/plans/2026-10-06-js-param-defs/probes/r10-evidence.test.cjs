'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {classify,identifierName}=require('./r9-early-errors.cjs');
const {bucket,rowBucket,afterOverride}=require('./r10-buckets.cjs');
for(const [source,status] of [
 ['function q(π){let π;}','EARLY_ERROR'],
 ['function q(\\u03c0){let π;}','EARLY_ERROR'],
 ['function q(𐐀){let 𐐀;}','EARLY_ERROR'],
 ['((a.b)=>1)','EARLY_ERROR'],
 ['function q([a.b]){}','EARLY_ERROR'],
 ['((a.b: number)=>1)','INADMISSIBLE'],
 ['function q([a: number]){}','INADMISSIBLE'],
 ['function q(a: number){}','INADMISSIBLE'],
 ['static m(){}','INADMISSIBLE'],
 ['function q(){super()}','INADMISSIBLE'],
 ['function q(a.x){}','INADMISSIBLE']
])test(status+' '+source,()=>{
 let error=null;try{new Function('return ('+source+');');}catch(e){error={name:e.name,message:e.message};}
 assert.equal(classify([error,error]).status,status);
 assert.equal(afterOverride('CORRECT',classify([error,error])),status==='EARLY_ERROR'?'WRONG':'CORRECT');
});
test('Unicode identifier message validation',()=>{
 for(const name of ['a','$a','π','a\u200c','\\u03c0','\\u{10400}'])assert(identifierName(name),name);
 for(const name of ['1a','a-b','a b','\\u{110000}','\\uD800',"a' b"])assert(!identifierName(name),name);
 assert.equal(classify([{name:'SyntaxError',message:"Identifier 'a-b' has already been declared"}]).status,'INADMISSIBLE');
 assert.equal(classify([null,{name:'SyntaxError',message:"Identifier 'a' has already been declared"}]).status,'VALID');
});
test('three buckets and cross-file priority',()=>{
 assert.equal(bucket('case.js',[{code:1487}]),'2');
 assert.equal(bucket('case.jsx',[{code:8010}]),'1');
 assert.equal(bucket('case.js',[{code:1005},{code:8010}]),'1');
 assert.equal(bucket('case.ts',[{code:8010}]),'3');
 assert.equal(bucket('case.js',[]),'3');
 const files=new Map([['a.js','1'],['b.js','2']]);
 assert.equal(rowBucket({from:{file:'a.js'},to:{file:'b.js'}},files),'1');
 assert.equal(rowBucket({from:{file:'c.js'},to:{file:'b.js'}},files),'2');
});
