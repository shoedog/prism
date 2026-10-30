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
  assert.equal(r.gt_reason,'no_unique_http_request_handler');
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
  assert.equal(r.gt_status,'available');assert.deepEqual(r.source.data_parameters[0].names,['key']);assert.equal(r.source.data_parameters[0].ordinal,0);
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

test('stable callback aliases never become payload evidence or terminal values',()=>{
  const src='function run(input, cb) {\n  exec("fixed", cb);\n}\nmodule.exports={run};';
  const r=fixture(src,"const done=()=>fs.writeFileSync('marker',''); require('p').run('payload',done);");
  assert.deepEqual(r.source.supplied_ordinals,[0]);
  assert.equal(r.gt_reason,'sink_has_no_data_value');
  assert.equal(fixture(basic,"const data='payload';const done=()=>{};require('p').run(data,done);").gt_status,'available');
});
test('lexical parameter and block shadowing cannot bind a package import',()=>{
  for(const exploit of ["const p=require('p');function use(p){p.run('payload')}use({run(x){return x}})",
    "const p=require('p');{const p={run(x){return x}};p.run('payload')}" ])
    assert.equal(fixture(basic,exploit).gt_status,'gt_unavailable');
  assert.equal(fixture(basic,"const p=require('p');function use(){p.run('payload')}use()").gt_status,'available');
});
test('reassigned package and exploit bindings refuse obsolete initializers',()=>{
  const src='function old(x){\n  exec(x);\n}\nfunction current(x){return x}\nlet run=old;run=current;module.exports={run};';
  assert.equal(fixture(src,"require('p').run('payload')").gt_status,'gt_unavailable');
  assert.equal(fixture(basic,"let p=require('p');p={run(x){return x}};p.run('payload')").gt_status,'gt_unavailable');
});
test('multiline terminal uses its own line and keeps metadata coordinates',()=>{
  const r=fixture('function run(x){\n  exec(\n    x\n  );\n}\nmodule.exports={run};',"require('p').run('payload')");
  assert.equal(r.sink.line,2);assert.equal(r.sink.value_occurrences[0].line,3);
  assert.equal(r.sink.value_occurrences[0].line_occurrences,1);
});
test('this member terminal keeps the consumed full path and use bytes',()=>{
  const r=fixture('function run(x){\n  this.command=x;exec(this.command);\n}\nmodule.exports={run};',"require('p').run('payload')",'index.js:2:18');
  assert.equal(r.sink.value_occurrences[0].path,'this.command');
  assert.equal(r.sink.value_occurrences[0].name,'this');
});
test('constant-only terminal is unavailable before querying Prism',()=>{
  assert.equal(fixture('function run(x){\n  exec("npm -v");\n}\nmodule.exports={run};',"require('p').run('payload')").gt_reason,'sink_has_no_data_value');
});
test('constructor setup yields to payload-bearing instance method',()=>{
  const src='function API(config){}\nAPI.prototype.run=function(x){\n  exec(x);\n};\nmodule.exports=API;';
  const r=fixture(src,"const API=require('p');const p=new API('setup');p.run('payload');",'index.js:3:3');
  assert.equal(r.gt_status,'available');assert.equal(r.source.name,'API.prototype.run');
});
test('HTTP source is unique registered request handler, with clone key',()=>{
  const src='const http=require("http");\nhttp.createServer(function(req,res){\n  fs.readFile(req.url);\n}).listen(3000);';
  const r=fixture(src,'const cmd="node ./node_modules/p/index.js & curl http://localhost/../flag";', 'index.js:3:3','path-traversal');
  assert.equal(r.gt_status,'available');assert.equal(r.source.binding_mode,'http_request_parameter');
  assert.deepEqual(r.source.data_parameters[0].names,['req']);assert.match(r.source.normalized_handler_sha256,/^[a-f0-9]{64}$/);
  const ambiguous=src+'\nhttp.createServer(function(req,res){fs.readFile(req.url)});';
  assert.equal(fixture(ambiguous,'const cmd="node ./node_modules/p/index.js & curl http://localhost/../flag";', 'index.js:3:3','path-traversal').gt_status,'available');
  assert.equal(fixture(src,'const cmd="curl http://localhost/../flag";', 'index.js:3:3','command-injection').gt_status,'gt_unavailable');
  const command=fixture(src,'const cmd="node ./node_modules/p/index.js & curl http://localhost/../flag";', 'index.js:3:3','command-injection');
  assert.equal(command.gt_status,'available');assert.equal(command.source.binding_mode,'http_request_parameter');
});
test('checker resolves attached static functions and returned APIs uniquely',()=>{
  const src='function api(){}\nfunction run(x){\n  exec(x);\n}\napi.run=run;module.exports=api;';
  const r=fixture(src,"require('p').run('payload')",'index.js:3:3');
  assert.equal(r.gt_status,'available');assert.equal(r.source.resolution_mode,'typescript_checker');
  const factory='module.exports=function setup(config){\n  return function run(x){exec(x);};\n};';
  const f=fixture(factory,"const p=require('p');const fn=p({});fn('payload');",'index.js:2:26');
  assert.equal(f.gt_status,'available');assert.deepEqual(f.source.data_parameters[0].names,['x']);
  const ambiguous='module.exports=function setup(c){return c?function a(x){exec(x)}:function b(x){exec(x)}};';
  assert.equal(fixture(ambiguous,"require('p')({})('payload')",'index.js:1:71').gt_status,'gt_unavailable');
});

test('directory sink coordinates and unbound fake HTTP registrations stay unavailable',()=>{
  assert.equal(fixture(basic,"require('p').run('payload')",'.:2:3').gt_reason,'sink_not_regular_file');
  const fake='const http={createServer(fn){}};\nhttp.createServer(function(req,res){\n fs.readFile(req.url);\n});';
  assert.equal(fixture(fake,'const cmd="node ./node_modules/p/index.js & curl http://localhost/../flag";', 'index.js:3:2','path-traversal').gt_status,'gt_unavailable');
});
test('mutated callable metadata consumed as data is separate from callback bodies',()=>{
  const source='function run(fn){\n  exec(fn.name);\n}\nmodule.exports={run};';
  const exploit="function data(){};Object.defineProperty(data,'name',{value:'payload'});require('p').run(data);";
  const r=fixture(source,exploit);assert.equal(r.gt_status,'available');
  assert.equal(r.source.calls[0].args[0].data_role,'function_metadata_value');
  assert.equal(fixture(source,"function data(){fs.writeFileSync('payload','')};require('p').run(data);").gt_status,'gt_unavailable');
});
test('export assignment is not callback-argument registration',()=>{
  const source='module.exports=function run(x){\n  exec(x);\n};';
  const r=fixture(source,"require('p')('payload')");
  assert.equal(r.source.callback_expression_statement,false);
});
