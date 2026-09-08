import {REFERENCE_LIMIT,entryKey} from './schema.mjs';

// Entry observations only, not a closure-policy decision. Pinned Program caches
// and inclusion reasons are read after construction; no new resolver or I/O.
export function observeEntries(ts,program,toId) {
  const rows=[],options=program.getCompilerOptions(),sources=new Set(program.getSourceFiles());
  const includes=program.getFileIncludeReasons(),names=program.getAutomaticTypeDirectiveNames();
  const cache=program.getAutomaticTypeDirectiveResolutions(),roots=program.getRootFileNames().length;
  function row(kind,origin,index,name) {
    if(rows.length>=REFERENCE_LIMIT)throw Error('budget_exceeded');
    const r={kind,origin,index,name,mode:null,status:'unproven',reason:'unprocessed',target:null,inclusion:false};rows.push(r);return r;
  }
  function selected(r,file,match) {
    const target=program.getSourceFile(file),id=target&&toId(target.fileName);
    if(!target||!sources.has(target)||!id){r.reason='target_not_in_program';return;}
    if(!includes.get(target.path)?.some(match)){r.reason='missing_inclusion';return;}
    r.status='observed';r.reason=null;r.target=id;r.inclusion=true;
  }
  // The compiler uses undefined mode for both explicit types and discovery.
  // Repeated names retain their option/enumeration index; kind8 has no index.
  for(const [index,name] of (options.types??names).entries()) {
    const r=row('types',options.types?'configured':'automatic',index,name);
    if(!roots||names[index]!==name)continue;
    const resolution=cache?.get(name,undefined);if(!resolution)continue;
    const file=resolution.resolvedTypeReferenceDirective?.resolvedFileName;
    if(!file){r.reason='unresolved';continue;}
    selected(r,file,i=>i.kind===8&&i.typeReference===name);
  }
  if(options.lib) {
    for(const [index,name] of options.lib.entries()) {
      const r=row('lib','configured',index,name);
      if(!roots||options.noLib)continue;
      const cached=program.resolvedLibReferences?.get(name);if(!cached)continue;
      if(!cached.actual){r.reason='unresolved';continue;}
      selected(r,cached.actual,i=>i.kind===6&&i.index===index);
    }
  } else {
    const r=row('lib','default',0,ts.getDefaultLibFileName(options));
    if(roots&&!options.noLib) {
      // Default libs do not use pathForLibFile/cache. Read the indexless kind6
      // selection itself; incidental membership must not create an entry.
      const candidates=[...includes].filter(([,ii])=>ii.some(i=>i.kind===6&&i.index===undefined));
      if(candidates.length>1)throw Error('unsupported_input');
      if(candidates.length)selected(r,candidates[0][0],i=>i.kind===6&&i.index===undefined);
    }
  }
  return rows.sort((a,b)=>entryKey(a)<entryKey(b)?-1:entryKey(a)>entryKey(b)?1:0);
}
