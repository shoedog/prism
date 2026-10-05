import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {inspectEntry,syntaxCounts,loadCompiler} from './inspect.mjs';
const compiler=process.env.PRISM_TYPESCRIPT||'/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c-installed/source/node_modules/typescript/lib/typescript.js';
const ts=loadCompiler(compiler);
function fixture(source,exploit,sink='index.js:2:3',cls='command-injection',extraFiles={}) {
  const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'secbench-test-'));
  const input=path.join(tmp,'inputs'),pkgs=path.join(tmp,'packages'),relative=cls+'/p_1';
  fs.mkdirSync(path.join(input,relative),{recursive:true});fs.mkdirSync(path.join(pkgs,relative,'src/package'),{recursive:true});
  fs.writeFileSync(path.join(input,relative,'p.test.js'),exploit);
  fs.writeFileSync(path.join(input,relative,'package.json'),JSON.stringify({dependencies:{p:'1'},sink}));
  fs.writeFileSync(path.join(pkgs,relative,'src/package/package.json'),'{}');
  fs.writeFileSync(path.join(pkgs,relative,'src/package/index.js'),source);
  for(const [name,content] of Object.entries(extraFiles)) {
    const file=path.join(pkgs,relative,'src/package',name);
    fs.mkdirSync(path.dirname(file),{recursive:true});fs.writeFileSync(file,content);
  }
  try{return inspectEntry(ts,{class:cls,entry:'p_1',id:'test',deps:{p:'1'},sink},input,pkgs);}
  finally{fs.rmSync(tmp,{recursive:true,force:true});}
}
const basic='function run(x, cb) {\n  exec(x, () => cb());\n}\nmodule.exports = {run};';
test('derive export and test-fed data parameter; omit callbacks at sink',()=>{
  const r=fixture(basic,"const p=require('p'); p.run('payload', () => {});");
  assert.equal(r.gt_status,'available');assert.equal(r.source.name,'run');
  assert.deepEqual(r.source.supplied_ordinals,[0]);assert.deepEqual(r.sink.value_names,['x']);
  assert.equal(r.syntactic_path.length,1);
});
test('require property alias resolves exported member',()=>{
  assert.equal(fixture(basic,"const run=require('p').run; run('payload');").gt_status,'available');
});
test('require destructuring alias resolves exported member',()=>{
  assert.equal(fixture(basic,"const {run:r}=require('p'); r('payload');").gt_status,'available');
});
test('non-exported same-name function is not used as fallback',()=>{
  assert.equal(fixture(basic,"const p=require('p'); p.missing('payload');").gt_reason,'unresolved_export_or_api_chain');
});
test('multiple API functions are unavailable',()=>{
  const src=basic+'\nfunction other(x){return x;}\nmodule.exports.other=other;';
  assert.equal(fixture(src,"const p=require('p');p.run('a');p.other('b');").gt_reason,'multiple_exported_entry_functions');
});
test('invalid or out-of-range sink is unavailable',()=>{
  assert.equal(fixture(basic,"require('p').run('a')",'index.js:0:1').gt_reason,'invalid_sink_coordinate');
  assert.equal(fixture(basic,"require('p').run('a')",'index.js:99:1').gt_reason,'sink_out_of_range');
});
test('HTTP-only entry has no invented exported source',()=>{
  assert.equal(fixture(basic,"require('http').get('http://localhost/payload')").gt_reason,'no_direct_package_api_call_with_data');
});
test('syntax ignores strings and comments; counts all forms',()=>{
  const sf=ts.createSourceFile('x.js',`// function f({...x}){}\nconst text='...x {a}';\nfunction f({a}, ...args){const {b,...rest}=a;return {...rest, c:[...args]};} Object.assign({}, text);`,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
  const c=syntaxCounts(ts,sf);
  assert.equal(c.destructure_parameter,1);assert.equal(c.destructure_declaration,1);
  assert.equal(c.rest_parameter,1);assert.equal(c.rest_binding,1);assert.equal(c.spread_object,1);
  assert.equal(c.spread_argument_or_array,1);assert.equal(c.object_assign,1);
  const negative=syntaxCounts(ts,ts.createSourceFile('x.js',"const text='...args'; // {...x}",ts.ScriptTarget.Latest,true));
  assert.equal(negative.spread_object+negative.rest_parameter+negative.destructure_parameter,0);
});
test('rest and destructured data parameters retain binding names',()=>{
  const r=fixture('function run({x}, ...rest) {\n  exec(x);\n}\nmodule.exports={run};',"require('p').run({x:'payload'}, 'more')");
  assert.equal(r.gt_status,'available');assert.equal(r.source.data_parameters[0].destructure,true);
  assert.equal(r.source.parameters[1].rest,true);assert.equal(r.source.data_parameters.length,1);
  const both=fixture('function run({x}, ...rest) {\n  exec(x);\n}\nmodule.exports={run};',"require('p').run({x:'payload'}, '__proto__')");
  assert.equal(both.source.data_parameters[1].rest,true);
});
test('parser rejects ambiguous repeated exports and malformed source',()=>{
  assert.equal(fixture(basic+'\nmodule.exports={run};',"require('p').run('payload')").gt_reason,'unresolved_export_or_api_chain');
  assert.equal(fixture(basic,"require('p').run(;").gt_reason,'exploit_parse_diagnostics');
});
test('class constructor and object spread have no property name; no parser crash',()=>{
  const r=fixture('class API {constructor(){} run(x){\n  exec(x);\n}}\nmodule.exports=API;',"const API=require('p');const p=new API();p.run({...{a:'payload'}});");
  assert.equal(r.gt_status,'available');assert.equal(r.source.name,'run');
});
test('HTTP setup is not the payload source; forwarded request is',()=>{
  const r=fixture(basic,"const p=require('p');p.run(3000);const cmd=`curl --path-as-is http://localhost/../flag`;",'index.js:2:3','path-traversal');
  assert.equal(r.gt_reason,'http_payload_not_exported_api_argument');
  const forwarded=fixture(basic,"const p=require('p');http.createServer((req,res)=>p.run(req,res));const cmd=`curl --path-as-is http://localhost/../flag`;",'index.js:2:3','path-traversal');
  assert.equal(forwarded.gt_status,'available');assert.deepEqual(forwarded.source.supplied_ordinals,[0]);
});
test('literal or initializer payload selects its parameter instead of benign target',()=>{
  const r=fixture('function run(target, key) {\n  exec(key);\n}\nmodule.exports={run};',"const p=require('p');const evil='__proto__.polluted';p.run({},evil);");
  assert.deepEqual(r.source.supplied_ordinals,[1]);
});
test('path sink excludes stream options; ReDoS includes receiver',()=>{
  const src='function run(file) {\n  fs.createReadStream(file, {start: range});\n}\nmodule.exports={run};';
  assert.deepEqual(fixture(src,"require('p').run('../flag')",'index.js:2:3','path-traversal').sink.value_names,['file']);
  const regex='function run(str) {\n  str.match(pattern);\n}\nmodule.exports={run};';
  assert.deepEqual(fixture(regex,"require('p').run('a'.repeat(1000))",'index.js:2:3','redos').sink.value_names,['pattern','str']);
});
test('BOM preserves raw UTF8 coordinates and sink occurrence identity',()=>{
  const r=fixture('\uFEFF'+basic,"require('p').run('payload')");
  assert.equal(r.source.parameters[0].start_byte,16);
  assert.equal(r.sink.value_occurrences.length,1);
  assert.equal(r.sink.value_occurrences[0].name,'x');
});
test('returned API payload cannot be misbound to the factory setup argument',()=>{
  const r=fixture('module.exports=function setup(config){\n  return function(key){exec(key);};\n};',"const p=require('p');const f=p({});f.call({},'__proto__.polluted');",'index.js:2:29');
  assert.equal(r.gt_reason,'gt_parser_unsupported_returned_api');
});
test('member sink requires member value rather than base-only use',()=>{
  const r=fixture('function run(fn) {\n  eval(wrap(fn.name));\n}\nmodule.exports={run};',"require('p').run(payload)");
  const field=r.sink.value_occurrences.find(o=>o.name==='fn');
  assert.equal(field.path,'fn.name');assert.equal(field.line_occurrences,1);
});
test('unmarked multiple data parameters stay unavailable; sole input is unambiguous',()=>{
  const src='function run(target, data) {\n  exec(data);\n}\nmodule.exports={run};';
  assert.equal(fixture(src,"require('p').run({}, unknown)").gt_reason,'ambiguous_payload_parameter');
  assert.equal(fixture(basic,"require('p').run(unknown)").source.binding_mode,'single_supplied_data_parameter');
});
test('invalid UTF8 is retained as unavailable ground truth and unparsed census',()=>{
  const r=fixture(Buffer.concat([Buffer.from([0xff]),Buffer.from(basic)]),"require('p').run('payload')");
  assert.equal(r.gt_status,'gt_unavailable');assert.equal(r.gt_reason,'invalid_utf8_ground_truth');
  assert.equal(r.census.unparsed_files.length,1);assert.equal(r.census.files,0);
  assert.ok(r.identities.some(f=>f.path==='index.js'));
});
test('sink basename repair requires one candidate; unsafe and missing paths stay unavailable',()=>{
  const source='module.exports=require("./lib/api");',exploit="require('p').run('payload')";
  const r=fixture(source,exploit,'api.js:2:3','command-injection',{'lib/api.js':basic});
  assert.equal(r.gt_status,'available');assert.equal(r.sink.basename_repair,true);
  assert.equal(r.sink.file,'lib/api.js');
  assert.equal(fixture(source,exploit,'api.js:2:3','command-injection',{'lib/api.js':basic,'other/api.js':basic}).gt_reason,'ambiguous_sink_basename');
  assert.equal(fixture(basic,exploit,'missing.js:2:3').gt_reason,'missing_sink_file');
  assert.equal(fixture(basic,exploit,'../index.js:2:3').gt_reason,'unsafe_sink_path');
});
test('default source binding is recorded and callback-only calls have no data source',()=>{
  const r=fixture("function run(x='default') {\n  exec(x);\n}\nmodule.exports={run};","require('p').run('payload')");
  assert.equal(r.source.data_parameters[0].default,true);
  assert.equal(fixture(basic,"require('p').run(()=>{})").gt_reason,'no_direct_package_api_call_with_data');
});
test('compiler hash mismatch is rejected before the module is loaded',()=>{
  const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'secbench-compiler-'));
  try {
    const file=path.join(tmp,'compiler.js');fs.writeFileSync(file,'throw new Error("must not load");');
    assert.throws(()=>loadCompiler(file),/TypeScript compiler pin mismatch/);
  } finally {fs.rmSync(tmp,{recursive:true,force:true});}
});
