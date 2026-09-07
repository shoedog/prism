// Declaration observations only. Generic arguments are anchored, not substituted.
export function traceProvenance(ts,checker,annotation,anchor,maxSteps) {
  const result={status:"unproven",reason:null,steps_used:0,hops:[],terminal:null};
  const stop=reason=>{throw {provenanceReason:reason};};
  const tick=()=>{if(result.steps_used>=maxSteps)stop("step_limit");result.steps_used++;};
  const scopes=new WeakMap();
  function declarations(symbol) {
    const found=new Set(symbol?.declarations??[]);
    for(const d of [...found]) {
      if(!d.name || !ts.isIdentifier(d.name))continue;
      let scope=d.parent;
      while(scope && !scope.statements)scope=scope.parent;
      if(!scope)continue;
      if(!scopes.has(scope)) {
        const names=new Map();
        const add=(n,prefix="local:")=>{if(n?.name && ts.isIdentifier(n.name)) {
          const key=prefix+n.name.text;names.set(key,[...(names.get(key)??[]),n]);
        }};
        for(const s of scope.statements) {
          if(ts.isTypeAliasDeclaration(s) || ts.isInterfaceDeclaration(s) || ts.isClassDeclaration(s)
            || ts.isFunctionDeclaration(s) || ts.isEnumDeclaration(s) || ts.isModuleDeclaration(s)
            || ts.isImportEqualsDeclaration(s)) {
            add(s);
            if(s.modifiers?.some(m=>m.kind===ts.SyntaxKind.ExportKeyword))add(s,"export:");
          }
          if(ts.isExportDeclaration(s) && s.exportClause && ts.isNamedExports(s.exportClause))
            s.exportClause.elements.forEach(n=>add(n,"export:"));
          if(ts.isImportDeclaration(s)) {
            add(s.importClause);const bindings=s.importClause?.namedBindings;
            if(bindings && ts.isNamedImports(bindings))bindings.elements.forEach(n=>add(n));else add(bindings);
          }
          if(ts.isVariableStatement(s))s.declarationList.declarations.forEach(n=>add(n));
        }
        scopes.set(scope,names);
      }
      // Error recovery can assign different symbols to duplicate declarations.
      // Include syntactic peers rather than treating the chosen symbol as unique.
      const key=(ts.isExportSpecifier(d)?"export:":"local:")+d.name.text;
      for(const peer of scopes.get(scope).get(key)??[])found.add(peer);
    }
    return [...found];
  }
  const anchors=nodes=>nodes.map(anchor);
  const emptyModule=()=>({module:null,module_declarations:[],module_exports:[],module_bindings:[]});
  function namespaceTarget(d) {
    if(!d.isExportEquals || !ts.isIdentifier(d.expression))return null;
    const symbol=checker.getSymbolAtLocation(d.expression),defs=declarations(symbol);
    return symbol && !(symbol.flags & ts.SymbolFlags.Alias) && defs.length===1
      && ts.isModuleDeclaration(defs[0]) && defs[0].parent===d.parent ? symbol : null;
  }
  function moduleEvidence(declaration) {
    let node=declaration;
    while(node && !ts.isImportDeclaration(node) && !ts.isExportDeclaration(node))node=node.parent;
    const use=node?.moduleSpecifier;
    if(!use)return {record:emptyModule(),reason:null};
    const defs=declarations(checker.getSymbolAtLocation(use));
    const statements=defs.flatMap(d=>ts.isSourceFile(d)?[...d.statements]
      :ts.isModuleDeclaration(d) && d.body && ts.isModuleBlock(d.body)?[...d.body.statements]:[]);
    // Immediate symbol resolution can skip export-star modules; never claim that
    // shortcut constitutes an explicit chain, even for an unrelated star export.
    const stars=statements.filter(s=>ts.isExportDeclaration(s) && (!s.exportClause || ts.isNamespaceExport(s.exportClause)));
    const exports=statements.filter(ts.isExportAssignment);
    const reason=!defs.length?"unresolved_symbol":defs.length!==1?"ambiguous_declaration"
      :stars.length?"unsupported_export_star":exports.some(d=>d.isExportEquals && !namespaceTarget(d))?"unsupported_declaration":null;
    return {reason,record:{module:anchor(use),module_declarations:anchors(defs),
      module_exports:anchors([...exports,...stars]),
      module_bindings:anchors(exports.flatMap(d=>declarations(checker.getSymbolAtLocation(d.expression))))}};
  }
  function follow(symbol,output,namespace=false) {
    const seen=new Set();
    while(symbol && (symbol.flags & ts.SymbolFlags.Alias)) {
      tick();if(seen.has(symbol))stop("cycle");seen.add(symbol);
      const defs=declarations(symbol);
      if(defs.length!==1)stop(defs.length?"ambiguous_declaration":"unresolved_symbol");
      const declaration=defs[0];
      const gateway=namespace && ts.isExportAssignment(declaration) && namespaceTarget(declaration);
      if(!ts.isImportSpecifier(declaration) && !ts.isImportClause(declaration)
        && !ts.isNamespaceImport(declaration) && !ts.isExportSpecifier(declaration) && !gateway)stop("unsupported_declaration");
      const evidence=moduleEvidence(declaration);
      const item={declarations:anchors(defs),target:[],...evidence.record};output.push(item);
      if(evidence.reason)stop(evidence.reason);
      const target=checker.getImmediateAliasedSymbol(symbol);
      item.target=anchors(declarations(target));
      if(gateway && target!==gateway)stop("unsupported_declaration");
      if(!target || !declarations(target).length)stop("unresolved_symbol");
      symbol=target;
    }
    if(!symbol || !declarations(symbol).length)stop("unresolved_symbol");
    return symbol;
  }
  function qualifier(name,output) {
    if(ts.isQualifiedName(name))qualifier(name.left,output);
    tick();const item={use:anchor(name),aliases:[],declarations:[]};output.push(item);
    const symbol=follow(checker.getSymbolAtLocation(name),item.aliases,true);
    const defs=declarations(symbol);item.declarations=anchors(defs);
    if(defs.some(d=>!ts.isSourceFile(d) && !ts.isModuleDeclaration(d)))stop("unsupported_declaration");
    // Namespace merges are inventoried but deliberately not traversed as a unique binding.
    if(defs.length!==1)stop("ambiguous_declaration");
  }
  const seenTypes=new Set();
  let type=annotation;
  try {
    for(;;) {
      tick();
      while(ts.isParenthesizedTypeNode(type)) {tick();type=type.type;}
      if(ts.isFunctionTypeNode(type) || ts.isTypeLiteralNode(type) && type.members.some(ts.isCallSignatureDeclaration)) {
        result.terminal=anchor(type);break;
      }
      if(!ts.isTypeReferenceNode(type))stop("unsupported_type");
      const hop={reference:anchor(type),type_arguments:anchors(type.typeArguments??[]),
        type_parameters:[],qualifiers:[],aliases:[],declarations:[]};result.hops.push(hop);
      if(ts.isQualifiedName(type.typeName))qualifier(type.typeName.left,hop.qualifiers);
      const symbol=follow(checker.getSymbolAtLocation(type.typeName),hop.aliases);
      const defs=declarations(symbol);hop.declarations=anchors(defs);
      hop.type_parameters=anchors(defs.flatMap(d=>d.typeParameters??[]));
      if(seenTypes.has(symbol))stop("cycle");seenTypes.add(symbol);
      if(defs.length!==1)stop("ambiguous_declaration");
      const declaration=defs[0];
      if(ts.isTypeAliasDeclaration(declaration)) {type=declaration.type;continue;}
      if(ts.isInterfaceDeclaration(declaration)) {
        if(declaration.heritageClauses?.length)stop("unsupported_heritage");
        if(!declaration.members.some(ts.isCallSignatureDeclaration))stop("unsupported_type");
        result.terminal=anchor(declaration);break;
      }
      stop("unsupported_declaration");
    }
    result.status="traced";
  } catch(error) {
    if(!error?.provenanceReason)throw error;
    result.reason=error.provenanceReason;
  }
  return result;
}
