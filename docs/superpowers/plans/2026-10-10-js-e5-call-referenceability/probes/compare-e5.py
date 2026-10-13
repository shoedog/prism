#!/usr/bin/env python3
"""Call-site multiset comparison. Details stay in OUT; stdout contains aggregates."""
import argparse,json
from pathlib import Path
from collections import Counter,defaultdict
JS={'.js','.jsx','.mjs','.cjs','.ts','.tsx'}
def canonical(row):return json.dumps(row,sort_keys=True,separators=(',',':'))
def identity(site):return canonical({k:v for k,v in site.items() if k not in {'drop','resolved_targets','exact_target'}})
def oracle_key(site,target):return identity(site)+'|'+canonical(target)
def load(path):return [json.loads(l) for l in path.read_text().splitlines() if l]
def compare(base,head,base_census,head_census,out):
 out.mkdir(parents=True,exist_ok=True);bs=[r for r in load(base) if r.get('record_kind')=='call_site'];hs=[r for r in load(head) if r.get('record_kind')=='call_site']
 verdicts={}
 for file in [base_census,head_census]:
  if file and file.exists():
   for r in load(file):verdicts[oracle_key(r['site'],r['target'])]=r
 groups=[]
 for rows in [bs,hs]:
  d=defaultdict(list)
  for r in rows:d[identity(r)].append(r)
  groups.append(d)
 changed=[];counts=Counter();targetcounts=Counter();base_changed=[];head_changed=[];targetrows=[]
 for key in sorted(set(groups[0])|set(groups[1])):
  b,h=groups[0].get(key,[]),groups[1].get(key,[])
  if Counter(map(canonical,b))==Counter(map(canonical,h)):continue
  changed.append({'base':b,'head':h});base_changed.extend(b);head_changed.extend(h)
  counts['CHANGED' if b and h else 'LOST' if b else 'ADDED']+=max(len(b),len(h))
  bt=Counter(canonical(t) for s in b for t in s['resolved_targets']);ht=Counter(canonical(t) for s in h for t in s['resolved_targets'])
  for change,targets,sites in [('removed',bt-ht,b),('added',ht-bt,h)]:
   for raw,n in targets.items():
    target=json.loads(raw);site=next(s for s in sites if target in s['resolved_targets']);o=verdicts.get(oracle_key(site,target['function_id']))
    verdict=o['verdict'] if o else 'INADMISSIBLE';why=o['why'] if o else 'not_yet_adjudicated'
    targetcounts['|'.join([change,target['kind'],target['confidence'],verdict])]+=n
    targetrows.extend({'change':change,'site':site,'target':target,'verdict':verdict,'why':why,'oracle':o} for _ in range(n))
 for suffix,rows in [('call-changes',changed),('target-changes',targetrows),('base.changed-sites',base_changed),('head.changed-sites',head_changed)]:
  (out/(suffix+'.jsonl')).write_text(''.join(canonical(r)+'\n' for r in rows))
 # Compare the exact serialized non-JS call-site bytes, in producer order.
 nonjs=lambda rows:''.join(canonical(r)+'\n' for r in rows if Path(r['caller']['file']).suffix not in JS)
 result={'base_sites':len(bs),'head_sites':len(hs),'site_changes':dict(counts),'target_changes':dict(targetcounts),'non_js_call_sites_identical':nonjs(bs)==nonjs(hs)}
 (out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n');return result
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('base',type=Path);ap.add_argument('head',type=Path);ap.add_argument('base_census',type=Path);ap.add_argument('head_census',type=Path);ap.add_argument('out',type=Path);a=ap.parse_args()
 print(json.dumps(compare(a.base,a.head,a.base_census,a.head_census,a.out),sort_keys=True))
