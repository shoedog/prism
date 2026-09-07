// Source-backed UMD bridge acceptance and preserved audit negatives; no runtime consumer.
import test from "node:test";
import assert from "node:assert/strict";
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,realpathSync,cpSync} from "node:fs";
import {tmpdir} from "node:os";
import path from "node:path";
import {createRequire} from "node:module";
import {COMPILER_HASH,hash} from "./schema.mjs";
import {produce,validate} from "./index.mjs";

const compiler=process.env.PRISM_TYPESCRIPT,profiles=process.env.PRISM_CALLABLE_PROFILES;
assert(compiler && profiles,"explicit pinned compiler and profiles required");
assert.equal(hash(readFileSync(compiler)),COMPILER_HASH);
const ts=createRequire(import.meta.url)(compiler);
const provider=(local="Widget")=>`export = ${local}; export as namespace Widget;
declare namespace ${local} { interface View<P> { (props:P):null; } const marker:number; }`;
const app="class Client {m(){}} export const run: Widget.View<{client:Client}> = ({client}) => {const cb=()=>client.m();return null;};";
function fixture(run) {
  const root=realpathSync(mkdtempSync(path.join(tmpdir(),"prism-umd-audit-")));
  const config={compilerOptions:{strict:true,noEmit:true,target:"ES2022",module:"ESNext",moduleResolution:"Bundler",types:[],libReplacement:false},include:["src"]};
  const put=(file,text)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),text);};
  put("package.json",'{"type":"module"}');put("tsconfig.json",JSON.stringify(config));put("src/umd.d.ts",provider());put("src/app.ts",app);
  const options={root,compiler,config:"tsconfig.json"};
  const inspect=()=>{
    const read=ts.readConfigFile(path.join(root,"tsconfig.json"),ts.sys.readFile);
    const parsed=ts.parseJsonConfigFileContent(read.config,ts.sys,root,undefined,path.join(root,"tsconfig.json"));
    assert(!read.error);assert.deepEqual(parsed.errors,[]);
    const program=ts.createProgram(parsed.fileNames,parsed.options),checker=program.getTypeChecker();
    const sf=program.getSourceFile(path.join(root,"src/app.ts"));let annotation;
    function visit(n){if(ts.isVariableDeclaration(n)&&n.name.getText(sf)==="run")annotation=n.type;ts.forEachChild(n,visit);}visit(sf);
    assert(annotation && ts.isTypeReferenceNode(annotation) && ts.isQualifiedName(annotation.typeName));
    let symbol=checker.getSymbolAtLocation(annotation.typeName.left);const chain=[],seen=new Set();
    while(symbol && !seen.has(symbol) && chain.length<8){seen.add(symbol);chain.push({symbol,declarations:symbol.declarations??[]});if(!(symbol.flags&ts.SymbolFlags.Alias))break;symbol=checker.getImmediateAliasedSymbol(symbol);}
    const exports=program.getSourceFiles().flatMap(f=>f.statements.filter(n=>ts.isNamespaceExportDeclaration(n)&&n.name.text===annotation.typeName.left.getText(sf)));
    const context=checker.getContextualType(annotation.parent.initializer);
    const signatures=context?checker.getSignaturesOfType(context,ts.SignatureKind.Call):[];
    return {program,checker,sf,annotation,chain,exports,signatures,codes:[...new Set(ts.getPreEmitDiagnostics(program).map(d=>d.code))].sort((a,b)=>a-b)};
  };
  try {return run({root,put,config,options,inspect});} finally {rmSync(root,{recursive:true,force:true});}
}
const kinds=raw=>raw.chain.map(h=>h.declarations.map(d=>ts.SyntaxKind[d.kind]));
function withheld(p) {
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert.equal(p.observations[0].provenance.status,"unproven");
  assert(p.observations[0].nested.calls.every(c=>c.props_class.status==="unproven"));
}
function supported(p) {
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert.equal(p.status,"observed");assert.equal(p.observations[0].provenance.status,"traced");
  const aliases=p.observations[0].provenance.hops[0].qualifiers[0].aliases;
  assert.deepEqual(aliases.map(a=>a.declarations.map(d=>d.kind)),[["NamespaceExportDeclaration"],["ExportAssignment"]]);
  assert.equal(aliases[0].module.kind,"SourceFile");
  assert.deepEqual(aliases[0].module_declarations,[aliases[0].module]);
  assert.deepEqual(aliases[0].module_exports,aliases[0].target);
  assert.deepEqual(aliases[0].module_bindings,aliases[1].target);
  assert.equal(p.observations[0].nested.calls.length,1);
  assert.equal(p.observations[0].nested.calls[0].props_class.status,"observed");
}

