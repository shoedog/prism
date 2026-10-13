#!/usr/bin/env python3
import argparse,concurrent.futures,importlib.util,json,subprocess,time
from pathlib import Path
spec=importlib.util.spec_from_file_location('compare_e5',Path(__file__).with_name('compare-e5.py'));mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
ap=argparse.ArgumentParser();ap.add_argument('base',type=Path);ap.add_argument('head',type=Path);ap.add_argument('out',type=Path);ap.add_argument('oracle',type=Path);ap.add_argument('--watch',action='store_true');a=ap.parse_args();seen=set();results=[];a.out.mkdir(parents=True,exist_ok=True)
while True:
 pending=[]
 for p in a.head.glob('*.status.json'):
  if '.facts.' in p.name:continue
  s=json.loads(p.read_text());name=s['name']
  if name in seen:continue
  seen.add(name)
  if s['exit']==0:pending.append(s)
 def one(s):
  n=s['name'];out=a.out/n;bc=a.base/(n+'.census.jsonl');hc=a.head/(n+'.census.jsonl')
  r=mod.compare(a.base/(n+'.sites.jsonl'),a.head/(n+'.sites.jsonl'),bc,hc,out)
  if r['site_changes']:
   for side in ['base','head']:
    command=['node','--max-old-space-size=8000',str(Path(__file__).with_name('census-e5.cjs')),str(a.oracle),s['root'],str(out/(side+'.changed-sites.jsonl')),str(out/(side+'.changed-census.jsonl')),'--all-resolved']
    facts=getattr(a,side)/(n+'.facts.jsonl')
    if facts.exists():command+=['--facts',str(facts)]
    start=time.monotonic()
    with (out/(side+'.oracle.stdout')).open('wb') as o,(out/(side+'.oracle.stderr')).open('wb') as e:
     try:rc=subprocess.run(command,stdout=o,stderr=e,timeout=1500).returncode
     except subprocess.TimeoutExpired:rc=124
    (out/(side+'.oracle.receipt.json')).write_text(json.dumps({'exit':rc,'seconds':round(time.monotonic()-start,3)})+'\n')
    if rc:raise RuntimeError(f'{n} {side} native oracle inadmissible')
   r=mod.compare(a.base/(n+'.sites.jsonl'),a.head/(n+'.sites.jsonl'),out/'base.changed-census.jsonl',out/'head.changed-census.jsonl',out)
  return {'name':n,'root':s['root'],**r}
 with concurrent.futures.ThreadPoolExecutor(2) as pool:
  for r in pool.map(one,pending):results.append(r);print(json.dumps(r),flush=True)
 if (a.head/'summary.json').exists() or not a.watch:break
 time.sleep(5)
(a.out/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
