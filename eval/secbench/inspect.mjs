// Parse pre-fetched JavaScript/TypeScript without loading or executing packages.
import fs from 'node:fs';
import {lexicalBindings,checkerResolver} from './bindings.mjs';
import path from 'node:path';
import crypto from 'node:crypto';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
export const COMPILER_SHA = '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675';
export const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const suffix = /\.(?:[cm]?js|jsx|[cm]?ts|tsx)$/;
const callable = (ts,n) => Boolean(n?.body) && (ts.isFunctionDeclaration(n) || ts.isFunctionExpression(n)
  || ts.isArrowFunction(n) || ts.isMethodDeclaration(n) || ts.isConstructorDeclaration(n));
const unwrap = (ts,n) => ts.isParenthesizedExpression(n) ? unwrap(ts,n.expression) : n;
const prop = (ts,n) => n && (ts.isIdentifier(n) || ts.isStringLiteral(n) || ts.isNumericLiteral(n)) ? n.text : null;
function walk(ts,n,fn) { fn(n); ts.forEachChild(n,c => walk(ts,c,fn)); }
export function syntaxCounts(ts,sf) {
  const counts = {destructure_parameter:0,destructure_declaration:0,rest_parameter:0,
    rest_binding:0,spread_object:0,spread_argument_or_array:0,object_assign:0,default_parameter:0,
    callables:0,parameters:0,diagnostics:sf.parseDiagnostics.length};
  walk(ts,sf,n => {
    if (callable(ts,n)) { counts.callables++; counts.parameters += n.parameters.length; }
    if (ts.isParameter(n)) {
      if (ts.isObjectBindingPattern(n.name) || ts.isArrayBindingPattern(n.name)) counts.destructure_parameter++;
      if (n.dotDotDotToken) counts.rest_parameter++;
      if (n.initializer) counts.default_parameter++;
    }
    if (ts.isVariableDeclaration(n) && (ts.isObjectBindingPattern(n.name) || ts.isArrayBindingPattern(n.name))) counts.destructure_declaration++;
    if (ts.isBindingElement(n) && n.dotDotDotToken) counts.rest_binding++;
    if (ts.isSpreadAssignment(n)) counts.spread_object++;
    if (ts.isSpreadElement(n)) counts.spread_argument_or_array++;
    if (ts.isCallExpression(n) && n.expression.getText(sf) === 'Object.assign') counts.object_assign++;
  });
  return counts;
}
function flags(ts,n,sf,skipNested=false) {
  const out=[];
  function visit(v) {
    if (skipNested && v!==n && callable(ts,v)) return;
    let category;
    if (ts.isParameter(v)) {
      if (ts.isObjectBindingPattern(v.name)||ts.isArrayBindingPattern(v.name)) category='B-destructure';
      else if (v.dotDotDotToken) category='B-rest-spread';
      else if (v.initializer) category='B-default';
    }
    if (ts.isVariableDeclaration(v) && (ts.isObjectBindingPattern(v.name)||ts.isArrayBindingPattern(v.name))) category='B-destructure';
    if (ts.isSpreadAssignment(v)||ts.isSpreadElement(v)||(ts.isCallExpression(v)&&v.expression.getText(sf)==='Object.assign')) category='B-rest-spread';
    if (ts.isElementAccessExpression(v)) category='C-dynamic-key';
    if (ts.isPropertyAccessExpression(v)) category='C-member';
    if (ts.isForInStatement(v)) category='C-merge';
    else if (ts.isForStatement(v)||ts.isForOfStatement(v)||ts.isWhileStatement(v)||ts.isDoStatement(v)) category='A-loop';
    if (ts.isAwaitExpression(v)) category='H-callback/promise/event';
    if (category) out.push({category,line:sf.getLineAndCharacterOfPosition(v.getStart(sf)).line+1,text:v.getText(sf).slice(0,220)});
    ts.forEachChild(v,visit);
  }
  visit(n);return out;
}
function names(ts,n) {
  const out=[];
  walk(ts,n,v => {
    if (!ts.isIdentifier(v)) return;
    if (ts.isPropertyAccessExpression(v.parent)&&v.parent.name===v) return;
    if ((ts.isPropertyAssignment(v.parent)||ts.isMethodDeclaration(v.parent))&&v.parent.name===v) return;
    out.push(v.text);
  });return [...new Set(out)];
}
function bindingNames(ts,n) {
  if (ts.isIdentifier(n)) return [n.text];
  if (ts.isObjectBindingPattern(n)||ts.isArrayBindingPattern(n)) return n.elements.flatMap(e => ts.isBindingElement(e) ? bindingNames(ts,e.name) : []);
  return [];
}
function readTree(root) {
  const out=[];
  function visit(dir) {
    for (const e of fs.readdirSync(dir,{withFileTypes:true}).sort((a,b)=>a.name<b.name?-1:a.name>b.name?1:0)) {
      const f=path.join(dir,e.name);
      if (e.isSymbolicLink()) throw Error(`symlink in input ${f}`);
      if (e.isDirectory()) visit(f);
      else if (e.isFile()) out.push(f);
      else throw Error(`nonregular input ${f}`);
    }
  }
  visit(root);return out;
}
export function inspectEntry(ts,entry,inputs,packages) {
  const root=path.join(packages,entry.class,entry.entry,'src/package');
  const exploitRoot=path.join(inputs,entry.class,entry.entry);
  const cache=new Map(), identities=[],counts={},diagnosticCounts={},census={files:0,bytes:0,declarations:0,diagnostic_files:0,files_destructure:0,files_rest_spread:0,unparsed_files:[]};
  function parse(file) {
    const bytes=fs.readFileSync(file),text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes);
    const sf=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true,file.endsWith('x')?ts.ScriptKind.TSX:file.endsWith('ts')?ts.ScriptKind.TS:ts.ScriptKind.JS);
    return sf;
  }
  const files=readTree(root);
  for(const f of files) {
    const b=fs.readFileSync(f);identities.push({path:path.relative(root,f),bytes:b.length,sha256:sha(b)});
    if(!suffix.test(f))continue;
    let sf;
    try {sf=parse(f);} catch(e) {
      if(e.code!=='ERR_ENCODING_INVALID_ENCODED_DATA')throw e;
      census.unparsed_files.push({path:path.relative(root,f),reason:'invalid_utf8',bytes:b.length});continue;
    }
    const c=syntaxCounts(ts,sf);census.files++;census.bytes+=b.length;
    census.declarations+=Number(f.endsWith('.d.ts'));
    census.diagnostic_files+=Number(c.diagnostics>0);
    census.files_destructure+=Number(c.destructure_parameter+c.destructure_declaration>0);
    census.files_rest_spread+=Number(c.rest_parameter+c.rest_binding+c.spread_object+c.spread_argument_or_array+c.object_assign>0);
    for(const [k,v] of Object.entries(c)) {counts[k]=(counts[k]||0)+v;if(c.diagnostics)diagnosticCounts[k]=(diagnosticCounts[k]||0)+v;}
  }
  census.counts=counts;census.diagnostic_counts=diagnosticCounts;
  census.package_destructure=census.files_destructure>0;census.package_rest_spread=census.files_rest_spread>0;
  function inside(f) {return f===root||f.startsWith(root+path.sep);}
  function resolveFile(base) {
    if(!inside(path.resolve(base)))return null;
    for(const f of [base,base+'.js',base+'.cjs',base+'.mjs',base+'.ts']) if(fs.existsSync(f)&&fs.statSync(f).isFile())return f;
    if(fs.existsSync(path.join(base,'package.json'))) {
      const pkg=JSON.parse(fs.readFileSync(path.join(base,'package.json'),'utf8'));
      if(typeof pkg.main==='string') {const f=resolveFile(path.join(base,pkg.main));if(f)return f;}
    }
    for(const ext of ['js','cjs','mjs','ts']){const f=path.join(base,'index.'+ext);if(fs.existsSync(f))return f;}
    return null;
  }
  function info(file) {
    if(cache.has(file))return cache.get(file);
    const sf=parse(file),bindings=new Map(),assignments=[],functions=[];
    const inf={sf,bindings,assignments,functions,lexical:lexicalBindings(ts,sf)};cache.set(file,inf);
    function add(name,node) {if(!bindings.has(name))bindings.set(name,[]);bindings.get(name).push(node);}
    walk(ts,sf,n => {
      if(ts.isVariableDeclaration(n)&&ts.isIdentifier(n.name)&&n.initializer)add(n.name.text,n.initializer);
      if((ts.isFunctionDeclaration(n)||ts.isClassDeclaration(n))&&n.name)add(n.name.text,n);
      if(ts.isBinaryExpression(n)&&n.operatorToken.kind===ts.SyntaxKind.EqualsToken)assignments.push(n);
      if(callable(ts,n))functions.push(n);
    });return inf;
  }
  function uniqueBinding(file,name,at) {
    const d=info(file).lexical.stable(name,at);
    return d&&!ts.isParameter(d)?d.initializer||d:null;
  }
  function objectGet(n,key) {
    if(!ts.isObjectLiteralExpression(n))return null;
    const m=n.properties.filter(p=>prop(ts,p.name)===key);
    if(m.length!==1)return null;
    const p=m[0];return ts.isPropertyAssignment(p)?p.initializer:ts.isShorthandPropertyAssignment(p)?p.name:ts.isMethodDeclaration(p)?p:null;
  }
  function deref(file,node,parts=[],seen=new Set()) {
    if(!node)return null;node=unwrap(ts,node);
    const key=file+':'+node.pos+':'+parts.join('.');if(seen.has(key)||seen.size>40)return null;seen=new Set(seen).add(key);
    if(ts.isIdentifier(node)) {
      const target=uniqueBinding(file,node.text,node);return target?deref(file,target,parts,seen):null;
    }
    if(ts.isPropertyAccessExpression(node))return deref(file,node.expression,[node.name.text,...parts],seen);
    if(ts.isCallExpression(node)&&node.expression.getText(info(file).sf)==='require'&&node.arguments.length===1&&ts.isStringLiteral(node.arguments[0])) {
      const spec=node.arguments[0].text;if(!spec.startsWith('.'))return null;
      const f=resolveFile(path.resolve(path.dirname(file),spec));return f?exported(f,parts,seen):null;
    }
    if(parts.length&&ts.isObjectLiteralExpression(node))return deref(file,objectGet(node,parts[0]),parts.slice(1),seen);
    if(parts.length&&(ts.isClassDeclaration(node)||ts.isClassExpression(node))) {
      const method=node.members.filter(m=>prop(ts,m.name)===parts[0]&&callable(ts,m));
      return method.length===1?deref(file,method[0],parts.slice(1),seen):null;
    }
    if(parts.length&&callable(ts,node)&&node.name) {
      const target=info(file).assignments.filter(a=>a.left.getText(info(file).sf)===`${node.name.text}.prototype.${parts.join('.')}`);
      if(target.length===1)return deref(file,target[0].right,[],seen);
    }
    if(!parts.length&&callable(ts,node))return {file,node};
    if(!parts.length&&(ts.isClassDeclaration(node)||ts.isClassExpression(node))) {
      const constructors=node.members.filter(ts.isConstructorDeclaration);return constructors.length===1?{file,node:constructors[0]}:null;
    }
    return null;
  }
  function exported(file,parts,seen=new Set()) {
    const inf=info(file);if(inf.sf.parseDiagnostics.length)return null;
    const suffix=parts.join('.');
    const targets=inf.assignments.filter(a=>a.left.getText(inf.sf)===(suffix?'module.exports.'+suffix:'module.exports')||suffix&&a.left.getText(inf.sf)==='exports.'+suffix);
    if(targets.length===1)return deref(file,targets[0].right,[],seen);
    if(targets.length>1)return null;
    const whole=inf.assignments.filter(a=>a.left.getText(inf.sf)==='module.exports');
    if(whole.length===1)return deref(file,whole[0].right,parts,seen);
    const es=inf.sf.statements.filter(n=>ts.isExportAssignment(n));
    if(!parts.length&&es.length===1)return deref(file,es[0].expression,[],seen);
    const named=inf.sf.statements.filter(n=>n.modifiers?.some(m=>m.kind===ts.SyntaxKind.ExportKeyword)&&n.name?.text===parts[0]);
    if(named.length===1)return deref(file,named[0],parts.slice(1),seen);
    return null;
  }
  function record(file,n) {
    const sf=info(file).sf,coord=x=>sf.getLineAndCharacterOfPosition(x);
    const start=n.getStart(sf),a=coord(start),end=coord(n.end);
    const name=n.name?.getText(sf)|| (ts.isVariableDeclaration(n.parent)?n.parent.name.getText(sf):ts.isBinaryExpression(n.parent)?n.parent.left.getText(sf):ts.isPropertyAssignment(n.parent)?n.parent.name.getText(sf):'<anonymous>');
    return {file:path.relative(root,file),name,start_line:a.line+1,end_line:end.line+1,start_byte:Buffer.byteLength(sf.text.slice(0,start)),end_byte:Buffer.byteLength(sf.text.slice(0,n.end)),
      parameters:n.parameters.map((p,ordinal)=>({ordinal,text:p.getText(sf),names:bindingNames(ts,p.name),line:coord(p.getStart(sf)).line+1,start_byte:Buffer.byteLength(sf.text.slice(0,p.name.getStart(sf))),end_byte:Buffer.byteLength(sf.text.slice(0,p.name.end)),destructure:ts.isObjectBindingPattern(p.name)||ts.isArrayBindingPattern(p.name),rest:Boolean(p.dotDotDotToken),default:Boolean(p.initializer),
        ...(()=>{let bare_references=0,member_references=0;walk(ts,n.body,v=>{if(ts.isIdentifier(v)&&bindingNames(ts,p.name).includes(v.text)&&info(file).lexical.declaration(v.text,v)?.pos===p.pos){if((ts.isPropertyAccessExpression(v.parent)||ts.isElementAccessExpression(v.parent))&&v.parent.expression===v)member_references++;else if(!(ts.isPropertyAccessExpression(v.parent)&&v.parent.name===v))bare_references++;}});return {bare_references,member_references};})()})),
      arguments_used:/\barguments\b/.test(n.body.getText(sf)),
      callback_expression_statement:(()=>{if(!ts.isCallExpression(n.parent)&&!ts.isNewExpression(n.parent)||!n.parent.arguments?.includes(n))return false;for(let v=n;v.parent;v=v.parent){if(ts.isVariableDeclaration(v.parent))return false;if(ts.isExpressionStatement(v.parent))return ts.isSourceFile(v.parent.parent);}return false;})(),
      features:flags(ts,n,sf,true)};
  }
  const tests=readTree(exploitRoot).filter(f=>f.endsWith('.test.js'));
  const exploits=tests.map(f=>({path:path.relative(inputs,f),sha256:sha(fs.readFileSync(f))}));
  const packageMetadata=fs.readFileSync(path.join(exploitRoot,'package.json'));
  const result={class:entry.class,entry:entry.entry,id:entry.id,census,identities,exploits,metadata_sha256:sha(packageMetadata),gt_status:'gt_unavailable',gt_reason:null};
  try {
  let sinkFile,sinkLine,sinkCol;
  const match=/^(.+):(\d+):(\d+)$/.exec(entry.sink||'');
  if(!match||+match[2]<1||+match[3]<1){result.gt_reason=entry.sink?'invalid_sink_coordinate':'missing_sink_coordinate';return result;}
  [,sinkFile,sinkLine,sinkCol]=match;sinkLine=+sinkLine;sinkCol=+sinkCol;
  if(path.isAbsolute(sinkFile)||sinkFile.split('/').includes('..')){result.gt_reason='unsafe_sink_path';return result;}
  let sinkPath=path.join(root,sinkFile),repair=false;
  if(fs.existsSync(sinkPath)&&!fs.statSync(sinkPath).isFile()){result.gt_reason='sink_not_regular_file';return result;}
  if(!fs.existsSync(sinkPath)) {
    const matches=files.filter(f=>path.basename(f)===path.basename(sinkFile));
    if(matches.length!==1){result.gt_reason=matches.length?'ambiguous_sink_basename':'missing_sink_file';return result;}
    sinkPath=matches[0];repair=true;
  }
  const inf=info(sinkPath),sf=inf.sf,lines=sf.text.split(/\r?\n/);
  if(sinkLine>lines.length||sinkCol>lines[sinkLine-1].length+1){result.gt_reason='sink_out_of_range';return result;}
  if(sf.parseDiagnostics.length){result.gt_reason='sink_parse_diagnostics';return result;}
  const offset=sf.getPositionOfLineAndCharacter(sinkLine-1,sinkCol-1);
  const enclosing=inf.functions.filter(n=>n.getStart(sf)<=offset&&n.end>offset).sort((a,b)=>(a.end-a.pos)-(b.end-b.pos));
  if(!enclosing.length){result.gt_reason='sink_not_in_callable';return result;}
  const sinkNode=enclosing[0];
  const sites=[];walk(ts,sinkNode,n=>{if(n.getStart(sf)<=offset&&n.end>offset&&(ts.isCallExpression(n)||ts.isNewExpression(n)||ts.isBinaryExpression(n)))sites.push(n);});
  const site=sites.sort((a,b)=>(a.end-a.pos)-(b.end-b.pos))[0];
  if(!site){result.gt_reason='sink_coordinate_not_call_or_assignment';return result;}
  const sink=record(sinkPath,sinkNode);
  sink.line=sinkLine;sink.column=sinkCol;sink.original=entry.sink;sink.basename_repair=repair;
  sink.expression=site.getText(sf).slice(0,1000);
  const sinkArgs=[...(site.arguments||[])].filter(a=>!callable(ts,inf.lexical.value(a)||a));
  const valueArgs=entry.class==='path-traversal'?sinkArgs.slice(0,1):sinkArgs;
  const valueNodes=ts.isCallExpression(site)||ts.isNewExpression(site)?[...valueArgs]:[site];
  sink.value_names=ts.isCallExpression(site)||ts.isNewExpression(site)
    ? [...new Set(valueArgs.flatMap(a=>names(ts,a)))] : names(ts,site);
  if(entry.class==='redos'&&ts.isCallExpression(site)&&ts.isPropertyAccessExpression(site.expression)) {
    sink.value_names=[...new Set([...sink.value_names,...names(ts,site.expression.expression)])];
    valueNodes.push(site.expression.expression);
  }
  sink.value_occurrences=[];
  for(const value of valueNodes)walk(ts,value,n=>{
    if(!(ts.isIdentifier(n)||n.kind===ts.SyntaxKind.ThisKeyword)||ts.isPropertyAccessExpression(n.parent)&&n.parent.name===n
      ||ts.isPropertyAssignment(n.parent)&&n.parent.name===n)return;
    let expression=n;
    while(expression.parent&&(ts.isPropertyAccessExpression(expression.parent)||ts.isElementAccessExpression(expression.parent))&&expression.parent.expression===expression)expression=expression.parent;
    // A method receiver influences the call result; the method name itself is
    // not the data member being consumed. Keep data members such as fn.name.
    if(expression!==n&&ts.isCallExpression(expression.parent)&&expression.parent.expression===expression)expression=expression.expression;
    const expectedPath=expression.getText(sf).replace(/\[[^\]]*\]/g,'[]');
    const occurrenceLine=sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1;
    let lineOccurrences=0;
    walk(ts,sf,v=>{if((ts.isIdentifier(v)||v.kind===ts.SyntaxKind.ThisKeyword)&&v.getText(sf)===n.getText(sf)&&sf.getLineAndCharacterOfPosition(v.getStart(sf)).line+1===occurrenceLine)lineOccurrences++;});
    sink.value_occurrences.push({name:n.getText(sf),path:expectedPath,line:occurrenceLine,start_byte:Buffer.byteLength(sf.text.slice(0,n.getStart(sf))),end_byte:Buffer.byteLength(sf.text.slice(0,n.kind===ts.SyntaxKind.ThisKeyword?expression.end:n.end)),line_occurrences:lineOccurrences});
  });
  sink.value_selection=entry.class==='path-traversal'?'first_path_argument':entry.class==='redos'?'arguments_and_regex_or_string_receiver':'non_callback_arguments_or_assignment';
  sink.kind=ts.isBinaryExpression(site)?'assignment':'call';
  sink.nested_callback=enclosing.some(n=>n!==sinkNode)&&!ts.isFunctionDeclaration(sinkNode);
  sink.capture_names=[...new Set(sink.value_occurrences.filter(o=>{const d=inf.lexical.declaration(o.name,site);return d&&ts.isParameter(d)&&d.parent!==sinkNode;}).map(o=>o.name))];
  result.sink=sink;
  const dep=Object.keys(entry.deps)[0],candidates=[],bootstrapFiles=new Set();let httpExploit=false;
  for(const test of tests) {
    const sf=parse(test);if(sf.parseDiagnostics.length){result.gt_reason='exploit_parse_diagnostics';return result;}
    const aliases=new Map(),lexical=lexicalBindings(ts,sf);let checker;
    function payloadEvidence(n,seen=new Set()) {
      if(callable(ts,lexical.value(n)||n))return [];
      const text=n.getText(sf),evidence=[];
      if(/__proto__|polluted|writeFileSync|spawnSync|touch\s|(?:\.\.\/)|\b(?:payload|attack_str(?:ing)?|userInput|genstr)\b/.test(text)
        ||entry.class==='redos'&&/\.repeat\s*\(|Array\s*\(/.test(text))evidence.push(text.slice(0,220));
      walk(ts,n,v=>{if(ts.isIdentifier(v)&&!seen.has(v.text)) {
        const d=lexical.stable(v.text,v),defs=d?.initializer?[d.initializer]:[];
        if(defs.length===1&&seen.size<20)evidence.push(...payloadEvidence(defs[0],new Set(seen).add(v.text)));
      }});
      return [...new Set(evidence)].slice(0,8);
    }
    function requestOrdinals(call) {
      for(let n=call.parent;n;n=n.parent)if(callable(ts,n)&&n.parameters?.length) {
        const parent=n.parent;
        if(ts.isCallExpression(parent)&&parent.expression.getText(sf).endsWith('.createServer')&&parent.arguments.includes(n)) {
          const name=n.parameters[0].name.getText(sf);
          return [...(call.arguments||[])].flatMap((a,i)=>a.getText(sf)===name?[i]:[]);
        }
      }
      return [];
    }
    function origin(n,seen=new Set()) {
      if(!n)return null;n=unwrap(ts,n);
      if(ts.isIdentifier(n)) {
        if(seen.has(n.text))return null;const d=lexical.stable(n.text,n);if(!d||ts.isParameter(d))return null;
        if(ts.isBindingElement(d)) {const init=d.parent.parent.initializer,o=origin(init,new Set(seen).add(n.text)),key=prop(ts,d.propertyName||d.name);return o&&key?{...o,parts:[...o.parts,key]}:null;}
        return d.initializer?origin(d.initializer,new Set(seen).add(n.text)):null;
      }
      if(ts.isPropertyAccessExpression(n)){const a=origin(n.expression,seen);return a?{...a,parts:[...a.parts,n.name.text]}:null;}
      if(ts.isNewExpression(n)){const a=origin(n.expression,seen);return a?{...a,instance:true}:null;}
      if(ts.isCallExpression(n)&&n.expression.getText(sf)==='require'&&n.arguments.length===1&&ts.isStringLiteral(n.arguments[0])) {
        const spec=n.arguments[0].text;return spec===dep||spec.startsWith(dep+'/')?{spec,parts:[],require_line:sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1}:null;
      }
      if(ts.isCallExpression(n)) {
        const o=origin(n.expression,seen);return o?{...o,returned_api:true}:null;
      }
      return null;
    }
    walk(ts,sf,n=>{if(ts.isVariableDeclaration(n)&&n.initializer) {
      if(ts.isIdentifier(n.name)){const k=n.name.text;if(!aliases.has(k))aliases.set(k,[]);aliases.get(k).push(n.initializer);}
      else if(ts.isObjectBindingPattern(n.name))for(const e of n.name.elements)if(ts.isIdentifier(e.name)){
        const o=origin(n.initializer);if(o){const key=prop(ts,e.propertyName||e.name);if(key)aliases.set(e.name.text,[ts.factory.createPropertyAccessExpression(n.initializer,key)]);}
      }
    }});
    walk(ts,sf,n=>{
      if(ts.isStringLiteral(n)||ts.isNoSubstitutionTemplateLiteral(n)||ts.isTemplateExpression(n)) {
        const text=n.getText(sf),escaped=dep.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
        for(const m of text.matchAll(new RegExp('node_modules/'+escaped+'/([^\\s"\'`;&]+)','g'))) {
          const file=resolveFile(path.resolve(root,m[1]));if(file)bootstrapFiles.add(file);
        }
        for(const m of text.matchAll(new RegExp('node_modules/'+escaped+'(?:[\\s"\'`;&]|$)','g'))) {
          const file=resolveFile(root);if(file)bootstrapFiles.add(file);
        }
      }
      if((ts.isStringLiteral(n)||ts.isNoSubstitutionTemplateLiteral(n)||ts.isTemplateExpression(n))&&/curl[^\n]*http(?:s)?:\/\//.test(n.getText(sf)))httpExploit=true;
      if(!ts.isCallExpression(n)&&!ts.isNewExpression(n))return;
      const o=origin(n.expression);if(!o)return;
      const args=[...(n.arguments||[])].map((a,ordinal)=>({ordinal,text:a.getText(sf).slice(0,1000),callback:callable(ts,lexical.value(a)||a),properties:ts.isObjectLiteralExpression(a)?a.properties.map(p=>prop(ts,p.name)).filter(Boolean):[],payload_evidence:payloadEvidence(a)}));
      const sub=o.spec.slice(dep.length).replace(/^\//,'');
      const file=resolveFile(sub?path.join(root,sub):root);
      let target=file&&!o.returned_api?exported(file,o.parts):null,resolution_mode='syntactic_export';
      if(!target&&file) {
        checker ||= checkerResolver(ts,test,dep,root,resolveFile);
        target=checker.resolve(n.getStart(sf),n.end,ts.isNewExpression(n));
        if(target)resolution_mode='typescript_checker';
      }
      if(target?.shift) {
        const actual=target.mode==='apply'?[...n.arguments[1].elements]:[...n.arguments].slice(1);
        args.splice(0,args.length,...actual.map((a,ordinal)=>({ordinal,text:a.getText(sf),callback:callable(ts,lexical.value(a)||a),properties:[],payload_evidence:payloadEvidence(a)})));
      }
      // Callable bodies are never payload evidence. A separately mutated
      // function metadata field is data only when the API consumes that field.
      if(target)for(const arg of args.filter(a=>a.callback)) {
        const actual=n.arguments[arg.ordinal],value=lexical.value(actual);
        const formal=target.node.parameters[arg.ordinal];
        if(!formal||!ts.isIdentifier(formal.name))continue;
        const fields=new Set();walk(ts,target.node.body,v=>{if(ts.isPropertyAccessExpression(v)&&ts.isIdentifier(v.expression)&&v.expression.text===formal.name.text)fields.add(v.name.text);});
        walk(ts,sf,v=>{
          if(ts.isCallExpression(v)&&v.expression.getText(sf)==='Object.defineProperty'&&lexical.value(v.arguments[0])===value&&ts.isStringLiteral(v.arguments[1])&&fields.has(v.arguments[1].text)) {
            const evidence=payloadEvidence(v.arguments[2]);
            if(evidence.length){arg.callback=false;arg.data_role='function_metadata_value';arg.payload_evidence=evidence;}
          }
        });
      }
      if(!args.some(a=>!a.callback))return;
      candidates.push({test:path.relative(inputs,test),line:sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1,call:n.expression.getText(sf),...o,args,request_ordinals:requestOrdinals(n),resolution_mode,target});
    });
  }
  result.api_candidates=candidates.map(({target,...c})=>({...c,resolved:target?record(target.file,target.node):null}));
  let httpTarget=null;
  if(httpExploit&&!candidates.some(c=>c.request_ordinals.length)
    &&(entry.class==='path-traversal'||!candidates.some(c=>c.args.some(a=>!a.callback&&a.payload_evidence.length)))) {
    if(!bootstrapFiles.size&&!candidates.length){result.gt_reason='http_bootstrap_not_bound';return result;}
    const handlers=[];
    // Only a launched file, its local imports, or the API/sink file is admitted.
    const httpFiles=new Set(bootstrapFiles);
    for(const c of candidates){const sub=c.spec.slice(dep.length).replace(/^\//,'');const file=resolveFile(sub?path.join(root,sub):root);if(file)httpFiles.add(file);}
    for(const file of httpFiles) {
      walk(ts,info(file).sf,n=>{if(ts.isCallExpression(n)&&n.expression.getText()==='require'&&ts.isStringLiteral(n.arguments[0])&&n.arguments[0].text.startsWith('.')){const f=resolveFile(path.resolve(path.dirname(file),n.arguments[0].text));if(f)httpFiles.add(f);}});
    }
    for(const file of httpFiles) {
      const current=info(file);if(current.sf.parseDiagnostics.length)continue;
      walk(ts,current.sf,n=>{
        if(!ts.isCallExpression(n)||!ts.isPropertyAccessExpression(n.expression))return;
        const method=n.expression.name.text;
        function moduleOrigin(v,seen=new Set()) {
          if(!v||seen.has(v))return null;seen=new Set(seen).add(v);
          if(ts.isIdentifier(v)){const d=current.lexical.stable(v.text,v);return d?.initializer?moduleOrigin(d.initializer,seen):null;}
          if(ts.isCallExpression(v)&&v.expression.getText()==='require'&&ts.isStringLiteral(v.arguments[0]))return v.arguments[0].text;
          if(ts.isCallExpression(v))return moduleOrigin(ts.isPropertyAccessExpression(v.expression)?v.expression.expression:v.expression,seen);
          return null;
        }
        const module=moduleOrigin(n.expression.expression);
        const registration=method==='createServer'||method==='on'&&n.arguments[0]?.text==='request'
          ||['use','get','all'].includes(method);
        if(!registration||!['http','https','express','connect'].includes(module))return;
        for(const a of n.arguments) {
          const resolved=callable(ts,a)?{file,node:a}:deref(file,a);
          if(!resolved||!resolved.node.parameters.length||!ts.isIdentifier(resolved.node.parameters[0].name))continue;
          const req=resolved.node.parameters[0].name.text;
          if(!/^(?:req|request)$/.test(req))continue;
          const queue=[resolved],seen=new Set();let reaches=false;
          while(queue.length&&seen.size<200) {
            const t=queue.shift(),key=t.file+':'+t.node.pos;if(seen.has(key))continue;seen.add(key);
            if(t.file===sinkPath&&t.node.getStart(info(t.file).sf)<=offset&&t.node.end>offset){reaches=true;break;}
            walk(ts,t.node,v=>{if(ts.isCallExpression(v)){const d=deref(t.file,v.expression);if(d)queue.push(d);}});
          }
          if(reaches)handlers.push({...resolved,registration:{file:path.relative(root,file),line:current.sf.getLineAndCharacterOfPosition(n.getStart(current.sf)).line+1,expression:n.expression.getText(current.sf)}});
        }
      });
    }
    const unique=[...new Map(handlers.map(h=>[h.file+':'+h.node.pos,h])).values()];
    if(unique.length!==1){result.gt_reason=unique.length?'ambiguous_http_request_handler':'no_unique_http_request_handler';return result;}
    httpTarget=unique[0];
    candidates.splice(0,candidates.length,{target:httpTarget,spec:dep,parts:[],args:[{ordinal:0,callback:false,payload_evidence:['exploit HTTP URL/path']}],request_ordinals:[0],resolution_mode:'http_registration',registration:httpTarget.registration});
  }
  if(!candidates.length){result.gt_reason='no_direct_package_api_call_with_data';return result;}
  // Setup constructors must not mask the payload method; retain unresolved
  // method calls rather than quietly falling back to a resolved constructor.
  const selected=candidates.filter(c=>c.parts.length||c.args.some(a=>!a.callback&&a.payload_evidence.length)||!candidates.some(d=>d!==c&&d.spec===c.spec&&(d.instance||d.parts.length)));
  const relevant=(selected.length?selected:candidates).filter(c=>!candidates.some(d=>d!==c&&d.spec===c.spec&&d.returned_api&&d.args.some(a=>!a.callback&&a.payload_evidence.length))||c.returned_api);
  if(relevant.some(c=>!c.target)){result.gt_reason=relevant.some(c=>c.returned_api)?'gt_parser_unsupported_returned_api':'unresolved_export_or_api_chain';return result;}
  const targets=new Set(relevant.map(c=>c.target.file+':'+c.target.node.pos));
  if(targets.size!==1){result.gt_reason='multiple_exported_entry_functions';return result;}
  const target=relevant[0].target,source=record(target.file,target.node);
  if(httpTarget){source.registration=httpTarget.registration;
    const scanner=ts.createScanner(ts.ScriptTarget.Latest,true,ts.LanguageVariant.Standard,httpTarget.node.getText(info(httpTarget.file).sf));let normalized=[];
    for(let k=scanner.scan();k!==ts.SyntaxKind.EndOfFileToken;k=scanner.scan())normalized.push(k===ts.SyntaxKind.Identifier?'ID':k===ts.SyntaxKind.StringLiteral?'STR':k===ts.SyntaxKind.NumericLiteral?'NUM':scanner.getTokenText());
    source.normalized_handler_sha256=sha(normalized.join(' '));}
  source.calls=relevant.map(({target,...c})=>c);
  const payloadArgs=relevant.flatMap(c=>c.args.filter(a=>!a.callback&&a.payload_evidence.length).map(a=>a.ordinal));
  const requestArgs=relevant.flatMap(c=>c.request_ordinals);
  source.binding_mode=requestArgs.length?'http_request_parameter':payloadArgs.length?'payload_evidence':'single_supplied_data_parameter';
  source.supplied_ordinals=[...new Set(requestArgs.length?requestArgs:payloadArgs.length?payloadArgs:relevant.flatMap(c=>c.args.filter(a=>!a.callback).map(a=>a.ordinal)))].sort((a,b)=>a-b);
  source.data_parameters=source.parameters.filter(p=>source.supplied_ordinals.includes(p.ordinal)||p.rest&&source.supplied_ordinals.some(i=>i>=p.ordinal));
  if(!source.data_parameters.length){result.gt_reason='no_bindable_data_parameter';result.source=source;return result;}
  if(!requestArgs.length&&!payloadArgs.length&&source.data_parameters.length!==1){result.gt_reason='ambiguous_payload_parameter';result.source=source;return result;}
  source.resolution_mode=relevant[0].resolution_mode;
  const callbackNames=new Set(source.parameters.filter(p=>relevant.every(c=>c.args.find(a=>a.ordinal===p.ordinal)?.callback)).flatMap(p=>p.names));
  sink.value_occurrences=sink.value_occurrences.filter(o=>!callbackNames.has(o.name));
  result.source=source;
  if(!sink.value_occurrences.length){result.gt_reason='sink_has_no_data_value';return result;}
  result.sink=sink;result.gt_status='available';result.gt_reason=null;
  // Independent, syntactic call-chain candidate. It proves syntax occurrence,
  // not value dependence. Unknown paths remain unknown in the report.
  const key=t=>t.file+':'+t.node.pos,queue=[[target]],seen=new Set();let chain=null;
  while(queue.length&&seen.size<2000) {
    const q=queue.shift(),cur=q.at(-1),k=key(cur);if(seen.has(k))continue;seen.add(k);
    if(cur.file===sinkPath&&cur.node.pos===sinkNode.pos&&cur.node.end===sinkNode.end){chain=q;break;}
    const current=info(cur.file);
    function visit(n) {
      if(n!==cur.node&&callable(ts,n)){if(n===sinkNode&&cur.file===sinkPath)queue.push([...q,{file:cur.file,node:n}]);return;}
      if(ts.isCallExpression(n)||ts.isNewExpression(n)) {
        const callee=deref(cur.file,n.expression);
        if(callee&&!seen.has(key(callee)))queue.push([...q,callee]);
        for(const a of n.arguments||[])if(callable(ts,a))queue.push([...q,{file:cur.file,node:a}]);
      }
      ts.forEachChild(n,visit);
    }
    visit(cur.node);
  }
  result.syntactic_path=chain?chain.map(t=>record(t.file,t.node)):null;
  result.path_kind=chain?'candidate_call_chain_not_value_proof':'unknown';
  return result;
  } catch(e) {
    if(e.code!=='ERR_ENCODING_INVALID_ENCODED_DATA')throw e;
    result.gt_status='gt_unavailable';result.gt_reason='invalid_utf8_ground_truth';return result;
  }
}
export function loadCompiler(file) {
  if(sha(fs.readFileSync(file))!==COMPILER_SHA)throw Error('TypeScript compiler pin mismatch');
  const ts=createRequire(import.meta.url)(file);if(ts.version!=='5.9.3')throw Error('compiler version mismatch');return ts;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [compiler,manifest,inputs,packages,out]=process.argv.slice(2);
  const ts=loadCompiler(compiler),entries=JSON.parse(fs.readFileSync(manifest,'utf8')).entries;
  const fd=fs.openSync(out,'w');
  try {for(const e of entries.filter(e=>e.status==='ok').sort((a,b)=>(a.class+'/'+a.entry).localeCompare(b.class+'/'+b.entry,'en'))) {
    const row=inspectEntry(ts,e,inputs,packages);fs.writeSync(fd,JSON.stringify(row)+'\n');process.stderr.write(`${e.class}/${e.entry}\n`);
  }}finally{fs.closeSync(fd);}
}
