"""Compare every js_export_* counter with complete, successful base/head stats.
Usage: export_counters_s1b4.py BASE HEAD OUT --repo LABEL PATH [--repo ...]
Private sources and full outputs remain in OUT; stdout is aggregate only.
"""
import argparse,json,subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('base');p.add_argument('head');p.add_argument('out');p.add_argument('--repo',nargs=2,action='append',required=True);a=p.parse_args()
out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
assert len({label for label,_ in a.repo})==len(a.repo),'duplicate labels'
def check(pair):
 label,root=pair;results=[]
 assert label and '/' not in label and label not in ('.','..'),'unsafe label'
 for lane,binary in [('base',a.base),('head',a.head)]:
  r=subprocess.run([binary,'nav','--no-cache','call-stats','--repo',root],capture_output=True,timeout=180)
  (out/(label+'-'+lane+'.json')).write_bytes(r.stdout);(out/(label+'-'+lane+'.stderr')).write_bytes(r.stderr)
  assert r.returncode==0 and not r.stderr and r.stdout,'inadmissible stats run: '+label+' '+lane
  stats=json.loads(r.stdout);assert isinstance(stats,dict) and 'total_call_sites' in stats
  counters={k:v for k,v in stats.items() if k.startswith('js_export_')};assert counters,'missing export counters'
  results.append(counters)
 return dict(label=label,equal=results[0]==results[1],base=results[0],head=results[1])
with ThreadPoolExecutor(max_workers=2) as pool:rows=list(pool.map(check,a.repo))
report=dict(repositories=len(rows),changed=sum(not r['equal'] for r in rows),rows=rows)
(out/'comparison.json').write_text(json.dumps(report,indent=1));print(json.dumps({k:v for k,v in report.items() if k!='rows'},sort_keys=True))
raise SystemExit(0 if report['changed']==0 else 1)
