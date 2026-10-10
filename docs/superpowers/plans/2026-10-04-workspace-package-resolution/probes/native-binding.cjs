// Fresh canonical module + actual writer-owner certificate for a changed proof.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const [tsPath,rootArg,writer,spec]=process.argv.slice(2),ts=require(tsPath),root=fs.realpathSync(rootArg),abs=path.join(root,writer);
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
if(ts.version!=='5.9.3'||sha(tsPath)!=='3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675')throw Error('oracle drift');
const inputs=new Map(),host={...ts.sys,getCurrentDirectory:()=>root,writeFile(){throw Error('write');},watchFile:()=>({close(){}}),watchDirectory:()=>({close(){}}),setTimeout:()=>0,clearTimeout(){},readFile(p){const s=ts.sys.readFile(p);if(s!==undefined)inputs.set(p,sha(p));return s;}};
const logger={hasLevel:()=>false,loggingEnabled:()=>false,info(){},msg(){},perftrc(){},startGroup(){},endGroup(){},getLogFileName:()=>undefined};
const service=new ts.server.ProjectService({host,logger,cancellationToken:{isCancellationRequested:()=>false},useSingleInferredProject:false,useInferredProjectPerProjectRoot:true,typingsInstaller:ts.server.nullTypingsInstaller});
service.openClientFile(abs,undefined,undefined,root);
const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(abs),true),opts=project.getCompilerOptions(),sf=project.getLanguageService().getProgram().getSourceFile(abs);
if(!sf)throw Error('writer absent');let literal;
function walk(n){if((ts.isImportDeclaration(n)||ts.isExportDeclaration(n))&&n.moduleSpecifier?.text===spec)literal=n.moduleSpecifier;ts.forEachChild(n,walk);}walk(sf);
if(!literal)throw Error('specifier absent');const mode=ts.getModeForUsageLocation(sf,literal,opts),r=ts.resolveModuleName(spec,abs,opts,host,undefined,undefined,mode).resolvedModule;
for(const [p,h] of inputs)if(sha(p)!==h)throw Error('input drift');
console.log(JSON.stringify({writer,spec,owner:project.projectKind===ts.server.ProjectKind.Configured?path.relative(root,project.getProjectName()):null,target:r?path.relative(root,fs.realpathSync(r.resolvedFileName)):null,mode,inputs:Object.fromEntries(inputs)}));service.closeClientFile(abs);
