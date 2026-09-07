// Source observations only. Filesystem outcomes and closure policy stay in worker.
import {wildcardObserver} from './wildcard.mjs';
import {mergedWildcardObserver} from './merged-wildcard.mjs';
export function observeExactAmbient(ts,program,checker,requests,anchor,anchorInSource) {
  const sources=new Set(program.getSourceFiles()),population=new Map();
  const owned=n=>n && n.pos>=0 && n.end>=n.pos && sources.has(n.getSourceFile());
  // One census per Program, including declarations hidden by merged-symbol/error
  // recovery. Never reuse this population across snapshots or compiler Programs.
  function visit(n,source) {
    if(ts.isModuleDeclaration(n) && ts.isStringLiteral(n.name)) {
      const entry=population.get(n.name.text)??{providers:[],augmentations:[]};
      entry[ts.isExternalModuleAugmentation(n)?'augmentations':'providers'].push({node:n,source});
      population.set(n.name.text,entry);
    }
    ts.forEachChild(n,child=>visit(child,source));
  }
  for(const sf of sources) {
    // Pinned TS package-ID redirects inherit another file's AST/parent pointers.
    // Census the original parsed bytes, not the shared redirected node tree.
    const source=sf.redirectInfo?.unredirected??sf;
    visit(source,source);
  }
  const contextOf=l=>{
    if(!owned(l))return 'synthetic';
    const p=l.parent;
    if(ts.isImportDeclaration(p) && p.moduleSpecifier===l)return 'import';
    if(ts.isExportDeclaration(p) && p.moduleSpecifier===l)return 'export';
    if(ts.isLiteralTypeNode(p) && ts.isImportTypeNode(p.parent) && p.parent.argument===p)return 'import_type';
    if(ts.isExternalModuleReference(p) && p.expression===l)return 'import_equals';
    if(ts.isModuleDeclaration(p) && p.name===l)return 'augmentation_name';
    if(ts.isCallExpression(p) && p.arguments[0]===l) {
      if(p.expression.kind===ts.SyntaxKind.ImportKeyword)return 'dynamic_import';
      if(ts.isIdentifier(p.expression) && p.expression.text==='require')return 'require';
    }
    return 'other';
  };
  const anchors=nodes=>nodes.filter(owned).map(anchor);
  const censusAnchors=entries=>entries.map(({node,source})=>anchorInSource(node,source));
  const observeWildcard=wildcardObserver(ts,checker,population,owned,censusAnchors);
  const observeMergedWildcard=mergedWildcardObserver(ts,checker,population,owned,anchor);
  for(const {resolution,literal,source} of requests) {
    const context=contextOf(literal),synthetic=context==='synthetic';
    const symbol=synthetic?undefined:checker.getSymbolAtLocation(literal);
    const declarations=symbol?.declarations??[];
    const {providers,augmentations}=population.get(resolution.specifier)??{providers:[],augmentations:[]};
    const observation={status:'unproven',reason:null,context,request:synthetic?null:anchor(literal),
      declarations:anchors(declarations),providers:censusAnchors(providers),augmentations:censusAnchors(augmentations)};
    resolution.lookup=observation;
    let reason;
    if(synthetic)reason='synthetic_request';
    else if(context==='augmentation_name')reason='augmentation_request';
    else if(!['import','export','import_type','import_equals'].includes(context)
      || !ts.isStringLiteral(literal) || literal.getSourceFile()!==source)reason='unsupported_request';
    else if(resolution.target!==null)reason='filesystem_target';
    else if(augmentations.length)reason='augmentation';
    else if(providers.length>1)reason='duplicate_provider';
    else if(!symbol || !declarations.length)reason='unresolved_symbol';
    else if(declarations.length!==1)reason='ambiguous_binding';
    else {
      const d=declarations[0],sf=d.getSourceFile();
      if(!ts.isModuleDeclaration(d) || !ts.isStringLiteral(d.name)
        || d.name.text!==resolution.specifier || d.name.text.includes('*')
        || ts.isExternalModuleNameRelative(d.name.text))reason='non_exact_binding';
      else if(providers.length!==1 || providers[0].node!==d || providers[0].source!==sf || !owned(d)
        || !(symbol.flags & ts.SymbolFlags.ValueModule)
        || checker.getSymbolAtLocation(d.name)!==symbol)reason='binding_mismatch';
      else if(!sf.isDeclarationFile || ts.isExternalModule(sf) || d.parent!==sf
        || !d.body || !ts.isModuleBlock(d.body))reason='unsupported_provider';
    }
    if(reason)observation.reason=reason;
    else observation.status='observed';
    observation.wildcard=observeWildcard(resolution,literal,source,observation,symbol,declarations);
    observation.merged_wildcard=observeMergedWildcard(resolution,literal,source,observation,symbol,declarations);
  }
}
