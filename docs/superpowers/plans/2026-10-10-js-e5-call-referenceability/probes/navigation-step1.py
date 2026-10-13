#!/usr/bin/env python3
"""Static symbol inventories and sampled caller/callee queries on affected roots."""
import concurrent.futures,json,sys
from pathlib import Path
from capture import check_root,run
base,head,capture,out,comparison=map(Path,sys.argv[1:6]);out.mkdir(parents=True,exist_ok=True);cache=Path.home()/'prism-evidence/js-e5/cache'
selected=[]
for p in capture.glob('*.status.json'):
 if '.facts.' in p.name:continue
 s=json.loads(p.read_text())
 if s['exit']!=0:continue
 changes=comparison/s['name']/'target-changes.jsonl'
 rows=[json.loads(l) for l in changes.open()] if changes.exists() else []
 selected.append((s,rows))
def one(pair):
 s,rows=pair;name=s['name'];check_root(s['root']);receipt={'name':name,'root':s['root'],'producers':{},'identical':{}}
 prior_dir=Path.home()/'prism-evidence/js-e5/step0/navigation'
 prior_file=prior_dir/(name+'.readmitted-receipt.json')
 prior=json.loads(prior_file.read_text()) if prior_file.exists() else {}
 def produce(side,mode,args,file):
  command=[base if side=='base' else head,'nav','--cache-dir',cache,*args]
  old=prior.get('producers',{}).get(side+'|'+mode,{})
  source=prior_dir/(name+'.'+side+'.'+mode+'.json')
  if side=='base' and old.get('exit')==0 and old.get('command')==[str(v) for v in command] and source.exists():
   file.symlink_to(source);return {**old,'reused_from':str(source)}
  return run(command,file,1500)
 for side,binary in [('base',base),('head',head)]:
  for mode in ['functions','repo-map']:
   file=out/(name+'.'+side+'.'+mode+'.json');receipt['producers'][side+'|'+mode]=produce(side,mode,[mode,'--repo',s['root'],'--format','json'],file)
 for mode in ['functions','repo-map']:
  if all(receipt['producers'][side+'|'+mode]['exit']==0 for side in ['base','head']):receipt['identical'][mode]=(out/(name+'.base.'+mode+'.json')).read_bytes()==(out/(name+'.head.'+mode+'.json')).read_bytes()
 old_sample=prior.get('sample')
 sample=next((v for v in rows if old_sample and v['site']['caller']==old_sample['caller'] and v['target']['function_id']==old_sample['target']),None)
 if sample is None:sample=next((v for v in rows if v['verdict']=='WRONG'),rows[0] if rows else None)
 if sample:
  receipt['sample']={'caller':sample['site']['caller'],'target':sample['target']['function_id']}
  for direction,seed in [('callers',sample['target']['function_id']),('callees',sample['site']['caller'])]:
   for side,binary in [('base',base),('head',head)]:
    file=out/(name+'.'+side+'.'+direction+'.json');receipt['producers'][side+'|'+direction]=produce(side,direction,[direction,'--repo',s['root'],'--location',seed['file']+':'+str(seed['start_line']),'--format','json'],file)
 (out/(name+'.receipt.json')).write_text(json.dumps(receipt,indent=2)+'\n');return receipt
with concurrent.futures.ThreadPoolExecutor(2) as pool:
 results=list(pool.map(one,selected))
 for r in results:print(json.dumps(r),flush=True)
(out/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
