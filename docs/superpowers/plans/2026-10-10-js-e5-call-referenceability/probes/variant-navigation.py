#!/usr/bin/env python3
"""Real caller/callee navigation samples for each experimental rule and corpus."""
import concurrent.futures,json,sys
from pathlib import Path
from capture import check_root,run
E=Path(sys.argv[1]);out=E/'variant-navigation';out.mkdir(exist_ok=True);cache=Path.home()/'prism-evidence/js-e5/cache';tasks=[]
for variant in ['R-inferred','R-bound','NameOnly']:
 folder=E/'variants'/variant;comparison=folder/('comparison-repaired' if variant=='R-inferred' else 'comparison');binary=folder/('prism-repaired' if variant=='R-inferred' else 'prism');selected={}
 for f in sorted(comparison.glob('*/target-changes.jsonl')):
  corpus='SB' if f.parent.name.startswith('SB__') else f.parent.name
  for line in f.open():
   r=json.loads(line)
   if r['change']!='removed' or (corpus in selected and selected[corpus][1]['verdict']=='WRONG'):continue
   if corpus not in selected or r['verdict']=='WRONG':selected[corpus]=(f.parent.name,r)
 for corpus,(name,row) in selected.items():tasks.append((variant,corpus,name,row,binary))
def one(task):
 variant,corpus,name,row,binary=task;s=json.loads((E/'base'/(name+'.status.json')).read_text());check_root(s['root']);label=variant+'.'+corpus;r={'variant':variant,'corpus':corpus,'root':name,'sample':row,'producers':{}}
 for direction,seed in [('callers',row['target']['function_id']),('callees',row['site']['caller'])]:
  for side,exe in [('base',E/'bin/base-prism'),('head',binary)]:
   file=out/(label+'.'+side+'.'+direction+'.json');r['producers'][side+'|'+direction]=run([exe,'nav','--cache-dir',cache,direction,'--repo',s['root'],'--location',seed['file']+':'+str(seed['start_line']),'--format','json'],file,1500)
 r['status']='COMPLETE' if all(v['exit']==0 for v in r['producers'].values()) else 'INADMISSIBLE'
 (out/(label+'.receipt.json')).write_text(json.dumps(r,indent=2)+'\n');return r
with concurrent.futures.ThreadPoolExecutor(3) as pool:
 rows=list(pool.map(one,tasks))
(out/'summary.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps({'samples':len(rows),'admitted':sum(r['status']=='COMPLETE' for r in rows)}))
