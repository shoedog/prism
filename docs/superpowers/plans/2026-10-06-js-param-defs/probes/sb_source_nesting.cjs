// PR-B probe: nesting class of each SecBench source callable (top_level / under_named / under_anon).
// Usage: node sb_source_nesting.cjs TS_JS ENTRIES.jsonl PACKAGES_ROOT
const ts=require(process.argv[2]),fs=require('fs'),path=require('path');
const rows=fs.readFileSync(process.argv[3],'utf8').split('\n').filter(Boolean).map(JSON.parse);
const pk=process.argv[4];const agg={};
for(const r of rows){const f=path.join(pk,r.class,r.entry,'src/package',r.source.file);
 const sf=ts.createSourceFile(f,fs.readFileSync(f,'utf8'),ts.ScriptTarget.ESNext,true);
 let hit=null;const visit=n=>{if(!hit&&ts.isFunctionLike(n)&&sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1===r.source.start_line&&n.parameters&&n.parameters.some(p=>r.source.data_parameters.some(d=>d.names.includes(p.name.getText(sf)))))hit=n;ts.forEachChild(n,visit)};visit(sf);
 let k='no_callable';if(hit){k='top_level';for(let p=hit.parent;p;p=p.parent)if(ts.isFunctionLike(p)){k=(p.name||ts.isVariableDeclaration(p.parent)||ts.isPropertyAssignment(p.parent))?'under_named':'under_anon';if(k==='under_named')break;}}
 agg[k]=(agg[k]||0)+1;}
console.log(JSON.stringify(agg));
