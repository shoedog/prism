#!/usr/bin/env node
// PR-B joint-population counterfactual (planner probe; never executes package code).
// For each callback_argument_parameter_registration row, copy the package and insert ONE bare
// read `;void <payload>;` as the first statement of the anonymous source callback (the R1 `member`
// checkpoint WITHOUT the R1 callback-naming rewrite). The callback stays anonymous, so this
// isolates what PR-B's synthetic identity contributes to the PR-B+PR-C joint population.
// Row byte coordinates after the insertion point are remapped. Output: rows.jsonl for measure().
'use strict';
const fs=require('fs'),path=require('path');
const [tsPath,entries,pkgs,out]=process.argv.slice(2);const ts=require(tsPath);
const rows=fs.readFileSync(entries,'utf8').split('\n').filter(Boolean).map(JSON.parse);
fs.mkdirSync(path.join(out,'pkgs'),{recursive:true});
const outRows=[],skipped=[];
for(const r of rows){
 const src=path.join(pkgs,r.class,r.entry,'src/package');const dst=path.join(out,'pkgs',r.class,r.entry,'src/package');
 fs.mkdirSync(path.dirname(dst),{recursive:true});fs.cpSync(src,dst,{recursive:true});
 const file=path.join(dst,r.source.file),text=fs.readFileSync(file,'utf8');
 const sf=ts.createSourceFile(file,text,ts.ScriptTarget.ESNext,true);
 const names=r.source.data_parameters.flatMap(d=>d.names);
 let fn=null;const visit=n=>{if(!fn&&ts.isFunctionLike(n)&&n.body&&ts.isBlock(n.body)&&sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1===r.source.start_line&&
   n.parameters.some(p=>names.includes(p.name.getText(sf))))fn=n;ts.forEachChild(n,visit)};visit(sf);
 if(!fn){skipped.push(r.entry);continue;}
 const at=fn.body.getStart(sf)+1,ins=';'+names.map(n=>`void ${n};`).join('');
 // Byte delta (inserted text is ASCII); positions are UTF-16 in TS, convert to UTF-8 byte offset.
 const atByte=Buffer.byteLength(text.slice(0,at),'utf8'),delta=Buffer.byteLength(ins,'utf8');
 if(sf.getLineAndCharacterOfPosition(at).line+1!==r.source.start_line){skipped.push(r.entry);continue;}
 fs.writeFileSync(file,text.slice(0,at)+ins+text.slice(at));
 const shift=(o,f)=>{if(o&&f===r.source.file){for(const k of ['start_byte','end_byte'])if(Number.isInteger(o[k])&&o[k]>=atByte)o[k]+=delta;}};
 const row=JSON.parse(JSON.stringify(r));delete row.invocations;delete row.outcome;delete row.trace_detail;delete row.first_break;
 delete row.callee_tolerant_outcome;delete row.heuristic_break;delete row.attribution_status;delete row.dfg_stats;delete row.error_mechanism;
 shift(row.source,row.source.file);
 for(const p of row.source.parameters||[])shift(p,row.source.file);
 for(const p of row.source.data_parameters){shift(p,row.source.file);p.bare_references=(p.bare_references||0)+1;}
 for(const o of row.sink.value_occurrences||[])shift(o,row.sink.file);
 shift(row.sink,row.sink.file);
 row.checkpoint={file:r.source.file,byte:atByte,inserted:ins};
 outRows.push(row);
}
fs.writeFileSync(path.join(out,'rows.jsonl'),outRows.map(x=>JSON.stringify(x)).join('\n')+'\n');
console.log(JSON.stringify({rows:rows.length,checkpointed:outRows.length,skipped}));
