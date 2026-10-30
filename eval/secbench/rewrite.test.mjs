import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import {inspectEntry,loadCompiler} from './inspect.mjs';
import {rewrite} from './rewrite.mjs';
const ts=loadCompiler(process.env.PRISM_TYPESCRIPT||'/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js');
function fixture(source,pattern,exploit="require('p').run('payload')",sink='index.js:2:10') {
  const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'secbench-rewrite-test-')),input=path.join(tmp,'inputs'),packages=path.join(tmp,'packages'),root=path.join(packages,'command-injection/p_1/src/package');
  fs.mkdirSync(root,{recursive:true});fs.mkdirSync(path.join(input,'command-injection/p_1'),{recursive:true});
  fs.writeFileSync(path.join(root,'index.js'),source);fs.writeFileSync(path.join(root,'package.json'),'{}');
  fs.writeFileSync(path.join(input,'command-injection/p_1/p.test.js'),exploit);fs.writeFileSync(path.join(input,'command-injection/p_1/package.json'),'{}');
  try {
    const row=inspectEntry(ts,{class:'command-injection',entry:'p_1',deps:{p:'1'},sink},input,packages);
    const value=rewrite(ts,root,row,pattern),output=fs.readFileSync(path.join(root,'index.js'),'utf8');
    return {value,output};
  } finally {fs.rmSync(tmp,{recursive:true,force:true});}
}
function execute(source,args) {
  // Only safe test fixtures, never acquired packages or exploit code.
  const context={module:{exports:{}},exec:x=>x};vm.runInNewContext(source,context);
  return context.module.exports.run.apply({marker:42},args);
}
test('bare-use rewrite preserves strict directive, member result and bytes',()=>{
  const source='function run(x){"use strict";\n  return exec(x.cmd);\n}\nmodule.exports={run};';
  const {value,output}=fixture(source,'member');
  assert.equal(execute(output,[{cmd:'data'}]),execute(source,[{cmd:'data'}]));
  assert.match(output,/"use strict";;void x;/);
  const p=value.row.source.data_parameters[0];assert.equal(Buffer.from(output).subarray(p.start_byte,p.end_byte).toString(),'x');
  const o=value.row.sink.value_occurrences[0];assert.equal(Buffer.from(output).subarray(o.start_byte,o.end_byte).toString(),'x');
  assert.equal(Buffer.from(output).subarray(value.row.sink.end_byte-1,value.row.sink.end_byte).toString(),'}');
});
test('rest normalization preserves arity, array identity, lexical arguments/this and zero arguments',()=>{
  const source='function run(...args){\n  return exec([args,arguments.length,this.marker,run.length]);\n}\nmodule.exports={run};';
  const {value,output}=fixture(source,'rest');
  for(const args of [[],['payload'],['payload','second']])assert.equal(JSON.stringify(execute(output,args)),JSON.stringify(execute(source,args)));
  assert.equal(value.row.source.data_parameters[0].rest,false);
  const p=value.row.source.data_parameters[0];assert.equal(Buffer.from(output).subarray(p.start_byte,p.end_byte).toString(),p.names[0]);
});
test('legacy arguments checkpoint preserves object semantics and refuses eval scope',()=>{
  const source='function run(x){\n  return exec([arguments[0],arguments.length,this.marker]);\n}\nmodule.exports={run};';
  const {output}=fixture(source,'arguments');
  assert.equal(JSON.stringify(execute(output,['payload','second'])),JSON.stringify(execute(source,['payload','second'])));
  assert.throws(()=>fixture('function run(...args){\n  return exec(eval(args[0]));\n}\nmodule.exports={run};','rest'),/reflective scope/);
});
test('capture hoist preserves same lexical scope and maps moved terminal',()=>{
  const source='function run(x){\n  return new Promise(function(resolve){\n    exec(x);\n  });\n}\nmodule.exports={run};';
  const {value,output}=fixture(source,'callback_capture',"require('p').run('payload')",'index.js:3:5');
  assert.match(output,/const __secbench_callback=\(value=>value\)\(function/);
  const o=value.row.sink.value_occurrences[0];assert.equal(Buffer.from(output).subarray(o.start_byte,o.end_byte).toString(),'x');
  assert.equal(value.row.sink.nested_callback,false);
});
test('registration rewrite refuses a named source outside a top-level expression',()=>{
  assert.throws(()=>fixture('function run(x){\n exec(x);\n}\nmodule.exports={run};','callback_registration',"require('p').run('payload')",'index.js:2:2'),/not a top-level expression/);
});
test('callback hoist preserves anonymous name and refuses conditional scope changes',()=>{
  const source='function run(x){\n return ((cb)=>cb.name)(function(){\n  exec(x);\n });\n}\nmodule.exports={run};';
  const {output}=fixture(source,'callback_capture',undefined,'index.js:3:3');
  assert.equal(execute(output,[]),execute(source,[]));
  assert.throws(()=>fixture('function run(x){\n if(x) consume(function(){\n  exec(x);\n });\n}\nmodule.exports={run};','callback_capture',undefined,'index.js:3:3'),/conditional|scope/);
});
test('rest rewrite refuses shadowing declarations and introspection; preserves property names',()=>{
  const source='function run(...args){\n return exec({args:args, nested:(args)=>args});\n}\nmodule.exports={run};';
  assert.throws(()=>fixture(source,'rest',undefined,'index.js:2:9'),/shadow/);
  const simple='function run(...args){\n return exec({args:args});\n}\nmodule.exports={run};';
  const {output}=fixture(simple,'rest',undefined,'index.js:2:9');
  assert.deepEqual(JSON.parse(JSON.stringify(execute(output,[]))),JSON.parse(JSON.stringify(execute(simple,[]))));
  assert.throws(()=>fixture('function run(...args){\n return exec([args,run.toString()]);\n}\nmodule.exports={run};','rest',undefined,'index.js:2:9'),/reflect/);
});
test('rest shorthand terminal keeps the value byte identity and legacy normalization refuses multiple inputs',()=>{
  const {value,output}=fixture('function run(...args){\n return exec({args});\n}\nmodule.exports={run};','rest',undefined,'index.js:2:9');
  const o=value.row.sink.value_occurrences[0];
  assert.equal(Buffer.from(output).subarray(o.start_byte,o.end_byte).toString(),o.name);
  assert.throws(()=>fixture('function run(x,y){\n return exec(arguments[0]);\n}\nmodule.exports={run};','arguments',undefined,'index.js:2:9'),/broaden source identity/);
});
