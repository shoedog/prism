#!/usr/bin/env python3
import concurrent.futures,json,sys
from pathlib import Path
from capture import check_root,run
binary,byte_binary,out=map(Path,sys.argv[1:4]);out.mkdir(exist_ok=True,parents=True);home=Path.home()
roots={'black':home/'code/bench-repos/black','caddy':home/'code/bench-repos/caddy','rust':home/'.local/share/prism/corpora/prism-20c8490591a3/source'}
def one(item):
 name,root=item;check_root(root);result={'name':name,'root':str(root),'producers':{}}
 for mode,args in [('sites',['call-stats','--dump-sites']),('functions',['functions','--format','json']),('modules',['repo-map','--format','json'])]:result['producers'][mode]=run([binary,'nav','--cache-dir',home/'prism-evidence/js-e5/cache',args[0],'--repo',root,*args[1:]],out/(name+'.'+mode+'.jsonl'),1500)
 result['producers']['bytes']=run([byte_binary,root],out/(name+'.bytes.jsonl'),1500);(out/(name+'.receipt.json')).write_text(json.dumps(result,indent=2)+'\n');return result
with concurrent.futures.ThreadPoolExecutor(3) as pool:
 for result in pool.map(one,roots.items()):print(json.dumps(result),flush=True)