test("singleton UMD global export and same-named local namespace are distinct symbols",()=>fixture(({inspect,options})=>{
  const raw=inspect();assert.deepEqual(raw.codes,[]);
  assert.deepEqual(kinds(raw),[["NamespaceExportDeclaration"],["ExportAssignment"],["ModuleDeclaration"]]);
  assert.equal(raw.exports.length,1);assert.equal(raw.signatures.length,1);
  const [global,assignment,local]=raw.chain;
  assert.notEqual(global.symbol,local.symbol);
  assert.equal(global.symbol,global.declarations[0].parent.symbol.globalExports.get("Widget"));
  assert.equal(raw.checker.getSymbolAtLocation(assignment.declarations[0].expression),local.symbol);
  assert.equal(raw.checker.getSymbolAtLocation(local.declarations[0].name),local.symbol);
  const p=produce(options);supported(p);
  assert.equal(validate(JSON.stringify(p),options).valid,true);
}));

test("renamed module-local namespace has the same source-backed singleton bridge",()=>fixture(({put,inspect,options})=>{
  put("src/umd.d.ts",provider("Implementation"));const raw=inspect();
  assert.deepEqual(raw.codes,[]);assert.deepEqual(kinds(raw),[["NamespaceExportDeclaration"],["ExportAssignment"],["ModuleDeclaration"]]);
  assert.equal(raw.chain[2].symbol.getName(),"Implementation");
  supported(produce(options));
}));

for(const profile of ["react18","react19"])test(`${profile} UMD type annotation needs source identity, not an FC spelling exception`,()=>fixture(({root,put,inspect,options})=>{
  cpSync(path.join(profiles,profile,"node_modules"),path.join(root,"node_modules"),{recursive:true});
  rmSync(path.join(root,"src/umd.d.ts"));put("src/app.ts","import 'react';"+app.replace("Widget.View","React.FC"));
  const raw=inspect();assert.deepEqual(raw.codes,[]);
  assert.deepEqual(kinds(raw),[["NamespaceExportDeclaration"],["ExportAssignment"],["ModuleDeclaration"]]);
  assert.equal(raw.exports.length,1);assert.equal(raw.signatures.length,1);
  assert.equal(path.relative(root,raw.chain[0].declarations[0].getSourceFile().fileName),"node_modules/@types/react/index.d.ts");
  supported(produce(options));
}));

