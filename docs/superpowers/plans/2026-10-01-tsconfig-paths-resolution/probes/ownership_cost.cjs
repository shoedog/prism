// Pre-fold cost: root-file selection is evaluated solely to count fall-through.
// Usage: node ownership_cost.cjs TS_JS ROOT ALIAS_SITES CHANGES OUT
const fs=require('fs'),path=require('path');
const [tsPath,root,aliasPath,changesPath,out]=process.argv.slice(2),ts=require(tsPath);
const aliases=JSON.parse(fs.readFileSync(aliasPath)),changes=new Set(JSON.parse(fs.readFileSync(changesPath)).map(c=>JSON.stringify(c.key))),cache=new Map();
function members(p){if(!cache.has(p)){const c=ts.getParsedCommandLineOfConfigFile(p,{}, {...ts.sys,onUnRecoverableConfigFileDiagnostic:()=>{}});cache.set(p,new Set(c?.fileNames.map(x=>path.resolve(x))||[]));}return cache.get(p);}
const rows=aliases.filter(a=>{if(!a.config)return false;const file=path.resolve(root,a.key[0]),selected=path.resolve(root,a.config);let d=path.dirname(file);while(d.startsWith(path.resolve(root))){const p=path.join(d,'tsconfig.json');if(p===selected)break;if(fs.existsSync(p)&&!members(p).has(file))return true;if(d===root)break;d=path.dirname(d);}return false;});
const result={alias_sites:aliases.length,fallthrough_sites:rows.length,fallthrough_changed_rows:rows.filter(r=>changes.has(JSON.stringify(r.key))).length,fallthrough_files:new Set(rows.map(r=>r.key[0])).size,rows:rows.map(r=>({key:r.key,selected:r.config,changed:changes.has(JSON.stringify(r.key))}))};
fs.writeFileSync(out,JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({...result,rows:undefined}));
