// Worker confines synchronous compiler work to a parent-enforced timeout/heap cap.
import {readFileSync} from "node:fs";
import {createRequire} from "node:module";
import path from "node:path";
import {COMPILER_HASH,relative,hash,canonical} from "./schema.mjs";
import {emptyPacket} from "./index.mjs";
import {traceProvenance} from "./provenance.mjs";
import {observeNested} from "./nested.mjs";
import {observePropsClasses} from "./props-class.mjs";
import {snapshot} from "./inventory.mjs";

const options=JSON.parse(readFileSync(0,"utf8"));
const fail=reason=>{throw Error(reason);};
function build() {
  const first=snapshot(options);
  const compilerId="compiler/"+path.basename(options.compiler);
  if(hash(first.read(compilerId)??"")!==COMPILER_HASH) fail("compiler_mismatch");
  const ts=createRequire(import.meta.url)(options.compiler);
  if(ts.version!=="5.9.3") fail("compiler_mismatch");
  const packet=emptyPacket(options,"invalid_config");packet.reasons=[];packet.compiler.verified=true;
  const caseSensitive=ts.sys.useCaseSensitiveFileNames;
  const canonicalFile=ts.createGetCanonicalFileName(caseSensitive),canonicalIds=new Map();
  packet.scope.case_sensitive=caseSensitive;
  for(const id of [...first.files.keys(),...first.directories.keys(),...first.links.map(l=>l.id)]) {
    const key=canonicalFile(id);
    if(canonicalIds.has(key) && canonicalIds.get(key)!==id)fail("unsupported_input");
    canonicalIds.set(key,id);
  }
  const canonicalId=id=>first.resolve(id,v=>canonicalIds.get(canonicalFile(v))??v);
  const reasons=new Set(),reads=new Set(),missing=new Set(),refused=new Set();let outside=false;
  // Virtual paths make observations portable across equivalent caller-owned roots.
  const toId=f=>{
    const n=path.posix.normalize(f).replace(/\/+$/,"");
    if(n.split("/").some(p=>p.toLowerCase()===".git"))fail("unsupported_input");
    for(const root of ["project","compiler"]) if(n==="/__prism__/"+root || n.startsWith("/__prism__/"+root+"/")) {
      const id=n.slice("/__prism__/".length);
      // Compiler probes may contain virtual module spellings, not safe file IDs.
      // Preserve opaque refusal evidence without consulting files or claiming absence.
      if(!relative(id)){refused.add(hash(n));reasons.add("unsupported_lookup");return null;}
      return canonicalId(id);
    }
    outside=true;return null;
  };
  const virtual=id=>"/__prism__/"+id;
  const read=f=>{
    const id=toId(f);if(!id)return undefined;
    if(!first.files.has(id)){missing.add(id);return undefined;}
    reads.add(id);
    const bytes=first.read(id);
    try{return new TextDecoder("utf-8",{fatal:true,ignoreBOM:true}).decode(bytes);}
    catch{fail("unsupported_input");}
  };
  const entries=f=>{
    const id=toId(f);if(!id)return {files:[],directories:[]};
    if(!first.directories.has(id))missing.add(id);
    // Keep metadata boundaries visible to matchFiles; actual traversal refuses.
    const names=first.directories.get(id)??[];
    return {files:names.filter(n=>n.toLowerCase()!==".git" && first.files.has(canonicalId(id+"/"+n))),
      directories:names.filter(n=>n.toLowerCase()===".git" || first.directories.has(canonicalId(id+"/"+n)))};
  };
  const basic={readFile:read,fileExists:f=>{const id=toId(f);if(!id)return false;
      if(!first.files.has(id))missing.add(id);return first.files.has(id);},
    directoryExists:f=>{const id=toId(f);if(!id)return false;
      if(!first.directories.has(id))missing.add(id);return first.directories.has(id);},
    getDirectories:f=>entries(f).directories,realpath:f=>{const id=toId(f);return id?virtual(id):f;},getCurrentDirectory:()=>virtual("project"),
    readDirectory:(dir,extensions,excludes,includes,depth)=>ts.matchFiles(dir,extensions,excludes,includes,caseSensitive,virtual("project"),depth,entries,f=>{const id=toId(f);return id?virtual(id):f;})};
  const configFile=virtual("project/"+options.config);
  const config=ts.readConfigFile(configFile,read);
  const parsed=ts.parseJsonConfigFileContent(config.config??{}, {...basic,useCaseSensitiveFileNames:caseSensitive},path.posix.dirname(configFile),undefined,configFile);
  const configFiles=[...reads].sort();
  if(config.error || parsed.errors.length) reasons.add("invalid_config");
  if(parsed.projectReferences?.length) reasons.add("unsupported_references");
  if(parsed.options.plugins?.length) reasons.add("unsupported_plugins");
  if(first.links.length && parsed.options.preserveSymlinks)fail("unsupported_input");
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
  const programIds=program.getSourceFiles().map(f=>toId(f.fileName));
  if(new Set(programIds).size!==programIds.length)fail("unsupported_input");
  const checker=program.getTypeChecker();
  packet.diagnostics=[config.error,...parsed.errors,...ts.getPreEmitDiagnostics(program)].filter(Boolean)
    .map(d=>({code:d.code,file:d.file?toId(d.file.fileName):null,start:d.start??null}))
    .sort((a,b)=>canonical(a)<canonical(b)?-1:1);
  if(packet.diagnostics.length)reasons.add("compiler_diagnostics");
  function anchor(node) {
    const sf=node.getSourceFile(),file=toId(sf.fileName),start=node.getStart(sf),end=node.end;
    const startByte=Buffer.byteLength(sf.text.slice(0,start)),endByte=Buffer.byteLength(sf.text.slice(0,end));
    const bytes=first.read(file);
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
        observePropsClasses(ts,checker,fn,signatures,observation,anchor,options.limits.props_type_args);
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
  const second=snapshot(options);
  if(first.digest!==second.digest)reasons.add("unstable_snapshot");
  if(outside)reasons.add("outside_lookup");
  const complete=reasons.size===0;
  packet.status=complete?"observed":"unproven";packet.reasons=[...reasons].sort();
  if(!complete)for(const o of packet.observations)for(const c of o.nested.calls) {
    if(c.props_class.status==="observed"){c.props_class.status="unproven";c.props_class.reason="program_unproven";}
  }
  packet.closure={stable_snapshot:first.digest===second.digest,dependencies:!outside && !refused.size && !packet.diagnostics.length && !reasons.has("unresolved_module"),
    references:!reasons.has("unsupported_references"),augmentation:complete,resolution:complete};
  packet.compiler.library_sha256=hash(canonical(first.manifest.filter(f=>f.id.startsWith("compiler/"))));
  packet.snapshot={sha256:first.digest,files:first.manifest,directories:first.dirs,links:first.links,
    roots:parsed.fileNames.map(toId).sort(),config_files:configFiles,program_files:program.getSourceFiles().map(f=>toId(f.fileName)).sort(),
    reads:[...reads].sort(),failed_lookups:[...missing].sort(),refused_lookup_sha256:[...refused].sort(),outside_lookups:outside,options_sha256:hash(canonical(parsed.options))};
  packet.resolutions.sort((a,b)=>canonical(a)<canonical(b)?-1:1);
  return packet;
}
try {console.log(JSON.stringify(build()));}
catch(error) {
  const reason=["budget_exceeded","unsupported_input","compiler_mismatch","unstable_snapshot"].includes(error.message)?error.message:"worker_failed";
  console.log(JSON.stringify(emptyPacket(options,reason)));
}
