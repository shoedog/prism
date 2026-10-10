'use strict';
const fs=require('fs'),assert=require('assert');
// Each exact message or anchored, identifier-specific pattern has a positive fixture.
const allowlist=[
 {id:'duplicate-formal',message:'Duplicate parameter name not allowed in this context',source:'(a,a)=>a'},
 {id:'strict-non-simple',message:"Illegal 'use strict' directive in function with non-simple parameter list",source:'function q(a=0){"use strict";return a}'},
 {id:'lexical-redeclaration',pattern:"^Identifier '[A-Za-z_$][A-Za-z0-9_$]*' has already been declared$",source:'function q(a){let a;}'},
 {id:'strict-eval-arguments',message:'Unexpected eval or arguments in strict mode',source:'function q(arguments){"use strict";}'}
];
function classify(errors){
 const proofs=errors.filter(Boolean).map(e=>({error:e,allow:allowlist.find(a=>e.name==='SyntaxError'&&(a.message===e.message||a.pattern&&new RegExp(a.pattern).test(e.message)))?.id||null}));
 // One context-appropriate compilation suffices for an early-error proof.
 // Success in either wrapper defeats a claimed callable early error.
 if(errors.some(e=>e===null))return {status:'VALID',proofs};
 return {status:proofs.some(e=>e.allow)?'EARLY_ERROR':'INADMISSIBLE',proofs};
}
function compile(source){try{new Function('return ('+source+');');return null;}catch(e){return {name:e.name,message:e.message};}}
if(require.main===module){
 const fixtures=allowlist.map(a=>{const error=compile(a.source);assert(error);assert(a.message===error.message||a.pattern&&new RegExp(a.pattern).test(error.message),JSON.stringify({a,error}));return {...a,error};});
 const negatives=['function q(a: string){}','static m(){}','function q(){super()}','function q(a.x){}'].map(source=>{const error=compile(source);assert(error);assert(!classify([error,error]).proofs.some(p=>p.allow));return {source,error,status:'INADMISSIBLE'};});
 console.log(JSON.stringify({node:process.version,allowlist:fixtures,negative_controls:negatives},null,2));
}
module.exports={allowlist,classify};
