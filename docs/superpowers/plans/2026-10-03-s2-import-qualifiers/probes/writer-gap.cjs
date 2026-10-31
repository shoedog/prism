// READ: S2-0 census using the caller's native TypeScript ProjectService checker.
// No Prism module/export resolution is used by the oracle. No source writes.
// Usage: node census.cjs TS_JS ROOT BASE_SITES FACTS OUT_DIRECTORY
'use strict';
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto'), zlib = require('node:zlib');
const [tsPath, rootArg, sitesPath, factsPath, out] = process.argv.slice(2);
const ts = require(tsPath), root = fs.realpathSync(rootArg);
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
if (ts.version !== '5.9.3' || sha(tsPath) !== '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675') throw Error('oracle drift');
const readLines = p => (p.endsWith('.gz') ? zlib.gunzipSync(fs.readFileSync(p)).toString('utf8') : fs.readFileSync(p, 'utf8')).trim().split('\n').filter(Boolean).map(JSON.parse);
const sites = readLines(sitesPath).filter(r => r.record_kind === 'call_site');
if (!sites.length) throw Error('zero-site probe is inadmissible');
const facts = readLines(factsPath);
const rel = p => path.relative(root, p).split(path.sep).join('/');
const inside = p => rel(p) !== '..' && !rel(p).startsWith('../') && !path.isAbsolute(rel(p));
const sourceHashes = {};
for (const f of facts) {
  const actual = sha(path.join(root, f.file));
  const expected = Array.isArray(f.hash) ? Buffer.from(f.hash).toString('hex') : f.hash;
  if (actual !== expected) throw Error('source/facts drift');
  sourceHashes[f.file] = actual;
}
const inputs = new Map();
const textInputs = new Map();
// Decode one retained read exactly as pinned TypeScript 5.9.3 sys.readFile
// (typescript.js:8525-8549). Bind raw bytes separately from BOM-stripped text.
function readOracleInput(p) {
  let bytes;
  try { bytes = fs.readFileSync(p); } catch { return undefined; }
  const rawHash = crypto.createHash('sha256').update(bytes).digest('hex');
  if (inputs.has(p) && inputs.get(p) !== rawHash) throw Error('oracle input drift during run');
  let text;
  if (bytes.length >= 2 && bytes[0] === 254 && bytes[1] === 255) {
    const swapped = Buffer.from(bytes);
    for (let i = 0; i < (swapped.length & ~1); i += 2) {
      const first = swapped[i]; swapped[i] = swapped[i + 1]; swapped[i + 1] = first;
    }
    text = swapped.toString('utf16le', 2);
  } else if (bytes.length >= 2 && bytes[0] === 255 && bytes[1] === 254) {
    text = bytes.toString('utf16le', 2);
  } else if (bytes.length >= 3 && bytes[0] === 239 && bytes[1] === 187 && bytes[2] === 191) {
    text = bytes.toString('utf8', 3);
  } else {
    text = bytes.toString('utf8');
  }
  inputs.set(p, rawHash);
  textInputs.set(p, crypto.createHash('sha256').update(text).digest('hex'));
  return text;
}
const host = {...ts.sys, getCurrentDirectory: () => root,
  writeFile() { throw Error('oracle source write refused'); },
  watchFile: () => ({close(){}}), watchDirectory: () => ({close(){}}),
  setTimeout: () => 0, clearTimeout(){}, readFile: readOracleInput};
const logger = {hasLevel:()=>false, loggingEnabled:()=>false, info(){}, msg(){}, perftrc(){}, startGroup(){}, endGroup(){}, getLogFileName:()=>undefined};
const service = new ts.server.ProjectService({host, logger, cancellationToken:{isCancellationRequested:()=>false},
  useSingleInferredProject:false, useInferredProjectPerProjectRoot:true, typingsInstaller:ts.server.nullTypingsInstaller});

const records=[];
for (const f of facts) {
  if(!(f.writer_resolutions||[]).length)continue;
  const absolute=path.join(root,f.file);
  service.openClientFile(absolute, undefined, undefined, root);
  const project=service.getDefaultProjectForFile(ts.server.toNormalizedPath(absolute),true);
  const opts=project?.getCompilerOptions(), sf=project?.getLanguageService().getProgram()?.getSourceFile(absolute);
  if(!sf)throw Error('writer absent '+f.file);
  const literals=[];
  function visit(n){
    if((ts.isImportDeclaration(n)||ts.isExportDeclaration(n))&&n.moduleSpecifier&&ts.isStringLiteral(n.moduleSpecifier))literals.push(n.moduleSpecifier);
    if(ts.isImportEqualsDeclaration(n)&&ts.isExternalModuleReference(n.moduleReference)&&n.moduleReference.expression&&ts.isStringLiteral(n.moduleReference.expression))literals.push(n.moduleReference.expression);
    if(ts.isCallExpression(n)&&((ts.isIdentifier(n.expression)&&n.expression.text==='require')||n.expression.kind===ts.SyntaxKind.ImportKeyword)&&n.arguments.length===1&&ts.isStringLiteral(n.arguments[0]))literals.push(n.arguments[0]);
    ts.forEachChild(n,visit);
  } visit(sf);
  for(const status of f.writer_resolutions){
    const uses=literals.filter(n=>n.text===status.specifier);
    if(!uses.length)throw Error('specifier absent '+f.file+' '+status.specifier);
    const modes=new Map();for(const literal of uses){const mode=ts.getModeForUsageLocation(sf,literal,opts);modes.set(mode,mode);}
    for(const mode of modes.values()){
      const r=ts.resolveModuleName(status.specifier,absolute,opts,host,undefined,undefined,mode).resolvedModule;
      const target=r?fs.realpathSync(r.resolvedFileName):null;
      const reason=status.resolution?.Unsupported;
      const category=status.specifier.startsWith('node:')?'builtin':status.specifier.startsWith('#')?'package_import':status.specifier.includes(':')?'loader_scheme':status.specifier.startsWith('@excalidraw/')?'workspace_package':'bare_or_alias';
      records.push({writer:f.file,specifier:status.specifier,mode:mode??null,category,prism_resolution:status.resolution,prism_owner:status.owner,native_owner:project.projectKind===ts.server.ProjectKind.Configured?rel(project.getProjectName()):null,native_target:target?rel(target):null,reason:reason??null,gap:!!reason&&!!target,false_absence:status.resolution==='ProvenUnresolved'&&!!target});
    }
  }
  service.closeClientFile(absolute);
}
for(const [p,h] of inputs)if(sha(p)!==h)throw Error('oracle input drift');
const counts={};for(const r of records.filter(r=>r.gap)){const key=r.category+' | '+r.reason;counts[key]=(counts[key]||0)+1;}
fs.writeFileSync(path.join(out,'writer-gap.json'),JSON.stringify({scope:'B writer probes for non-relative static import bindings; union by writer/specifier/usage mode',records,gap_count:records.filter(r=>r.gap).length,false_absence:records.filter(r=>r.false_absence),counts,sourceHashes,inputs:Object.fromEntries(inputs),oracle_sha256:sha(tsPath)},null,2)+'\n');
console.log(JSON.stringify({writers:records.length,gaps:records.filter(r=>r.gap).length,false_absence:records.filter(r=>r.false_absence).length,counts}));
