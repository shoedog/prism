// Public-only explanation of the old 179 rows using actual head refusal facts.
// Native syntax labels explain a witnessed refusal; they do not grant admission.
const fs = require('fs'), path = require('path'), zlib = require('zlib');
const [tsPath, root, prior, current] = process.argv.slice(2);
const ts = require(tsPath);
const read = p => JSON.parse(p.endsWith('.gz') ? zlib.gunzipSync(fs.readFileSync(p)) : fs.readFileSync(p));
const old = read(prior), changed = read(path.join(current, 'changed.json'));
const facts = fs.readFileSync(path.join(current, 'facts.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
const byFile = new Map(facts.map(f => [f.file, f]));
const key = r => JSON.stringify(r.key);
const survivors = new Set(changed.map(key));
const classes = new Map();
for (const r of old) {
  const d = r.native.qualifier_declarations[0], k = d.file + ':' + d.name;
  if (!classes.has(k)) classes.set(k, {file:d.file, name:d.name, rows:[], witnesses:[]});
  classes.get(k).rows.push(r);
}
function typePosition(n) {
  for (let p=n.parent; p; p=p.parent) {
    if (ts.isTypeNode(p)) {
      if (ts.isExpressionWithTypeArguments(p) && p.parent.token===ts.SyntaxKind.ExtendsKeyword && ts.isClassDeclaration(p.parent.parent)) continue;
      return true;
    }
  }
  return false;
}
function direct(n) {
  const p=n.parent;
  return (ts.isNewExpression(p) && p.expression===n) ||
    (ts.isPropertyAccessExpression(p) && !p.questionDotToken && p.expression===n &&
     ts.isCallExpression(p.parent) && p.parent.expression===p);
}
function owner(n) {
  for(let p=n.parent; p; p=p.parent) {
    if(ts.isClassDeclaration(p)) return p.name?.text;
    if(ts.isObjectLiteralExpression(p) && ts.isVariableDeclaration(p.parent)) return p.parent.name.text;
  }
}
function cause(n) {
  const p=n.parent;
  if(typePosition(n) || direct(n)) return null;
  if ((ts.isClassDeclaration(p)||ts.isVariableDeclaration(p)||ts.isFunctionDeclaration(p)||ts.isFunctionExpression(p)||ts.isParameter(p)||ts.isModuleDeclaration(p)) && p.name===n) return null;
  if(ts.isImportSpecifier(p)||ts.isImportClause(p)||ts.isNamespaceImport(p)||ts.isNamespaceExport(p)) return null;
  if(ts.isExportSpecifier(p)) return p.propertyName && p.propertyName.text!==p.name.text ? 'renamed_export' : null;
  if(ts.isExportAssignment(p)) return null; // handled against other direct uses below
  if(ts.isPropertyAccessExpression(p)||ts.isElementAccessExpression(p)) {
    const up=p.parent;
    if((ts.isBinaryExpression(up) && up.left===p && up.operatorToken.kind>=ts.SyntaxKind.FirstAssignment && up.operatorToken.kind<=ts.SyntaxKind.LastAssignment)||ts.isDeleteExpression(up)||ts.isPostfixUnaryExpression(up)||ts.isPrefixUnaryExpression(up)) return 'member_write';
    return ts.isElementAccessExpression(p) ? 'computed_access' : 'member_value_or_chain';
  }
  if((ts.isVariableDeclaration(p)||ts.isBinaryExpression(p)) && (p.initializer===n||p.right===n)) return 'alias_or_assignment';
  if(ts.isReturnStatement(p)||ts.isArrowFunction(p)) return 'returned_value';
  if(ts.isCallExpression(p)||ts.isNewExpression(p)) return 'argument_or_reflection';
  if(ts.isSpreadAssignment(p)||ts.isSpreadElement(p)) return 'spread_value';
  if(ts.isPropertyAssignment(p)||ts.isShorthandPropertyAssignment(p)||ts.isArrayLiteralExpression(p)||ts.isBindingElement(p)) return 'stored_or_destructured_value';
  return 'other_lexical_value_use';
}
const ranks=['member_write','alias_or_assignment','renamed_export','computed_access','returned_value','argument_or_reflection','spread_value','stored_or_destructured_value','member_value_or_chain','other_lexical_value_use'];
const sources = new Map();
function source(file) {
  if(!sources.has(file)) sources.set(file,ts.createSourceFile(file,fs.readFileSync(path.join(root,file),'utf8'),ts.ScriptTarget.Latest,true));
  return sources.get(file);
}
function relative(from,spec) {
  const stem=path.posix.normalize(path.posix.join(path.posix.dirname(from),spec));
  return [stem,stem+'.ts',stem+'.tsx',stem+'.js',stem+'.jsx',stem+'/index.ts',stem+'/index.tsx'].find(p=>byFile.has(p));
}
function exportsIdentity(file,c,depth=0,seen=new Set()) {
  if(file===c.file) return true;
  if(!file||depth>2||seen.has(file)) return false;
  seen=new Set(seen);seen.add(file);
  // Source-bound star/named barrel path; the namespace module itself must
  // already have a product module proof in the importing file's facts.
  for(const s of source(file).statements) {
    if(!ts.isExportDeclaration(s)||!s.moduleSpecifier||!ts.isStringLiteral(s.moduleSpecifier)) continue;
    if(s.exportClause && (!ts.isNamedExports(s.exportClause)||!s.exportClause.elements.some(e=>(e.propertyName||e.name).text===c.name))) continue;
    if(!s.moduleSpecifier.text.startsWith('.')) continue;
    if(exportsIdentity(relative(file,s.moduleSpecifier.text),c,depth+1,seen)) return true;
  }
  return false;
}
for(const f of facts) {
  const refused=new Set(f.qualifier_facts?.written||[]);
  const sf=source(f.file);
  const nodes=[]; const visit=n=>{nodes.push(n);ts.forEachChild(n,visit);}; visit(sf);
  for(const c of classes.values()) {
    for(const b of f.bindings||[]) {
      if(b.member!==null || !refused.has(b.local)) continue;
      const proof=f.module_proofs.find(p=>p.specifier===b.module_path);
      if(!proof || !exportsIdentity(proof.module,c)) continue;
      for(const n of nodes) {
        if(!ts.isIdentifier(n)||n.text!==b.local) continue;
        const why=cause(n);if(!why) continue;
        c.witnesses.push({file:f.file,line:sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1,
          cause:'namespace_'+why,node:ts.SyntaxKind[n.kind],parent:ts.SyntaxKind[n.parent.kind],
          text:n.parent.getText(sf).slice(0,240),namespace:b.local,module:proof.module,owner:proof.owner,provider:false});
      }
    }
    // Only explain names actually refused by the product; native syntax alone
    // is not evidence that head ran a particular refusal branch.
    if(!refused.has(c.name)) continue;
    for(const n of nodes) {
      if(!(ts.isIdentifier(n)&&n.text===c.name) && !(n.kind===ts.SyntaxKind.ThisKeyword&&owner(n)===c.name)) continue;
      const why=cause(n); if(!why) continue;
      const line=sf.getLineAndCharacterOfPosition(n.getStart(sf)).line+1;
      c.witnesses.push({file:f.file,line,cause:why,node:ts.SyntaxKind[n.kind],parent:ts.SyntaxKind[n.parent.kind],text:n.parent.getText(sf).slice(0,240),provider:f.file===c.file});
    }
  }
}
const counts={}, details=[];
for(const c of classes.values()) {
  const provider=byFile.get(c.file).qualifier_facts;
  const refusedProvider=provider.written.includes(c.name);
  const rank=w=>{const i=ranks.indexOf(w.cause.replace(/^namespace_/,''));return i<0?ranks.length:i;};
  const eligible=c.witnesses.filter(w=>!refusedProvider||w.provider).sort((a,b)=>rank(a)-rank(b)||a.file.localeCompare(b.file)||a.line-b.line);
  const kept=c.rows.filter(r=>survivors.has(key(r))).length, lost=c.rows.length-kept;
  if(lost) {
    if(!eligible.length) throw Error('refusal lacks a source witness: '+c.name);
    const bucket=(refusedProvider?'provider_':'visible_file_')+eligible[0].cause;
    counts[bucket]=(counts[bucket]||0)+lost;
    details.push({file:c.file,name:c.name,prior:c.rows.length,survived:kept,refused:lost,primary:bucket,witness:eligible[0],other_causes:[...new Set(eligible.map(w=>w.cause))]});
  } else details.push({file:c.file,name:c.name,prior:c.rows.length,survived:kept,refused:0});
}
const s={prior_rows:old.length,survived:old.filter(r=>survivors.has(key(r))).length,refused:old.filter(r=>!survivors.has(key(r))).length,refusal_causes:counts,details};
if(s.survived+s.refused!==179 || Object.values(counts).reduce((a,b)=>a+b,0)!==s.refused) throw Error('denominator drift');
fs.writeFileSync(path.join(current,'yield-refusals.json'),JSON.stringify(s,null,2)+'\n');
console.log(JSON.stringify({...s,details:undefined}));
