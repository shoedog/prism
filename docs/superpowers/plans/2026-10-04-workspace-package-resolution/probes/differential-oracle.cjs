const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const [tsPath,manifest]=process.argv.slice(2),ts=require(tsPath);
const hash=crypto.createHash('sha256').update(fs.readFileSync(tsPath)).digest('hex');
if(ts.version!=='5.9.3'||hash!=='3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675')throw Error('oracle drift');
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
let n=0;
for(const c of JSON.parse(fs.readFileSync(manifest))){
 const root=fs.realpathSync(c.root),abs=path.join(root,c.writer);
 const host={...ts.sys,getCurrentDirectory:()=>root,writeFile(){throw Error('write');},watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout(){}};
 const service=new ts.server.ProjectService({host,logger,cancellationToken:{isCancellationRequested:()=>false},useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
 service.openClientFile(abs,undefined,undefined,root);
 const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(abs),true),opts=project.getCompilerOptions(),program=project.getLanguageService().getProgram(),sf=program.getSourceFile(abs);
 if(!sf)throw Error('writer excluded: '+c.id);
 const imp=sf.statements.find(ts.isImportDeclaration),mode=ts.getModeForUsageLocation(sf,imp.moduleSpecifier,opts);
 const r=ts.resolveModuleName(c.specifier,abs,opts,host,undefined,undefined,mode).resolvedModule;
 let target=null;if(r)target=path.relative(root,fs.realpathSync(r.resolvedFileName));
 const checker=program.getTypeChecker();let sym=checker.getSymbolAtLocation(imp.importClause.namedBindings.elements[0].name);
 if(sym&&(sym.flags&ts.SymbolFlags.Alias))sym=checker.getAliasedSymbol(sym);
 const declarations=(sym?.declarations||[]).map(d=>({file:path.relative(root,d.getSourceFile().fileName),name:d.name?.text,start_line:d.getSourceFile().getLineAndCharacterOfPosition(d.getStart()).line+1,end_line:d.getSourceFile().getLineAndCharacterOfPosition(d.getEnd()-1).line+1}));
 console.log(JSON.stringify({id:c.id,target,owner:path.relative(root,project.getProjectName()),mode,declarations,inProgram:!!(r&&program.getSourceFile(r.resolvedFileName)),diagnostics:ts.getPreEmitDiagnostics(program,sf).map(d=>d.code)}));
 service.closeClientFile(abs);
 if(++n%100===0){process.stderr.write(`oracle ${n}\n`);if(global.gc)global.gc();}
}
