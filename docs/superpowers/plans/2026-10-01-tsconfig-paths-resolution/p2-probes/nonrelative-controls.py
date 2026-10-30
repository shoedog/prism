"""Public caller-program controls for the non-relative P2 slice; no private reads."""
import itertools, json, os, subprocess, sys
from pathlib import Path
from compare import load
packet = Path(__file__).resolve().parent
out = Path(sys.argv[2]).resolve(); assert not out.exists()
root = out/'source'; root.mkdir(parents=True)
p1 = Path.home()/'code/prism-paths-impl/target/repair-r5/head/prism'
facts = Path.home()/'code/prism-paths-impl/target/repair-r1/base/dump_imports'
head = Path(sys.argv[1]).resolve()
ts = Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
cases = []
def write(d, name, text):
 p=d/name; p.parent.mkdir(parents=True,exist_ok=True); p.write_text(text)
for grammar, member, shape in itertools.product(['jsx','tsx'],[False,True],['named','star','forward','two-alias','relative-alias','alias-relative','empty-star','missing-star','renamed','cjs','export-equals','bare-package','package-directory','declaration','competition','ambient','unsupported-extension','missing','forward-arrow','binding-write']):
 name=f'{grammar}-{shape}-{int(member)}'; d=root/name
 paths={'@lib':['entry.ts'],'@/x':['real'],'@/middle':['middle'],'@/empty':['empty']}
 entry="export { real } from '@/x';\n"; leaf='export function real() { return null; }\n'
 if shape=='star': entry="export * from '@/x';\n"
 if shape=='forward': entry="import { real } from '@/x'; export { real };\n"
 if shape in ['two-alias','relative-alias','alias-relative']:
  entry="export { real } from './middle.js';\n" if shape=='relative-alias' else "export { real } from '@/middle';\n"
  write(d,'middle.js',"export { real } from './real';\n" if shape=='alias-relative' else "export { real } from '@/x';\n")
 if shape=='empty-star':
  entry="export * from '@/empty'; export * from '@/x';\n"
  write(d,'empty.js','export function unrelated() { return null; }\n')
 if shape=='missing-star':
  entry="export * from '@/empty'; export * from '@/x';\n"
 if shape=='renamed': entry="export { leaf as real } from '@/x';\n"; leaf='export function leaf() { return null; }\n'
 if shape=='cjs': leaf='function real() { return null; }\nexports.real = real;\n'
 if shape=='export-equals': leaf='function real() { return null; }\nexport = real;\n'; paths['@/x']=['real.ts']; entry="export { default as real } from '@/x';\n"
 if shape=='bare-package': entry="export { real } from 'pkg';\n"; write(d,'node_modules/pkg/index.js',leaf)
 if shape=='package-directory': write(d,'real/package.json','{"main":"../real.js"}\n')
 if shape=='declaration': write(d,'real.d.ts','export declare function real(): unknown;\n')
 if shape=='competition': write(d,'real/index.js',leaf)
 if shape=='ambient':
  paths['@blocked']=['real']; entry="export { real } from '@blocked';\n"
  write(d,'ambient.d.ts',"declare module '@blocked' { export function real(): unknown; }\n")
 if shape=='unsupported-extension': paths['@/x']=['real.js']
 if shape=='missing': paths['@/x']=['absent']
 if shape=='forward-arrow': entry="import { real } from '@/x'; export { real };\n"; leaf='export const real = () => null;\n'
 if member and shape not in ['cjs','export-equals']: leaf+=("leaf" if shape=='renamed' else 'real')+".displayName = 'Real';\n"
 if shape=='binding-write': leaf+='real = () => null;\n'
 write(d,'tsconfig.json',json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'allowSyntheticDefaultImports':True,'paths':paths},'include':['**/*']})+'\n')
 write(d,'entry.ts',entry); write(d,'real.ts' if shape=='export-equals' else 'real.js',leaf)
 write(d,'app.'+grammar,"import { real as Real } from '@lib';\nexport function run() { Real(); return <Real />; }\n")
 cases.append({'case':name,'shape':shape,'gain':shape in ['named','star','forward','two-alias','relative-alias','alias-relative','empty-star','renamed']})
for grammar in ['jsx','tsx']:
 d=root/('caller-options-'+grammar)
 for project in ['a','b']:
  write(d,project+'/tsconfig.json',json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'baseUrl':'.','paths':{'@lib':['../shared/entry.ts'],'@/x':['real']}},'include':['**/*']}))
  write(d,project+'/real.js','export function real() { return null; }\n')
  write(d,project+'/app.'+grammar,"import { real as Real } from '@lib';\nexport function run() { Real(); return <Real />; }\n")
 write(d,'shared/tsconfig.json',json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'paths':{'@/x':['decoy']}}}))
 write(d,'shared/entry.ts',"export { real } from '@/x';\n"); write(d,'shared/decoy.js','export function real() { return null; }\n')
 cases.append({'case':d.name,'shape':'caller-options','gain':True})
(out/'manifest.json').write_text(json.dumps(cases,indent=2)+'\n')
env=dict(os.environ,CORPUS_F_ROOT=str(root),PRIVATE_EVIDENCE_ROOT=str(out/'evidence'),TS_JS=str(ts))
with (out/'aggregate.json').open('w') as f:
 subprocess.run(['bash',str(packet/'CONTROLLER-p2.sh'),str(p1),str(facts),str(head)],env=env,stdout=f,check=True)
a=load(out/'evidence/p1-sites.jsonl'); b=load(out/'evidence/p2-sites.jsonl'); changes=0
for c in cases:
 ks=[k for k in a if k[0].split('/')[0]==c['case']]; assert len(ks)==(4 if c['shape']=='caller-options' else 2),(c,len(ks))
 for k in ks:
  assert (a[k]!=b[k])==c['gain'],(c,k,a[k],b[k]); changes+=a[k]!=b[k]
agg=json.loads((out/'aggregate.json').read_text()); assert agg['prototype']['classes']=={'CORRECT_STATIC_BINDING':changes}
assert str(root) not in (out/'aggregate.json').read_text()
r={'scenarios':len(cases),'sites':len(a),'new_correct':changes,'preserved':len(a)-changes,'classes':agg['prototype']['classes']}
(out/'summary.json').write_text(json.dumps(r,indent=2)+'\n'); print(json.dumps(r))
