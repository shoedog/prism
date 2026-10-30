"""Compare complete scenario sections and site keys, rejecting empty/partial runs.
Usage: compare_controls_s1b4.py BASE_DIR HEAD_DIR OUT_JSON
Unlike controls_diff, this compares inventory and unpaired rows too.
"""
import json,sys
from pathlib import Path
from valueflow_guard_s1b4 import load

def sections(path):
 out={}; key=None
 for line in path.read_text().splitlines(keepends=True):
  if line.startswith('== '): key=line.split()[1]; assert key not in out; out[key]=''
  assert key is not None
  out[key]+=line
 return out

a,b=map(Path,sys.argv[1:3]);sa,sb=sections(a/'SUMMARY.txt'),sections(b/'SUMMARY.txt')
assert set(sa)==set(sb),'missing scenario sections'
changes=[]
for name in sorted(sa):
 for root in [a,b]:
  assert (root/(name+'.dump.jsonl')).stat().st_size,'empty dump'
  json.load((root/(name+'.functions.json')).open())
  stderr=list(root.glob(name+'.dump.stderr'))
  assert all(p.stat().st_size==0 for p in stderr),'stderr'
 aa,bb=load(a/(name+'.dump.jsonl')),load(b/(name+'.dump.jsonl'))
 assert set(aa)==set(bb),'missing call-site keys: '+name
 if sa[name]!=sb[name]: changes.append(dict(scenario=name,base=sa[name],head=sb[name]))
report=dict(scenarios=len(sa),changed_sections=len(changes),unchanged_sections=len(sa)-len(changes),all_keys_equal=True,changes=changes)
Path(sys.argv[3]).write_text(json.dumps(report,indent=1))
print(json.dumps({k:v for k,v in report.items() if k!='changes'},sort_keys=True))
for row in changes: print(row['base'],row['head'])
