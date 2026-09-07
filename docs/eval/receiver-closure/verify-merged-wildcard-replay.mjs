// Reproduce the historical source census and check the new lane separately.
// node verify-merged-wildcard-replay.mjs <compiler> <source-root> <base-packet> <packet>
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {createRequire} from 'node:module';import {fileURLToPath} from 'node:url';
import {parsePacket,canonical,hash,COMPILER_HASH} from '../../../scripts/callable-observations/schema.mjs';
const [compiler,root,baseFile,packetFile]=process.argv.slice(2);assert(compiler&&root&&baseFile&&packetFile);
assert.equal(hash(fs.readFileSync(compiler)),COMPILER_HASH);const ts=createRequire(import.meta.url)(compiler);
const raw=spawnSync(process.execPath,[fileURLToPath(new URL('./audit-merged-wildcard.mjs',import.meta.url)),compiler,root,baseFile],{encoding:'utf8',maxBuffer:32*1024*1024});
assert.equal(raw.status,0,raw.stderr);const census=JSON.parse(raw.stdout);assert.equal(census.binding_occurrences,84);
const old=JSON.parse(fs.readFileSync(baseFile)),bytes=fs.readFileSync(packetFile),p=parsePacket(bytes.toString());
const project=p=>{const q=structuredClone(p);delete q.schema;delete q.producer;for(const r of q.resolutions)delete r.lookup.merged_wildcard;return q;};
assert.deepEqual(project(p),project(old),'all old fields and occurrence ordering must be unchanged');
const priorByRequest=new Map(census.rows.map(r=>[canonical(r.request),r]));
const sources=new Map();let checked=0;
function sourceEntry(e){const a=e.declaration;let sf=sources.get(a.file);
  if(!sf){assert(a.file.startsWith('project/'));const data=fs.readFileSync(path.join(root,a.file.slice(8)));assert.equal(hash(data),a.sha256);sf=ts.createSourceFile(a.file,data.toString(),ts.ScriptTarget.Latest,true);sources.set(a.file,sf);}
  let node;function visit(n){if(ts.isModuleDeclaration(n)&&n.getStart(sf)===a.start_utf16&&n.end===a.end_utf16)node=n;ts.forEachChild(n,visit);}visit(sf);
  assert(node);assert.equal(hash(Buffer.from(sf.text)),a.sha256);assert.equal(a.kind,'ModuleDeclaration');
  assert.equal(Buffer.byteLength(sf.text.slice(0,node.getStart(sf))),a.start_byte);assert.equal(Buffer.byteLength(sf.text.slice(0,node.end)),a.end_byte);
  assert.equal(node.name.text,'*.scss');assert.equal(node.parent,sf);assert(sf.isDeclarationFile&&!ts.isExternalModule(sf));
  assert.equal(e.shape,!node.body?'shorthand':ts.isModuleBlock(node.body)&&node.body.statements.length===0?'empty_block':'other');checked++;
}
const rows=p.resolutions.filter(r=>r.lookup.merged_wildcard).map(r=>{
  const m=r.lookup.merged_wildcard,prior=priorByRequest.get(canonical(r.lookup.request));assert(prior);assert.equal(prior.side_effect_only,true);
  assert.equal(m.status,'observed');assert.equal(m.reason,null);assert.equal(r.lookup.wildcard.reason,'duplicate_provider');
  assert.deepEqual(m.declarations.map(e=>e.declaration),prior.checker_declarations);m.declarations.forEach(sourceEntry);sourceEntry(m.selected);
  assert(m.declarations.some(e=>canonical(e)===canonical(m.selected)));
  return {from:r.from,specifier:r.specifier,request:r.lookup.request,declarations:m.declarations,selected:m.selected};
});
assert.equal(rows.length,84);assert.equal(new Set(rows.map(r=>canonical(r.request))).size,84);
const count=xs=>xs.reduce((out,x)=>(out[x]=(out[x]??0)+1,out),{});
const classes=p.observations.flatMap(o=>o.nested.calls).map(c=>c.props_class).filter(c=>c.class_declaration);
assert.equal(classes.length,4);assert(classes.every(c=>c.reason==='program_unproven'&&c.status==='unproven'));
console.log(JSON.stringify({schema:'prism.merged-wildcard-replay/1',audit_only:true,authorizes_runtime_edge:false,
  base:'e45eaef07e24764597bfbd904aeca83771f97cbb',compiler_sha256:COMPILER_HASH,producer:p.producer,
  base_packet_sha256:hash(fs.readFileSync(baseFile)),packet_sha256:hash(bytes),old_projection_equal:true,
  historical_source_census_reproduced:true,program_source_files_checked:census.program_source_files_checked,
  merged_observed:rows.length,selected_shapes:count(rows.map(r=>r.selected.shape)),selected_files:count(rows.map(r=>r.selected.declaration.file)),
  source_entry_occurrences_checked:checked,asset_presence_authority:false,closure_admission:false,
  resolutions:p.resolutions.length,null_filesystem_targets:p.resolutions.filter(r=>r.target===null).length,
  exact_observed:p.resolutions.filter(r=>r.lookup.status==='observed').length,
  singleton_observed:p.resolutions.filter(r=>r.lookup.wildcard?.status==='observed').length,
  legacy_wildcard_duplicates:p.resolutions.filter(r=>r.lookup.wildcard?.reason==='duplicate_provider').length,
  refusals:p.snapshot.refused_lookup_sha256.length,outside_lookups:p.snapshot.outside_lookups,closure:p.closure,reasons:p.reasons,
  observations:p.observations.length,nested_calls:p.observations.reduce((n,o)=>n+o.nested.calls.length,0),class_candidates_still_unproven:classes.length,rows},null,2));