test("same global name from two provider files is a distinct negative population",()=>fixture(({put,inspect,options})=>{
  put("src/umd.d.ts",provider("One"));put("src/second.d.ts",provider("Two"));
  const raw=inspect();assert.equal(raw.exports.length,2);
  assert.equal(new Set(raw.exports.map(d=>d.getSourceFile().fileName)).size,2);
  withheld(produce(options));
}));
test("duplicate global-export declarations in one file must not become a singleton",()=>fixture(({put,inspect,options})=>{
  put("src/umd.d.ts",provider("Implementation")+"\nexport as namespace Widget;");
  const raw=inspect();assert.equal(raw.exports.length,2);assert(raw.codes.length>0);
  const p=produce(options);assert.equal(p.status,"unproven");withheld(p);
}));
test("merged module-local namespace remains distinct from a singleton UMD bridge",()=>fixture(({put,inspect,options})=>{
  put("src/umd.d.ts",provider()+"\ndeclare namespace Widget {const extra:number;}");
  const raw=inspect();assert.deepEqual(raw.codes,[]);assert.equal(raw.chain.at(-1).declarations.length,2);
  withheld(produce(options));
}));
test("external module augmentation cannot be hidden by a unique global export",()=>fixture(({put,inspect,options})=>{
  put("src/augment.d.ts","import './umd';declare module './umd' {interface View<P>{(props:P,extra:number):null;}}");
  const raw=inspect();assert.equal(raw.exports.length,1);assert.equal(raw.signatures.length,2);
  assert(raw.chain.at(-1).declarations.length>1);withheld(produce(options));
}));
test("global namespace augmentation is not a harmless local spelling peer",()=>fixture(({put,inspect,options})=>{
  put("src/augment.d.ts","export {};declare global {namespace Widget {interface Extra {tag:string}}}");
  const raw=inspect();assert.equal(raw.exports.length,1);
  assert(raw.codes.length>0 || raw.chain[0].declarations.some(d=>ts.isModuleDeclaration(d)));
  const p=produce(options);assert.equal(p.authorizes_runtime_edge,false);
  assert(p.status==="unproven" || p.observations[0].provenance.status==="unproven");
}));
test("local namespace shadow wins over the available UMD global",()=>fixture(({put,inspect,options})=>{
  put("src/app.ts","namespace Widget {export type View<P>=(props:P)=>null;}"+app);
  const raw=inspect();assert.deepEqual(raw.codes,[]);assert.equal(raw.exports.length,1);
  assert.deepEqual(kinds(raw),[["ModuleDeclaration"]]);
  const p=produce(options);assert.equal(p.status,"observed");assert.equal(p.observations[0].provenance.status,"traced");
  assert.equal(p.observations[0].provenance.terminal.file,"project/src/app.ts");
  assert.equal(p.authorizes_runtime_edge,false);
}));
test("explicit namespace import uses its existing import bridge, not the UMD global",()=>fixture(({put,inspect,options})=>{
  put("src/app.ts","import * as Widget from './umd';"+app);
  const raw=inspect();assert.deepEqual(raw.codes,[]);assert.equal(raw.exports.length,1);
  assert.deepEqual(kinds(raw),[["NamespaceImport"],["ExportAssignment"],["ModuleDeclaration"]]);
  const p=produce(options);assert.equal(p.status,"observed");assert.equal(p.observations[0].provenance.status,"traced");
  assert.equal(p.authorizes_runtime_edge,false);
}));
test("missing provider and unresolved export target stay unproven",()=>fixture(({root,put,inspect,options})=>{
  rmSync(path.join(root,"src/umd.d.ts"));let raw=inspect();assert.equal(raw.exports.length,0);
  assert(raw.codes.includes(2503));assert.equal(raw.chain.flatMap(h=>h.declarations).length,0,"unresolved transient symbols are not declarations");
  withheld(produce(options));
  put("src/umd.d.ts","export = Missing;export as namespace Widget;");raw=inspect();assert(raw.codes.length>0);
  withheld(produce(options));
}));
test("global export syntax outside a declaration file is not compiler-bound global authority",()=>fixture(({root,put,inspect,options})=>{
  rmSync(path.join(root,"src/umd.d.ts"));put("src/umd.ts",provider());
  const raw=inspect();assert.equal(raw.exports.length,1);assert(raw.codes.includes(1315));
  assert.equal(raw.chain.flatMap(h=>h.declarations).length,0,"invalid export syntax did not bind a global declaration");
  withheld(produce(options));
}));
test("allowUmdGlobalAccess governs value-use diagnostics, not this type-only qualifier",()=>fixture(({put,config,inspect,options})=>{
  for(const enabled of [false,true]) {
    put("tsconfig.json",JSON.stringify({...config,compilerOptions:{...config.compilerOptions,allowUmdGlobalAccess:enabled}}));
    put("src/app.ts",app);assert.deepEqual(inspect().codes,[]);supported(produce(options));
    put("src/app.ts",app+"export const value=Widget.marker;");
    assert.equal(inspect().codes.includes(2686),!enabled);
  }
}));
test("provider changes and removal replace the source-backed epoch",()=>fixture(({root,put,options})=>{
  const before=produce(options);assert.equal(validate(JSON.stringify(before),options).valid,true);
  put("src/umd.d.ts",provider("Implementation"));assert.equal(validate(JSON.stringify(before),options).valid,false);
  const renamed=produce(options);supported(renamed);
  rmSync(path.join(root,"src/umd.d.ts"));assert.equal(validate(JSON.stringify(renamed),options).valid,false);
  put("src/umd.d.ts",provider());assert.equal(validate(JSON.stringify(before),options).valid,true);
}));

