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
cache_bins=list(cache.rglob('cpg-cache.bin'));assert len(cache_bins)==1,cache_bins
cache_bin=cache_bins[0]
cache_before=(cache_bin.stat().st_mtime_ns,__import__('hashlib').sha256(cache_bin.read_bytes()).hexdigest())
(root/'unrelated.txt').write_text('irrelevant')
results['text_added_hit']=query('text_added_hit',head,'left')
assert results['text_added_hit']==results['head_hit']
assert 'rebuilding' not in (out/'text_added_hit.stderr').read_text().lower()
assert cache_before==(cache_bin.stat().st_mtime_ns,__import__('hashlib').sha256(cache_bin.read_bytes()).hexdigest()), 'cache was rebuilt'
(root/'unrelated.txt').unlink()
results['text_removed_hit']=query('text_removed_hit',head,'left')
assert results['text_removed_hit']==results['head_hit']
assert 'rebuilding' not in (out/'text_removed_hit.stderr').read_text().lower()
assert cache_before==(cache_bin.stat().st_mtime_ns,__import__('hashlib').sha256(cache_bin.read_bytes()).hexdigest())
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
# Candidate occupancy addition/removal must agree with fresh construction.
blocker=root/'right/util.d.ts';blocker.write_text('export declare function real(): number;')
results['candidate_added']=query('candidate_added',head,'right')
results['candidate_added_nocache']=query('candidate_added_nocache',head,'right',False)
assert results['candidate_added']==results['candidate_added_nocache']
assert not functions(results['candidate_added'])
blocker.unlink()
results['candidate_removed']=query('candidate_removed',head,'right')
results['candidate_removed_nocache']=query('candidate_removed_nocache',head,'right',False)
assert results['candidate_removed']==results['candidate_removed_nocache']==results['config_changed_new']
# Local extends and package.json occupancy controls.
(root/'tsconfig.parent.json').write_text((root/'tsconfig.json').read_text())
(root/'tsconfig.json').write_text('{"extends":"./tsconfig.parent.json"}')
results['extends_before']=query('extends_before',head,'right')
(root/'tsconfig.parent.json').write_text(json.dumps({'compilerOptions':{'moduleResolution':'node','paths':{'@lib':['left/util']}},'include':['**/*']}))
results['extends_after']=query('extends_after',head,'left')
assert results['extends_after']==query('extends_after_nocache',head,'left',False)
assert functions(results['extends_after'])
(root/'left/util').mkdir();(root/'left/util/package.json').write_text('{}')
results['package_added']=query('package_added',head,'left')
assert results['package_added']==query('package_added_nocache',head,'left',False)
assert not functions(results['package_added'])
(root/'left/util/package.json').unlink()
results['package_removed']=query('package_removed',head,'left')
assert results['package_removed']==query('package_removed_nocache',head,'left',False)==results['extends_after']
(out/'summary.json').write_text(json.dumps({'claim':'MEASURED','cross_binary_red':True,'cold_full_hit_sidecar_nocache_equal':True,'config_only_change_moves_caller_to_right':True,'unrelated_text_add_remove_keeps_hit':True,'candidate_add_remove_parity':True,'extends_parent_edit':True,'package_add_remove_parity':True},indent=2))
print((out/'summary.json').read_text())
