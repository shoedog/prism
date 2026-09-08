import {hash,REFERENCE_LIMIT,referenceKey} from './schema.mjs';

// Source-written directives only; configured/automatic entry channels are not
// invented here. Read pinned Program caches, never run a second resolver.
export function observeTypeLib(ts,program,toId,read) {
  const rows=[],sources=new Set(program.getSourceFiles()),includes=program.getFileIncludeReasons();
  const options=program.getCompilerOptions();
  for(const sf of sources) {
    const source=sf.redirectInfo?.unredirected??sf;
    if(!source.typeReferenceDirectives.length&&!source.libReferenceDirectives.length)continue;
    const file=toId(source.fileName),text=read(source.fileName);
    if(!file||file!==toId(sf.fileName)||text!==source.text)throw Error('unsupported_input');
    const sha256=hash(Buffer.from(text));
    for(const [kind,directives,includeKind] of [['types',source.typeReferenceDirectives,5],['lib',source.libReferenceDirectives,7]]) {
      for(const [index,ref] of directives.entries()) {
        if(rows.length>=REFERENCE_LIMIT)throw Error('budget_exceeded');
        if(text.slice(ref.pos,ref.end)!==ref.fileName)throw Error('unsupported_input');
        const modeValue=kind==='types'?(ref.resolutionMode??program.getDefaultResolutionModeForFile(source)):undefined;
        if(modeValue!==undefined&&modeValue!==ts.ModuleKind.CommonJS&&modeValue!==ts.ModuleKind.ESNext)throw Error('unsupported_input');
        const row={kind,index,name:ref.fileName,mode:modeValue===undefined?null:modeValue===ts.ModuleKind.CommonJS?'require':'import',
          request:{file,sha256,kind:kind==='types'?'TypeReferenceDirective':'LibReferenceDirective',start_utf16:ref.pos,end_utf16:ref.end,
            start_byte:Buffer.byteLength(text.slice(0,ref.pos)),end_byte:Buffer.byteLength(text.slice(0,ref.end))},
          status:'unproven',reason:'unprocessed',target:null,inclusion:false};
        rows.push(row);
        // Redirect originals were not traversed. Global lib caches or inherited
        // directive arrays are insufficient even if another source included it.
        if(sf.redirectInfo||(kind==='types'?options.noResolve:options.noLib))continue;
        let selected;
        if(kind==='types') {
          const cached=program.getResolvedTypeReferenceDirectiveFromTypeReferenceDirective(ref,source);
          if(!cached)continue;
          selected=cached.resolvedTypeReferenceDirective?.resolvedFileName;
        } else {
          const lib=ts.libMap.get(ref.fileName.toLowerCase());
          if(lib){const cached=program.resolvedLibReferences?.get(lib);if(!cached)continue;selected=cached.actual;}
        }
        if(!selected){row.reason='unresolved';continue;}
        const target=program.getSourceFile(selected),targetId=target&&toId(target.fileName);
        if(!target||!sources.has(target)||!targetId){row.reason='target_not_in_program';continue;}
        if(!includes.get(target.path)?.some(r=>r.kind===includeKind&&r.file===sf.path&&r.index===index)){
          row.reason='missing_inclusion';continue;
        }
        row.status='observed';row.reason=null;row.target=targetId;row.inclusion=true;
      }
    }
  }
  return rows.sort((a,b)=>referenceKey(a)<referenceKey(b)?-1:referenceKey(a)>referenceKey(b)?1:0);
}
