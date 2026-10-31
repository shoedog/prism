"""Persisted CPG + nav sidecar regression, including true warm hits.
Usage: python3 cache-invalidation.py HEAD_BIN OUT
Dirty-sidecar override is confined to these synthetic subprocesses. Each edit
must invalidate both persisted artifacts; warm and cold output must agree.
"""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('binary',type=Path);p.add_argument('out',type=Path);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
root=a.out/'fixture';cache=a.out/'cache';root.mkdir()
def write(name,value):
 q=root/name;q.parent.mkdir(parents=True,exist_ok=True);q.write_text(value if isinstance(value,str) else json.dumps(value))
fn='export function real(){return 1;}\n'
write('app.ts',"import {real as picked} from '@ws/lib'; export function run(){picked();}\n")
for lib in ['lib','alt']:
 for name in ['index','other']:write(f'packages/{lib}/{name}.ts',fn)
 write(f'packages/{lib}/package.json',{'name':'@ws/lib','exports':'./index.ts'})
write('package.json',{'type':'commonjs'})
write('tsconfig.json',{'compilerOptions':{'moduleResolution':'bundler','module':'esnext'},'include':['**/*']})
link=root/'node_modules/@ws/lib';link.parent.mkdir(parents=True);link.symlink_to('../../packages/lib',target_is_directory=True)
env={**os.environ,'PRISM_NAV_EDGE_CACHE_LOAD_DIRTY':'1'}
def query(label,cold=False):
 args=[str(a.binary),'nav',*(['--no-cache'] if cold else ['--cache-dir',str(cache)]),'call-stats','--repo',str(root),'--dump-sites']
 r=subprocess.run(args,env=env,capture_output=True,check=True,timeout=120)
 (a.out/(label+'.jsonl')).write_bytes(r.stdout);(a.out/(label+'.stderr')).write_bytes(r.stderr)
 rows=[json.loads(s) for s in r.stdout.splitlines()];row=next(r for r in rows if r.get('record_kind')=='call_site' and r.get('callee_text')=='picked')
 exact=[r['function_id']['file'] for r in row['resolved_targets'] if r['confidence']=='exact']
 navargs=[str(a.binary),'nav',*(['--no-cache'] if cold else ['--cache-dir',str(cache)]),'callees','--repo',str(root),'--symbol','run','--format','json']
 nav=subprocess.run(navargs,env=env,capture_output=True,check=True,timeout=120)
 (a.out/(label+'-nav.json')).write_bytes(nav.stdout);(a.out/(label+'-nav.stderr')).write_bytes(nav.stderr)
 items=json.loads(nav.stdout)['items'];navexact=[x['location']['file'] for x in items if x['score']==1.0 and x['symbol'] is not None]
 assert navexact==exact,(label,'nav/call-graph divergence',navexact,exact)
 return r.stdout+nav.stdout,(r.stderr+nav.stderr).decode(),exact
records=[]
def artifacts():
 return {p.name:{'path':str(p),'mtime':p.stat().st_mtime_ns,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in cache.rglob('*.bin')}
def check(label,expected):
 before=artifacts();warm,err,targets=query(label)
 assert targets==expected,(label,targets,expected)
 after=artifacts();assert set(after)=={'cpg-cache.bin','resolved-call-edge-index.bin'},after
 assert 'CPG cache miss' in err or 'partial CPG cache hit' in err,(label,'edit did not invalidate CPG',err)
 if before:assert all(before[k]['mtime']!=after[k]['mtime'] for k in before),(label,'persisted artifact not rebuilt')
 warm2,err2,targets2=query(label+'-fullhit')
 assert warm==warm2 and targets2==expected
 assert 'cache miss' not in err2 and 'partial CPG' not in err2,(label,'not full hit',err2)
 assert artifacts()==after,(label,'warm sidecar rewritten instead of loaded')
 cold,_,targets3=query(label+'-cold',True);assert cold==warm and targets3==expected,(label,'cached/cold divergence')
 records.append({'edit':label,'expected':expected,'full_hit':True,'warm_cold_bytes_equal':True,'artifacts':after})
 (a.out/'summary.json').write_text(json.dumps({'binary':str(a.binary.resolve()),'sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'checks':records},indent=2)+'\n')
check('initial',['packages/lib/index.ts'])
write('packages/lib/package.json',{'name':'@ws/lib','exports':'./other.ts'});check('package-json',['packages/lib/other.ts'])
write('packages/lib/package.json',{'name':'@ws/lib','exports':{'import':'./other.ts','require':'./index.ts'}});check('condition-map',['packages/lib/other.ts'])
write('tsconfig.json',{'compilerOptions':{'moduleResolution':'bundler','module':'commonjs'},'include':['**/*']});check('tsconfig-only',['packages/lib/index.ts'])
link.unlink();link.symlink_to('../../packages/alt',target_is_directory=True);check('symlink-only',['packages/alt/index.ts'])
write('packages/alt/package.json',{'name':'@ws/lib','exports':{'types':'./dist/index.d.ts','default':'./index.ts'}});check('missing-declaration',['packages/alt/index.ts'])
write('packages/alt/dist/index.d.ts','export declare function real():void;\n');check('declaration-created',[])
(root/'packages/alt/dist/index.d.ts').unlink();check('declaration-removed',['packages/alt/index.ts'])
write('packages/alt/package.json',{'name':'@ws/lib','exports':{'import':'./other.ts','require':'./index.ts'}})
write('tsconfig.json',{'compilerOptions':{'moduleResolution':'node16','module':'node16'},'include':['**/*']})
(root/'package.json').unlink();outer=a.out/'package.json';outer.write_text(json.dumps({'type':'module'}));check('outer-scope-created',['packages/alt/other.ts'])
outer.write_text(json.dumps({'type':'commonjs'}));check('outer-scope-type-only',['packages/alt/index.ts'])
outer.write_text('{');check('outer-scope-ambiguous',[])
outer.write_text(json.dumps({'type':'module'}));check('outer-scope-restored',['packages/alt/other.ts'])
print(json.dumps({'persisted_checks':len(records),'all_full_hits':True,'all_cold_equal':True}))
