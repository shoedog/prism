'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('fs'),os=require('os'),path=require('path'),cp=require('child_process');
const shell=fs.readFileSync(path.join(__dirname,'../CONTROLLER-pd.sh'),'utf8');
const script=shell.split("<<'PY' >&3\n")[1].split('\nPY')[0];
for(const [bucket,proof,expected] of [['1','INADMISSIBLE','COMPLETE'],['2','INADMISSIBLE','STOP'],['3','INADMISSIBLE','STOP'],['2','EARLY_ERROR','COMPLETE'],['3','EARLY_ERROR','COMPLETE']])test(`F summary bucket ${bucket} proof ${proof}`,()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'pd-summary-'));
 try{
  const raw={[bucket+'|LOST|CORRECT']:1},counts={[bucket+'|LOST|'+(proof==='EARLY_ERROR'?'WRONG':'CORRECT')]:1};
  const files={'binding.json':{inputs:{}},'census.json':{files:1},'dfg-diff.json':{},'byte-diff.json':{LOST:1},'adjudication.json':{},'projection.json':{},'corrected-admissibility.json':{counts,raw,STOP:expected==='STOP',type_annotated_JS:bucket==='1'?['case.js']:[],other_diagnostic_JS:bucket==='2'?['case.js']:[],overrides:proof==='EARLY_ERROR'?[{}]:[],inadmissible_proofs:proof==='INADMISSIBLE'?[{}]:[]}};
  for(const [f,v] of Object.entries(files))fs.writeFileSync(path.join(root,f),JSON.stringify(v));
  for(const f of ['base.sites.jsonl','head.sites.jsonl'])fs.writeFileSync(path.join(root,f),'');
  const result=JSON.parse(cp.execFileSync('python3',['-c',script,root,'diff'],{encoding:'utf8'}));
  assert.equal(result.status,expected);assert.deepEqual(result.corrected_admissibility.raw,raw);assert.deepEqual(result.corrected_admissibility.after_override,counts);
 }finally{fs.rmSync(root,{recursive:true,force:true});}
});
for(const bucket of ['1','2','3'])test(`F summary added WRONG bucket ${bucket}`,()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'pd-summary-added-'));
 try{
  const counts={[bucket+'|ADDED|WRONG']:1};
  const files={'binding.json':{inputs:{}},'census.json':{files:1},'dfg-diff.json':{},'byte-diff.json':{ADDED:1},'adjudication.json':{},'projection.json':{},'corrected-admissibility.json':{counts,raw:counts,STOP:bucket!=='1',type_annotated_JS:bucket==='1'?['case.js']:[],other_diagnostic_JS:bucket==='2'?['case.js']:[],overrides:[],inadmissible_proofs:[]}};
  for(const [f,v] of Object.entries(files))fs.writeFileSync(path.join(root,f),JSON.stringify(v));
  for(const f of ['base.sites.jsonl','head.sites.jsonl'])fs.writeFileSync(path.join(root,f),'');
  const result=JSON.parse(cp.execFileSync('python3',['-c',script,root,'diff'],{encoding:'utf8'}));
  assert.equal(result.status,bucket==='1'?'COMPLETE':'STOP');
  assert.deepEqual(result.stops,bucket==='1'?[]:['ADDED WRONG binding']);
 }finally{fs.rmSync(root,{recursive:true,force:true});}
});
