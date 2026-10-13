#!/usr/bin/env python3
"""Paired real CPG producers; preserve every differing row and producer receipt."""
import argparse,concurrent.futures,json
from pathlib import Path
from collections import Counter
from capture import check_root,run
ap=argparse.ArgumentParser();ap.add_argument('base',type=Path);ap.add_argument('head',type=Path);ap.add_argument('capture',type=Path);ap.add_argument('out',type=Path);ap.add_argument('--jobs',type=int,default=3);ap.add_argument('--names',nargs='*');ap.add_argument('--reuse-base-from',nargs='*',type=Path,default=[]);a=ap.parse_args();a.out.mkdir(parents=True,exist_ok=True)
statuses=[json.loads(p.read_text()) for p in a.capture.glob('*.status.json') if '.facts.' not in p.name]
def one(s):
 name=s['name'];result={'name':name,'root':s['root'],'producers':{}}
 if s['exit']!=0:result.update(status='EXCLUDED',reason=s['reason'],base_seconds=s['seconds']);return result
 if a.names is not None and name not in a.names:return {'name':name,'status':'NOT_SELECTED'}
 check_root(s['root'])
 for mode,binary in [('base',a.base),('head',a.head)]:
  dest=a.out/(name+'.'+mode+'.graph.jsonl');reused=False
  if mode=='base':
   for directory in a.reuse_base_from:
    receipt=directory/(name+'.receipt.json');source=directory/(name+'.base.graph.jsonl')
    if not receipt.exists():continue
    prior=json.loads(receipt.read_text())
    if prior.get('producers',{}).get('base',{}).get('exit')!=0:continue
    assert prior['producers']['base']['command']==[str(binary),s['root']]
    if dest.exists():dest.unlink()
    dest.symlink_to(source.resolve());result['producers'][mode]={**prior['producers']['base'],'reused_from':str(source)};reused=True;break
  if not reused:result['producers'][mode]=run([binary,s['root']],dest,1500)
 if any(p['exit']!=0 for p in result['producers'].values()):result['status']='INADMISSIBLE'
 else:
  counters=[]
  for mode in ['base','head']:
   counters.append(Counter((a.out/(name+'.'+mode+'.graph.jsonl')).read_text().splitlines()))
  changes=[];counts=Counter()
  for change,rows in [('removed',counters[0]-counters[1]),('added',counters[1]-counters[0])]:
   for text,count in sorted(rows.items()):
    row=json.loads(text);counts[change+'|'+row['kind']]+=count
    changes.extend({'change':change,'row':row} for _ in range(count))
  (a.out/(name+'.changes.jsonl')).write_text(''.join(json.dumps(c,sort_keys=True)+'\n' for c in changes))
  result.update(status='COMPLETE',counts=dict(counts))
 (a.out/(name+'.receipt.json')).write_text(json.dumps(result,indent=2)+'\n');return result
with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool:
 results=list(pool.map(one,statuses))
 for r in results:print(json.dumps(r),flush=True)
(a.out/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
