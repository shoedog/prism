'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('fs'),os=require('os'),path=require('path');
const ts=require('/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js');
const {prove}=require('./type-predicate-certificate.cjs');
function fixture(source,to,direction,alter={}){
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'pd-f9-'));fs.writeFileSync(path.join(root,'case.ts'),source);
 const start=source.indexOf('x:'),use=source.indexOf(to)+to.lastIndexOf('x'),byte=p=>Buffer.byteLength(source.slice(0,p));
 const owner={name:'q',start_line:1,start_byte:0,end_byte:Buffer.byteLength(source)};
 const endpoint=(p,access,parameter)=>({file:'case.ts',line:source.slice(0,p).split('\n').length,owner,start_byte:byte(p),end_byte:byte(p+1),path:{base:'x',fields:[]},access,parameter});
 const row={from:endpoint(start,'def',true),to:endpoint(use,'use',false)};Object.assign(row.to,alter);
 try{return prove(ts,root,[{row,direction}])[0].proof;}finally{fs.rmSync(root,{recursive:true,force:true});}
}
for(const suffix of ['x is string','asserts x','asserts x is string'])test('erased predicate '+suffix,()=>assert.equal(fixture(`function q(x: unknown): ${suffix} { return !!x; }`,suffix,'LOST').pass,true));
test('runtime read is checker-bound to formal',()=>{const p=fixture('function q(x: unknown): x is string { return !!x; }','!!x','ADDED');assert.equal(p.pass,true);assert.equal(p.checker_same_formal,true);});
test('shadowed read does not bind to formal',()=>{const p=fixture('function q(x: unknown): x is string { { let x; use(x); } return false; }','use(x','ADDED');assert.equal(p.checker_same_formal,false);assert.equal(p.pass,false);});
test('type occurrence is not runtime',()=>assert.equal(fixture('function q(x: unknown): x is string { return !!x; }','x is string','ADDED').pass,false));
test('runtime endpoint is not removed predicate',()=>assert.equal(fixture('function q(x: unknown): x is string { return !!x; }','!!x','LOST').pass,false));
test('different line is not admitted addition',()=>assert.equal(fixture('function q(x: unknown): x is string {\n return !!x;\n}','!!x','ADDED').pass,false));
test('wrong endpoint extent fails',()=>assert.equal(fixture('function q(x: unknown): x is string { return !!x; }','!!x','ADDED',{end_byte:999}).pass,false));
test('unicode prefix respects byte positions',()=>assert.equal(fixture('function q(x: unknown): x is string { const label="é"; return !!x; }','!!x','ADDED').pass,true));
test('simple assignment target is not a runtime read',()=>{const p=fixture('function q(x: unknown): x is string { x = 0; return false; }','x =','ADDED');assert.equal(p.checker_same_formal,true);assert.equal(p.pass,false);});
test('compound assignment still reads the formal',()=>assert.equal(fixture('function q(x: unknown): x is string { x += 1; return false; }','x +=','ADDED').pass,true));
for(const operator of ['of','in'])test('for-'+operator+' target is not a runtime read',()=>assert.equal(fixture(`function q(x: unknown): x is string { for (x ${operator} []) {} return false; }`,'for (x','ADDED').pass,false));
