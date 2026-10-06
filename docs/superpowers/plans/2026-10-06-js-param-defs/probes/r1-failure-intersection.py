#!/usr/bin/env python3
"""Run actual old/new aggregation code on owned producer-output fixtures only.
Expected: old admits failed roots (RED); new excludes both sides and retains good
roots. Partial nonempty output distinguishes failure exclusion from empty-row
coincidence. No corpus package or package code is accessed or executed.
"""
import json, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[5]
P=Path(__file__).resolve().parent
E=Path.home()/'prism-evidence/js-param-defs/repair-r1'
OUT=E/'failure-intersection-controls';OUT.mkdir(exist_ok=True)
old=subprocess.check_output(['git','show','6b0be4ff:docs/superpowers/plans/2026-10-06-js-param-defs/probes/measure-secbench-rows.sh'],cwd=ROOT).decode()
new=(P/'measure-secbench-rows.sh').read_text()
marker="python3 - \"$OUT\" <<'PY'\n"
def aggregation(shell):return shell.split(marker,1)[1].split("\nPY\n",1)[0]
row={'from':{'file':'case.js','line':1,'access':'def','path':{'base':'x','fields':[]}},'to':{'file':'case.js','line':2,'access':'use','path':{'base':'x','fields':[]}},'confidence':'exact','doubt':None}
summary={}
for case,side,operation in [('base_bytes','base','dfg'),('head_partial','head','dfg'),('head_sites','head','sites')]:
 observed={}
 for label,shell in [('old',old),('new',new)]:
  out=OUT/case/label;per=out/'per';per.mkdir(parents=True,exist_ok=True)
  (out/'roots.txt').write_text('owned/good\nowned/failed\n')
  (out/'failures.txt').write_text(f'owned/failed {side} {operation}_failed 124\n')
  for rel in ['owned/good','owned/failed']:
   for s in ['base','head']:
    payload=json.dumps(row)+'\n'
    if rel=='owned/failed' and s=='base' and case=='base_bytes':payload=''
    (per/f"{rel.replace('/','__')}.{s}.dfg").write_text(payload)
    (per/f"{rel.replace('/','__')}.{s}.sites").write_text('partial' if rel=='owned/failed' and s==side and operation=='sites' else '[]')
  result=subprocess.run([sys.executable,'-',str(out)],input=aggregation(shell),text=True,capture_output=True,check=True)
  report=json.loads(result.stdout)
  passed=report['packages']==1 and report.get('excluded_packages')==['owned/failed']
  observed[label]={'report':report,'exclusion_assertion':passed}
  if label=='new':
   assert passed,(case,report)
   for s in ['base','head']:
    rows=(out/f'{s}.dfg.jsonl').read_text();assert 'owned/failed' not in rows and 'owned/good' in rows
  else:assert not passed,(case,'old code unexpectedly excludes failed root')
 # Active byte driver consumer: successful partial site/row files do not admit
 # a failed status. The good root uses equal bytes, avoiding checker invocation.
 capture=OUT/case/'byte';per=capture/'per';per.mkdir(parents=True,exist_ok=True)
 (capture/'binding.json').write_text(json.dumps({'roots':{'owned/good':str(capture),'owned/failed':str(capture)}}))
 for rel in ['owned/good','owned/failed']:
  for s in ['base','proto','head']:
   failed=rel=='owned/failed' and s==side
   ops={'bytes':{'exit':124 if failed and operation=='dfg' else 0}}
   if s!='proto':ops['sites']={'exit':124 if failed and operation=='sites' else 0}
   tag=rel.replace('/','__')
   (per/f'{tag}.{s}.status.json').write_text(json.dumps({'operations':ops}))
   (per/f'{tag}.{s}.bytes.jsonl').write_text(json.dumps(row)+'\n')
   (per/f'{tag}.{s}.sites.jsonl').write_text('[]')
 subprocess.run([sys.executable,str(P/'adjudicate-captures.py'),str(capture)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
 actual=json.loads((capture/'adjudication/summary.json').read_text())
 assert actual['excluded']==['owned/failed'] and list(actual['corpora'])==['owned/good'],actual
 observed['byte']={'excluded':actual['excluded'],'admitted':list(actual['corpora'])}
 summary[case]=observed
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({'old_expected_failures':3,'new_shell_passes':3,'active_byte_passes':3}))
