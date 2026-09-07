// Worker confines synchronous compiler work to a parent-enforced timeout/heap cap.
import {readFileSync,readdirSync,lstatSync,realpathSync} from "node:fs";
import {createRequire} from "node:module";
import path from "node:path";
import {COMPILER_HASH,relative,hash,canonical} from "./schema.mjs";
import {emptyPacket} from "./index.mjs";
import {traceProvenance} from "./provenance.mjs";
import {observeNested} from "./nested.mjs";

const options=JSON.parse(readFileSync(0,"utf8"));
const fail=reason=>{throw Error(reason);};
function snapshot() {
  const files=new Map(),directories=new Map();let bytes=0;
  const roots={project:realpathSync(options.root),compiler:realpathSync(path.dirname(options.compiler))};
  if(roots.project===path.parse(roots.project).root || roots.project===roots.compiler) fail("unsupported_input");
  function walk(absolute,id,depth) {
    if(depth>options.limits.depth || files.size+directories.size>=options.limits.files) fail("budget_exceeded");
    const stat=lstatSync(absolute);
    if(stat.isSymbolicLink()) fail("unsupported_input");
    if(stat.isDirectory()) {
      const entries=readdirSync(absolute).sort();
      directories.set(id,entries);
      for(const name of entries) {
        // Keep an excluded-boundary sentinel so explicit include patterns cannot
        // silently turn existing metadata inputs into an apparently empty directory.
        if(name.toLowerCase()===".git") {directories.set(id+"/"+name,[]);continue;}
        if(!relative(name) || name.includes("/")) fail("unsupported_input");
        walk(path.join(absolute,name),id+"/"+name,depth+1);
      }
    } else if(stat.isFile()) {
      if(bytes+stat.size>options.limits.bytes) fail("budget_exceeded");
      const content=readFileSync(absolute);bytes+=content.length;
      if(bytes>options.limits.bytes) fail("budget_exceeded");
      files.set(id,content);
    } else fail("unsupported_input");
  }
  for(const [id,absolute] of Object.entries(roots)) walk(absolute,id,0);
  const manifest=[...files].map(([id,b])=>({id,sha256:hash(b),size:b.length})).sort((a,b)=>a.id<b.id?-1:1);
  const dirs=[...directories.keys()].sort();
  return {files,directories,roots,manifest,dirs,digest:hash(canonical({files:manifest,directories:dirs}))};
}
function build() {
  const first=snapshot();
  const compilerId="compiler/"+path.basename(options.compiler);
  if(hash(first.files.get(compilerId)??"")!==COMPILER_HASH) fail("compiler_mismatch");
  const ts=createRequire(import.meta.url)(options.compiler);
  if(ts.version!=="5.9.3") fail("compiler_mismatch");
  const packet=emptyPacket(options,"invalid_config");packet.reasons=[];packet.compiler.verified=true;
  const caseSensitive=ts.sys.useCaseSensitiveFileNames;
  const canonicalFile=ts.createGetCanonicalFileName(caseSensitive),canonicalIds=new Map();
  packet.scope.case_sensitive=caseSensitive;
  for(const id of [...first.files.keys(),...first.directories.keys()]) {
    const key=canonicalFile(id);
    if(canonicalIds.has(key) && canonicalIds.get(key)!==id)fail("unsupported_input");
    canonicalIds.set(key,id);
  }
  const reasons=new Set(),reads=new Set(),missing=new Set();let outside=false;
  // Virtual paths make observations portable across equivalent caller-owned roots.
  const toId=f=>{
    const n=path.posix.normalize(f).replace(/\/+$/,"");
    if(n.split("/").some(p=>p.toLowerCase()===".git"))fail("unsupported_input");
    for(const root of ["project","compiler"]) if(n==="/__prism__/"+root || n.startsWith("/__prism__/"+root+"/")) {
      const id=n.slice("/__prism__/".length);
      return canonicalIds.get(canonicalFile(id))??id;
    }
    outside=true;return null;
  };
  const virtual=id=>"/__prism__/"+id;
  const read=f=>{
    const id=toId(f);if(!id)return undefined;
    if(!first.files.has(id)){missing.add(id);return undefined;}
    reads.add(id);
    try{return new TextDecoder("utf-8",{fatal:true,ignoreBOM:true}).decode(first.files.get(id));}
    catch{fail("unsupported_input");}
  };
  const entries=f=>{
    const id=toId(f);if(!id)return {files:[],directories:[]};
    if(!first.directories.has(id))missing.add(id);
    return {files:(first.directories.get(id)??[]).filter(n=>first.files.has(id+"/"+n)),
      directories:(first.directories.get(id)??[]).filter(n=>first.directories.has(id+"/"+n))};
  };
  const basic={readFile:read,fileExists:f=>{const id=toId(f);if(!id)return false;
      if(!first.files.has(id))missing.add(id);return first.files.has(id);},
    directoryExists:f=>{const id=toId(f);if(!id)return false;
      if(!first.directories.has(id))missing.add(id);return first.directories.has(id);},
    getDirectories:f=>entries(f).directories,realpath:f=>{const id=toId(f);return id?virtual(id):f;},getCurrentDirectory:()=>virtual("project"),
    readDirectory:(dir,extensions,excludes,includes,depth)=>ts.matchFiles(dir,extensions,excludes,includes,caseSensitive,virtual("project"),depth,entries,f=>f)};
  const configFile=virtual("project/"+options.config);
  const config=ts.readConfigFile(configFile,read);
  const parsed=ts.parseJsonConfigFileContent(config.config??{}, {...basic,useCaseSensitiveFileNames:caseSensitive},path.posix.dirname(configFile),undefined,configFile);
  const configFiles=[...reads].sort();
  if(config.error || parsed.errors.length) reasons.add("invalid_config");
  if(parsed.projectReferences?.length) reasons.add("unsupported_references");
  if(parsed.options.plugins?.length) reasons.add("unsupported_plugins");
  const host={...basic,getSourceFile:(f,v)=>{const text=read(f);return text===undefined?undefined:ts.createSourceFile(f,text,v,true);},
    getDefaultLibFileName:o=>virtual("compiler/"+ts.getDefaultLibFileName(o)),
    getDefaultLibLocation:()=>virtual("compiler"),writeFile:()=>fail("unsupported_input"),
    getCanonicalFileName:canonicalFile,useCaseSensitiveFileNames:()=>caseSensitive,getNewLine:()=>"\n",
    getEnvironmentVariable:()=>"",
    resolveModuleNameLiterals:(literals,from,redirected,compilerOptions,source)=>literals.map(l=>{
      const result=ts.resolveModuleName(l.text,from,compilerOptions,host,undefined,redirected,ts.getModeForUsageLocation(source,l,compilerOptions));
      const target=result.resolvedModule?toId(result.resolvedModule.resolvedFileName):null;
      packet.resolutions.push({from:toId(from),specifier:l.text,target});
      if(!target)reasons.add("unresolved_module");
      return result;
    })};
  // References/plugins are recorded but never traversed/executed in this bounded slice.
  const program=ts.createProgram(parsed.fileNames,parsed.options,host);
  const checker=program.getTypeChecker();
  packet.diagnostics=[config.error,...parsed.errors,...ts.getPreEmitDiagnostics(program)].filter(Boolean)
    .map(d=>({code:d.code,file:d.file?toId(d.file.fileName):null,start:d.start??null}))
    .sort((a,b)=>canonical(a)<canonical(b)?-1:1);
  if(packet.diagnostics.length)reasons.add("compiler_diagnostics");
  function anchor(node) {
    const sf=node.getSourceFile(),file=toId(sf.fileName),start=node.getStart(sf),end=node.end;
    const startByte=Buffer.byteLength(sf.text.slice(0,start)),endByte=Buffer.byteLength(sf.text.slice(0,end));
    const bytes=first.files.get(file);
    if(!bytes || !bytes.subarray(startByte,endByte).equals(Buffer.from(sf.text.slice(start,end))))fail("unsupported_input");
    return {file,sha256:hash(bytes),kind:ts.SyntaxKind[node.kind],start_utf16:start,end_utf16:end,start_byte:startByte,end_byte:endByte};
  }
  for(const sf of program.getSourceFiles()) {
    const id=toId(sf.fileName);
    if(!id?.startsWith("project/") || id.includes("/node_modules/") || sf.isDeclarationFile)continue;
    function visit(node) {
      if(ts.isVariableDeclaration(node) && node.type && node.initializer
          && (ts.isArrowFunction(node.initializer) || ts.isFunctionExpression(node.initializer))) {
        if(packet.observations.length>=options.limits.observations)fail("budget_exceeded");
        const fn=node.initializer,context=checker.getContextualType(fn);
        const signatures=context?checker.getSignaturesOfType(context,ts.SignatureKind.Call):[];
        const observation={annotation:anchor(node.type),implementation:anchor(fn),parameter:fn.parameters[0]?anchor(fn.parameters[0]):null,
          provenance:traceProvenance(ts,checker,node.type,anchor,options.limits.provenance_steps),
          nested:observeNested(ts,checker,fn,anchor,options.limits),
          explicit_parameter:!!fn.parameters[0]?.type,signatures:signatures.flatMap(s=>s.declaration?[anchor(s.declaration)]:[]),
          callable_declarations:[...new Set([...(context?.symbol?.declarations??[]),...(context?.aliasSymbol?.declarations??[])])].map(anchor),calls:[]};
        function calls(n) {
          // Class initializers/static blocks also have their own lexical scope.
          if(ts.isFunctionLike(n) || ts.isClassDeclaration(n) || ts.isClassExpression(n))return;
          if(ts.isCallExpression(n) && ts.isPropertyAccessExpression(n.expression)) {
            observation.calls.push({call:anchor(n),receiver:anchor(n.expression.expression),
              receiver_type:checker.typeToString(checker.getTypeAtLocation(n.expression.expression)),
              declarations:(checker.getSymbolAtLocation(n.expression.name)?.declarations??[]).map(anchor)});
          }
          ts.forEachChild(n,calls);
        }
        calls(fn.body);packet.observations.push(observation);
      }
      ts.forEachChild(node,visit);
    }
    visit(sf);
  }
  const second=snapshot();
  if(first.digest!==second.digest)reasons.add("unstable_snapshot");
  if(outside)reasons.add("outside_lookup");
  const complete=reasons.size===0;
  packet.status=complete?"observed":"unproven";packet.reasons=[...reasons].sort();
  packet.closure={stable_snapshot:first.digest===second.digest,dependencies:!outside && !packet.diagnostics.length && !reasons.has("unresolved_module"),
    references:!reasons.has("unsupported_references"),augmentation:complete,resolution:complete};
  packet.compiler.library_sha256=hash(canonical(first.manifest.filter(f=>f.id.startsWith("compiler/"))));
  packet.snapshot={sha256:first.digest,files:first.manifest,directories:first.dirs,
    roots:parsed.fileNames.map(toId).sort(),config_files:configFiles,program_files:program.getSourceFiles().map(f=>toId(f.fileName)).sort(),
    reads:[...reads].sort(),failed_lookups:[...missing].sort(),outside_lookups:outside,options_sha256:hash(canonical(parsed.options))};
  packet.resolutions.sort((a,b)=>canonical(a)<canonical(b)?-1:1);
  return packet;
}
try {console.log(JSON.stringify(build()));}
catch(error) {
  const reason=["budget_exceeded","unsupported_input","compiler_mismatch"].includes(error.message)?error.message:"worker_failed";
  console.log(JSON.stringify(emptyPacket(options,reason)));
}