for(const [label,extra] of [
  ["competing global provider",provider("Second")],
  ["global namespace augmentation","export {};declare global {namespace Widget {interface Extra {tag:string}}}"],
  ["global script namespace","declare namespace Widget {interface Extra {tag:string}}"],
  ["external module augmentation","import './umd';declare module './umd' {interface View<P>{(props:P,extra:number):null;}}"],
])test(`skipLibCheck does not erase ${label}`,()=>fixture(({put,config,options})=>{
  put("tsconfig.json",JSON.stringify({...config,compilerOptions:{...config.compilerOptions,skipLibCheck:true}}));
  put("src/extra.d.ts",extra);const p=produce(options);
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
  assert(p.observations[0].provenance.status==="unproven" || p.observations[0].provenance.terminal.file==="project/src/extra.d.ts");
  assert(p.observations[0].nested.calls.every(c=>c.props_class.status==="unproven"));
}));
for(const [label,text] of [
  ["duplicate export assignment",provider("Implementation")+"export = Implementation;"],
  ["duplicate global export",provider("Implementation")+"export as namespace Widget;"],
  ["merged local target",provider()+"declare namespace Widget {const extra:number;}"],
  ["star export",provider()+"export * from './extra';"],
  ["imported namespace target","import * as Implementation from './extra';export = Implementation;export as namespace Widget;"],
  ["qualified export target","declare namespace Outer {namespace Inner {interface View<P>{(props:P):null;}}}export = Outer.Inner;export as namespace Widget;"],
])test(`UMD bridge excludes ${label} even with skipLibCheck`,()=>fixture(({put,config,options})=>{
  put("tsconfig.json",JSON.stringify({...config,compilerOptions:{...config.compilerOptions,skipLibCheck:true}}));
  put("src/extra.d.ts","export interface View<P>{(props:P):null;}export const unrelated:number;");
  put("src/umd.d.ts",text);withheld(produce(options));
}));
test("UMD traversal consumes the existing step budget",()=>fixture(({options})=>{
  const p=produce({...options,limits:{provenance_steps:2}});withheld(p);
  assert.equal(p.observations[0].provenance.reason,"step_limit");
  supported(produce(options));
}));
test("traced UMD candidates cannot overcome incomplete Program closure",()=>fixture(({put,options})=>{
  put("src/app.ts","import './missing';"+app);const p=produce(options);
  assert.equal(p.status,"unproven");assert(p.reasons.includes("unresolved_module"));
  assert.equal(p.observations[0].provenance.status,"traced");
  assert.equal(p.observations[0].nested.calls[0].props_class.reason,"program_unproven");
  assert.equal(p.authorizes_runtime_edge,false);assert.equal(p.scope.class_authority,false);
}));
test("traced UMD cannot bypass receiver write barriers",()=>fixture(({put,options})=>{
  put("src/app.ts",app.replace("const cb=()","client = new Client();const cb=()"));
  const p=produce(options);assert.equal(p.observations[0].provenance.status,"traced");
  assert.equal(p.observations[0].nested.calls[0].binding.reason,"write_barrier");
  assert.equal(p.observations[0].nested.calls[0].props_class.status,"unproven");
}));
test("UMD source anchors are recomputed, not accepted from a tampered packet",()=>fixture(({options})=>{
  const p=produce(options);supported(p);
  p.observations[0].provenance.hops[0].qualifiers[0].aliases[0].module_bindings=[];
  assert.equal(validate(JSON.stringify(p),options).valid,false);
}));
