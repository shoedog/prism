'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const fs=require('fs'),os=require('os'),path=require('path'),cp=require('child_process');
const script=process.env.R1_CHECKPOINT_SCRIPT||path.join(__dirname,'sb_joint_checkpoint.cjs');
const compiler='/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js';

function capture(text){
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'r1-checkpoint-'));
 const root=path.join(dir,'pkgs/fixture/entry/src/package');fs.mkdirSync(root,{recursive:true});
 const data=Buffer.from(text),start=data.indexOf('function'),parameter=data.indexOf('input');
 fs.writeFileSync(path.join(root,'x.js'),data);
 const row={class:'fixture',entry:'entry',source:{file:'x.js',name:'<anonymous>',start_line:1,
  start_byte:start,end_byte:data.lastIndexOf('}')+1,
  data_parameters:[{names:['input'],start_byte:parameter,end_byte:parameter+5}]},sink:{file:'x.js'}};
 fs.writeFileSync(path.join(dir,'entries.jsonl'),JSON.stringify(row)+'\n');
 const run=cp.spawnSync(process.execPath,[script,compiler,path.join(dir,'entries.jsonl'),path.join(dir,'pkgs'),path.join(dir,'out')],{encoding:'utf8'});
 assert.equal(run.status,0,run.stderr);
 const rows=fs.readFileSync(path.join(dir,'out/rows.jsonl'),'utf8').trim().split('\n').filter(Boolean).map(JSON.parse);
 return {dir,rows,summary:JSON.parse(run.stdout),data:fs.readFileSync(path.join(dir,'out/pkgs/fixture/entry/src/package/x.js'))};
}

for(const label of ['ascii','π'])test('physical callable span after insertion: '+label,()=>{
 const text=`call(function(input) { use(input); const label = '${label}'; });\n`;
 const result=capture(text);
 try{
  assert.equal(result.rows.length,1);
  const source=result.rows[0].source;
  assert.equal(result.data.subarray(source.start_byte,source.end_byte).toString(),
   `function(input) {;void input; use(input); const label = '${label}'; }`);
  for(const p of source.data_parameters)assert.equal(result.data.subarray(p.start_byte,p.end_byte).toString(),'input');
 }finally{fs.rmSync(result.dir,{recursive:true});}
});

test('multiline body-start checkpoint is refused',()=>{
 const result=capture('call(function(input)\n{ use(input); });\n');
 try{assert.equal(result.rows.length,0);assert.deepEqual(result.summary.skipped,['entry']);}
 finally{fs.rmSync(result.dir,{recursive:true});}
});
