// Fixed-population source audit, not a producer or an authority consumer.
// node audit-merged-wildcard.mjs <typescript.js> <source-root> <packet.json>
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {canonical,hash,parsePacket,COMPILER_HASH} from '../../../scripts/callable-observations/schema.mjs';
const [compiler,root,packetFile]=process.argv.slice(2);
assert(compiler&&root&&packetFile,'compiler, source root and packet required');
assert.equal(hash(fs.readFileSync(compiler)),COMPILER_HASH);
const ts=createRequire(import.meta.url)(compiler),bytes=fs.readFileSync(packetFile),p=parsePacket(bytes.toString());
const baseline=JSON.parse(fs.readFileSync(new URL('./2026-09-07-callable-singleton-wildcard-evidence.json',import.meta.url)));
assert.equal(hash(bytes),baseline.packet_sha256,'entire packet, including closure, must be unchanged');
const inventory=new Map(p.snapshot.files.map(f=>[f.id,f])),sources=new Map(),providers=[],augmentations=[];
const anchor=(n,sf)=>({file:sf.fileName,sha256:inventory.get(sf.fileName).sha256,kind:ts.SyntaxKind[n.kind],
  start_utf16:n.getStart(sf),end_utf16:n.end,start_byte:Buffer.byteLength(sf.text.slice(0,n.getStart(sf))),end_byte:Buffer.byteLength(sf.text.slice(0,n.end))});
for(const id of p.snapshot.program_files){
  const local=id.startsWith('project/')?path.join(root,id.slice(8)):path.join(path.dirname(compiler),id.slice(9));
  const data=fs.readFileSync(local);assert.equal(hash(data),inventory.get(id).sha256);
  const sf=ts.createSourceFile(id,data.toString(),ts.ScriptTarget.Latest,true);sources.set(id,sf);
  function visit(n){
    if(ts.isModuleDeclaration(n)&&ts.isStringLiteral(n.name)){
      const entry={name:n.name.text,anchor:anchor(n,sf),shape:!n.body?'shorthand':ts.isModuleBlock(n.body)&&n.body.statements.length===0?'empty_block':'other',
        top_level:n.parent===sf,declaration_file:sf.isDeclarationFile,external:ts.isExternalModule(sf),text:n.getText(sf)};
      (ts.isExternalModuleAugmentation(n)?augmentations:providers).push(entry);
    }
    ts.forEachChild(n,visit);
  }visit(sf);
}
const matches=(pattern,s)=>{const parts=pattern.split('*');return parts.length===2&&s.length>=parts[0].length+parts[1].length&&s.startsWith(parts[0])&&s.endsWith(parts[1]);};
const normalized=xs=>xs.map(canonical).sort();
const population=p.resolutions.filter(r=>r.lookup.wildcard?.reason==='duplicate_provider');assert.equal(population.length,84);
const rows=population.map(r=>{
  const l=r.lookup,w=l.wildcard,sf=sources.get(r.from);let use;
  function visit(n){if(ts.isStringLiteral(n)&&n.getStart(sf)===l.request.start_utf16&&n.end===l.request.end_utf16)use=n;ts.forEachChild(n,visit);}visit(sf);
  assert(use);assert.deepEqual(anchor(use,sf),l.request);assert.equal(use.text,r.specifier);
  assert(ts.isImportDeclaration(use.parent)&&use.parent.moduleSpecifier===use&&!use.parent.importClause,'side-effect import only');
  assert.equal(w.pattern,'*.scss');assert.equal(r.target,null);assert.equal(l.status,'unproven');
  const candidates=providers.filter(e=>matches(e.name,r.specifier)),augs=augmentations.filter(e=>e.name===r.specifier||matches(e.name,r.specifier));
  assert.equal(candidates.length,2);assert(candidates.every(e=>e.name==='*.scss'&&e.top_level&&e.declaration_file&&!e.external));
  assert.deepEqual(candidates.map(e=>e.shape).sort(),['empty_block','shorthand']);assert.equal(augs.length,0);
  assert.equal(providers.filter(e=>e.name===r.specifier).length,0);
  for(const entries of [l.declarations,w.providers,w.matches])assert.deepEqual(normalized(entries),normalized(candidates.map(e=>e.anchor)));
  assert.deepEqual(w.augmentations,[]);assert.deepEqual(l.providers,[]);assert.deepEqual(l.augmentations,[]);
  assert(r.specifier.startsWith('./')||r.specifier.startsWith('../'));
  const candidate=path.posix.normalize(path.posix.join(path.posix.dirname(r.from),r.specifier));assert(candidate.startsWith('project/'));
  const asset=inventory.get(candidate);if(asset)assert.equal(hash(fs.readFileSync(path.join(root,candidate.slice(8)))),asset.sha256);
  return {from:r.from,specifier:r.specifier,request:l.request,checker_declarations:l.declarations,
    side_effect_only:true,asset_inventory_candidate:candidate,asset_inventory_sha256:asset?.sha256??null};
});
console.log(JSON.stringify({schema:'prism.merged-wildcard-source-audit/1',audit_only:true,authorizes_runtime_edge:false,
  base:'d66328d1ad5b11ef937cd0823fac093f5e8c01bd',compiler_sha256:COMPILER_HASH,packet_sha256:hash(bytes),producer:p.producer,
  packet_byte_identical_to_pr274:true,program_source_files_checked:sources.size,binding_occurrences:rows.length,
  unique_request_files:new Set(rows.map(r=>r.from)).size,side_effect_only:rows.length,
  providers:providers.filter(e=>e.name==='*.scss'),competing_matching_patterns:0,matching_augmentations:0,exact_providers:0,
  asset_inventory_occurrences_present:rows.filter(r=>r.asset_inventory_sha256).length,
  unique_asset_inventory_candidates:new Set(rows.map(r=>r.asset_inventory_candidate)).size,
  asset_presence_authority:false,closure_admission:false,public_selected_value_declaration:'not measured by packet',
  rows},null,2));
