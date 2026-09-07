// Lexical observations only: no class, alias-effect or runtime value proof.
export function observeNested(ts,checker,fn,anchor,limits) {
  const result={calls:[],barriers:[]},parameter=fn.parameters[0];
  const definitions=s=>s?.declarations??[];
  const symbols=new Map(),peers=new Map(),writes=new Map();
  const addName=(name,decl)=>{
    if(ts.isIdentifier(name))peers.set(name.text,[...(peers.get(name.text)??[]),decl]);
    else if(ts.isObjectBindingPattern(name) || ts.isArrayBindingPattern(name))
      for(const e of name.elements)if(ts.isBindingElement(e))addName(e.name,e);
  };
  for(const p of fn.parameters)addName(p.name,p);
  function inventory(n) {
    if(ts.isFunctionLike(n) || ts.isClassDeclaration(n) || ts.isClassExpression(n)) {
      if(n.parent===fn.body && n.name)addName(n.name,n);
      return;
    }
    if(ts.isVariableDeclaration(n)) {
      const list=n.parent,statement=list.parent;
      if(ts.isVariableDeclarationList(list) && (!(list.flags & ts.NodeFlags.BlockScoped)
        || ts.isVariableStatement(statement) && statement.parent===fn.body))addName(n.name,n);
    }
    ts.forEachChild(n,inventory);
  }
  inventory(fn.body);
  const validParameter=parameter && !parameter.initializer && !parameter.dotDotDotToken && !parameter.questionToken;
  if(validParameter) {
    if(ts.isIdentifier(parameter.name))symbols.set(checker.getSymbolAtLocation(parameter.name),parameter);
    else if(ts.isObjectBindingPattern(parameter.name))for(const e of parameter.name.elements) {
      if(ts.isIdentifier(e.name) && !e.initializer && !e.dotDotDotToken
        && (!e.propertyName || ts.isIdentifier(e.propertyName) || ts.isStringLiteral(e.propertyName)))
        symbols.set(checker.getSymbolAtLocation(e.name),e);
    }
  }
  symbols.delete(undefined);
  function rootOf(n,strict=false) {
    while(ts.isPropertyAccessExpression(n) || !strict && ts.isElementAccessExpression(n)) {
      if(strict && n.questionDotToken)return null;
      n=n.expression;
    }
    return ts.isIdentifier(n)?n:null;
  }
  function mark(root,event,symbol) {
    if(!root)return;
    symbol??=checker.getSymbolAtLocation(root);
    if(symbols.has(symbol)) {
      if(!writes.has(symbol))writes.set(symbol,new Set());
      writes.get(symbol).add(event);
    }
  }
  function targets(n,event) {
    if(ts.isParenthesizedExpression(n) || ts.isAsExpression(n) || ts.isTypeAssertionExpression(n)
      || ts.isNonNullExpression(n))return targets(n.expression,event);
    if(ts.isBinaryExpression(n) && n.operatorToken.kind===ts.SyntaxKind.EqualsToken)return targets(n.left,event);
    if(ts.isObjectLiteralExpression(n)) {
      for(const p of n.properties) {
        if(ts.isShorthandPropertyAssignment(p))mark(p.name,event,checker.getShorthandAssignmentValueSymbol(p));
        else if(ts.isPropertyAssignment(p))targets(p.initializer,event);
        else if(ts.isSpreadAssignment(p))targets(p.expression,event);
      }
    } else if(ts.isArrayLiteralExpression(n)) {
      for(const e of n.elements)targets(ts.isSpreadElement(e)?e.expression:e,event);
    } else mark(rootOf(n),event);
  }
  function scanWrites(n) {
    if(ts.isBinaryExpression(n) && n.operatorToken.kind>=ts.SyntaxKind.FirstAssignment
      && n.operatorToken.kind<=ts.SyntaxKind.LastAssignment)targets(n.left,n);
    if((ts.isPrefixUnaryExpression(n) || ts.isPostfixUnaryExpression(n))
      && [ts.SyntaxKind.PlusPlusToken,ts.SyntaxKind.MinusMinusToken].includes(n.operator))targets(n.operand,n);
    if(ts.isDeleteExpression(n))targets(n.expression,n);
    if(ts.isForInStatement(n) || ts.isForOfStatement(n))targets(n.initializer,n);
    ts.forEachChild(n,scanWrites);
  }
  scanWrites(fn.body);
  function binding(receiver,optional) {
    const root=optional?null:rootOf(receiver,true),symbol=root && checker.getSymbolAtLocation(root);
    const defs=definitions(symbol),own=symbols.get(symbol);
    const events=[...(writes.get(symbol)??[])];
    const reason=!root?"unsupported_receiver":!symbol?"unresolved_symbol"
      :!validParameter?"unsupported_parameter":!own?"other_binding"
      :defs.length!==1 || (peers.get(root.text)?.length??0)!==1?"duplicate_binding"
      :events.length?"write_barrier":null;
    return {status:reason?"unproven":"linked",reason,use:root?anchor(root):null,
      parameter:parameter?anchor(parameter):null,declarations:defs.map(anchor),writes:events.map(anchor)};
  }
  const callLimit={};
  function visit(n,functions) {
    if(ts.isArrowFunction(n) || ts.isFunctionExpression(n)) {
      if(functions.length>=limits.nested_depth) {
        result.barriers.push({scope:anchor(n),reason:"depth_limit"});return;
      }
      visit(n.body,[...functions,anchor(n)]);return;
    }
    if(ts.isFunctionLike(n) || ts.isClassDeclaration(n) || ts.isClassExpression(n)) {
      result.barriers.push({scope:anchor(n),reason:"unsupported_scope"});return;
    }
    if(functions.length && ts.isCallExpression(n) && ts.isPropertyAccessExpression(n.expression)) {
      if(result.calls.length>=limits.nested_calls) {
        result.barriers.push({scope:anchor(n),reason:"call_limit"});throw callLimit;
      }
      const receiver=n.expression.expression;
      result.calls.push({call:anchor(n),receiver:anchor(receiver),functions,
        receiver_type:checker.typeToString(checker.getTypeAtLocation(receiver)),
        declarations:definitions(checker.getSymbolAtLocation(n.expression.name)).map(anchor),
        binding:binding(receiver,n.questionDotToken || n.expression.questionDotToken)});
    }
    ts.forEachChild(n,c=>visit(c,functions));
  }
  try {visit(fn.body,[]);}catch(error){if(error!==callLimit)throw error;}
  return result;
}
