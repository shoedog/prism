"""Generate a lean, reproducible TS 5.9.3 package-resolution matrix; no installs.
All four mode, seven writer extension, three package type, fifteen entry,
and four link axes are crossed (5,040 cases), plus R1 reviewer witnesses.
"""
import argparse, itertools, json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('out',type=Path);a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=False)
fn='export function real(){return 1;}\n'
manifest=[]
def emit(c):
 root=a.out/c['id'];root.mkdir(parents=True)
 def write(name,value):
  q=root/name;q.parent.mkdir(parents=True,exist_ok=True);q.write_text(value if isinstance(value,str) else json.dumps(value))
 mode=c['mode'];opts={'moduleResolution':mode,'module':mode if mode in ('node16','nodenext') else 'esnext','allowJs':True,'noLib':True,'jsx':'react-jsx',**c.get('options',{})}
 if c.get('nojsx'):opts.pop('jsx')
 writer=c.get('writer','app.tsx');spec=c.get('spec','@ws/lib');link=c.get('link','npm')
 if link=='self':writer='packages/lib/src/'+Path(writer).name
 elif link=='pnpm':writer='packages/app/src/'+Path(writer).name
 write('tsconfig.json',{'compilerOptions':opts,'include':c.get('include',['**/*'])})
 if not c.get('outer'):
  write('package.json',{'private':True,'workspaces':({'packages':['packages/*']} if link=='yarn' else ['packages/*']),**c.get('rootmeta',{})})
 else:
  # Isolated parent outside the prism root, never an uncontrolled common parent.
  outer=a.out/(c['id']+'-outer');outer.mkdir();write('package.json',{'private':True})
  (root/'package.json').unlink();(outer/'package.json').write_text(json.dumps(c.get('outermeta',{'type':'module'})))
  root.rename(outer/'repo');root=outer/'repo'
 write('packages/lib/package.json',{'name':'@ws/lib',**c.get('meta',{})})
 write(writer,f"import {{real as picked}} from '{spec}'; export function run(){{picked();}}\n")
 for name,value in c.get('files',{}).items():write('packages/lib/'+name,value)
 for name,value in c.get('rootfiles',{}).items():write(name,value)
 lexical=root/('packages/app/node_modules/@ws/lib' if link=='pnpm' else 'node_modules/@ws/lib')
 if link!='self' or c.get('selflink'):
  lexical.parent.mkdir(parents=True,exist_ok=True);lexical.symlink_to('../../../lib' if link=='pnpm' else '../../packages/lib',target_is_directory=True)
 if c.get('inner'):
  folder=Path(writer).parent/'node_modules/@ws/lib'
  write(str(folder/'package.json'),{'name':'@ws/lib',**c['inner']['meta']})
  for name,value in c['inner'].get('files',{}).items():write(str(folder/name),value)
 if spec=='#alias':
  (root/'node_modules').mkdir(exist_ok=True);(root/'node_modules/#alias').symlink_to('../packages/lib',target_is_directory=True)
 record={'id':c['id'],'root':str(root.resolve()),'writer':writer,'specifier':spec,'feature':c.get('feature',c.get('shape',c['id']))}
 if 'expected' in c:record['expected']=c['expected']
 manifest.append(record)
shapes={
 'exports-string':({'exports':'./index.ts'},{'index.ts':fn},''),
 'exports-conditions':({'exports':{'import':'./other.ts','require':'./index.ts'}},{'index.ts':fn,'other.ts':fn},''),
 'exports-subpath':({'exports':{'./feature':'./feature.js'}},{'feature.ts':fn},'/feature'),
 'exports-pattern':({'exports':{'./*':'./*.ts'}},{'feature.ts':fn},'/feature'),
 'exports-null':({'exports':None,'types':'index.ts'},{'index.ts':fn},''),
 'exports-array':({'exports':['./index.ts']},{'index.ts':fn},''),
 'legacy-types':({'types':'src/index.d.ts'},{'src/index.ts':fn,'index.ts':fn},''),
 'legacy-main':({'main':'src/index'},{'src/index.ts':fn,'index.ts':fn},''),
 'legacy-module':({'module':'other.ts'},{'index.ts':fn,'other.ts':fn},''),
 'missing-target':({'exports':'./dist/index.ts'},{'index.ts':fn},''),
 'index-fallback':({},{'index.ts':fn},''),
 'ts-sibling-only':({'exports':{'types':'./a.ts','default':'./index.ts'}},{'a.tsx':fn,'index.ts':fn},''),
 'literal-star':({'exports':'./index*.ts'},{'index.ts':fn},''),
 'typesversions-subpath':({'typesVersions':{'*':{'feature':['other.ts']}}},{'feature.ts':fn,'other.ts':fn},'/feature'),
 'declaration':({'exports':{'types':'./index.d.ts','default':'./index.ts'}},{'index.d.ts':'export declare function real():void;\n','index.ts':fn},''),
}
for mode,ext,kind,shape,link in itertools.product(['node10','node16','nodenext','bundler'],['ts','tsx','mts','cts','js','mjs','cjs'],['absent','module','commonjs'],shapes,['npm','yarn','pnpm','self']):
 meta,files,sub=shapes[shape];ptype={} if kind=='absent' else {'type':kind}
 emit({'id':f'M-{mode}-{ext}-{kind}-{shape}-{link}','mode':mode,'writer':'app.'+ext,'rootmeta':ptype,'meta':{**meta,**ptype},'files':files,'spec':'@ws/lib'+sub,'link':link,'shape':shape})
