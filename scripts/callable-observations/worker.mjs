// Worker confines synchronous compiler work to a parent-enforced timeout/heap cap.
import {readFileSync} from "node:fs";
import {createRequire} from "node:module";
import path from "node:path";
import {COMPILER_HASH,hash,canonical} from "./schema.mjs";
import {emptyPacket} from "./index.mjs";
import {traceProvenance} from "./provenance.mjs";
import {observeNested} from "./nested.mjs";
import {observePropsClasses} from "./props-class.mjs";
import {observeExactAmbient} from "./exact-ambient.mjs";
import {hasUnprovenRequiredPath} from "./required-paths.mjs";
import {observeTypeLib} from "./type-lib.mjs";
import {observeEntries} from "./entries.mjs";
import {snapshot} from "./inventory.mjs";
import {createSearchProvenance,classifyBoundary} from "./search-provenance.mjs";
import {projectSyntheticAddress} from "./identity-domains.mjs";
import {libraryNameFromLibFile} from "./lib-search.mjs";
import {createConfigCapture,observeConfigProvenance} from "./config-provenance.mjs";

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
  const search=createSearchProvenance(hash);packet.search_provenance={module_requests:search.module_requests,
    type_batches:search.type_batches,type_requests:search.type_requests,type_searches:search.type_searches,
    lib_searches:search.lib_searches,boundary_events:search.boundary_events};
  // Virtual paths make observations portable across equivalent caller-owned roots.
  const toId=(f,operation="identity")=>{
    const classified=classifyBoundary(f),n=classified.normalized;
    if(classified.kind==='unsafe')fail("unsupported_input");
    if(classified.kind==='in_root') {
      const id=classified.id;
      // Compiler probes may contain virtual module spellings, not safe file IDs.
      // Preserve opaque refusal evidence without consulting files or claiming absence.
      return canonicalId(id);
    }
    if(classified.kind==='refused'){refused.add(hash(n));search.boundary("refused",operation,n);reasons.add("unsupported_lookup");return null;}
    outside=true;search.boundary("outside",operation,n);return null;
  };
  const virtual=id=>"/__prism__/"+id;
  const pureId=f=>{const classified=classifyBoundary(f);if(classified.kind==='unsafe')fail('unsupported_input');
    return classified.kind==='in_root'?canonicalId(classified.id):null;};
  const read=f=>{
    const id=toId(f,"readFile");if(!id)return undefined;
    if(!first.files.has(id)){missing.add(id);return undefined;}
    reads.add(id);
    const bytes=first.read(id);
    try{return new TextDecoder("utf-8",{fatal:true,ignoreBOM:true}).decode(bytes);}
    catch{fail("unsupported_input");}
  };
  const entries=f=>{
    const id=toId(f,"entries");if(!id)return {files:[],directories:[]};
    if(!first.directories.has(id))missing.add(id);
    // Keep metadata boundaries visible to matchFiles; actual traversal refuses.
    const names=first.directories.get(id)??[];
    return {files:names.filter(n=>n.toLowerCase()!==".git" && first.files.has(canonicalId(id+"/"+n))),
      directories:names.filter(n=>n.toLowerCase()===".git" || first.directories.has(canonicalId(id+"/"+n)))};
  };
  const basic={readFile:read,fileExists:f=>{const id=toId(f,"fileExists");if(!id)return false;
      if(!first.files.has(id))missing.add(id);return first.files.has(id);},
    directoryExists:f=>{const id=toId(f,"directoryExists");if(!id)return false;
      if(!first.directories.has(id))missing.add(id);return first.directories.has(id);},
    getDirectories:f=>entries(f).directories,realpath:f=>{const id=toId(f,"realpath");return id?virtual(id):f;},getCurrentDirectory:()=>virtual("project"),
    readDirectory:(dir,extensions,excludes,includes,depth)=>ts.matchFiles(dir,extensions,excludes,includes,caseSensitive,virtual("project"),depth,entries,f=>{const id=toId(f,"realpath");return id?virtual(id):f;})};
  const configFile=virtual("project/"+options.config);
  let rootConfigText;
  const config=ts.readConfigFile(configFile,f=>{const text=read(f);rootConfigText=text;return text;});
  const configCapture=createConfigCapture();
  const parsed=ts.parseJsonConfigFileContent(config.config??{}, {...basic,useCaseSensitiveFileNames:caseSensitive},path.posix.dirname(configFile),undefined,configFile,undefined,undefined,configCapture);
  const configFiles=[...reads].sort();
  try {
    packet.config_provenance=observeConfigProvenance({ts,source:rootConfigText===undefined?null:ts.parseJsonText(configFile,rootConfigText),
      effectiveOptions:parsed.options,capturedRecords:configCapture.capturedRecords,canonicalId:pureId,
      configReadMembership:new Set(configFiles),inventory:first.files,configDiagnostics:[config.error,...parsed.errors].filter(Boolean),caseSensitive});
  } catch {packet.config_provenance={status:"unproven",reason:"unavailable",files:[],extends:[],options:[]};}
  if(config.error || parsed.errors.length) reasons.add("invalid_config");
  if(parsed.projectReferences?.length) reasons.add("unsupported_references");
  if(parsed.options.plugins?.length) reasons.add("unsupported_plugins");
  if(first.links.length && parsed.options.preserveSymlinks)fail("unsupported_input");
  const lookupRequests=[],typeLookupRequests=[],libLookupSearches=[];
  // Match the pinned Program's private cache construction without another host call.
  const typeResolutionCache=ts.createTypeReferenceDirectiveResolutionCache(virtual('project'),canonicalFile,undefined,undefined,undefined);
  const libResolutionCache=ts.createModuleResolutionCache(virtual('project'),canonicalFile,parsed.options,undefined);
  const host={...basic,getSourceFile:(f,v)=>{const text=read(f);return text===undefined?undefined:ts.createSourceFile(f,text,v,true);},
    getDefaultLibFileName:o=>virtual("compiler/"+ts.getDefaultLibFileName(o)),
    getDefaultLibLocation:()=>virtual("compiler"),writeFile:()=>fail("unsupported_input"),
    getCanonicalFileName:canonicalFile,useCaseSensitiveFileNames:()=>caseSensitive,getNewLine:()=>"\n",
    getEnvironmentVariable:()=>"",
    resolveModuleNameLiterals:(literals,from,redirected,compilerOptions,source)=>literals.map(l=>{
      const rawMode=ts.getModeForUsageLocation(source,l,compilerOptions);
      const record={id:search.claimRequest(),resolution:null,literal:l,source,
        mode:rawMode===ts.ModuleKind.ESNext?'import':rawMode===ts.ModuleKind.CommonJS?'require':null};
      return search.withRequest(record,()=>{
        const result=ts.resolveModuleName(l.text,from,compilerOptions,host,undefined,redirected,rawMode);
        // Keep ownership active through target identity and legacy insertion.
        const target=result.resolvedModule?toId(result.resolvedModule.resolvedFileName):null;
        const resolution={from:toId(from),specifier:l.text,target};
        packet.resolutions.push(resolution);record.resolution=resolution;lookupRequests.push(record);
        if(!target)reasons.add("unresolved_module");
        return result;
      });
    }),
    resolveTypeReferenceDirectiveReferences:(entries,from,redirected,compilerOptions,source)=>{
      const origin=source?'source':compilerOptions.types!==undefined?'configured':'automatic';
      const fromId=source?pureId(from):projectSyntheticAddress(from);
      const batch={id:search.claimTypeBatch(),origin,from:fromId,size:entries.length};search.typeBatch(batch);
      const resultsByKey=new Map(),effectiveOptions=redirected?.commandLine.options||compilerOptions;
      return entries.map((entry,index)=>{
        const id=search.claimTypeRequest(),name=typeof entry==='string'?entry:entry.fileName;
        const rawMode=ts.getModeForFileReference(entry,source&&ts.getDefaultResolutionModeForFileWorker(source,effectiveOptions));
        const mode=rawMode===ts.ModuleKind.ESNext?'import':rawMode===ts.ModuleKind.CommonJS?'require':null;
        const key=ts.createModeAwareCacheKey(name,rawMode);let execution=resultsByKey.get(key);
        if(!execution) {
          const searchRecord={id:search.claimTypeSearch()};
          execution=search.withTypeSearch(searchRecord,()=>{
            const result=ts.resolveTypeReferenceDirective(name,from,compilerOptions,host,redirected,typeResolutionCache,rawMode);
            const target=result.resolvedTypeReferenceDirective?pureId(result.resolvedTypeReferenceDirective.resolvedFileName):null;
            search.typeSearch({id:searchRecord.id,from:fromId,name,mode,target});return {id:searchRecord.id,result};
          });
          resultsByKey.set(key,execution);
        }
        typeLookupRequests.push({id,batch:batch.id,origin,from:fromId,name,mode,ref:entry,source,index,execution:execution.id});
        return execution.result;
      });
    },
    resolveLibrary:(name,from,compilerOptions,libFileName)=>{
      const record={id:search.claimLibSearch(),name,from:projectSyntheticAddress(from),libFile:libFileName,result:null,target:null};
      return search.withLibSearch(record,()=>{
        const result=ts.resolveLibrary(name,from,compilerOptions,host,libResolutionCache);
        record.result=result;record.target=result.resolvedModule?pureId(result.resolvedModule.resolvedFileName):null;
        libLookupSearches.push(record);return result;
      });
    }};
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
    return anchorInSource(node,node.getSourceFile());
  }
  function anchorInSource(node,sf) {
    const file=toId(sf.fileName),start=node.getStart(sf),end=node.end;
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
          provenance:traceProvenance(ts,checker,node.type,anchor,options.limits.provenance_steps,program),
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
  observeExactAmbient(ts,program,checker,lookupRequests,anchor,anchorInSource);
  // The observer owns source anchoring. Reuse that exact request anchor here.
  for(const record of lookupRequests)search.request({id:record.id,from:record.resolution.from,specifier:record.resolution.specifier,
    mode:record.mode===undefined?null:record.mode,request:record.resolution.lookup.request,target:record.resolution.target});
  if(hasUnprovenRequiredPath(program,toId,read))reasons.add("unproven_path_reference");
  packet.type_lib_references=observeTypeLib(ts,program,toId,read);
  if(packet.type_lib_references.some(r=>r.status==='unproven'))reasons.add("unproven_type_lib_reference");
  packet.type_lib_entries=observeEntries(ts,program,toId);
  const usedRowsByBatch=new Map(search.type_batches.map(batch=>[batch.id,new Set()]));
  for(const record of typeLookupRequests) {
    let request=null,row,key;const usedRows=usedRowsByBatch.get(record.batch);
    if(!usedRows)fail('unsupported_input');
    if(record.origin==='source') {
      if(record.source?.typeReferenceDirectives?.[record.index]!==record.ref)fail('unsupported_input');
      row=packet.type_lib_references.find(r=>r.kind==='types'&&r.request.file===record.from&&r.index===record.index);
      key=row&&canonical({file:row.request.file,index:row.index});request=row?.request??null;
    } else {
      row=packet.type_lib_entries.find(r=>r.kind==='types'&&r.origin===record.origin&&r.index===record.index);
      key=row&&canonical({origin:row.origin,index:row.index});
    }
    if(!row||usedRows.has(key)||row.name!==record.name||row.mode!==record.mode)fail('unsupported_input');usedRows.add(key);
    search.typeRequest({id:record.id,batch:record.batch,origin:record.origin,from:record.from,name:record.name,mode:record.mode,
      request,index:record.index,execution:record.execution});
  }
  for(const record of libLookupSearches) {
    const selectedRecord=program.resolvedLibReferences?.get(record.libFile);
    if(libraryNameFromLibFile(record.libFile)!==record.name||!selectedRecord
      ||selectedRecord.resolution!==record.result||typeof selectedRecord.actual!=='string')fail('unsupported_input');
    const selected=pureId(selectedRecord.actual),beneficiaries=[];
    for(const row of packet.type_lib_references)if(row.kind==='lib'&&row.status==='observed'
      &&ts.libMap.get(row.name.toLowerCase())===record.libFile&&row.target===selected) {
      search.libBeneficiaries(beneficiaries,[{origin:'source',request:row.request,index:row.index}]);
    }
    for(const row of packet.type_lib_entries)if(row.kind==='lib'&&row.origin==='configured'&&row.status==='observed'
      &&row.name===record.libFile&&row.target===selected) {
      search.libBeneficiaries(beneficiaries,[{origin:'configured',request:null,index:row.index}]);
    }
    search.libSearch({id:record.id,name:record.name,from:record.from,lib_file:record.libFile,target:record.target,selected,beneficiaries});
  }
  const second=snapshot(options);
  if(first.digest!==second.digest)reasons.add("unstable_snapshot");
  if(outside)reasons.add("outside_lookup");
  const complete=reasons.size===0;
  packet.status=complete?"observed":"unproven";packet.reasons=[...reasons].sort();
  if(!complete)for(const o of packet.observations)for(const c of o.nested.calls) {
    if(c.props_class.status==="observed"){c.props_class.status="unproven";c.props_class.reason="program_unproven";}
  }
  const requiredReferences=!reasons.has("unproven_path_reference")&&!reasons.has("unproven_type_lib_reference");
  packet.closure={stable_snapshot:first.digest===second.digest,dependencies:!outside && !refused.size && !packet.diagnostics.length && !reasons.has("unresolved_module") && requiredReferences,
    references:!reasons.has("unsupported_references") && requiredReferences,augmentation:complete,resolution:complete};
  packet.compiler.library_sha256=hash(canonical(first.manifest.filter(f=>f.id.startsWith("compiler/"))));
  packet.snapshot={sha256:first.digest,files:first.manifest,directories:first.dirs,links:first.links,
    roots:parsed.fileNames.map(f=>toId(f)).sort(),config_files:configFiles,program_files:program.getSourceFiles().map(f=>toId(f.fileName)).sort(),
    reads:[...reads].sort(),failed_lookups:[...missing].sort(),refused_lookup_sha256:[...refused].sort(),outside_lookups:outside,options_sha256:hash(canonical(parsed.options))};
  // Preserve filesystem triples and all schema9 evidence before comparing the
  // new lane. Mixed repeated imports can have different merged dispositions.
  const resolutionKey=({from,specifier,target})=>canonical({from,specifier,target});
  const legacyKey=({lookup,...resolution})=>{
    const {merged_wildcard,...legacy}=lookup;
    return canonical({...resolution,lookup:legacy});
  };
  packet.resolutions.sort((a,b)=>{
    const x=resolutionKey(a),y=resolutionKey(b);
    if(x!==y)return x<y?-1:1;
    const u=legacyKey(a),v=legacyKey(b);
    return u<v?-1:u>v?1:canonical(a)<canonical(b)?-1:canonical(a)>canonical(b)?1:0;
  });
  return packet;
}
try {console.log(JSON.stringify(build()));}
catch(error) {
  const reason=["budget_exceeded","unsupported_input","compiler_mismatch","unstable_snapshot"].includes(error.message)?error.message:"worker_failed";
  console.log(JSON.stringify(emptyPacket(options,reason)));
}
