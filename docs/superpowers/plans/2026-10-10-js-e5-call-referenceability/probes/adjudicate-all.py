#!/usr/bin/env python3
import argparse,concurrent.futures,json,subprocess,time
from pathlib import Path
ap=argparse.ArgumentParser();ap.add_argument('graphs',type=Path);ap.add_argument('calls',type=Path);ap.add_argument('oracle',type=Path);ap.add_argument('--watch',action='store_true');a=ap.parse_args();seen=set();results=[]
while True:
 pending=[]
 for p in a.graphs.glob('*.receipt.json'):
  s=json.loads(p.read_text());name=s['name']
  if name in seen:continue
  if s['status']=='COMPLETE' and s.get('counts') and not (a.calls/name/'comparison.json').exists():continue
  seen.add(name);pending.append(s)
 def one(s):
  n=s['name'];out=a.graphs/(n+'.adjudication');out.mkdir(exist_ok=True)
  if s['status']!='COMPLETE':return {'name':n,'status':s['status']}
  if not s.get('counts'):
   (out/'graph.adjudicated.jsonl').write_text('');(out/'graph.adjudication-summary.json').write_text('{}\n');return {'name':n,'status':'COMPLETE','counts':{}}
  command=['node','--max-old-space-size=8000',str(Path(__file__).with_name('adjudicate-graphs.cjs')),str(a.oracle),s['root'],str(a.graphs/(n+'.changes.jsonl')),str(a.calls/n/'target-changes.jsonl'),str(out)]
  start=time.monotonic()
  with (out/'stdout.json').open('wb') as o,(out/'stderr').open('wb') as e:
   try:rc=subprocess.run(command,stdout=o,stderr=e,timeout=1500).returncode
   except subprocess.TimeoutExpired:rc=124
  return {'name':n,'status':'COMPLETE' if rc==0 else 'INADMISSIBLE','exit':rc,'seconds':round(time.monotonic()-start,3),'counts':json.loads((out/'graph.adjudication-summary.json').read_text()) if rc==0 else {}}
 with concurrent.futures.ThreadPoolExecutor(2) as pool:
  for r in pool.map(one,pending):results.append(r);print(json.dumps(r),flush=True)
 if (a.graphs/'summary.json').exists() and (a.calls/'summary.json').exists() or not a.watch:break
 time.sleep(5)
(a.graphs/'adjudication-summary.json').write_text(json.dumps(results,indent=2)+'\n')
