"""Full-row preservation for every finite Option-K control; positive gain coverage."""
import json,sys
from pathlib import Path
from rowdiff import load,raw_load
root,base,head,changed=map(Path,sys.argv[1:5]);a,b=load(base),load(head)
assert a.keys()==b.keys(),'site keys changed'
changes=json.loads(changed.read_text());counts={}
classified=json.loads(changed.with_name(changed.stem.replace('-changes','-classified')+'.json').read_text())
assert len(classified)==len(changes)
assert all(c['class'] in ('CORRECT_STATIC_BINDING','CORRECT_STATIC_REFUSAL') for c in classified), 'independent audit did not pass'
raw_a,raw_b=raw_load(base),raw_load(head)
for c in changes:counts[c['key'][0].split('/')[0]]=counts.get(c['key'][0].split('/')[0],0)+1
for case in json.loads((root/'manifest.json').read_text()):
 n=counts.get(case['case'],0)
 if case['expectation']=='base':
  assert n==0,case
  for k in a:
   if k[0].split('/')[0]==case['case']:assert raw_a[k]==raw_b[k],case
 else:
  assert n>0,case
  if case['expectation']=='refusal':
   assert all(c['class']=='CORRECT_STATIC_REFUSAL' for c in classified if c['key'][0].split('/')[0]==case['case']),case
print(json.dumps({'claim':'MEASURED','scenarios':len(json.loads((root/'manifest.json').read_text())),'sites':len(a),'changed':len(changes),'preservation_violations':0}))
