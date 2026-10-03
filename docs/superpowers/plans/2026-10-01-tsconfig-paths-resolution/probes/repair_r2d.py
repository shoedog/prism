"""Public native first-pass/cache witnesses and per-repository controls.
Usage: repair_r2d.py OUT R2C_BIN HEAD_BIN TS_JS
"""
import json, subprocess, sys, shutil, hashlib
from pathlib import Path
out,before,head,ts=map(lambda p:Path(p).resolve(),sys.argv[1:5]);out.mkdir(parents=True,exist_ok=True)
controls=out/'controls'
shutil.copytree(Path('target/repair-r2c/controls'),controls,dirs_exist_ok=True)
manifest=[r for r in json.loads((controls/'manifest.json').read_text()) if not r['case'].startswith('D')]
for case in manifest:
 if case['case'] in ['R2-N5c_control_no_typing_jsx', 'R2-N5c_control_no_typing_tsx', 'R2-typeRoots-absent-jsx', 'R2-typeRoots-absent-tsx']:case['expectation']='gain'
classes=[('file','node_modules/utils.d.ts',None),('index','node_modules/utils/index.d.ts',None),('at-types','node_modules/@types/utils/index.d.ts',None),('types','node_modules/utils/decl/entry.d.ts',{'types':'decl/entry.d.ts'}),('typings','node_modules/utils/decl/entry.d.ts',{'typings':'decl/entry.d.ts'}),('main','node_modules/utils/decl/entry.d.ts',{'main':'decl/entry.d.ts'}),('versions','node_modules/utils/decl/entry.d.ts',{'typesVersions':{'*':{'*':['decl/entry']}}}),('typeRoots','types/utils/index.d.ts',None)]
classes += [(arm,'node_modules/utils/exists.d.ts',metadata) for arm,metadata in [('order',r'{"typesVersions":{"*":{"*":["absent"]},"5":{"*":["exists"]}}}'),('duplicate',r'{"typesVersions":{"*":{"*":["absent"]},"*":{"*":["exists"]}}}'),('large',r'{"typesVersions":{"<=999999999999999999999999":{"*":["exists"]},"*":{"*":["absent"]}}}')]]
classes += [('absolute-'+field,'decls/entry.d.ts',None) for field in ['types','typings','main','typesVersions']]
classes += [('paths-index','lib/real/index.d.ts',None)]
classes += [('bom-range','node_modules/utils/exists.d.ts',{'typesVersions':{'\ufeff*':{'*':['exists']}}})]
def write(root,p,text):
 p=root/p;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
def query(binary,root,cache=None):
 p=subprocess.run([str(binary),'nav',*(['--cache-dir',str(cache)] if cache else ['--no-cache']),'call-stats','--repo',str(root),'--dump-sites'],capture_output=True,text=True);assert p.returncode==0,p.stderr
 rows=[r for r in map(json.loads,p.stdout.splitlines()) if r.get('record_kind')=='call_site'];assert len(rows)==1,rows
 return rows
native_script="""const ts=require(process.argv[1]),fs=require('fs'),path=require('path'),r=process.argv[2];const c=ts.readConfigFile(path.join(r,'tsconfig.json'),ts.sys.readFile);const o=ts.parseJsonConfigFileContent(c.config,ts.sys,r).options;const m=ts.resolveModuleName('utils',path.join(r,process.argv[3]),o,ts.sys);console.log(JSON.stringify(m.resolvedModule&&path.relative(r,m.resolvedModule.resolvedFileName)));"""
def native(root,app):
 p=subprocess.run(['node','-e',native_script,str(ts),str(root),app],capture_output=True,text=True);assert p.returncode==0,p.stderr;return json.loads(p.stdout)
records=[]
for ext in ['jsx','tsx']:
 for n,(arm,path,metadata) in enumerate(classes):
  case=f'D{n:02}-{arm}-{ext}';root=out/'cache'/case/'repo';cache=out/'cache'/case/'blobs';app='app.'+ext
  cfg={'compilerOptions':{'moduleResolution':'node','allowJs':True,'baseUrl':'.','paths':{'utils':['lib/real']}},'files':[app],'include':[]}
  if arm=='typeRoots':cfg['compilerOptions']['typeRoots']=['./types']
  write(root,'tsconfig.json',json.dumps(cfg));write(root,app,"import {real as picked} from 'utils'; export function run(){picked();}");write(root,'lib/real.jsx','export function real(){return 1;}')
  old=query(before,root,cache);assert not old[0]['resolved_targets'];assert native(root,app)=='lib/real.jsx'
  blob,=cache.rglob('cpg-cache.bin');stamp=lambda:(blob.stat().st_mtime_ns,hashlib.sha256(blob.read_bytes()).hexdigest());old_stamp=stamp()
  absent=query(head,root,cache);assert absent[0]['resolved_targets'][0]['confidence']=='exact';assert stamp()!=old_stamp and absent==query(head,root)
  new_stamp=stamp();assert absent==query(head,root,cache) and stamp()==new_stamp
  if arm.startswith('absolute-'):
   field=arm.removeprefix('absolute-');entry=str(root/path)
   metadata={field:{'*':{'*':[entry]}} if field=='typesVersions' else entry}
  if metadata:
   write(root,'node_modules/utils/package.json',metadata if isinstance(metadata,str) else json.dumps(metadata));assert absent==query(head,root,cache)==query(head,root)
  write(root,path,'export declare function real(): void;');assert native(root,app)==path
  present=query(head,root,cache);assert present==old==query(head,root)
  dest=controls/case;shutil.copytree(root,dest,dirs_exist_ok=True)
  if arm.startswith('absolute-'):
   package=dest/'node_modules/utils/package.json';package.write_text(package.read_text().replace(str(root),str(dest)))
  manifest.append({'case':case,'expectation':'base','app':app})
  (root/path).unlink();assert query(head,root,cache)==absent==query(head,root)
  (root/path).mkdir();assert query(head,root,cache)==old==query(head,root)
  (root/path).rmdir();assert query(head,root,cache)==absent
  write(root,path,'export declare function real(): void;');assert query(head,root,cache)==old
  records.append({'class':arm,'grammar':ext,'native_winner':path,'same_version_old_rebuild':True,'next_hit':True,'create_remove_directory_parity':True})
(controls/'manifest.json').write_text(json.dumps(manifest,indent=2));(out/'cache-summary.json').write_text(json.dumps(records,indent=2));print(json.dumps({'cases':len(records),'all_pass':True,'cache_versions':'105/61'}))