matrix_count=len(manifest)
# Preserve every sol probe input and its independent expected native result.
for c in json.loads((Path(__file__).parent/'reviewer-cases.json').read_text()):
 emit({**c,'id':'sol-'+c['id'],'nojsx':True})
# All Opus appendix witnesses. Its published appendix has no definitions for
# C3/C15/C17-C21/C23/C24/C26; retain coverage labels without inventing repros.
extra=[
 dict(id='C1',mode='bundler',meta={'exports':{'types':'./a.ts','default':'./b.ts'}},files={'a.tsx':fn,'b.ts':fn}),
 dict(id='C2',mode='node10',meta={'types':'src/index.d.ts'},files={'src/index.ts':fn,'index.ts':fn}),
 dict(id='C4',mode='nodenext',rootmeta={'type':'module'},meta={'type':'module','types':'src/index'},files={'src/index.ts':fn,'index.ts':fn}),
 dict(id='C5',mode='nodenext',writer='app.mjs',meta={'exports':{'import':'./other.ts','require':'./index.ts'}},files={'index.ts':fn,'other.ts':fn}),
 dict(id='C6',mode='bundler',writer='app.cjs',meta={'exports':{'import':'./other.ts','require':'./index.ts'}},files={'index.ts':fn,'other.ts':fn}),
 dict(id='C6b',mode='nodenext',rootmeta={'type':'module'},writer='app.cjs',meta={'exports':{'import':'./other.ts','require':'./index.ts'}},files={'index.ts':fn,'other.ts':fn}),
 dict(id='C7',mode='bundler',spec='#lib',rootmeta={'name':'app','imports':{'#lib':'./packages/lib/index.ts'}},files={'index.ts':fn}),
 dict(id='C8',mode='bundler',options={'paths':{'@ws/lib':['./missing']}},meta={'exports':'./index.ts'},files={'index.ts':fn}),
 dict(id='C9',mode='nodenext',rootmeta={'type':'module'},files={'index.ts':fn}),
 dict(id='C10',mode='bundler',meta={'exports':None,'types':'index.ts'},files={'index.ts':fn}),
 dict(id='C11',mode='bundler',meta={'exports':'./index.tsx'},files={'index.ts':fn}),
 dict(id='C12',mode='node10',spec='@ws/lib/feature',files={'feature.ts':fn}),
 dict(id='C13',mode='bundler',writer='apps/web/app.ts',meta={'exports':'./index.ts'},files={'index.ts':fn},inner={'meta':{'exports':{'./x':'./x.ts'}},'files':{}}),
 dict(id='C14',mode='bundler',link='pnpm',files={'index.ts':fn},meta={'exports':'./index.ts'}),
 dict(id='C16',mode='bundler',nojsx=True,include=['app.ts'],meta={'exports':'./index.tsx'},files={'index.tsx':fn}),
 dict(id='C22',mode='nodenext',outer=True,meta={'exports':{'import':'./other.ts','require':'./index.ts'}},files={'index.ts':fn,'other.ts':fn}),
 dict(id='C25',mode='bundler',options={'resolvePackageJsonExports':False},link='self',selflink=True,meta={'exports':'./other.ts','types':'index.ts'},files={'index.ts':fn,'other.ts':fn}),
 dict(id='C27',mode='bundler',meta={'exports':{'types':'./dist/index.d.ts','default':'./src/index.ts'}},files={'dist/index.d.ts':'export declare function real():void;\n','src/index.ts':fn}),
 dict(id='C29',mode='bundler',files={'index.ts':fn}),
 dict(id='C30',mode='bundler',meta={'main':'index.ts'},spec='@ws/lib/utils',files={'utils.ts':fn}),
]
extra += [dict(id='scheme-self',mode='bundler',spec='node:local',rootmeta={'name':'node:local','exports':'./packages/lib/index.ts'},files={'index.ts':fn}),dict(id='outer-self',mode='nodenext',outer=True,outermeta={'name':'@ws/lib','type':'module','exports':'./repo/packages/lib/other.ts'},meta={'exports':'./index.ts'},files={'index.ts':fn,'other.ts':fn})]
for c in extra:emit({**c,'id':'opus-'+c['id'],'writer':c.get('writer','app.ts')})
for mode in ['node10','bundler']:
 for spec,target in [('node:url','url'),('virtual:pwa','pwa')]:
  emit(dict(id=f'opus-C28-{mode}-{target}',mode=mode,writer='app.ts',spec=spec,options={'paths':{'node:url':['./shim/url.ts'],'virtual:*':['./shim/*.ts']}},rootfiles={'shim/url.ts':fn,'shim/pwa.ts':fn}))
(a.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'total':len(manifest),'cross_product':matrix_count,'reviewer_cases':len(manifest)-matrix_count,'manifest':str(a.out/'manifest.json')}))
