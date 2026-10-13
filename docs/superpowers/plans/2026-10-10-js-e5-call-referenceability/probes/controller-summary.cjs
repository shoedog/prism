'use strict';
const fs=require('fs'),{summarize}=require('./census-e5.cjs');
function summary(rows){const raw=rows.map(r=>({...r,verdict:r.raw_verdict}));return {raw:summarize(raw),after_override:summarize(rows),overrides:rows.filter(r=>r.override!==null&&r.override!==undefined).length};}
module.exports={summary};
if(require.main===module){const rows=fs.readFileSync(process.argv[2],'utf8').split('\n').filter(Boolean).map(JSON.parse);console.log(JSON.stringify(summary(rows)));}
