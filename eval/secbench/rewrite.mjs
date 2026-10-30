// AST-guided counterfactuals; only scratch packages may be passed here.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {loadCompiler} from './inspect.mjs';
import {lexicalBindings} from './bindings.mjs';
const walk=(ts,n,fn)=>{fn(n);ts.forEachChild(n,c=>walk(ts,c,fn));};
const propertyName=(ts,n)=>ts.isPropertyAccessExpression(n.parent)&&n.parent.name===n
  ||(ts.isPropertyAssignment(n.parent)||ts.isMethodDeclaration(n.parent))&&n.parent.name===n;
function refuseReflection(text) {
  if(/\beval\s*\(|\bsuper\b|\.(?:caller|callee)\b|\.toString\s*\(|\b(?:__filename|__dirname)\b/.test(text))
    throw Error('reflective scope cannot be safely rewritten');
}
export function admissionPlan(ts,root) {
  const mapping=name=>name.split('/').map(p=>['dist','build'].includes(p)?'_secbench_'+p:p).join('/');
  const rewrites={};
  function visit(dir) {for(const e of fs.readdirSync(dir,{withFileTypes:true})) {
    const file=path.join(dir,e.name);if(e.isDirectory()){visit(file);continue;}
    if(!e.isFile())throw Error('admission semantics: nonregular source');
    const name=path.relative(root,file);if(!/\.(?:[cm]?js|ts|json)$/.test(name))continue;
    const text=fs.readFileSync(file,'utf8');
    if(name.endsWith('.json')) {
      if(name==='package.json') {const pkg=JSON.parse(text);if(pkg.main&&mapping(pkg.main)!==pkg.main){pkg.main=mapping(pkg.main);rewrites[name]=JSON.stringify(pkg,null,2)+'\n';}}
      continue;
    }
    if(/\b(?:__dirname|__filename)\b|\brequire\s*\.\s*(?:resolve|cache|main)\b|\bimport\s*\.\s*meta\b/.test(text))throw Error('admission semantics: runtime path introspection');
    const sf=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true),edits=[];
    if(sf.parseDiagnostics.length)throw Error('admission semantics: unsupported syntax');
    walk(ts,sf,n=>{
      if(!ts.isStringLiteral(n)&&!ts.isNoSubstitutionTemplateLiteral(n))return;
      if(!n.text.startsWith('.'))return;
      const p=n.parent,isImport=(ts.isCallExpression(p)&&p.arguments[0]===n&&p.arguments.length===1&&p.expression.getText(sf)==='require')
        ||(ts.isImportDeclaration(p)||ts.isExportDeclaration(p))&&p.moduleSpecifier===n;
      if(!isImport) {if(mapping(n.text)!==n.text)throw Error('admission semantics: non-import path string');return;}
      const absolute=path.resolve(path.dirname(file),n.text);
      if(!absolute.startsWith(root+path.sep))throw Error('admission semantics: external relative import');
      const relocated=path.join(root,mapping(path.relative(root,absolute)));
      let spec=path.relative(path.dirname(path.join(root,mapping(name))),relocated);if(!spec.startsWith('.'))spec='./'+spec;
      if(spec!==n.text)edits.push({start:n.getStart(sf),end:n.end,value:JSON.stringify(spec)});
    });
    let output=text;for(const e of edits.sort((a,b)=>b.start-a.start))output=output.slice(0,e.start)+e.value+output.slice(e.end);
    if(output!==text)rewrites[name]=output;
  }}
  visit(root);return rewrites;
}
export function rewrite(ts,root,row,pattern) {
  if(pattern==='callback_capture')return rewriteCapture(ts,root,row);
  if(pattern==='callback_registration') {
    if(!row.source.callback_expression_statement)throw Error('not a top-level expression registration');
    const result=rewriteCapture(ts,root,row,true);
    result.row.source.callback_expression_statement=false;
    result.pattern=pattern;
    return result;
  }
  const file=path.join(root,row.source.file),text=fs.readFileSync(file,'utf8');
  const sf=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
  refuseReflection(text);
  const byte=p=>Buffer.byteLength(text.slice(0,p));
  let source;
  walk(ts,sf,n=>{if(ts.isFunctionLike(n)&&n.body&&byte(n.getStart(sf))===row.source.start_byte&&byte(n.end)===row.source.end_byte)source=n;});
  if(!source)throw Error('source byte identity missing');
  const edits=[];let newSource=source,replacementName=null;
  const add=(start,end,value)=>edits.push({start,end,value});
  const fresh=stem=>{let v=stem;while(text.includes(v))v+='_';return v;};
  const block=source.body;if(!ts.isBlock(block))throw Error('expression body rewrite requires separate design');
  let insertion=block.getStart(sf)+1;
  for(const s of block.statements) {if(ts.isExpressionStatement(s)&&ts.isStringLiteral(s.expression))insertion=s.end;else break;}
  if(pattern==='member') {
    const parameters=row.source.data_parameters.filter(p=>!p.rest&&!p.destructure).flatMap(p=>p.names);
    if(!parameters.length)throw Error('no simple input for bare-reference rewrite');
    add(insertion,insertion,parameters.map(p=>`;void ${p};`).join(''));
  } else if(pattern==='rest') {
    const parameters=row.source.data_parameters.filter(p=>p.rest);
    if(parameters.length!==1||parameters[0].names.length!==1||source.asteriskToken||source.modifiers?.some(m=>m.kind===ts.SyntaxKind.AsyncKeyword))throw Error('rest wrapper requires one synchronous simple rest');
    const old=parameters[0].names[0],helper=fresh('__secbench_rest'),formal=fresh('__secbench_values');
    const lexical=lexicalBindings(ts,sf),parameter=source.parameters[parameters[0].ordinal];
    // Preserve the engine-created rest array, function length, lexical this,
    // arguments and return value; name only the previously missing array Def.
    add(insertion,insertion,`const ${helper}=(${formal})=>{`);
    add(block.end-1,block.end-1,`};return ${helper}(${old});`);
    walk(ts,block,n=>{
      if(ts.isIdentifier(n)&&n.text===old&&!propertyName(ts,n)) {
        // A shadowing declaration is conservatively refused, not renamed.
        if(lexical.declaration(old,n)!==parameter)throw Error('shadowed rest reference');
        if(ts.isShorthandPropertyAssignment(n.parent))add(n.getStart(sf),n.end,`${old}:${formal}`);
        else add(n.getStart(sf),n.end,formal);
      }
    });
    replacementName=formal;
  } else if(pattern==='arguments') {
    // Preserve the actual arguments object and all arity/alias semantics by
    // adding one named arrow checkpoint around the unchanged body.
    if(source.asteriskToken||source.modifiers?.some(m=>m.kind===ts.SyntaxKind.AsyncKeyword)||/\beval\s*\(|\bsuper\b/.test(block.getText(sf)))throw Error('arguments wrapper scope is unsupported');
    if(source.parameters.length!==1||row.source.data_parameters.length!==1||row.source.data_parameters[0].ordinal!==0)
      throw Error('arguments checkpoint would broaden source identity');
    const helper=fresh('__secbench_legacy'),formal=fresh('__secbench_arguments');
    add(insertion,insertion,`const ${helper}=(${formal})=>{`);
    add(block.end-1,block.end-1,`};return ${helper}(arguments);`);
    walk(ts,block,n=>{if(ts.isIdentifier(n)&&n.text==='arguments'&&!propertyName(ts,n)) {
      let owner=n.parent;while(owner&&owner!==source&&(!ts.isFunctionLike(owner)||ts.isArrowFunction(owner)))owner=owner.parent;
      if(owner===source) {
        if(ts.isShorthandPropertyAssignment(n.parent))add(n.getStart(sf),n.end,`arguments:${formal}`);
        else add(n.getStart(sf),n.end,formal);
      }
    }});
    replacementName=formal;
  } else throw Error(`unsupported rewrite ${pattern}`);
  function remap(pos) {let delta=0;for(const e of edits){if(e.end<=pos)delta+=e.value.length-(e.end-e.start);else if(e.start<pos)return e.start+delta;}return pos+delta;}
  let output=text;
  for(const e of [...edits].sort((a,b)=>b.start-a.start||b.end-a.end))output=output.slice(0,e.start)+e.value+output.slice(e.end);
  const outsf=ts.createSourceFile(file,output,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
  if(outsf.parseDiagnostics.length)throw Error('rewrite introduced syntax diagnostics');
  const oldPosition=b=>Buffer.from(text).subarray(0,b).toString('utf8').length;
  const newByte=p=>Buffer.byteLength(output.slice(0,p));
  const coord=p=>outsf.getLineAndCharacterOfPosition(p).line+1;
  const result=structuredClone(row);
  const oldStart=source.getStart(sf),mappedStart=remap(oldStart);
  if(replacementName)walk(ts,outsf,n=>{if(ts.isArrowFunction(n)&&n.parameters[0]?.name.getText(outsf)===replacementName)newSource=n;});
  else walk(ts,outsf,n=>{if(ts.isFunctionLike(n)&&n.body&&n.getStart(outsf)===mappedStart)newSource=n;});
  result.source.start_byte=newByte(newSource.getStart(outsf));result.source.end_byte=newByte(newSource.end);
  result.source.start_line=coord(newSource.getStart(outsf));result.source.end_line=coord(newSource.end);
  result.source.callback_expression_statement=['callback_registration','rest','arguments'].includes(pattern)?false:row.source.callback_expression_statement;
  if(pattern==='arguments')result.source.arguments_used=false;
  if(replacementName) {
    const p=newSource.parameters[0];
    result.source.name=pattern==='rest'?'__secbench_rest':'__secbench_legacy';
    result.source.binding_mode=`counterfactual_${pattern}_normalization`;
    result.source.data_parameters=[{ordinal:0,text:p.getText(outsf),names:[replacementName],line:coord(p.getStart(outsf)),start_byte:newByte(p.name.getStart(outsf)),end_byte:newByte(p.name.end),rest:false,destructure:false,default:false,bare_references:1,member_references:0}];
    let bare=0,member=0;
    walk(ts,newSource.body,n=>{if(ts.isIdentifier(n)&&n.text===replacementName){if((ts.isPropertyAccessExpression(n.parent)||ts.isElementAccessExpression(n.parent))&&n.parent.expression===n)member++;else if(!(ts.isPropertyAccessExpression(n.parent)&&n.parent.name===n))bare++;}});
    result.source.data_parameters[0].bare_references=bare;result.source.data_parameters[0].member_references=member;
  } else for(const p of result.source.data_parameters) {
    const start=remap(oldPosition(p.start_byte)),end=remap(oldPosition(p.end_byte));
    p.start_byte=newByte(start);p.end_byte=newByte(end);p.line=coord(start);
    if(pattern==='member')p.bare_references=(p.bare_references||0)+1;
  }
  if(result.sink.file===row.source.file) {
    const sinkStart=remap(oldPosition(row.sink.start_byte)),sinkEnd=remap(oldPosition(row.sink.end_byte));
    result.sink.start_byte=newByte(sinkStart);result.sink.end_byte=newByte(sinkEnd);
    result.sink.start_line=coord(sinkStart);result.sink.end_line=coord(sinkEnd);
    for(const o of result.sink.value_occurrences) {
      const oldStart=oldPosition(o.start_byte),start=remap(oldStart),end=remap(oldPosition(o.end_byte));
      o.start_byte=newByte(start);o.end_byte=newByte(end);o.line=coord(start);
      if(replacementName) {const old=row.source.data_parameters.find(p=>p.rest)?.names[0]||'arguments';if(o.name===old){
        const edit=edits.find(e=>e.start===oldStart&&e.value===`${old}:${replacementName}`);
        if(edit)o.start_byte+=Buffer.byteLength(old+':');
        o.name=replacementName;o.path=o.path.replace(new RegExp('^'+old+'(?=[.\\[]|$)'),replacementName);o.end_byte=o.start_byte+Buffer.byteLength(replacementName);
      }}
    }
    result.sink.line=coord(remap(sf.getPositionOfLineAndCharacter(row.sink.line-1,0)));
  }
  // Recompute synthetic identity uniqueness on the rewritten occurrence lines.
  if(result.sink.file===row.source.file)for(const o of result.sink.value_occurrences) {
    let count=0;walk(ts,outsf,n=>{if((ts.isIdentifier(n)||n.kind===ts.SyntaxKind.ThisKeyword)&&n.getText(outsf)===o.name&&coord(n.getStart(outsf))===o.line)count++;});o.line_occurrences=count;
  }
  result.syntactic_path=null;
  fs.writeFileSync(file,output);
  return {row:result,pattern,edits};
}
function rewriteCapture(ts,root,row,registration=false) {
  const target=registration?row.source:row.sink;
  const file=path.join(root,target.file),text=fs.readFileSync(file,'utf8'),sf=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true);
  refuseReflection(text);
  const byte=p=>Buffer.byteLength(text.slice(0,p)),pos=b=>Buffer.from(text).subarray(0,b).toString('utf8').length;
  let callback;
  walk(ts,sf,n=>{if(ts.isFunctionLike(n)&&n.body&&byte(n.getStart(sf))===target.start_byte&&byte(n.end)===target.end_byte)callback=n;});
  if(!callback||!ts.isCallExpression(callback.parent)&&!ts.isNewExpression(callback.parent))throw Error('sink callable is not an inline callback');
  const call=callback.parent,index=[...call.arguments].indexOf(callback);
  if(index<0||[...call.arguments].slice(0,index).some(a=>!ts.isIdentifier(a)&&!ts.isLiteralExpression(a)))throw Error('callback creation order has side-effecting earlier arguments');
  let statement=call;while(statement.parent&&!ts.isBlock(statement.parent)&&!ts.isSourceFile(statement.parent))statement=statement.parent;
  if(!statement.parent)throw Error('callback has no same-scope statement insertion');
  for(let p=call.parent;p&&p!==statement.parent;p=p.parent)
    if(ts.isIfStatement(p)||ts.isConditionalExpression(p)||ts.isForStatement(p)||ts.isForOfStatement(p)||ts.isForInStatement(p)||ts.isWhileStatement(p)||ts.isDoStatement(p)||ts.isFunctionLike(p)&&p!==callback)
      throw Error('conditional callback scope is unsupported');
  if(!ts.isExpressionStatement(statement)&&!ts.isReturnStatement(statement)&&!ts.isVariableStatement(statement))throw Error('unsupported callback statement scope');
  let binding='__secbench_callback';while(text.includes(binding))binding+='_';
  // An effect-free identity call prevents NamedEvaluation from changing .name
  // while keeping the callable in Prism's recognized declaration context.
  const start=callback.getStart(sf),end=callback.end,insert=statement.getStart(sf),prefix=`const ${binding}=(value=>value)(`,value=prefix+callback.getText(sf)+');';
  const output=text.slice(0,insert)+value+text.slice(insert,start)+binding+text.slice(end);
  const outsf=ts.createSourceFile(file,output,ts.ScriptTarget.Latest,true);
  if(outsf.parseDiagnostics.length)throw Error('capture rewrite diagnostics');
  const map=p=>p>=start&&p<=end?insert+prefix.length+(p-start):p<insert?p:p<start?p+value.length:p+value.length+binding.length-(end-start);
  const outbyte=p=>Buffer.byteLength(output.slice(0,p)),line=p=>outsf.getLineAndCharacterOfPosition(p).line+1;
  const result=structuredClone(row);
  if(row.sink.file===target.file) {
  for(const o of result.sink.value_occurrences){const p=map(pos(o.start_byte)),q=map(pos(o.end_byte));o.start_byte=outbyte(p);o.end_byte=outbyte(q);o.line=line(p);let count=0;walk(ts,outsf,n=>{if((ts.isIdentifier(n)||n.kind===ts.SyntaxKind.ThisKeyword)&&n.getText(outsf)===o.name&&line(n.getStart(outsf))===o.line)count++;});o.line_occurrences=count;}
  const sinkStart=map(pos(row.sink.start_byte)),sinkEnd=map(pos(row.sink.end_byte));
  result.sink.start_byte=outbyte(sinkStart);result.sink.end_byte=outbyte(sinkEnd);result.sink.start_line=line(sinkStart);result.sink.end_line=line(sinkEnd);
  result.sink.line=result.sink.value_occurrences[0].line;
  if(target.start_byte===row.sink.start_byte)result.sink.nested_callback=false;
  }
  if(row.source.file===target.file) {
    const p=map(pos(row.source.start_byte)),q=map(pos(row.source.end_byte));result.source.start_byte=outbyte(p);result.source.end_byte=outbyte(q);result.source.start_line=line(p);result.source.end_line=line(q);
    for(const parameter of result.source.data_parameters){const p=map(pos(parameter.start_byte)),q=map(pos(parameter.end_byte));parameter.start_byte=outbyte(p);parameter.end_byte=outbyte(q);parameter.line=line(p);}
  }
  fs.writeFileSync(file,output);
  return {row:result,pattern:'callback_capture',edits:[{start:insert,end:insert,value},{start,end,value:binding}]};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [compiler,root,input,pattern,out]=process.argv.slice(2);
  const ts=loadCompiler(compiler);
  fs.writeFileSync(out,JSON.stringify(pattern==='admission_plan'?admissionPlan(ts,root):rewrite(ts,root,JSON.parse(fs.readFileSync(input)),pattern))+'\n');
}
