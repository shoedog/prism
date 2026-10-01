"""Full-row preservation for every finite Option-K control; positive gain coverage."""
import json,sys
from pathlib import Path
from rowdiff import load,raw_load
root,base,head,changed=map(Path,sys.argv[1:5]);a,b=load(base),load(head)
assert a.keys()==b.keys(),'site keys changed'
changes=json.loads(changed.read_text());counts={}
classified=json.loads(changed.with_name(changed.stem.replace('-changes','-classified')+'.json').read_text())
aliases=json.loads(changed.with_name(changed.stem.replace('-changes','-alias-sites')+'.json').read_text())
oracle_errors=[]
for row in aliases:
 case=row['key'][0].split('/')[0]
 if row['recoverable'] and ((row.get('options') or {}).get('moduleResolution')!=2 or row.get('ownership_barrier') or row.get('tsserver_disagreement')):
  oracle_errors.append((case,'recoverability outside Node/Node10 or past ownership barrier'))
 if case.startswith(('C01-','C56-','C72-')) and not row['recoverable']:
  oracle_errors.append((case,'ordinary/proven-empty positive lost'))
 if case.startswith('C17-') and row['recoverable']:
  oracle_errors.append((case,'NodeNext incorrectly recoverable'))
 for prefix,barrier in [('C45-','JSCONFIG_BARRIER'),('C73-','JSCONFIG_BARRIER'),('C46-','DELEGATED_CONFIG_BARRIER'),('C54-','DELEGATED_CONFIG_BARRIER')]:
  if case.startswith(prefix) and (row['recoverable'] or row.get('ownership_barrier')!=barrier):
   oracle_errors.append((case,'missing '+barrier))
 if case.startswith('C44-') and case.endswith('-tsx') and row['terminal']['class']!='ambiguous_star_diagnostic':
  oracle_errors.append((case,'TS2308 star conflict incorrectly certified'))
 for prefix,reason in [('C13-','DUPLICATE_CONFIG_KEY'),('C17-','MODULE_RESOLUTION_OUTSIDE_P1'),('C48-','OUTDIR_BARRIER'),('C55-','UNPROVEN_STAR_BRANCH'),('C57-','EXPORT_SYNTAX_DIAGNOSTIC'),('C64-','PACKAGE_FOLDER_MEMBERSHIP'),('C65-','WILDCARD_DECLARATION_PRIORITY'),('C66-','NONASCII_WILDCARD_MEMBERSHIP'),('C67-','CASE_VARIANT_MEMBERSHIP_SPEC'),('C68-','CASE_VARIANT_MEMBERSHIP_SPEC'),('C69-','CASE_VARIANT_CONFIG_NAME'),('C77-','QUESTION_PREFIX_INCLUDE')]:
  if case.startswith(prefix) and row.get('refusal_reason')!=reason:
   oracle_errors.append((case,'missing refusal reason '+reason))
 for prefix,project in [('C78-','pkg/jsconfig.json'),('C79-',None),('C80-','pkg/tsconfig.json')]:
  if case.startswith(prefix):
   expected=case+'/'+project if project else None
   if not row['tsserver_disagreement'] or row['config']!=expected:oracle_errors.append((case,'independent ProjectService ownership control failed'))
 if case.startswith(('C81-','C84-')) and row['refusal_reason']!='IMPORT_FORWARD_NOT_FORWARDABLE':oracle_errors.append((case,'written import forward reason missing'))
 if case.startswith(('C82-','C83-')) and row['refusal_reason']=='IMPORT_FORWARD_NOT_FORWARDABLE':oracle_errors.append((case,'unwritten import forward falsely classified'))
 if case.startswith(('C85-','C86-')) or (case=='C87-scenario-O' and row['key'][0].endswith(('app.tsx','b.jsx'))):
  if row['config'] is not None or row['target'] is not None or row['recoverable']:
   oracle_errors.append((case,'priority-dropped file incorrectly owns a configured alias'))
assert not oracle_errors, json.dumps({'oracle_contract_violations':oracle_errors})
assert len(classified)==len(changes)
assert all(c['class'] in ('CORRECT_STATIC_BINDING','CORRECT_STATIC_REFUSAL') or (c['class']=='UNPROVEN' and c['key'][0].startswith('C80-') and c['oracle']['tsserver_disagreement']) for c in classified), 'unexpected uncertified changed row'
raw_a,raw_b=raw_load(base),raw_load(head)
for k in a:
 if k[0].startswith('C87-scenario-O/') and k[0].endswith(('app.tsx','b.jsx')):
  assert raw_a[k]==raw_b[k], ('scenario O dropped caller', k)
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
print(json.dumps({'claim':'MEASURED','scenarios':len(json.loads((root/'manifest.json').read_text())),'sites':len(a),'changed':len(changes),'preservation_violations':0,'owner_question_unproven':sum(c['class']=='UNPROVEN' for c in classified)}))
