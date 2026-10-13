'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('fs'),os=require('os'),path=require('path'),cp=require('child_process');
const TS='/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js';
for(const [literal,strict] of [['"use strict"',true],["'use strict'",true],['"use\\x20strict"',false],['"use\\u0020strict"',false],['"use str\\u0069ct"',false],['"use relaxed"',false]])test('directive raw bytes '+literal,()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'pd-directive-'));
 try{
  const source=`function q(eval: number) { ${literal}; let eval; use(eval); }`;fs.writeFileSync(path.join(root,'case.ts'),source);
  const owner={name:'q',start_line:1,start_byte:0,end_byte:Buffer.byteLength(source)};
  const row={from:{file:'case.ts',owner},to:{file:'case.ts',owner}};
  fs.writeFileSync(path.join(root,'census.jsonl'),JSON.stringify({file:'case.ts',owner,source,admitted:false})+'\n');
  fs.writeFileSync(path.join(root,'details.jsonl'),JSON.stringify({row,class:'LOST',verdict:{step1:'CORRECT'}})+'\n');
  cp.execFileSync(process.execPath,[path.join(__dirname,'r9-admissibility.cjs'),TS,root,path.join(root,'details.jsonl'),path.join(root,'census.jsonl'),path.join(root,'result.json')]);
  const result=JSON.parse(fs.readFileSync(path.join(root,'result.json')));
  assert.deepEqual(result.counts,{['3|LOST|'+(strict?'WRONG':'CORRECT')]:1});
  assert.deepEqual(result.raw,{'3|LOST|CORRECT':1});
  assert.equal(result.STOP,!strict);
  assert.equal(result.inadmissible_proofs.length,strict?0:1);
  const emitted=`function q(eval) { ${literal}; return eval; }`;
  if(strict)assert.throws(()=>new Function('return ('+emitted+');'),SyntaxError);else assert.doesNotThrow(()=>new Function('return ('+emitted+');'));
 }finally{fs.rmSync(root,{recursive:true,force:true});}
});
