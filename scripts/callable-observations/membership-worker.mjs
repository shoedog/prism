// Separate research observations; no owner-proof consumer or change to schema20.
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {build} from './worker.mjs';
import {settings} from './index.mjs';
import {snapshot} from './inventory.mjs';
import {canonical,hash,parsePacket} from './schema.mjs';
const requireThat=(ok,reason)=>{if(!ok)throw Error(reason);};

function selections(ts,packet,inventory) {
  return packet.config_provenance.files.map(entry=>{
    const text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(inventory.read(entry.file));
    requireThat(hash(text)===entry.sha256,'membership_identity');
    const source=ts.parseJsonText(entry.file,text);
    requireThat(!source.parseDiagnostics.length,'membership_config');
    function inspect(node) {
      if(ts.isObjectLiteralExpression(node)) {
        const names=new Set();
        for(const p of node.properties) {
          requireThat(ts.isPropertyAssignment(p)&&ts.isStringLiteral(p.name),'membership_config');
          requireThat(!names.has(p.name.text),'membership_config');names.add(p.name.text);
        }
      }
      ts.forEachChild(node,inspect);
    }
    inspect(source);
    const object=source.statements[0]?.expression;
    requireThat(object&&ts.isObjectLiteralExpression(object),'membership_config');
    const fields=[];
    for(const p of object.properties)if(['files','include','exclude'].includes(p.name.text)) {
      requireThat(ts.isArrayLiteralExpression(p.initializer)&&p.initializer.elements.every(e=>
        ts.isStringLiteral(e)&&!e.text.includes('${configDir}')),'membership_config');
      const start=p.getStart(source),end=p.end;
      fields.push({name:p.name.text,values:p.initializer.elements.map(e=>e.text),anchor:{file:entry.file,
        sha256:entry.sha256,start_byte:Buffer.byteLength(text.slice(0,start)),end_byte:Buffer.byteLength(text.slice(0,end))}});
    }
    return {file:entry.file,sha256:entry.sha256,fields};
  });
}
export function deriveMembership(input) {
  const options=settings(input);let facts;
  const packet=build(options,({program,ts,anchor})=>{
    // Reuse the worker's byte-backed canonicalization, including links and case.
    facts=program.getSourceFiles().map(sf=>({id:anchor(sf).file,sha256:hash(sf.text),size:Buffer.byteLength(sf.text),
      declaration:sf.isDeclarationFile,json:sf.scriptKind===ts.ScriptKind.JSON})).sort((a,b)=>a.id<b.id?-1:1);
  });
  parsePacket(JSON.stringify(packet));
  requireThat(packet.compiler.verified&&packet.closure.stable_snapshot,'membership_unavailable');
  requireThat(packet.config_provenance.status==='observed'&&
    !packet.reasons.some(r=>['invalid_config','unsupported_references','unsupported_plugins'].includes(r)),'membership_config');
  requireThat(canonical(facts.map(f=>f.id))===canonical(packet.snapshot.program_files),'membership_identity');
  const inventory=snapshot(options);
  requireThat(inventory.digest===packet.snapshot.sha256,'unstable_snapshot');
  const roots=new Set(packet.snapshot.roots);
  const program=facts.map(f=>{
    const row=inventory.files.get(f.id);
    requireThat(row&&row.sha256===f.sha256&&row.size===f.size,'membership_identity');
    return {...f,domain:f.id.startsWith('compiler/')?'compiler':f.id.includes('/node_modules/')?'dependency':'repository',root:roots.has(f.id)};
  });
  const ts=createRequire(import.meta.url)(options.compiler);
  return {packet,selection:selections(ts,packet,inventory),program};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  try{console.log(JSON.stringify(deriveMembership(JSON.parse(readFileSync(0,'utf8')))));}
  catch(error){console.log(JSON.stringify({unavailable:['budget_exceeded','unsupported_input','compiler_mismatch','unstable_snapshot',
    'membership_unavailable','membership_config','membership_identity'].includes(error.message)?error.message:'worker_failed'}));}
}
