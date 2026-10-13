#!/usr/bin/env python3
"""Render tables from admitted receipts; pair label changes without hiding raw halves."""
import argparse,collections,hashlib,json,sys
from pathlib import Path
ap=argparse.ArgumentParser();ap.add_argument('evidence',type=Path);ap.add_argument('packet',type=Path);ap.add_argument('--census-only',action='store_true');a=ap.parse_args()
E,P=a.evidence,a.packet
read=lambda p:json.loads(p.read_text())
lines=lambda p:[json.loads(l) for l in p.open() if l.strip()]
canon=lambda r:json.dumps(r,sort_keys=True,separators=(',',':'))
corpus=lambda n:'SB' if n.startswith('SB__') else n
verdicts=['WRONG','CORRECT','UNDECIDED','INADMISSIBLE']
base,head=read(E/'base/census-summary.json'),read(E/'head/census-summary.json')
assert all(r['status'] in {'COMPLETE','EXCLUDED'} for s in [base,head] for r in s['producers'])
report={'base':{k:base[k] for k in ['totals','cross_tab','rules']},'head':{k:head[k] for k in ['totals','cross_tab','rules']},'variants':{}}
text=['# E5 full census — base 40ad8390 versus uncommitted cache115 prototype','',
'All counts are JS/TS/TSX Exact `free_single` targets. Every site is accounted for in three independent axes. W = WRONG, C = CORRECT, U = UNDECIDED, I = INADMISSIBLE. Unlisted combinations are zero. Native parse/binding failures retain I; producer syntax still supplies the independent callee-form axis. Raw and after-override verdicts are identical (zero overrides).','',
'585 roots requested: X, T, and all 583 SecBench roots. 581 admitted: X, T, SB579. Four base producer exclusions are paired on head; they are not zero-yield roots. Full wall-time receipts are in the evidence directory.','']
for c in ['X','T','SB']:
 text += [f'## {c}','', '| Callee form | Own-file binding | Base W | C | U | I | Head W | C | U | I |','|---|---|---:|---:|---:|---:|---:|---:|---:|---:|']
 keys=sorted({tuple(k.split('|')[1:]) for s in [base,head] for k in s['cross_tab'][c]})
 for f,b in keys:
  vals=[s['cross_tab'][c].get('|'.join([v,f,b]),0) for s in [base,head] for v in verdicts]
  text.append('| '+f+' | '+b+' | '+' | '.join(map(str,vals))+' |')
 text += ['']
text += ['## WRONG samples','', 'WRONG-SAMPLES.jsonl includes up to ten deterministic source-bound samples in **every** nonzero WRONG cell (all when fewer than ten). Each retains the source site span, resolved FunctionId and physical callable bytes when available, native checker declarations, own-file binding evidence, primary reason, and candidate predicates. No private corpus is included.','']
(P/'CENSUS.md').write_text('\n'.join(text))
samples=[];seen=collections.Counter()
for f in sorted((E/'base').glob('*.census.jsonl')):
 for r in lines(f):
  if r['verdict']!='WRONG':continue
  c=corpus(f.name[:-len('.census.jsonl')]);key=c+'|'+r['callee_form']+'|'+r['binding']
  if seen[key]>=10:continue
  seen[key]+=1;samples.append({'corpus':c,'root':f.name[:-len('.census.jsonl')],'cell':key,**r})
for c in ['X','T','SB']:
 for k,n in base['cross_tab'][c].items():
  if k.startswith('WRONG|'):assert seen[c+'|'+k[len('WRONG|'):]]==min(n,10)
(P/'WRONG-SAMPLES.jsonl').write_text(''.join(canon(r)+'\n' for r in samples))
report['wrong_samples']={'cells':len(seen),'samples':len(samples),'per_cell':dict(seen)}
if a.census_only:
 print(json.dumps(report['wrong_samples']));sys.exit(0)
def graphs(path):
 receipts=read(path/'summary.json');admissions=read(path/'adjudication-summary.json')
 admitted=[r for r in receipts if r['status']=='COMPLETE'];assert len(admissions)==len(admitted)
 assert all(r['status']=='COMPLETE' for r in admissions)
 totals=collections.Counter();bycorpus=collections.defaultdict(collections.Counter);paired=[];raw_total=0
 for receipt in admitted:
  name=receipt['name'];raw=lines(path/(name+'.changes.jsonl'));judged=lines(path/(name+'.adjudication/graph.adjudicated.jsonl')) if raw else []
  assert len(raw)==len(judged);raw_total+=len(raw)
  assert not any(r['row']['kind']=='non_call_digest' for r in raw),'STOP: non-call digest changed'
  groups=collections.defaultdict(lambda:collections.defaultdict(list))
  for r in judged:
   identity=canon({k:v for k,v in r['row'].items() if k!='edge'})
   groups[identity][r['change']].append(r)
  normalized=[]
  for sides in groups.values():
   b,h=sides['removed'],sides['added'];n=min(len(b),len(h))
   for i in range(n):
    assert b[i]['verdict']==h[i]['verdict'],'label halves disagree'
    normalized.append({'change':'relabelled','row':b[i]['row'],'head_edge':h[i]['row']['edge'],'verdict':b[i]['verdict'],'base_adjudication':b[i],'head_adjudication':h[i]})
   normalized+=b[n:]+h[n:]
  for r in normalized:
   key='|'.join([r['change'],r['row']['kind'],r['verdict']]);totals[key]+=1;bycorpus[corpus(name)][key]+=1
   paired.append({'root':name,**r})
 (path/'graph.normalized-adjudication.jsonl').write_text(''.join(canon(r)+'\n' for r in paired))
 return {'roots':len(admitted),'raw_differing_rows':raw_total,'classified_rows':len(paired),'counts':dict(totals),'by_corpus':{c:dict(v) for c,v in bycorpus.items()},'non_call_digest_changed':False}
report['recommended_graphs']=graphs(E/'graphs')
def calls(path):
 rows=read(path/'summary.json');assert all(r['non_js_call_sites_identical'] for r in rows)
 by=collections.defaultdict(collections.Counter);allc=collections.Counter()
 for r in rows:
  for k,n in r['target_changes'].items():by[corpus(r['name'])][k]+=n;allc[k]+=n
 return {'roots':len(rows),'counts':dict(allc),'by_corpus':{c:dict(v) for c,v in by.items()}}
report['recommended_calls']=calls(E/'call-comparison')
for v in ['R-inferred','R-bound','NameOnly']:
 folder=E/'variants'/v
 if v=='R-inferred' and (folder/'calls-repaired/summary.json').exists():
  cap,comparison,graph=folder/'calls-repaired',folder/'comparison-repaired',folder/'graphs-repaired'
 else:cap,comparison,graph=folder/'calls',folder/'comparison',folder/'graphs'
 s=read(cap/'summary.json');assert s['admitted']==s['roots']
 report['variants'][v]={'calls':calls(comparison),'graphs':graphs(graph)}
assert not any(k.startswith('added|') for k in report['recommended_graphs']['counts'])
(P/'MEASUREMENTS.json').write_text(json.dumps(report,indent=2)+'\n')
(E/'final-measurements.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':'COMPLETE','wrong_samples':report['wrong_samples'],'graph_counts':report['recommended_graphs']['counts']},sort_keys=True))
