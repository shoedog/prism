// Independent binding observation, not physical-asset or Program-closure proof.
export function wildcardObserver(ts,checker,population,owned,censusAnchors) {
  const patterns=[];
  for(const [name,entry] of population) {
    const pattern=ts.tryParsePattern(name);
    if(pattern && typeof pattern!=='string')patterns.push({pattern,...entry});
  }
  // This memo belongs only to this invocation's configured Program/source census.
  const matchesBySpecifier=new Map();
  const matching=specifier=>{
    if(!matchesBySpecifier.has(specifier)) {
      const matches=patterns.filter(p=>ts.isPatternMatch(p.pattern,specifier));
      matchesBySpecifier.set(specifier,{providers:matches.flatMap(p=>p.providers),
        augmentations:[...matches.flatMap(p=>p.augmentations),...(population.get(specifier)?.augmentations??[])]});
    }
    return matchesBySpecifier.get(specifier);
  };
  return (resolution,literal,source,lookup,symbol,declarations)=>{
    const names=new Set(declarations.filter(d=>ts.isModuleDeclaration(d)&&ts.isStringLiteral(d.name))
      .map(d=>d.name.text).filter(n=>n.includes('*')));
    if(names.size!==1)return null;
    const name=[...names][0],pattern=ts.tryParsePattern(name);
    if(!pattern || typeof pattern==='string')return null;
    const providers=population.get(name)?.providers??[],matched=matching(resolution.specifier);
    const result={status:'unproven',reason:null,pattern:name,providers:censusAnchors(providers),
      augmentations:censusAnchors([...new Set(matched.augmentations)]),matches:censusAnchors(matched.providers)};
    let reason;
    if(!lookup.request || !['import','export','import_type','import_equals'].includes(lookup.context)
      || !ts.isStringLiteral(literal) || literal.getSourceFile()!==source)reason='unsupported_request';
    else if(resolution.target!==null)reason='filesystem_target';
    else if(result.augmentations.length)reason='augmentation';
    else if(providers.length>1)reason='duplicate_provider';
    else if(matched.providers.length>1)reason='competing_pattern';
    else if(declarations.length!==1)reason='ambiguous_binding';
    else if(lookup.providers.length)reason='exact_provider';
    else {
      const d=declarations[0],sf=d.getSourceFile();
      if(!ts.isPatternMatch(pattern,resolution.specifier) || providers.length!==1 || matched.providers.length!==1
        || matched.providers[0]!==providers[0] || providers[0].node!==d || providers[0].source!==sf || !owned(d)
        || !(symbol.flags & ts.SymbolFlags.ValueModule) || checker.getSymbolAtLocation(d.name)!==symbol)reason='binding_mismatch';
      else if(!sf.isDeclarationFile || ts.isExternalModule(sf) || d.parent!==sf
        || !d.body || !ts.isModuleBlock(d.body))reason='unsupported_provider';
    }
    if(reason)result.reason=reason;else result.status='observed';
    return result;
  };
}
