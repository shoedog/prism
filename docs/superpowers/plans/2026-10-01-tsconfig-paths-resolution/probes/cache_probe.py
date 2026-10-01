"""Cross-binary and config-only cache invalidation with concrete caller artifacts."""
import json,subprocess,sys,shutil
from pathlib import Path
base,head=map(lambda p:str(Path(p).resolve()),sys.argv[1:3]);out=Path(sys.argv[3]).resolve();out.mkdir(parents=True,exist_ok=True)
root=out/'repo';cache=out/'cache';root.mkdir(exist_ok=True)
for name in ['left','right']:
 (root/name).mkdir(exist_ok=True);(root/name/'util.tsx').write_text('export function real() { return 1; }\n')
(root/'app.tsx').write_text("import { real as picked } from '@lib';\nexport function run() { picked(); }\n")
def cfg(target):
 (root/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'moduleResolution':'node','paths':{'@lib':[target]}},'include':['**/*']}))
def query(label,binary,target,cached=True):
 args=[binary,'nav',*(('--cache-dir',str(cache)) if cached else ('--no-cache',)),'callers','--repo',str(root),'--symbol','real','--file',target+'/util.tsx','--format','json']
 p=subprocess.run(args,capture_output=True,text=True);(out/(label+'.stderr')).write_text(p.stderr);(out/(label+'.json')).write_text(p.stdout)
 assert p.returncode==0,(label,p.stderr)
 data=json.loads(p.stdout);assert isinstance(data,dict),(label,data)
 return data
cfg('left/util');results={}
results['base_cache']=query('base_cache',base,'left')
results['head_cold']=query('head_cold',head,'left')
results['head_hit']=query('head_hit',head,'left')
results['head_nocache']=query('head_nocache',head,'left',False)
assert results['head_cold']==results['head_hit']==results['head_nocache']
assert results['base_cache']!=results['head_cold'],'cross-binary RED did not discriminate'
cfg('right/util')
results['config_changed_new']=query('config_changed_new',head,'right')
results['config_changed_old']=query('config_changed_old',head,'left')
results['changed_nocache']=query('changed_nocache',head,'right',False)
assert results['config_changed_new']==results['changed_nocache']
# Read concrete function identities, not process status or stderr cache labels.
def functions(r):
 return [x['symbol']['Function'] for x in r.get('items',[]) if isinstance(x.get('symbol'),dict) and 'Function' in x['symbol']]
assert any(x['file']=='app.tsx' and x['name']=='run' for x in functions(results['head_hit']))
assert any(x['file']=='app.tsx' and x['name']=='run' for x in functions(results['config_changed_new']))
assert not functions(results['config_changed_old'])
(out/'summary.json').write_text(json.dumps({'claim':'MEASURED','cross_binary_red':True,'cold_full_hit_sidecar_nocache_equal':True,'config_only_change_moves_caller_to_right':True},indent=2))
print((out/'summary.json').read_text())
