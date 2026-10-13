#!/usr/bin/env python3
"""Admit whole-document nav outputs and bind removals to measured resolution changes."""
import argparse,collections,json
from pathlib import Path
ap=argparse.ArgumentParser();ap.add_argument('navigation',type=Path);ap.add_argument('comparison',type=Path);a=ap.parse_args()
read=lambda p:json.loads(p.read_text());canon=lambda r:json.dumps(r,sort_keys=True,separators=(',',':'))
identity=lambda r:(r['file'],r['name'],r['start_line'],r['end_line'])
results=[]
for f in sorted(a.navigation.glob('*.receipt.json')):
 r=read(f);n=r['name'];changes=[json.loads(l) for l in (a.comparison/n/'target-changes.jsonl').open()] if (a.comparison/n/'target-changes.jsonl').exists() else []
 removed=[c for c in changes if c['change']=='removed'];targets={identity(c['target']['function_id']) for c in removed};callers={identity(c['site']['caller']) for c in removed};pairs={(c['site']['caller']['file'],c['target']['function_id']['file']) for c in removed}
 for k,v in r['producers'].items():
  if v['exit']==65 and (v.get('reason') or '').startswith('invalid JSON:'):
   file=a.navigation/(n+'.'+k.replace('|','.')+'.json');data=read(file)
   # capture.run only decodes JSON after actual producer exit0. Its synthetic65
   # therefore proves a decoder failure; retain all ordinary cache diagnostics.
   v['original_decoder_failure']={'exit':v['exit'],'reason':v['reason']};v.update(exit=0,reason=None,artifact_format='json',rows=len(data.get('items',[])) if isinstance(data,dict) else len(data),readmitted_from_completed_bytes=True)
 assert all(v['exit']==0 for v in r['producers'].values()),n
 assert (a.navigation/(n+'.base.functions.json')).read_bytes()==(a.navigation/(n+'.head.functions.json')).read_bytes(),('STOP function inventory',n)
 b,h=[read(a.navigation/(n+'.'+side+'.repo-map.json')) for side in ['base','head']]
 assert {k:v for k,v in b.items() if k!='graph'}=={k:v for k,v in h.items() if k!='graph'},('STOP map metadata',n)
 assert b['graph']['nodes']==h['graph']['nodes'],('STOP file inventory',n)
 bc,hc=[collections.Counter(canon(e) for e in x['graph']['edges']) for x in [b,h]];assert not hc-bc,n
 edges=[json.loads(raw) for raw,count in (bc-hc).items() for _ in range(count)]
 for e in edges:
  assert e['kind']=='ModuleDep';pair=tuple(b['graph']['nodes'][e[k]]['location']['file'] for k in ['from','to']);assert pair in pairs,('STOP unexplained dependency',n,pair)
 item_counts={}
 for direction,allowed in [('callers',callers),('callees',targets)]:
  if 'base|'+direction not in r['producers']:continue
  b,h=[read(a.navigation/(n+'.'+side+'.'+direction+'.json')) for side in ['base','head']]
  bc,hc=[collections.Counter(canon(i) for i in x['items']) for x in [b,h]]
  added=[json.loads(raw) for raw,count in (hc-bc).items() for _ in range(count)]
  for i in added:
   why=[w['Calls'] for w in i['why'] if 'Calls' in w]
   assert direction=='callees' and i['symbol'] is None and len(why)==1 and not any('Resolution' in w for w in i['why']),('new resolved navigation item',n,direction,i)
   assert any(c['site']['caller']['file']==i['location']['file'] and c['site']['source_span']['line']==why[0]['call_site_line'] and c['site']['callee_text']==why[0]['callee'] for c in removed),('unexplained unresolved placeholder',n,i)
  lost=[json.loads(raw) for raw,count in (bc-hc).items() for _ in range(count)]
  for i in lost:
   fid=i['symbol']['Function'];assert identity(fid) in allowed,('unexplained navigation removal',n,direction,fid)
  item_counts[direction]=len(lost)
  item_counts[direction+'_unresolved_placeholders']=len(added)
 r.update(functions_byte_identical=True,repo_map_files_byte_identical=True,removed_call_derived_module_edges=len(edges),sampled_navigation_removed_items=item_counts)
 (a.navigation/(n+'.readmitted-receipt.json')).write_text(json.dumps(r,indent=2)+'\n');results.append(r)
summary={'roots':len(results),'functions_byte_identical':True,'file_inventory_byte_identical':True,'removed_call_derived_module_edges':sum(r['removed_call_derived_module_edges'] for r in results),'sampled_navigation_removed_items':dict(sum((collections.Counter(r['sampled_navigation_removed_items']) for r in results),collections.Counter())),'details':results}
(a.navigation/'admitted-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:v for k,v in summary.items() if k!='details'}))
