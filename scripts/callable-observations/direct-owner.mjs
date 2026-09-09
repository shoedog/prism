// Detached compiler facts ONLY. The private Rust constructor additionally requires
// reproduced closure, exact owned input sets and Prism parser/member admissibility.
import {hash} from "./schema.mjs";

export function directOwnerFacts({ts,program,checker,anchor}) {
  const candidates=[], refusals=[];
  const project=program.getSourceFiles().filter(sf=>sf.fileName.startsWith('/__prism__/project/'));
  const sources=project.map(sf=>({file:sf.fileName.slice('/__prism__/project/'.length),sha256:hash(sf.text)})).sort((a,b)=>a.file<b.file?-1:1);
  const require=(condition,reason)=>{if(!condition)throw {ownerRefusal:reason};};
  const modifiers=(node,allowed=[])=>!(node.modifiers??[]).some(m=>!allowed.includes(m.kind));
  const one=(symbol,node)=>symbol?.declarations?.length===1 && symbol.declarations[0]===node;
  const symbol=node=>checker.getSymbolAtLocation(node);
  const resolved=node=>{const s=symbol(node);return s?.flags & ts.SymbolFlags.Alias?checker.getAliasedSymbol(s):s;};
  const declaration=node=>{const s=resolved(node);return s?.declarations?.length===1?s.declarations[0]:null;};
  const visit=(node,fn)=>{fn(node);ts.forEachChild(node,n=>visit(n,fn));};
  const projectBody=node=>node && project.includes(node.getSourceFile()) && !node.getSourceFile().isDeclarationFile
    && !node.getSourceFile().fileName.includes('/node_modules/');
  const key=node=>node && (ts.isIdentifier(node)||ts.isStringLiteral(node))?node.text:null;
  let effect=false;
  for(const sf of project)visit(sf,n=>{
    effect ||= ts.isModuleDeclaration(n) || !!(ts.getCombinedModifierFlags(n)&ts.ModifierFlags.Ambient);
    if(ts.isPropertyAccessExpression(n)||ts.isElementAccessExpression(n)) {
      const member=key(ts.isPropertyAccessExpression(n)?n.name:n.argumentExpression);
      effect ||= member==='prototype' || ts.isIdentifier(n.expression) && ['Object','Reflect'].includes(n.expression.text)
        && ['assign','defineProperty','defineProperties','set','deleteProperty','setPrototypeOf'].includes(member);
    }
  });
  function imported(reference,typeOnly,predicate,reason) {
    require(ts.isIdentifier(reference),reason);
    const importedSymbol=symbol(reference), spec=importedSymbol?.declarations?.[0];
    require(spec && one(importedSymbol,spec) && ts.isImportSpecifier(spec) && !spec.propertyName,reason);
    const clause=spec.parent.parent, statement=clause.parent;
    require(ts.isImportClause(clause) && !clause.name && ts.isImportDeclaration(statement)
      && ts.isSourceFile(statement.parent) && ts.isStringLiteral(statement.moduleSpecifier)
      && !statement.attributes && (clause.isTypeOnly||spec.isTypeOnly)===typeOnly,reason);
    const module=statement.moduleSpecifier.text;
    require(/^\.\.?\//.test(module) && !module.includes('\\'),reason);
    const target=declaration(reference), moduleSymbol=symbol(statement.moduleSpecifier);
    require(target && predicate(target) && projectBody(target) && ts.isSourceFile(target.parent)
      && one(moduleSymbol,target.getSourceFile()) && modifiers(target,[ts.SyntaxKind.ExportKeyword])
      && (ts.getCombinedModifierFlags(target)&ts.ModifierFlags.Export)!==0,reason);
    return {target,statement,module};
  }
  function written(scope,wanted) {
    let found=false;
    function target(node) {
      if(ts.isIdentifier(node))found ||= resolved(node)===wanted;
      else if(ts.isParenthesizedExpression(node)||ts.isArrayLiteralExpression(node)||ts.isObjectLiteralExpression(node)
        || ts.isSpreadElement(node)||ts.isSpreadAssignment(node)||ts.isPropertyAssignment(node)
        || ts.isShorthandPropertyAssignment(node))ts.forEachChild(node,target);
    }
    visit(scope,n=>{
      if(ts.isBinaryExpression(n) && n.operatorToken.kind>=ts.SyntaxKind.FirstAssignment && n.operatorToken.kind<=ts.SyntaxKind.LastAssignment)target(n.left);
      if(ts.isPrefixUnaryExpression(n)||ts.isPostfixUnaryExpression(n)) {
        if([ts.SyntaxKind.PlusPlusToken,ts.SyntaxKind.MinusMinusToken].includes(n.operator))target(n.operand);
      }
      if(ts.isForInStatement(n)||ts.isForOfStatement(n))target(n.initializer);
    });
    return found;
  }
  for(const sf of project) {
    if(!projectBody(sf))continue;
    for(const statement of sf.statements) {
      if(!ts.isVariableStatement(statement))continue;
      for(const variable of statement.declarationList.declarations) {
        if(!variable.type)continue;
        try {
          const fn=variable.initializer,parameter=fn?.parameters?.[0];
          require(!effect,'indexed_effect');
          require((statement.declarationList.flags&ts.NodeFlags.Const)!==0 && statement.declarationList.declarations.length===1
            && modifiers(statement,[ts.SyntaxKind.ExportKeyword]) && ts.isIdentifier(variable.name) && one(symbol(variable.name),variable)
            && fn && ts.isArrowFunction(fn) && !fn.typeParameters?.length && modifiers(fn)
            && fn.parameters.length===1 && !fn.type && parameter && !parameter.type && !parameter.questionToken
            && !parameter.initializer && !parameter.dotDotDotToken && modifiers(parameter)
            && ts.isObjectBindingPattern(parameter.name) && parameter.name.elements.length===1,'implementation');
          const binding=parameter.name.elements[0];
          require(!binding.propertyName && !binding.initializer && !binding.dotDotDotToken && ts.isIdentifier(binding.name),'binding');
          require(ts.isBlock(fn.body) && fn.body.statements.length===1 && ts.isExpressionStatement(fn.body.statements[0]),'direct_body');
          const call=fn.body.statements[0].expression, access=call.expression;
          require(ts.isCallExpression(call) && !call.questionDotToken && !call.typeArguments?.length && call.arguments.length===0
            && ts.isPropertyAccessExpression(access) && !access.questionDotToken && ts.isIdentifier(access.expression)
            && ts.isIdentifier(access.name) && one(symbol(access.expression),binding),'direct_call');
          const annotation=variable.type;
          require(ts.isTypeReferenceNode(annotation) && annotation.typeArguments?.length===1,'callable');
          const callable=imported(annotation.typeName,true,ts.isTypeAliasDeclaration,'callable');
          const alias=callable.target,binder=alias.typeParameters?.[0],signature=alias.type;
          require(alias.typeParameters?.length===1 && binder && !binder.constraint && !binder.default && modifiers(binder)
            && ts.isFunctionTypeNode(signature) && !signature.typeParameters?.length && signature.parameters.length===1
            && signature.type.kind===ts.SyntaxKind.VoidKeyword,'callable');
          const signatureParameter=signature.parameters[0];
          require(ts.isIdentifier(signatureParameter.name) && !signatureParameter.questionToken && !signatureParameter.dotDotDotToken
            && !signatureParameter.initializer && modifiers(signatureParameter) && signatureParameter.type
            && ts.isTypeReferenceNode(signatureParameter.type) && !signatureParameter.type.typeArguments?.length
            && declaration(signatureParameter.type.typeName)===binder,'substitution');
          const propsReference=annotation.typeArguments[0];
          require(ts.isTypeReferenceNode(propsReference) && ts.isIdentifier(propsReference.typeName) && !propsReference.typeArguments?.length,'props');
          const propsAlias=declaration(propsReference.typeName);
          require(propsAlias && ts.isTypeAliasDeclaration(propsAlias) && propsAlias.parent===sf && modifiers(propsAlias)
            && !propsAlias.typeParameters?.length && ts.isTypeLiteralNode(propsAlias.type) && propsAlias.type.members.length===1,'props');
          const property=propsAlias.type.members[0];
          require(ts.isPropertySignature(property) && ts.isIdentifier(property.name) && property.name.text===binding.name.text
            && !property.questionToken && modifiers(property) && property.type && ts.isTypeReferenceNode(property.type)
            && !property.type.typeArguments?.length,'property');
          const context=checker.getContextualType(fn),signatures=context?checker.getSignaturesOfType(context,ts.SignatureKind.Call):[];
          require(signatures.length===1 && signatures[0].declaration===signature && signatures[0].parameters.length===1,'substitution');
          const instantiated=checker.getTypeOfSymbolAtLocation(signatures[0].parameters[0],fn);
          require(instantiated===checker.getTypeFromTypeNode(propsReference) && one(instantiated.aliasSymbol,propsAlias)
            && one(instantiated.symbol,propsAlias.type),'substitution');
          const propertySymbol=checker.getPropertyOfType(instantiated,property.name.text);
          require(one(propertySymbol,property),'property');
          const owner=imported(property.type.typeName,false,ts.isClassDeclaration,'class');
          const klass=owner.target;
          require(klass.name && !klass.typeParameters?.length && !klass.heritageClauses?.length,'class');
          const classSymbol=resolved(property.type.typeName),instance=checker.getTypeOfSymbolAtLocation(propertySymbol,fn);
          require(one(classSymbol,klass) && instance.symbol===classSymbol && (instance.flags&ts.TypeFlags.Object)!==0
            && (instance.objectFlags&ts.ObjectFlags.Class)!==0 && checker.getTypeAtLocation(access.expression)===instance,'class');
          const memberSymbol=symbol(access.name),member=memberSymbol?.declarations?.[0];
          require(member && one(memberSymbol,member) && ts.isMethodDeclaration(member) && member.parent===klass
            && ts.isIdentifier(member.name) && member.name.text===access.name.text && member.body
            && !member.asteriskToken && !member.typeParameters?.length && modifiers(member,[ts.SyntaxKind.PublicKeyword])
            && !member.questionToken && klass.members.filter(m=>key(m.name)===member.name.text).length===1
            && !klass.members.some(m=>m.name && ts.isComputedPropertyName(m.name)),'member');
          require(!project.some(source=>written(source,classSymbol)||written(source,symbol(variable.name))),'binding_write');
          let thisWrite=false;
          visit(klass,n=>{
            const targets=[];
            if(ts.isBinaryExpression(n)&&n.operatorToken.kind>=ts.SyntaxKind.FirstAssignment&&n.operatorToken.kind<=ts.SyntaxKind.LastAssignment)targets.push(n.left);
            if(ts.isDeleteExpression(n)||ts.isPostfixUnaryExpression(n)||ts.isPrefixUnaryExpression(n))targets.push(n.expression??n.operand);
            if(ts.isForInStatement(n)||ts.isForOfStatement(n))targets.push(n.initializer);
            for(const target of targets)visit(target,t=>{
              if((ts.isPropertyAccessExpression(t)||ts.isElementAccessExpression(t)) && t.expression.kind===ts.SyntaxKind.ThisKeyword)
                thisWrite ||= ts.isElementAccessExpression(t)||key(t.name)===member.name.text;
            });
          });
          require(!thisWrite,'member_write');
          const classKeyword=klass.getChildren().find(n=>n.kind===ts.SyntaxKind.ClassKeyword);
          require(classKeyword,'class');
          const nodes={annotation,implementation:fn,parameter,binding,call,receiver:access.expression,
            callable_import:callable.statement,callable_alias:alias,binder,binder_use:signatureParameter.type,
            signature,signature_parameter:signatureParameter,props_reference:propsReference,props_alias:propsAlias,property,
            class_reference:property.type,class_import:owner.statement,class_declaration:klass,class_keyword:classKeyword,
            member_declaration:member,member_body:member.body};
          candidates.push({anchors:Object.fromEntries(Object.entries(nodes).map(([k,n])=>[k,anchor(n)])),
            class_name:klass.name.text,member_name:member.name.text,callable_module:callable.module,class_module:owner.module});
        } catch(error) {
          if(!error?.ownerRefusal)throw error;
          refusals.push({annotation:anchor(variable.type),reason:error.ownerRefusal});
        }
      }
    }
  }
  return {sources,candidates,refusals};
}
