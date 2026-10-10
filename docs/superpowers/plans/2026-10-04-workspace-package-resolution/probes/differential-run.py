"""Rerunnable acceptance: helper TS_JS fixture-manifest OUT [--reuse].
Bound must equal the native canonical module and actual writer owner; every
Exact callable must match the native declaration span. Native-resolved
Unsupported cases are counted by feature/reason. ProvenUnresolved must have
no native target. No S2 adoption is inferred.
"""
import argparse, collections, hashlib, json, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
for k in ('helper','ts','manifest','out'):p.add_argument(k,type=Path)
p.add_argument('--reuse',action='store_true');a=p.parse_args();a.out.mkdir(parents=True,exist_ok=a.reuse)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
binding={k:{'path':str(getattr(a,k).resolve()),'sha256':sha(getattr(a,k))} for k in ('helper','ts','manifest')}
cases=json.loads(a.manifest.read_text())
if not a.reuse:
 for label,args in [('prism',[a.helper,a.manifest]),('oracle',['node','--expose-gc',Path(__file__).with_name('differential-oracle.cjs'),a.ts,a.manifest])]:
  with (a.out/(label+'.jsonl')).open('w') as f,(a.out/(label+'.stderr')).open('w') as err:subprocess.run(list(map(str,args)),stdout=f,stderr=err,check=True,timeout=3600)
def read(label):
 rows=[json.loads(s) for s in (a.out/(label+'.jsonl')).read_text().splitlines()]
 assert len(rows)==len(cases),(label,len(rows),len(cases))
 result={r['id']:r for r in rows};assert len(result)==len(rows);return result
prism=read('prism');oracle=read('oracle');counts=collections.Counter();gaps=collections.Counter();wrong=[];native_expectation=[]
for c in cases:
 r=prism[c['id']];t=oracle[c['id']];res=r['resolution'];kind=res if isinstance(res,str) else next(iter(res));counts[kind]+=1
 if 'expected' in c and c['expected']!=t['target']:native_expectation.append({'id':c['id'],'expected':c['expected'],'native':t['target']})
 if kind=='Bound':
  bound=res['Bound']
  if bound!=t['target'] or r['owner']!=t['owner'] or r['production_module']!=[bound,r['owner']]:wrong.append({'id':c['id'],'kind':'binding','prism':r,'oracle':t})
 elif kind=='ProvenUnresolved' and t['target'] is not None:wrong.append({'id':c['id'],'kind':'false absence','prism':r,'oracle':t})
 elif kind=='Unsupported' and t['target'] is not None:gaps[(c['feature'],res['Unsupported'])]+=1
 for row in r['calls']:
  for edge in row['resolved_targets']:
   if edge['confidence']=='exact':
    identity=edge['function_id']
    if not any(all(identity[k]==d.get(k) for k in ['file','name','start_line','end_line']) for d in t['declarations']):wrong.append({'id':c['id'],'kind':'callable','prism':edge,'oracle':t})
summary={'binding':binding,'total':len(cases),'counts':dict(counts),'wrong':len(wrong),'native_expectation_mismatch':len(native_expectation),'unsupported_where_TS_binds':sum(gaps.values()),'unsupported_table':[{'feature':f,'reason':r,'count':n} for (f,r),n in sorted(gaps.items())]}
(a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');(a.out/'wrong.json').write_text(json.dumps(wrong,indent=2)+'\n');(a.out/'native-expectations.json').write_text(json.dumps(native_expectation,indent=2)+'\n')
assert all(sha(Path(v['path']))==v['sha256'] for v in binding.values()),'input drift'
print(json.dumps(summary,indent=2))
assert not wrong and not native_expectation,'differential gate rejected; read complete wrong population before retry'
