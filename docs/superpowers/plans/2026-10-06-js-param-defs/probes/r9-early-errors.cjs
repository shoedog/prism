'use strict';
const fs=require('fs'),assert=require('assert');
// Each exact message or anchored, identifier-specific pattern has a positive fixture.
const allowlist=[
 {id:'duplicate-formal',message:'Duplicate parameter name not allowed in this context',source:'(a,a)=>a'},
 {id:'strict-non-simple',message:"Illegal 'use strict' directive in function with non-simple parameter list",source:'function q(a=0){"use strict";return a}'},
 {id:'lexical-redeclaration',pattern:"^Identifier '(.+)' has already been declared$",source:'function q(a){let a;}'},
 {id:'strict-eval-arguments',message:'Unexpected eval or arguments in strict mode',source:'function q(arguments){"use strict";}'},
 {id:'invalid-destructuring-target',message:'Invalid destructuring assignment target',source:'((a.b)=>1)'},
 {id:'illegal-declaration-property',message:'Illegal property in declaration context',source:'function q([a.b]){}'}
];
function identifierName(raw){
 const decoded=raw.replace(/\\u(?:\{([0-9a-fA-F]{1,6})\}|([0-9a-fA-F]{4}))/g,(_,wide,narrow)=>{
  const cp=parseInt(wide||narrow,16);return cp<=0x10ffff?String.fromCodePoint(cp):'\\';
 });
 return /^[$_\p{ID_Start}][$_\u200c\u200d\p{ID_Continue}]*$/u.test(decoded);
}
function matches(a,error){
 if(error.name!=='SyntaxError')return false;
 if(a.message)return a.message===error.message;
 const match=new RegExp(a.pattern,'u').exec(error.message);
 return !!match&&identifierName(match[1]);
}
function classify(errors){
 const proofs=errors.filter(Boolean).map(e=>({error:e,allow:allowlist.find(a=>matches(a,e))?.id||null}));
 // One context-appropriate compilation suffices for an early-error proof.
 // Success in either wrapper defeats a claimed callable early error.
 if(errors.some(e=>e===null))return {status:'VALID',proofs};
 return {status:proofs.some(e=>e.allow)?'EARLY_ERROR':'INADMISSIBLE',proofs};
}
function compile(source){try{new Function('return ('+source+');');return null;}catch(e){return {name:e.name,message:e.message};}}
if(require.main===module){
 const fixtures=allowlist.map(a=>{const error=compile(a.source);assert(error);assert(matches(a,error),JSON.stringify({a,error}));return {...a,error};});
 for(const source of ['function q(π){let π;}','function q(\\u03c0){let π;}','function q(𐐀){let 𐐀;}']){
  const error=compile(source);assert.equal(classify([error,error]).status,'EARLY_ERROR');fixtures.push({source,error});
 }
 const negatives=['function q(a: string){}','static m(){}','function q(){super()}','function q(a.x){}','((a.b: number)=>1)','function q([a: number]){}'].map(source=>{const error=compile(source);assert(error);assert(!classify([error,error]).proofs.some(p=>p.allow));return {source,error,status:'INADMISSIBLE'};});
 console.log(JSON.stringify({node:process.version,allowlist:fixtures,negative_controls:negatives},null,2));
}
module.exports={allowlist,classify,identifierName};
