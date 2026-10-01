"""Generate finite P1 positive and Option-K controls in JSX and TSX grammars."""
import json,sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=True)
cases=[]
def add(name,ext,paths=None,opts=None,extra=None,spec='@lib',source=None,files=None,expected='gain',app=None):
 d=root/f'{name}-{ext}';d.mkdir(exist_ok=True)
 options={'moduleResolution':'node','allowJs':True,'paths':paths or {'@lib':['lib/real']}}
 options.update(opts or {})
 cfg={'compilerOptions':options,'include':['**/*']};cfg.update(extra or {})
 payload={'tsconfig.json':json.dumps(cfg),(app or f'app.{ext}'):source or f"import {{ real as picked }} from '{spec}';\nexport function run() {{ picked(); }}\n",f'lib/real.{ext}':'export function real() { return 1; }\n',f'decoy/real.{ext}':'export function real() { return 2; }\n'}
 payload.update(files or {})
 for p,s in payload.items():
  q=d/p;q.parent.mkdir(parents=True,exist_ok=True);q.write_text(s)
 cases.append({'case':d.name,'expectation':expected,'files':list(payload)})
for e in ['jsx','tsx']:
 add('C01-exact',e)
 add('C02-wildcard',e,{'@*':['lib/*']},spec='@real')
 add('C03-exact-wins',e,{'@lib':['lib/real'],'@*':['decoy/real']})
 add('C04-longest-prefix',e,{'@*':['decoy/real'],'@lib*':['lib/real']},spec='@libfoo')
 add('C05-wildcard-suffix',e,{'@lib/*end':['lib/*']},spec='@lib/realend')
 add('C06-tied-prefix',e,{'@*ib':['lib/real'],'@*lib':['decoy/real']},expected='base')
 add('C07-empty-capture',e,{'@lib*':['lib/*']},expected='base')
 add('C08-child-inherits-paths',e,files={'tsconfig.json':json.dumps({'compilerOptions':{'moduleResolution':'node','allowJs':True,'paths':{'@lib':['lib/real']}},'include':[]}),f'pkg/app.{e}':"import { real as picked } from '@lib';\nexport function run2() { picked(); }\n",'pkg/tsconfig.json':'{"extends":"../tsconfig.json","include":["**/*"]}'},expected='mixed')
 add('C09-baseurl-origin',e,opts={'baseUrl':'lib'},files={'tsconfig.json':'{"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":"lib"},"include":[]}',f'pkg/app.{e}':"import { real as picked } from '@lib';\nexport function run2() { picked(); }\n",'pkg/tsconfig.json':'{"extends":"../tsconfig.json","compilerOptions":{"paths":{"@lib":["real"]}},"include":["**/*"]}'},expected='mixed')
 add('C10-files-override-exclude',e,extra={'files':[f'app.{e}'],'include':[],'exclude':['**/*']})
 add('C11-excluded-file',e,extra={'include':['lib/**/*']},expected='base')
 add('C12-jsonc',e,files={'tsconfig.json':'{// c\n"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@\\u006cib":["lib/real"],},},"include":["**/*"],}'})
 add('C13-duplicate-key',e,files={'tsconfig.json':'{"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@lib":["lib/real"],"@lib":["decoy/real"]}},"include":["**/*"]}'},expected='base')
 add('C14-cycle',e,files={'tsconfig.json':'{"extends":"./tsconfig.json"}'},expected='base')
 add('C15-package-extends',e,files={'tsconfig.json':'{"extends":"@scope/config"}'},expected='base')
 add('C16-fallback-array',e,{'@lib':['missing','lib/real']},expected='base')
 add('C17-nodenext',e,opts={'moduleResolution':'NodeNext'},expected='base')
 add('C18-baseurl-bare',e,opts={'baseUrl':'.','paths':{}},spec='lib/real',expected='base')
 add('C19-js-to-ts',e,{'@lib':['lib/mapped.js']},files={'lib/mapped.ts':'export function real() { return 1; }\n'},expected='base')
 add('C20-index',e,{'@lib':['lib/indexed']},files={f'lib/indexed/index.{e}':'export function real() { return 1; }\n'})
 add('C21-declaration-blocker',e,files={'lib/real.d.ts':'export declare function real(): number;\n'},expected='base')
 add('C22-extension-competition',e,files={'lib/real.js':'export function real() { return 3; }\n'},expected='base')
 add('C23-package-manifest',e,files={'lib/real/package.json':'{"main":"index.js"}'},expected='base')
 add('C24-relative-barrel',e,{'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export { real } from './real';\n"})
 add('C25-default-member',e,source="import picked from '@lib';\nexport function run() { picked(); }\n",files={f'lib/real.{e}':'export function real() { return 1; }\nexport default real;\n'})
 add('C26-nonexported',e,files={f'lib/real.{e}':'function real() { return 1; }\n'},expected='base')
 add('C27-shadow-parameter',e,source="import { real as picked } from '@lib';\nexport function run(picked) { picked(); }\n",expected='base')
 add('C28-namespace',e,source="import * as Lib from '@lib';\nexport function run() { Lib.real(); }\n",expected='base')
 add('C29-relative-preservation',e,source="import { real as picked } from './lib/real';\nexport function run() { picked(); }\n",expected='base')
 add('C30-escape',e,{'@lib':['../../lib/real']},expected='base')
 add('C31-star-not-suffix-literal',e,{'@lib*':['lib/*']},spec='@lib/real')
 add('C32-file-index-competition',e,files={f'lib/real/index.{e}':'export function real() { return 3; }\n'},expected='base')
 add('C33-excluded-app',e,extra={'exclude':[f'app.{e}']},expected='base')
 add('C34-require',e,source="const { real: picked } = require('@lib');\nexport function run() { picked(); }\n",expected='base')
 add('C35-require-with-esm',e,source="import { real as other } from '@lib';\nconst { real: picked } = require('@lib');\nexport function run() { picked(); }\n",expected='base')
 add('C36-unmatched-target-star',e,{'@lib':['lib/*']},files={f'lib/index.{e}':'export function real() { return 3; }\n'},expected='base')
 wrapped={f'lib/real.{e}':"import { memo } from 'react';\nexport const real = memo(() => <div/>);\n"}
 add('C37-wrapped-jsx',e,source="import { real as Picked } from '@lib';\nexport function run() { return <Picked/>; }\n",files=wrapped)
 add('C38-wrapped-normal-call',e,source="import { real as Picked } from '@lib';\nexport function run() { Picked({}); }\n",files=wrapped,expected='refusal')
 # R1 membership, delegated-project, barrel and finite-probe regressions.
 near=lambda **kw: json.dumps({'compilerOptions':{'moduleResolution':'node','allowJs':True,'paths':{'@lib':['../decoy/real']}},'include':['**/*'],**kw})
 for label,pattern,folder in [('C39-exclude-parent-glob','src/*','src/a'),('C40-exclude-dotted-parent','src/v1.legacy','src/v1.legacy')]:
  add(label,e,app=f'pkg/{folder}/app.{e}',files={'pkg/tsconfig.json':near(exclude=[pattern])})
 for suffix in ['mjs','cjs']:
  add('C41-allowjs-off-'+suffix,e,app=f'pkg/app.{suffix}',files={'pkg/tsconfig.json':json.dumps({'compilerOptions':{'moduleResolution':'node','paths':{'@lib':['../decoy/real']}},'include':['**/*']})})
 for suffix in ['js','jsx']:
  add('C42-same-stem-'+suffix,e,app=f'pkg/app.{suffix}',files={'pkg/tsconfig.json':near(), 'pkg/app.ts' if suffix=='js' else 'pkg/app.tsx':'export const other = 0;'},expected='base')
 for folder in ['bower_components','jspm_packages']:
  add('C43-package-folder-'+folder,e,app=f'{folder}/x/app.{e}',expected='base')
 for reverse in [False,True]:
  stars=["export * from '@ext';", "export * from './a';"]
  if reverse: stars.reverse()
  add('C44-skipped-star-'+str(int(reverse)),e,paths={'@lib':['lib/barrel'],'@ext':['decoy/real']},files={f'lib/barrel.{e}':'\n'.join(stars)+'\n',f'lib/a.{e}':'export function real() { return 1; }\n',f'rel.{e}':"import { real as picked } from './lib/barrel';\nexport function rel() { picked(); }\n"},expected='base')
 add('C45-jsconfig-barrier',e,app=f'pkg/app.{e}',files={'pkg/jsconfig.json':near()},expected='base')
 add('C46-solution-barrier',e,app=f'pkg/app.{e}',files={'pkg/tsconfig.json':'{"files":[],"references":[{"path":"./tsconfig.app.json"}]}','pkg/tsconfig.app.json':near()},expected='base')
 add('C47-allowjs-false',e,opts={'allowJs':False},expected='base' if e=='jsx' else 'gain')
 add('C48-outdir-barrier',e,opts={'outDir':'generated'},expected='base')
 add('C49-dotted-segment',e,paths={'@lib':['lib/user.service']},files={f'lib/user.service.{e}':'export function real() { return 1; }\n'})
 add('C50-known-extension-cut',e,paths={'@lib':['lib/user.js']},files={f'lib/user.js.{e}':'export function real() { return 1; }\n'},expected='base')
 add('C51-leading-space-relative',e,source="import { real as picked } from ' ./lib/real';\nexport function run() { picked(); }\n",expected='base')
 add('C52-exclude-nonmatch',e,extra={'exclude':['**/*.test.*']})
 add('C53-files-priority-exempt',e,app='pkg/app.js',extra={'files':['pkg/app.js'],'include':[]},files={'pkg/app.ts':'export const other = 0;'})
 add('C54-files-empty-barrier',e,app=f'pkg/app.{e}',files={'pkg/tsconfig.json':'{"files":[]}'},expected='base')
 add('C55-nested-opaque-star',e,paths={'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export * from './nested';\nexport * from './real';\n",f'lib/nested.{e}':"export * from '@ext';\n"},expected='base')
 add('C56-proven-empty-star',e,paths={'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export * from './types';\nexport * from './real';\n",'lib/types.ts':'export type Empty = number;\n'})
 add('C57-opaque-syntax-star',e,paths={'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export * from './types';\nexport * from './real';\n",f'lib/types.{e}':'export type = ;\n'},expected='base')
 add('C58-leading-space-relative-cjs',e,source="import { real as picked } from ' ./lib/cjs';\nexport function run() { picked(); }\n",files={'lib/cjs.js':'function real() { return 1; }\nmodule.exports.real = real;\n'},expected='base')
 add('C59-opaque-cjs-star',e,paths={'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export * from './opaque';\nexport * from './real';\n",'lib/opaque.js':'module.exports = buildExports();\n',f'rel.{e}':"import { real as picked } from './lib/barrel';\nexport function rel() { picked(); }\n"},expected='base')
 add('C60-skipped-noncall-conflict',e,paths={'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export * from './opaque';\nexport * from './real';\n",f'lib/opaque.{e}':'export const real = 1;\n'},expected='base')
 add('C61-known-noncall-nonconflict',e,paths={'@lib':['lib/barrel']},files={f'lib/barrel.{e}':"export * from './opaque';\nexport * from './real';\n",f'lib/opaque.{e}':'export const other = 1;\n'})
 add('C62-raw-alias-no-borrow',e,source="import { real as other } from '@lib';\nimport { real as picked } from ' @lib';\nexport function run() { picked(); }\n",expected='base')
 add('C63-raw-alias-exact-key',e,paths={' @lib':['lib/real'],'@lib':['decoy/real']},source="import { real as other } from '@lib';\nimport { real as picked } from ' @lib';\nexport function run() { picked(); }\n")
 # Membership doubts are barriers: no guessed fall-through to an ancestor.
 for folder in ['bower_components','jspm_packages']:
  app=f'pkg/{folder}/x/app.{e}'
  add('C64-literal-package-'+folder,e,app=app,extra={'files':[app],'include':[]},files={'pkg/tsconfig.json':near(include=[folder+'/**/*'])},expected='base')
 app='pkg/src/app.d.ts'
 add('C65-declaration-priority',e,app=app,extra={'files':[app],'include':[]},files={'pkg/tsconfig.json':near(include=['src/**/*']),'pkg/src/app.ts':'export const other = 0;'},expected='base')
 app=f'pkg/src/app花.{e}'
 add('C66-unicode-glob-barrier',e,app=app,extra={'files':[app],'include':[]},files={'pkg/tsconfig.json':near(include=[f'src/app?.{e}'])},expected='base')
 for label,kwargs in [('C67-case-include',{'include':['SRC/**/*']}),('C68-case-exclude',{'exclude':['SRC/*']})]:
  app=f'pkg/src/app.{e}'
  add(label,e,app=app,extra={'files':[app],'include':[]},files={'pkg/tsconfig.json':near(**kwargs)},expected='base')
 add('C69-case-config-name',e,app=f'pkg/app.{e}',files={'pkg/TSCONFIG.JSON':near()},expected='base')
 add('C70-unicode-target',e,paths={'@lib':['lib/花']},files={f'lib/花.{e}':'export function real() { return 1; }\n'})
 app=f'app花.{e}'
 add('C71-unicode-explicit-file',e,app=app,extra={'files':[app],'include':[]})
(root/'manifest.json').write_text(json.dumps(cases,indent=2))
print('scenarios',len(cases))
