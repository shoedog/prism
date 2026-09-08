// Read-only fixed-public-source audit. No package installation or closure admission.
// node audit-type-lib-observations.mjs <compiler> <source> <packet> <previous-packet> <previous-cache> <source-manifest>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
import {parsePacket,hash,canonical,COMPILER_HASH} from '../../../scripts/callable-observations/schema.mjs';
const [compiler,root,packetFile,previousFile,cacheFile,manifestFile]=process.argv.slice(2);assert(manifestFile);
assert.equal(hash(fs.readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);
const bytes=fs.readFileSync(packetFile),p=parsePacket(bytes.toString()),previousBytes=fs.readFileSync(previousFile),previous=parsePacket(previousBytes.toString());
assert.equal(hash(previousBytes),'7808361410f98f5c984c1585bd7ba69578b3a1f5b6bf8771e15c70d9f83d89dd');
assert.equal(p.schema,'prism.callable-observation/11');assert.equal(p.producer.version,'0.12.0');
for(const key of Object.keys(previous))if(!['schema','producer','reasons','closure'].includes(key))assert.deepEqual(p[key],previous[key],key);
assert.deepEqual(p.reasons,[...previous.reasons,'unproven_type_lib_reference'].sort());assert.deepEqual(p.closure,{...previous.closure,references:false});
const manifest=JSON.parse(fs.readFileSync(manifestFile));assert.equal(manifest.manifest_sha256,'353187a695df2683a3631e4739c173cb6d33190901093549b61e28efeda60cbb');
assert.equal(hash(JSON.stringify(manifest.manifest)),manifest.manifest_sha256);assert.equal(manifest.manifest.length,1229);
const inventory=new Map(p.snapshot.files.map(f=>[f.id,f]));
function read(id){const physical=id.startsWith('project/')?path.join(root,id.slice(8)):path.join(path.dirname(compiler),id.slice(9));const bytes=fs.readFileSync(physical);assert.equal(hash(bytes),inventory.get(id)?.sha256,id);return bytes;}
const tracked=new Map();for(const f of manifest.manifest){const bytes=read('project/'+f.file);assert.equal(hash(bytes),f.sha256);assert.equal(bytes.length,f.size);tracked.set(f.file,bytes);}
const config=JSON.parse(tracked.get('tsconfig.json'));const converted=ts.convertCompilerOptionsFromJson(config.compilerOptions,'/__prism__/project');assert.deepEqual(converted.errors,[]);
const census=[];for(const id of p.snapshot.program_files){const data=read(id),text=data.toString('utf8'),sf=ts.createSourceFile(id,text,ts.ScriptTarget.Latest,true);
  for(const [kind,refs] of [['types',sf.typeReferenceDirectives],['lib',sf.libReferenceDirectives]])for(const [index,ref] of refs.entries()){
    const mode=kind==='types'?(ref.resolutionMode??ts.getDefaultResolutionModeForFileWorker(sf,converted.options)):undefined;assert.equal(mode,undefined,'fixed public configuration has no mode-dependent directive');
    census.push({kind,index,name:ref.fileName,mode:null,request:{file:id,sha256:hash(data),kind:kind==='types'?'TypeReferenceDirective':'LibReferenceDirective',start_utf16:ref.pos,end_utf16:ref.end,
      start_byte:Buffer.byteLength(text.slice(0,ref.pos)),end_byte:Buffer.byteLength(text.slice(0,ref.end))}});
  }
}
const projection=({kind,index,name,mode,request})=>({kind,index,name,mode,request}),sorted=xs=>xs.map(canonical).sort();
assert.deepEqual(sorted(p.type_lib_references.map(projection)),sorted(census));assert.equal(census.length,139);
const cacheBytes=fs.readFileSync(cacheFile);assert.equal(hash(cacheBytes),'e9f39176f6902eb340a79e92b737da58a6f46006723c8e0b2a946f83e0a309f3');
const cache=JSON.parse(cacheBytes),normalize=f=>{assert(f.startsWith('/__prism__/'));return f.slice(11);};
const key=r=>canonical([r.from,r.kind,r.name,r.start_utf16,r.end_utf16]);
const cached=new Map(cache.map(r=>[key({...r,from:normalize(r.from)}),r.target===null?null:normalize(r.target)]));assert.equal(cached.size,139);
for(const r of p.type_lib_references){const k=key({from:r.request.file,kind:r.kind,name:r.name,start_utf16:r.request.start_utf16,end_utf16:r.request.end_utf16});assert(cached.has(k));assert.equal(r.target,cached.get(k));
  if(r.target!==null){assert.equal(r.status,'observed');assert.equal(r.inclusion,true);}else{assert.equal(r.reason,'unresolved');assert.equal(r.inclusion,false);}}
const gaps=p.type_lib_references.filter(r=>r.status==='unproven');assert.equal(gaps.length,1);assert.equal(gaps[0].name,'react-scripts');assert.equal(gaps[0].request.file,'project/packages/excalidraw/react-app-env.d.ts');
const packages=[...tracked].filter(([f])=>path.posix.basename(f)==='package.json').map(([file,b])=>({file,json:JSON.parse(b)}));
const fields=['dependencies','devDependencies','peerDependencies','optionalDependencies','resolutions'];
const declared=packages.flatMap(p=>fields.flatMap(field=>Object.entries(p.json[field]??{}).filter(([name,value])=>name.includes('react-scripts')||String(value).includes('react-scripts')).map(([name,value])=>({file:p.file,field,name,value}))));
assert.deepEqual(declared,[]);assert(!tracked.get('yarn.lock').toString().includes('react-scripts'));
assert(!p.snapshot.files.some(f=>/(^|\/)node_modules\/(?:@types\/)?react-scripts\//.test(f.id)));
const rootPackage=packages.find(p=>p.file==='package.json').json,appPackage=packages.find(p=>p.file==='excalidraw-app/package.json').json;
assert.equal(rootPackage.devDependencies.vite,'5.0.12');assert(appPackage.scripts['build:app'].includes('vite build'));
const environment=tracked.get('packages/excalidraw/vite-env.d.ts').toString();assert(environment.includes('reference types="vite/client"'));assert(environment.includes('reference types="vite-plugin-svgr/client"'));
assert.equal(tracked.get('packages/excalidraw/react-app-env.d.ts').toString(),'/// <reference types="react-scripts" />\n');
const evidenceFiles=['package.json','excalidraw-app/package.json','packages/excalidraw/package.json','tsconfig.json','yarn.lock','packages/excalidraw/react-app-env.d.ts','packages/excalidraw/vite-env.d.ts'];
console.log(JSON.stringify({schema:'prism.type-lib-source-audit/1',audit_only:true,closure_admission:false,authorizes_runtime_edge:false,
  packet_sha256:hash(bytes),producer:p.producer,previous_packet_sha256:hash(previousBytes),previous_cache_sha256:hash(cacheBytes),source_manifest_sha256:manifest.manifest_sha256,
  unchanged_fields:Object.keys(previous).filter(k=>!['schema','producer','reasons','closure'].includes(k)),changed_previous_fields:['schema','producer','reasons','closure'],
  program_files:p.snapshot.program_files.length,source_directives:{types:census.filter(r=>r.kind==='types').length,lib:census.filter(r=>r.kind==='lib').length,total:139,observed:138,unproven:1},
  gaps,disposition:{classification:'source-declared unprovisioned type directive',acquisition_failure_proven:false,tracked_package_manifests:packages.length,manifest_declarations:declared,lockfile_mentions:0,installed_provider_files:0,
    app_build:appPackage.scripts['build:app'],vite_version:rootPackage.devDependencies.vite,remaining_cra_lint_tooling:rootPackage.devDependencies['eslint-config-react-app'],
    inference:'Likely obsolete CRA environment reference alongside existing Vite declarations; origin/removal safety is not proven by shallow source history.',
    recommendation:'Owner review of candidate removal as likely obsolete, or explicit provisioning if still intended; neither performed. Do not silently install react-scripts, substitute vite/client, or waive the reference.',
    source_changes:false,evidence:evidenceFiles.map(file=>({file,sha256:hash(tracked.get(file))}))},references:p.type_lib_references},null,2));
