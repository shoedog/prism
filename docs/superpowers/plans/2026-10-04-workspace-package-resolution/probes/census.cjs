// Public or controller-private census. Independent pinned TypeScript ProjectService.
// node census.cjs TS_JS ROOT BASE_SITES FACTS OUT_DIRECTORY
'use strict';
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const [tsPath,rootArg,sitesPath,factsPath,out]=process.argv.slice(2);
const ts=require(tsPath),root=fs.realpathSync(rootArg);
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
if(ts.version!=='5.9.3'||sha(tsPath)!=='3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675')throw Error('oracle drift');
const lines=p=>fs.readFileSync(p,'utf8').trim().split('\n').filter(Boolean).map(JSON.parse);
const facts=lines(factsPath),sites=lines(sitesPath).filter(r=>r.record_kind==='call_site');
if(!facts.length||!sites.length)throw Error('empty probe');
const rel=p=>path.relative(root,p).split(path.sep).join('/');
const inside=p=>rel(p)!=='..'&&!rel(p).startsWith('../')&&!path.isAbsolute(rel(p));
const indexed=new Set(facts.map(f=>fs.realpathSync(path.join(root,f.file))));
const inputs=new Map(),host={...ts.sys,getCurrentDirectory:()=>root,writeFile(){throw Error('oracle writes refused');},watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout(){},readFile(p){const t=ts.sys.readFile(p);if(t!==undefined)inputs.set(p,sha(p));return t;}};
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
const service=new ts.server.ProjectService({host,logger,cancellationToken:{isCancellationRequested:()=>false},useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
const packages=[];
function walk(d){for(const e of fs.readdirSync(d,{withFileTypes:true})){if(e.name.startsWith('.')||['node_modules','target','dist','build','vendor'].includes(e.name)||e.isSymbolicLink())continue;const p=path.join(d,e.name);if(e.isDirectory())walk(p);else if(e.name==='package.json'){try{const v=JSON.parse(host.readFile(p));if(v.name)packages.push({name:v.name,dir:path.dirname(p),metadata:v});}catch{}}}}
walk(root);
function installed(file,name){let d=path.dirname(file);while(inside(d)){const p=path.join(d,'node_modules',name);try{fs.lstatSync(p);return p;}catch{}if(d===root)break;d=path.dirname(d);}return null;}
const counts=Object.fromEntries(['paths','workspace','installed_external','node_builtin','other_scheme','unresolved'].map(k=>[k,0])),occurrences={...counts};
const records=[],potential=[];
for(const f of facts){const absolute=path.join(root,f.file);const expected=Array.isArray(f.hash)?Buffer.from(f.hash).toString('hex'):f.hash;if(sha(absolute)!==expected)throw Error('facts drift');const source=host.readFile(absolute);const refs=ts.preProcessFile(source,true,true).importedFiles.filter(x=>!ts.isExternalModuleNameRelative(x.fileName)&&!path.isAbsolute(x.fileName));if(!refs.length)continue;
 service.openClientFile(absolute,undefined,undefined,root);const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(absolute),true);const opts=project.getCompilerOptions();
 const sf=project.getLanguageService().getProgram()?.getSourceFile(absolute);
 const modeFor=pos=>{let mode;function visit(n){if(pos>=n.pos&&pos<=n.end){if(ts.isImportDeclaration(n)||ts.isExportDeclaration(n)||ts.isImportEqualsDeclaration(n))mode=ts.getModeForUsageLocation(sf,n.moduleSpecifier||n.moduleReference?.expression,opts);ts.forEachChild(n,visit);}}if(sf)visit(sf);return mode;};
 for(const specifier of new Set(refs.map(x=>x.fileName))){const ref=refs.find(x=>x.fileName===specifier),mode=modeFor(ref.pos);const r=ts.resolveModuleName(specifier,absolute,opts,host,undefined,undefined,mode),m=r.resolvedModule;let canonical=null;try{if(m)canonical=fs.realpathSync(m.resolvedFileName);}catch{}
  const name=specifier.startsWith('@')?specifier.split('/').slice(0,2).join('/'):specifier.split('/')[0],pkg=packages.filter(p=>p.name===name),entry=installed(absolute,name);
  let category=f.modules?.[specifier]?'paths':specifier.startsWith('node:')?'node_builtin':/^[A-Za-z][A-Za-z0-9+.-]*:/.test(specifier)?'other_scheme':pkg.length?'workspace':entry?'installed_external':'unresolved';
  const record={writer:f.file,specifier,occurrences:refs.filter(x=>x.fileName===specifier).length,category,owner:project.projectKind===ts.server.ProjectKind.Configured?rel(project.getProjectName()):null,project_kind:project.projectKind,module_resolution:ts.getEmitModuleResolutionKind(opts),resolution_mode:mode??null,target:m?rel(m.resolvedFileName):null,canonical:canonical&&inside(canonical)?rel(canonical):canonical,indexed:!!canonical&&indexed.has(canonical),workspace_candidates:pkg.map(p=>rel(p.dir)),installed_entry:entry?rel(entry):null,main_proof:f.modules?.[specifier]||null,failed_lookups:(r.failedLookupLocations||[]).filter(p=>inside(p)).map(rel)};
  counts[category]++;occurrences[category]+=record.occurrences;records.push(record);
  if(category==='workspace')for(const s of sites.filter(s=>s.caller.file===f.file&&!s.exact_target)){const binding=(f.bindings||[]).find(b=>b.module_path===specifier&&(b.local===s.callee_text||b.local===s.qualifier));if(binding)potential.push({key:[s.caller.file,s.source_span.start_byte,s.source_span.end_byte,s.callee_text],writer:f.file,specifier,native_resolves_indexed:record.indexed,base:s});}
 }
 service.closeClientFile(absolute);
}
for(const [p,h]of inputs)if(sha(p)!==h)throw Error('oracle inputs drift');
fs.mkdirSync(out,{recursive:true});const summary={oracle_version:ts.version,oracle_sha256:sha(tsPath),sites:sites.length,indexed_js_files:facts.length,pairs:records.length,occurrences:Object.values(occurrences).reduce((a,b)=>a+b,0),counts,occurrence_counts:occurrences,workspace_native_indexed:records.filter(r=>r.category==='workspace'&&r.indexed).length,workspace_native_unresolved:records.filter(r=>r.category==='workspace'&&!r.target).length,potential_direct_sites:potential.length,potential_native_indexed_sites:potential.filter(p=>p.native_resolves_indexed).length};
fs.writeFileSync(path.join(out,'census.json'),JSON.stringify({summary,packages:packages.map(p=>({name:p.name,dir:rel(p.dir),metadata:p.metadata})),records,potential,inputs:Object.fromEntries(inputs)},null,2)+'\n');console.log(JSON.stringify(summary));
