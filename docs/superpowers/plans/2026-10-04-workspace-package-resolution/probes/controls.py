"""Generate retained public-only package controls, no installs or Git writes."""
import json, sys
from pathlib import Path

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=False)
cases = [
 ('index', 'node10', {}, '', 'index.ts'),
 ('types', 'node10', {'types':'other.ts'}, '', 'other.ts'),
 ('typings', 'node10', {'typings':'other.ts','types':'index.ts'}, '', 'other.ts'),
 ('main', 'node10', {'main':'other.ts'}, '', 'other.ts'),
 ('module-ignored', 'node10', {'module':'other.ts'}, '', 'index.ts'),
 ('missing-types-index', 'node10', {'types':'dist/missing.d.ts','main':'other.ts'}, '', 'index.ts'),
 ('exports-ignored-node10', 'node10', {'exports':'./other.ts'}, '', 'index.ts'),
 ('types-first', 'bundler', {'exports':{'types':'./index.ts','default':'./other.ts'}}, '', 'index.ts'),
 ('default-first', 'bundler', {'exports':{'default':'./other.ts','types':'./index.ts'}}, '', 'other.ts'),
 ('require', 'node16', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'index.ts'),
 ('nodenext', 'nodenext', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'index.ts'),
 ('pattern', 'bundler', {'exports':{'./*':'./other.ts','./feature/*':'./*.ts'}}, '/feature/index', 'index.ts'),
 ('exact-null', 'bundler', {'exports':{'./feature/index':None,'./feature/*':'./*.ts'}}, '/feature/index', None),
 ('missing-export', 'bundler', {'exports':{'types':'./missing.d.ts','default':'./index.ts'}}, '', 'index.ts'),
 ('nested-missing', 'bundler', {'exports':{'types':{'default':'./missing.d.ts'},'default':'./index.ts'}}, '', 'index.ts'),
 ('escape', 'bundler', {'exports':'../lib/index.ts'}, '', None),
 ('declaration', 'bundler', {'exports':'./index.d.ts'}, '', 'index.d.ts'),
 ('uninstalled', 'node10', {'types':'index.ts'}, '', None),
 ('js-secondary', 'node10', {'main':'js.js'}, '', None),
 ('bundler-commonjs', 'bundler', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'index.ts'),
 ('bundler-preserve', 'bundler', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'other.ts'),
 ('bundler-amd-declined', 'bundler', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'other.ts'),
 ('bundler-default-declined', 'bundler', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'index.ts'),
 ('node16-esm', 'node16', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'other.ts'),
 ('nodenext-esm', 'nodenext', {'exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'other.ts'),
 ('self-reference', 'node16', {'type':'module','exports':{'import':'./other.ts','require':'./index.ts'}}, '', 'other.ts'),
 ('self-reference-legacy', 'node10', {'type':'module','types':'index.ts'}, '', None),
 ('custom', 'bundler', {'exports':{'development':'./other.ts','default':'./index.ts'}}, '', 'other.ts'),
 ('custom-no-match', 'bundler', {'exports':{'development':'./other.ts','default':'./index.ts'}}, '', 'index.ts'),
]
manifest = []
for grammar in ('tsx','jsx'):
 for label, mode, meta, sub, expected in cases:
  name=f'{label}-{grammar}';root=out/name
  def write(p,s):
   f=root/p; f.parent.mkdir(parents=True,exist_ok=True);f.write_text(s)
  spec='@ws/lib'+sub
  options={'moduleResolution':mode,'module':mode if mode in ('node16','nodenext') else 'esnext','allowJs':True}
  if label.startswith('bundler-'):options['module']=label.split('-')[1]
  if label=='bundler-default-declined':options.pop('module')
  if label in ('custom','custom-no-match'):options['customConditions']=['development'] if label=='custom' else []
  write('tsconfig.json',json.dumps({'compilerOptions':options,'include':['**/*']}))
  write('package.json',json.dumps({'private':True,'workspaces':['packages/*'],**({'type':'module'} if label in ('node16-esm','nodenext-esm') else {})}))
  write('packages/lib/package.json',json.dumps({'name':'@ws/lib',**meta}))
  writer=f'packages/lib/app.{grammar}' if label.startswith('self-reference') else f'app.{grammar}'
  write(writer,f"import {{real as picked}} from '{spec}'; export function run(){{picked();}}\n")
  for p in ('index.ts','other.ts'):write('packages/lib/'+p,'export function real(){return 1;}\n')
  if label=='declaration':write('packages/lib/index.d.ts','export declare function real():void;\n')
  if label=='js-secondary':
   for p in ('index.ts','other.ts'):(root/'packages/lib'/p).unlink()
   write('packages/lib/js.js','export function real(){return 1;}\n')
   # Native JS is a real resolution but prototype deliberately does not admit.
   expected='js.js'
  if label!='uninstalled' and not label.startswith('self-reference'):
   (root/'node_modules/@ws').mkdir(parents=True);(root/'node_modules/@ws/lib').symlink_to('../../packages/lib',target_is_directory=True)
  manifest.append({'name':name,'root':str(root.resolve()),'writer':writer,'specifier':spec,'native_target':None if expected is None else 'packages/lib/'+expected,'admit':expected is not None and label not in ('js-secondary','declaration','bundler-amd-declined','bundler-default-declined')})
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'controls':len(manifest)}))
