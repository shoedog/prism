#!/usr/bin/env python3
"""Static symbol inventories and sampled caller/callee queries on affected roots."""
import concurrent.futures,json,sys
from pathlib import Path
from capture import check_root,run
base,head,capture,out=map(Path,sys.argv[1:5]);out.mkdir(parents=True,exist_ok=True);cache=Path.home()/'prism-evidence/js-e5/cache'
selected=[]
for p in capture.glob('*.census.jsonl'):
 rows=[json.loads(l) for l in p.open()]
 if any(r['rules'].get('R-inferred') or r['rules'].get('R-bound') or r['rules'].get('R-recommended') for r in rows):
  s=json.loads(p.with_name(p.name.removesuffix('.census.jsonl')+'.status.json').read_text());selected.append((s,rows))
def one(pair):
 s,rows=pair;name=s['name'];check_root(s['root']);receipt={'name':name,'root':s['root'],'producers':{},'identical':{}}
 for side,binary in [('base',base),('head',head)]:
  for mode in ['functions','repo-map']:
   file=out/(name+'.'+side+'.'+mode+'.json');receipt['producers'][side+'|'+mode]=run([binary,'nav','--cache-dir',cache,mode,'--repo',s['root'],'--format','json'],file,1500)
 for mode in ['functions','repo-map']:
  if all(receipt['producers'][side+'|'+mode]['exit']==0 for side in ['base','head']):receipt['identical'][mode]=(out/(name+'.base.'+mode+'.json')).read_bytes()==(out/(name+'.head.'+mode+'.json')).read_bytes()
 sample=next((r for r in rows if r['verdict']=='WRONG' and r['rules']['R-recommended']),None)
 if sample:
  receipt['sample']={'caller':sample['site']['caller'],'target':sample['target']}
  for direction,seed in [('callers',sample['target']),('callees',sample['site']['caller'])]:
   for side,binary in [('base',base),('head',head)]:
    file=out/(name+'.'+side+'.'+direction+'.json');receipt['producers'][side+'|'+direction]=run([binary,'nav','--cache-dir',cache,direction,'--repo',s['root'],'--location',seed['file']+':'+str(seed['start_line']),'--format','json'],file,1500)
 (out/(name+'.receipt.json')).write_text(json.dumps(receipt,indent=2)+'\n');return receipt
with concurrent.futures.ThreadPoolExecutor(2) as pool:
 results=list(pool.map(one,selected))
 for r in results:print(json.dumps(r),flush=True)
(out/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
