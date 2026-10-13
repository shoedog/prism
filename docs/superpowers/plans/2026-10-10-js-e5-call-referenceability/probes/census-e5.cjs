#!/usr/bin/env node
'use strict';
// Static TypeScript 5.9.3 oracle. No corpus modules are loaded or executed.
const fs=require('fs'),path=require('path');
function census(tsPath,root,sitesFile,options={}) {
 const ts=require(tsPath); if(ts.version!=='5.9.3')throw Error('oracle must be 5.9.3');
 const factKey=(caller,span,name,origin,target)=>JSON.stringify([caller.file,caller.name,caller.start_line,caller.end_line,...span,name,origin,target.file,target.name,target.start_line,target.end_line]);
 const facts=new Map(options.factsFile?fs.readFileSync(options.factsFile,'utf8').split('\n').filter(Boolean).map(JSON.parse).map(f=>[factKey(f.caller,f.span,f.name,f.origin,f.target),f]):[]);
 root=path.resolve(root);const forbidden=p=>{const abs=path.resolve(p);return abs.split(path.sep).includes('frontend-portal')&&!(options.controllerPrivate&&(abs===root||abs.startsWith(root+path.sep)));};
 if(forbidden(root))throw Error('forbidden root');
 const skip=new Set(['target','node_modules','vendor','dist','build']); const files=[];
 function walk(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){if(e.isSymbolicLink())continue; const p=path.join(dir,e.name);if(e.isDirectory()){if(forbidden(p))throw Error('forbidden directory below root');if(!e.name.startsWith('.')&&!skip.has(e.name))walk(p);}else if(/\.(js|jsx|mjs|cjs|ts|tsx)$/.test(e.name)&&fs.statSync(p).size<=2*1024*1024)files.push(p);}}
 walk(root);
 const opts={allowJs:true,checkJs:true,types:[],jsx:ts.JsxEmit.Preserve,target:ts.ScriptTarget.ESNext,module:ts.ModuleKind.ESNext,moduleResolution:ts.ModuleResolutionKind.Bundler,skipLibCheck:true,noEmit:true};
 const host=ts.createCompilerHost(opts);
 for(const key of ['readFile','fileExists','directoryExists','getDirectories','realpath']){const original=host[key];if(original)host[key]=function(p,...a){if(forbidden(p))return key==='getDirectories'?[]:key==='fileExists'||key==='directoryExists'?false:undefined;return original.call(host,p,...a);};}
 const program=ts.createProgram(files,opts,host),checker=program.getTypeChecker(),indices=new Map();
 function index(rel){const abs=path.resolve(root,rel);if(forbidden(abs))throw Error('forbidden source');if(indices.has(abs))return indices.get(abs);
  const sf=program.getSourceFile(abs),functions=[],calls=[],bindings=[];let why=null,map=null;
  if(!sf)why='source_not_in_program';else{const raw=fs.readFileSync(abs),encoded=Buffer.from(sf.text);let bom=raw.length===encoded.length+3&&raw.subarray(0,3).equals(Buffer.from([239,187,191]))&&raw.subarray(3).equals(encoded);if(!raw.equals(encoded)&&!bom)why='physical_compiler_text_mismatch';else{map=new Uint32Array(sf.text.length+1);let b=bom?3:0;for(let i=0;i<sf.text.length;i++){map[i]=b;let cp=sf.text.codePointAt(i);if(cp>65535){map[++i]=b;b+=4;}else b+=cp<128?1:cp<2048?2:3;}map[sf.text.length]=b;}
   const visit=n=>{if((ts.isFunctionLike(n)&&n.body)||ts.isArrowFunction(n))functions.push(n);if(ts.isCallExpression(n)||ts.isNewExpression(n)||ts.isJsxOpeningElement(n)||ts.isJsxSelfClosingElement(n))calls.push(n);if(ts.isImportSpecifier(n)||ts.isImportClause(n)||ts.isImportEqualsDeclaration(n)||ts.isVariableDeclaration(n)||ts.isFunctionDeclaration(n)||ts.isParameter(n)||ts.isBindingElement(n))bindings.push(n);ts.forEachChild(n,visit);};visit(sf);}
  const out={sf,functions,calls,bindings,map,why};indices.set(abs,out);return out;
 }
 function ownerStart(n,sf){let start=n.getStart(sf);for(const m of n.modifiers||[])if(m.kind===ts.SyntaxKind.ExportKeyword||m.kind===ts.SyntaxKind.DefaultKeyword)start=m.end;const scanner=ts.createScanner(ts.ScriptTarget.ESNext,true,sf.languageVariant,sf.text);scanner.setTextPos(start);scanner.scan();return scanner.getTokenPos();}
 function line(sf,pos){return sf.getLineAndCharacterOfPosition(pos).line+1;}
 function name(n){if(n.name)return n.name.text??n.name.getText();const p=n.parent;if(ts.isVariableDeclaration(p)||ts.isPropertyAssignment(p)||ts.isPropertyDeclaration(p))return p.name.text??p.name.getText();if(ts.isCallExpression(p)&&ts.isVariableDeclaration(p.parent))return p.parent.name.text??p.parent.name.getText();if(ts.isBinaryExpression(p)&&p.right===n)return p.left.name?.text??p.left.text;return null;}
 function form(n){const p=n.parent;if(ts.isFunctionDeclaration(n))return 'function_declaration';if(ts.isMethodDeclaration(n)||ts.isGetAccessorDeclaration(n)||ts.isSetAccessorDeclaration(n)||ts.isConstructorDeclaration(n))return ts.isObjectLiteralExpression(p)?'object_shorthand_method':'class_member';if(ts.isPropertyDeclaration(p))return 'class_member';if(ts.isPropertyAssignment(p))return 'object_pair_value';if(ts.isVariableDeclaration(p))return 'variable_direct_function';if(ts.isBinaryExpression(p)&&p.right===n)return ts.isPropertyAccessExpression(p.left)||ts.isElementAccessExpression(p.left)?'member_assignment':'other:identifier_assignment';if(ts.isCallExpression(p)&&ts.isVariableDeclaration(p.parent))return 'variable_wrapper_call';if(ts.isFunctionExpression(n)&&n.name)return 'other:named_function_expression';return 'other:'+ts.SyntaxKind[p.kind];}
 function requireSource(d){for(let p=d;p&&!ts.isSourceFile(p);p=p.parent){if(ts.isVariableDeclaration(p)&&p.initializer){const x=p.initializer;if(ts.isCallExpression(x)&&ts.isIdentifier(x.expression)&&x.expression.text==='require'&&x.arguments.length===1&&ts.isStringLiteral(x.arguments[0]))return x.arguments[0].text;}}return null;}
 function importSource(d){for(let p=d;p;p=p.parent){if(ts.isImportDeclaration(p))return p.moduleSpecifier.text;if(ts.isImportEqualsDeclaration(p)&&ts.isExternalModuleReference(p.moduleReference))return p.moduleReference.expression?.text; }return null;}
 const knownGlobals=new Set('Array Object String Number Boolean Function Promise Math JSON Date RegExp Error TypeError RangeError SyntaxError Set Map WeakSet WeakMap Symbol Reflect Proxy BigInt NaN Infinity undefined parseInt parseFloat isNaN isFinite encodeURI decodeURI encodeURIComponent decodeURIComponent escape unescape eval globalThis Buffer setTimeout setInterval clearTimeout clearInterval setImmediate clearImmediate queueMicrotask fetch atob btoa require console process'.split(' '));
 function symbolDeclarations(id){let s=checker.getSymbolAtLocation(id);if(s&&(s.flags&ts.SymbolFlags.Alias)){let target=checker.getAliasedSymbol(s);if(target&&target.declarations?.length)return {symbol:target,declarations:target.declarations,alias:s};return {symbol:s,declarations:s.declarations||[],unresolvedAlias:true};}return {symbol:s,declarations:s?.declarations||[]};}
 function binding(id){const sym=checker.getSymbolAtLocation(id),ds=sym?.declarations||[];for(const d of ds){const source=importSource(d);if(source!==null)return {axis:'imported',source};const req=requireSource(d);if(req!==null&&ts.isBindingElement(d))return {axis:'require_destructuring',source:req};}
  if(ds.some(d=>path.resolve(d.getSourceFile().fileName)===path.resolve(id.getSourceFile().fileName))){
   const locals=ds.filter(d=>path.resolve(d.getSourceFile().fileName)===path.resolve(id.getSourceFile().fileName));
   const owners=locals.map(d=>{for(let p=d;p&&!ts.isSourceFile(p);p=p.parent)if(ts.isVariableDeclaration(p)||ts.isParameter(p))return p;return d;});
   const hasRequire=n=>{if(!n)return false;if(ts.isCallExpression(n)&&ts.isIdentifier(n.expression)&&n.expression.text==='require')return true;let found=false;ts.forEachChild(n,c=>{if(hasRequire(c))found=true;});return found;};
   return {axis:'local_declaration',declarations:locals.map(d=>ts.SyntaxKind[d.kind]),require_initialized:owners.some(d=>hasRequire(d.initializer)),literal_scalar_require:locals.some(d=>ts.isVariableDeclaration(d)&&ts.isIdentifier(d.name)&&requireSource(d)!==null),details:locals.map(d=>({kind:ts.SyntaxKind[d.kind],line:line(d.getSourceFile(),d.getStart()),text:d.getText().replace(/\s+/g,' ').slice(0,240)}))};
  }
  if(knownGlobals.has(id.text)||ds.some(d=>/[/\\]lib\.[^/\\]*\.d\.ts$/.test(d.getSourceFile().fileName)))return {axis:'known_global'};return {axis:'none'};
 }
 function targetDeclarationMatches(d,target){if(d===target)return true;if(ts.isVariableDeclaration(d)&&d.initializer===target)return true;if(ts.isPropertyAssignment(d)&&d.initializer===target)return true;if(ts.isBinaryExpression(d)&&d.right===target)return true;return false;}
 function decide(call,target){const expression=call.expression||call.tagName;const id=ts.isPropertyAccessExpression(expression)?expression.name:expression;if(!ts.isIdentifier(id))return {verdict:'INADMISSIBLE',why:'callee_not_identifier_or_property'};
  const sf=id.getSourceFile(),tf=target.getSourceFile();if(sf.parseDiagnostics.length||tf.parseDiagnostics.length)return {verdict:'INADMISSIBLE',why:'parse_diagnostics_in_endpoint_file'};
  const s=symbolDeclarations(id);if(s.unresolvedAlias)return {verdict:'UNDECIDED',why:'import_alias_target_unresolved'};
  if(s.declarations.some(d=>targetDeclarationMatches(d,target)))return {verdict:'CORRECT',why:'checker_declaration_matches_callable'};
  const sig=checker.getResolvedSignature(call);
  if(sig?.declaration===target){
   const p=target.parent;
   const scalarExport=ts.isBinaryExpression(p)&&p.right===target&&ts.isPropertyAccessExpression(p.left)&&p.left.name.text==='exports'&&ts.isIdentifier(p.left.expression)&&p.left.expression.text==='module'&&ts.isExpressionStatement(p.parent)&&ts.isSourceFile(p.parent.parent);
   const directRequire=scalarExport&&s.declarations.some(d=>{
    if(!ts.isVariableDeclaration(d)||!d.initializer||!ts.isCallExpression(d.initializer)||requireSource(d)===null)return false;
    const requireSym=checker.getSymbolAtLocation(d.initializer.expression);
    if(requireSym?.declarations?.some(d=>files.includes(path.resolve(d.getSourceFile().fileName))))return false;
    const resolved=ts.resolveModuleName(requireSource(d),d.getSourceFile().fileName,opts,host).resolvedModule;
    return resolved&&path.resolve(resolved.resolvedFileName)===path.resolve(target.getSourceFile().fileName);
   });
   if(directRequire)return {verdict:'CORRECT',why:'literal_require_binding_and_checker_callable_match'};
   return {verdict:'UNDECIDED',why:'signature_matches_without_value_binding_identity'};
  }
  if(!s.symbol)return {verdict:'INADMISSIBLE',why:'checker_callee_binding_unresolved'};
  if(s.declarations.length)return {verdict:'WRONG',why:'checker_declarations_differ',checker_targets:s.declarations.map(d=>({file:path.relative(root,d.getSourceFile().fileName),line:line(d.getSourceFile(),d.getStart()),kind:ts.SyntaxKind[d.kind]}))};
  return {verdict:'UNDECIDED',why:'callee_symbol_has_no_declarations'};
 }
 const rows=[];for(const text of fs.readFileSync(sitesFile,'utf8').split('\n')){if(!text)continue;const site=JSON.parse(text);if(site.record_kind!=='call_site')continue;for(const resolved of site.resolved_targets||[]){if((!options.allResolved&&(resolved.kind!=='free_single'||resolved.confidence!=='exact'))||!/\.(js|jsx|mjs|cjs|ts|tsx)$/.test(site.caller.file))continue;
   const target=resolved.function_id,ci=index(site.caller.file),ti=index(target.file),fact=facts.get(factKey(site.caller,[site.source_span.start_byte,site.source_span.end_byte],site.callee_text,site.origin,target));
   const row={site,target,target_kind:resolved.kind,target_confidence:resolved.confidence,raw_verdict:'UNDECIDED',verdict:'UNDECIDED',callee_form:'other:unidentified',binding:'none',why:'identity_not_matched',cross_file:site.caller.file!==target.file,es_module:!!(ci.sf&&ts.isExternalModule(ci.sf)),override:null};
   const shapes=fact?.forms||[];const forms=[...new Set(shapes.map(f=>f.form))];if(forms.length===1)row.callee_form=forms[0];else if(forms.length>1)row.callee_form='other:mixed:'+forms.sort().join(',');
   if(shapes.length===1){row.target_bytes=[shapes[0].start_byte,shapes[0].end_byte];row.name_source=shapes[0].name_explicit?'explicit':'inferred';row.callee_module=shapes[0].esm;row.callable_kind=shapes[0].kind;}
   if(fact)row.prism_binding=fact.local_binding;
   const calls=ci.why?[]:ci.calls.filter(n=>ci.map[n.getStart(ci.sf)]===site.source_span.start_byte&&ci.map[n.end]===site.source_span.end_byte);
   const call=calls.length===1?calls[0]:null;
   if(call){const expression=call.expression||call.tagName;row.invocation=call.tagName?'jsx':ts.isIdentifier(expression)?'ordinary':'qualified';const id=ts.isIdentifier(expression)?expression:null;
    if(id){const b=binding(id);row.binding=b.axis;row.binding_evidence=b;row.identifier_bytes=[ci.map[id.getStart(ci.sf)],ci.map[id.end]];}
    else row.binding_evidence={axis:'none',why:'qualified_source_has_no_bare_callee_identifier'};
   }else row.invocation=fact?.jsx?'jsx':fact?.qualifier?'qualified':fact&&fact.local_binding!=='Unchecked'?'ordinary':'unidentified';
   let targets=ti.why?[]:ti.functions.filter(n=>name(n)===target.name&&line(ti.sf,ownerStart(n,ti.sf))===target.start_line&&line(ti.sf,n.end)===target.end_line);
   if(shapes.length===1&&!ti.why)targets=ti.functions.filter(n=>ti.map[ownerStart(n,ti.sf)]===shapes[0].start_byte&&ti.map[n.end]===shapes[0].end_byte);
   if(targets.length===1){const fn=targets[0];if(!fact)row.callee_form=form(fn);row.target_bytes??=[ti.map[ownerStart(fn,ti.sf)],ti.map[fn.end]];row.name_source??=fn.name?'explicit':'inferred';row.callee_module??=ts.isExternalModule(ti.sf);row.callable_kind??=ts.SyntaxKind[fn.kind];}
   if(ci.why||ti.why){row.verdict='INADMISSIBLE';row.why=ci.why||ti.why;}
   else if(ci.sf.parseDiagnostics.length||ti.sf.parseDiagnostics.length){row.verdict='INADMISSIBLE';row.why='parse_diagnostics_in_endpoint_file';}
   else if(call&&targets.length===1)Object.assign(row,decide(call,targets[0]));
   else row.why=`identity_ambiguous:call=${calls.length},target=${targets.length}`;
   row.raw_verdict=row.verdict;
   const inferred=['object_shorthand_method'].includes(row.callee_form)||row.callable_kind==='method_definition'||row.name_source==='inferred'&&['object_pair_value','member_assignment','class_member','variable_wrapper_call'].includes(row.callee_form);
   const bare=row.invocation==='ordinary',source=bare&&site.origin==='Source',unbound=source&&row.prism_binding?.Unproven==='unbound';
   row.rules={'R-inferred':bare&&inferred,'R-inferred-form':bare&&['object_pair_value','object_shorthand_method','member_assignment','class_member','variable_wrapper_call'].includes(row.callee_form),'R-bound':row.cross_file&&row.es_module&&!['imported','require_destructuring'].includes(row.binding),'R-bound-source':row.cross_file&&row.es_module&&source&&!['imported','require_destructuring'].includes(row.binding),'R-unbound-inferred':unbound&&inferred,'R-unbound-module':unbound&&row.callee_module,'R-recommended':unbound&&(inferred||row.callee_module)};
   // Preserve historical census axes: scalar require is local_declaration.
   // Price its explicit exemption separately, without changing any oracle verdict.
   row.rules['R-local']=row.cross_file&&row.binding==='local_declaration';
   row.rules['R-local-no-require']=row.rules['R-local']&&!row.binding_evidence?.require_initialized;
   row.rules['R-union']=row.rules['R-recommended']||row.rules['R-bound'];
   rows.push(row);
  }}return {rows,files:files.length,oracle:ts.version};
}
function summarize(rows){const counts={},rules={};for(const r of rows){const key=[r.verdict,r.callee_form,r.binding].join('|');counts[key]=(counts[key]||0)+1;for(const [name,touched]of Object.entries(r.rules||{})){if(!touched)continue;rules[name]??={CORRECT:0,WRONG:0,UNDECIDED:0,INADMISSIBLE:0};rules[name][r.verdict]++;}}return {total:rows.length,cross_tab:counts,rules};}
module.exports={census,summarize};
if(require.main===module){const [tsPath,root,sitesFile,out,...flags]=process.argv.slice(2);const fi=flags.indexOf('--facts');const result=census(tsPath,root,sitesFile,{factsFile:fi>=0?flags[fi+1]:null,allResolved:flags.includes('--all-resolved'),controllerPrivate:flags.includes('--controller-private')});fs.writeFileSync(out,result.rows.map(r=>JSON.stringify(r)).join('\n')+(result.rows.length?'\n':''));fs.writeFileSync(out+'.summary.json',JSON.stringify({...summarize(result.rows),files:result.files,oracle:result.oracle},null,2)+'\n');console.log(JSON.stringify(summarize(result.rows)));}
