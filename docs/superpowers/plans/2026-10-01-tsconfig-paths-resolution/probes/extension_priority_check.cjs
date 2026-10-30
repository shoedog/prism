// Exhaust the pinned table against actual TS fileNames and the production kernel.
// Usage: node extension_priority_check.cjs TS_JS FIXTURE_ROOT KERNEL_DRIVER RECEIPT
const fs = require('fs'), path = require('path'), assert = require('assert');
const {execFileSync} = require('child_process');
const [tsPath, rootArg, driver, receipt] = process.argv.slice(2);
const ts = require(path.resolve(tsPath)), root = path.resolve(rootArg);
const source = fs.readFileSync(tsPath, 'utf8');
assert.equal(ts.version, '5.9.3');
const hash = require('crypto').createHash('sha256').update(source).digest('hex');
assert.equal(hash, '3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675');
// Read the priority groups from the compiler itself rather than a second copy.
const groups = JSON.parse(source.match(/^var allSupportedExtensions = (.*);$/m)[1]
  .replace(/\/\*.*?\*\//g, ''));
fs.mkdirSync(root, {recursive:true});
const cases = [], requests = [], errors = [];
for (const targetExt of ['jsx','tsx']) for (const group of groups) {
  for (let i = 1; i < group.length; i++) for (let j = 0; j < i; j++) {
    for (const literal of [false,true]) {
      const name = `priority-${String(cases.length).padStart(3,'0')}`;
      const dir = path.join(root,name), caller = 'app'+group[i];
      const sibling = 'app'+group[j], target = 'lib/real.'+targetExt;
      const config = {compilerOptions:{allowJs:true,jsx:'preserve',moduleResolution:'node',
        paths:{'@lib':['lib/real']}},include:literal?[]:['**/*']};
      if (literal) config.files = [caller];
      const payload = {'tsconfig.json':JSON.stringify(config),
        [caller]:"import { real } from '@lib';\nexport function run() { real(); }\n",
        [sibling]:'export const other = 0;\n',
        [target]:'export function real() { return 1; }\n'};
      for (const [file,body] of Object.entries(payload)) {
        const dest = path.join(dir,file); fs.mkdirSync(path.dirname(dest),{recursive:true});
        fs.writeFileSync(dest,body);
      }
      const parsed = ts.getParsedCommandLineOfConfigFile(path.join(dir,'tsconfig.json'),{},
        {...ts.sys,onUnRecoverableConfigFileDiagnostic:d=>{throw Error(ts.flattenDiagnosticMessageText(d.messageText,'\n'));}});
      assert(parsed && parsed.errors.length===0, name);
      const included = parsed.fileNames.includes(path.join(dir,caller));
      const exception = group[j]==='.d.ts' && ['.js','.jsx'].includes(group[i]);
      // TS's suffix stop at 43972 specially excludes .d.ts, but not .d.cts/.d.mts.
      // In TS's sorted wildcard walk, .cts precedes .d.cts, so both survive.
      // .mts follows .d.mts and removes it at 43985-43997. Prism barriers both.
      const compoundDeclaration = group[i]==='.d.cts' && group[j]==='.cts';
      if (included !== (literal || exception || compoundDeclaration)) errors.push(['TS membership',name,included]);
      cases.push({name,caller,sibling,target,literal,exception,included,compoundDeclaration,
        fileNames:parsed.fileNames.map(f=>path.relative(dir,f))});
      requests.push([name+'/'+caller,'@lib']);
    }
  }
}
const reqPath = path.join(root,'requests.json');
fs.writeFileSync(reqPath,JSON.stringify(requests));
const outputs = JSON.parse(execFileSync(path.resolve(driver),[root,reqPath],{encoding:'utf8'}));
assert.equal(outputs.length,cases.length);
for (let i=0;i<cases.length;i++) {
  const c=cases[i]; assert.deepEqual(outputs[i].slice(0,2),requests[i]);
  const expected = c.literal || c.exception ? c.name+'/'+c.target : null;
  if (outputs[i][2]!==expected) errors.push(['kernel membership',c.name,expected,outputs[i][2]]);
}
const result={claim:'MEASURED',compiler_sha256:hash,source_lines:'22530-22544;43966-43997',
  groups,cases:cases.length,dropped:cases.filter(c=>!c.included).length,
  literal_exemptions:cases.filter(c=>c.literal).length,
  declaration_js_exceptions:cases.filter(c=>!c.literal&&c.exception).length,
  conservative_declaration_barriers:cases.filter(c=>!c.literal&&c.included&&c.compoundDeclaration).length,
  errors,rows:cases};
fs.writeFileSync(receipt,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({...result,rows:undefined}));
assert.deepEqual(errors,[],'complete ordered-pair defect population');
