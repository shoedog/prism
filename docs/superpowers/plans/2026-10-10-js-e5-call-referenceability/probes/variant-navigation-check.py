#!/usr/bin/env python3
"""Compare physical symbol identities and preserve unresolved call placeholders."""
import collections,json,sys
from pathlib import Path
p=Path(sys.argv[1]);receipts=json.loads((p/'summary.json').read_text());out=[]
canon=lambda v:json.dumps(v,sort_keys=True,separators=(',',':'))
for r in receipts:
 assert r['status']=='COMPLETE',r
 label=r['variant']+'.'+r['corpus'];summary={'variant':r['variant'],'corpus':r['corpus'],'root':r['root'],'native_sample_verdict':r['sample']['verdict'],'queries':{}}
 for direction,seed in [('callers',r['sample']['target']['function_id']),('callees',r['sample']['site']['caller'])]:
  b,h=[json.loads((p/(label+'.'+side+'.'+direction+'.json')).read_text()) for side in ['base','head']]
  targeted=all(v['query'].startswith(direction+':'+seed['name']+'@') for v in [b,h]);assert targeted,('inadmissible sample selector',label,direction)
  groups=[]
  for doc in [b,h]:
   d=collections.defaultdict(list)
   for i in doc['items']:
    key=canon(i['symbol']) if i['symbol'] is not None else 'unresolved:'+canon(i['location']);d[key].append(i)
   groups.append(d)
  c=collections.Counter()
  for k in set(groups[0])|set(groups[1]):
   bs,hs=groups[0].get(k,[]),groups[1].get(k,[]);same=collections.Counter(map(canon,bs))&collections.Counter(map(canon,hs));bb=list((collections.Counter(map(canon,bs))-same).elements());hh=list((collections.Counter(map(canon,hs))-same).elements());c['RELABELLED']+=min(len(bb),len(hh));c['LOST']+=max(0,len(bb)-len(hh));c['ADDED_UNRESOLVED' if k.startswith('unresolved:') else 'ADDED_SYMBOL']+=max(0,len(hh)-len(bb))
  assert not c['ADDED_SYMBOL'],(label,direction,c)
  summary['queries'][direction]={'base_items':len(b['items']),'head_items':len(h['items']),'targeted_symbol':targeted,'changes':dict(c)}
 out.append(summary)
(p/'comparison-summary.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'samples':len(out),'all_targeted':True,'new_resolved_symbols':0}))
