// Side-effect source-pair observation only. Does not change legacy lookup policy.
export function mergedWildcardObserver(ts,checker,population,owned,anchor) {
  const module=n=>owned(n)&&ts.isModuleDeclaration(n)&&ts.isStringLiteral(n.name);
  const shape=n=>!n.body?'shorthand':ts.isModuleBlock(n.body)&&n.body.statements.length===0?'empty_block':'other';
  const entry=n=>module(n)?{declaration:anchor(n),shape:shape(n)}:null;
  return (resolution,literal,source,lookup,symbol,declarations)=>{
    const w=lookup.wildcard;
    if(!w || w.providers.length<2 && declarations.length<2)return null;
    const selected=symbol?.valueDeclaration;
    const result={status:'unproven',reason:null,declarations:declarations.filter(module).map(entry),selected:entry(selected)};
    const providers=population.get(w.pattern)?.providers??[],request=literal?.parent;
    let reason;
    if(!lookup.request || lookup.context!=='import' || !ts.isStringLiteral(literal)
      || literal.getSourceFile()!==source || !ts.isImportDeclaration(request) || request.moduleSpecifier!==literal
      || request.importClause || request.attributes || request.assertClause || request.modifiers?.length)reason='unsupported_request';
    else if(resolution.target!==null)reason='filesystem_target';
    else if(w.augmentations.length || lookup.augmentations.length)reason='augmentation';
    else if(providers.length!==2 || declarations.length!==2)reason='provider_count';
    else if(w.matches.length!==2)reason='competing_pattern';
    else if(lookup.providers.length)reason='exact_provider';
    else if(!(symbol?.flags & ts.SymbolFlags.ValueModule) || new Set(declarations).size!==2
      || !ts.isPatternMatch(ts.tryParsePattern(w.pattern),resolution.specifier)
      || declarations.some(d=>!module(d) || d.name.text!==w.pattern
        || !providers.some(p=>p.node===d&&p.source===d.getSourceFile())
        || checker.getSymbolAtLocation(d.name)!==symbol))reason='binding_mismatch';
    else if(declarations.some(d=>!d.getSourceFile().isDeclarationFile || ts.isExternalModule(d.getSourceFile())
        || d.parent!==d.getSourceFile())
      || declarations.filter(d=>shape(d)==='empty_block').length!==1
      || declarations.filter(d=>shape(d)==='shorthand').length!==1)reason='unsupported_provider';
    else if(!module(selected) || !declarations.includes(selected))reason='selected_declaration';
    if(reason)result.reason=reason;else result.status='observed';
    return result;
  };
}
