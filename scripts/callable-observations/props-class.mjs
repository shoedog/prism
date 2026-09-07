// Compiler-instantiated declared types only. Never a runtime receiver certificate.
export function observePropsClasses(ts,checker,fn,signatures,observation,anchor,limit) {
  const defs=s=>s?.declarations??[],anchors=nodes=>nodes.map(anchor);
  const calls=new Map();
  function collect(n){if(ts.isCallExpression(n))calls.set(n.getStart(),n);ts.forEachChild(n,collect);}
  collect(fn.body);
  for(const call of observation.nested.calls) {
    const out={status:"unproven",reason:null,signatures:[],props_type:null,props_declarations:[],
      instantiation:[],property_name:null,property_declarations:[],property_type:null,
      declared_type_declarations:[],class_declaration:null};call.props_class=out;
    const stop=reason=>{throw {propsReason:reason};};
    try {
      if(call.binding.status!=="linked")stop("binding_unproven");
      if(observation.explicit_parameter)stop("explicit_parameter");
      if(observation.provenance.status!=="traced")stop("callable_unproven");
      out.signatures=anchors(signatures.flatMap(s=>s.declaration?[s.declaration]:[]));
      if(signatures.length!==1 || signatures[0].parameters.length<1 || out.signatures.length!==1)stop("signature_ambiguity");
      const actual=calls.get(call.call.start_utf16),parts=[];
      let receiver=actual.expression.expression;
      while(ts.isPropertyAccessExpression(receiver)){parts.unshift(receiver.name.text);receiver=receiver.expression;}
      const parameter=fn.parameters[0];
      if(ts.isObjectBindingPattern(parameter.name)) {
        const binding=parameter.name.elements.find(e=>e.getStart()===call.binding.declarations[0].start_utf16);
        if(!binding || parts.length)stop("unsupported_path");
        parts.push((binding.propertyName??binding.name).text);
      }
      if(parts.length!==1)stop("unsupported_path");
      out.property_name=parts[0];
      const props=checker.getTypeOfSymbolAtLocation(signatures[0].parameters[0],fn);
      out.props_type=checker.typeToString(props);
      const declarations=defs(props.symbol),aliases=defs(props.aliasSymbol);
      out.props_declarations=anchors([...aliases,...declarations]);
      if(declarations.length!==1 || aliases.length>1)stop("ambiguous_declaration");
      const shape=declarations[0],owner=aliases[0]??shape;
      if(!(props.flags & ts.TypeFlags.Object) || (props.objectFlags & ts.ObjectFlags.Mapped)
        || !(ts.isInterfaceDeclaration(shape) || ts.isTypeLiteralNode(shape))
        || shape.heritageClauses?.length || aliases.length && (!ts.isTypeAliasDeclaration(owner)
          || !ts.isTypeLiteralNode(owner.type) || owner.type!==shape)
        || shape.members.some(m=>!ts.isPropertySignature(m) || ts.isComputedPropertyName(m.name)))stop("unsupported_props");
      const parameters=owner.typeParameters??[];
      const args=props.aliasTypeArguments??((props.objectFlags & ts.ObjectFlags.Reference)?checker.getTypeArguments(props):[]);
      if(args.length>limit)stop("type_argument_limit");
      if(parameters.length!==args.length)stop("unsupported_props");
      out.instantiation=parameters.map((p,i)=>({parameter:anchor(p),argument_type:checker.typeToString(args[i]),argument_declarations:anchors(defs(args[i].symbol))}));
      const property=checker.getPropertyOfType(props,out.property_name),propertyDefs=defs(property);
      out.property_declarations=anchors(propertyDefs);
      if(propertyDefs.length!==1)stop(propertyDefs.length?"ambiguous_declaration":"unsupported_property");
      const p=propertyDefs[0];
      if(!ts.isPropertySignature(p) || p.parent!==shape || p.questionToken || !p.type || !ts.isTypeReferenceNode(p.type))stop("unsupported_property");
      let declared=checker.getSymbolAtLocation(p.type.typeName);
      if(declared && declared.flags & ts.SymbolFlags.Alias)declared=checker.getAliasedSymbol(declared);
      const declaredDefs=defs(declared);out.declared_type_declarations=anchors(declaredDefs);
      if(declaredDefs.length!==1 || !(ts.isClassDeclaration(declaredDefs[0]) || parameters.includes(declaredDefs[0])))stop("unsupported_property");
      const type=checker.getTypeOfSymbolAtLocation(property,fn),classDefs=defs(type.symbol);
      out.property_type=checker.typeToString(type);
      // typeof C and instances share a ClassDeclaration. Require the instance type.
      if(!(type.flags & ts.TypeFlags.Object) || !(type.objectFlags & ts.ObjectFlags.Class)
        || classDefs.length!==1 || !ts.isClassDeclaration(classDefs[0])
        || classDefs[0].typeParameters?.length || classDefs[0].heritageClauses?.length)stop("unsupported_class");
      out.class_declaration=anchor(classDefs[0]);out.status="observed";
    } catch(error) {
      if(!error?.propsReason)throw error;
      out.reason=error.propsReason;
    }
  }
}
