#!/usr/bin/env python3
"""Adjudicate captured roots, watching a live capture without executing packages."""
import argparse,concurrent.futures,hashlib,json,subprocess,time
from collections import Counter
from pathlib import Path
ap=argparse.ArgumentParser();ap.add_argument('capture',type=Path);ap.add_argument('oracle',type=Path);ap.add_argument('--watch',action='store_true');ap.add_argument('--facts-dir',type=Path);a=ap.parse_args();worker=Path(__file__).with_name('census-e5.cjs');seen=set();statuses=[]
while True:
    pending=[]
    for p in sorted(a.capture.glob('*.status.json')):
        if '.facts.' in p.name: continue
        s=json.loads(p.read_text())
        if s['name'] in seen:continue
        seen.add(s['name'])
        if s['exit']==0:
            s['all_languages_free_single_exact']=s.get('free_single_exact',0)
            s['free_single_exact']=sum(1 for line in (a.capture/(s['name']+'.sites.jsonl')).open() if (lambda r:r.get('record_kind')=='call_site' and Path(r['caller']['file']).suffix in {'.js','.jsx','.mjs','.cjs','.ts','.tsx'} and any(t['kind']=='free_single' and t['confidence']=='exact' for t in r['resolved_targets']))(json.loads(line)))
        pending.append(s)
    def one(s):
        out=a.capture/(s['name']+'.census.jsonl');start=time.monotonic()
        if s['exit']!=0:r={'name':s['name'],'status':'EXCLUDED','reason':s['reason'],'seconds':0}
        elif not s.get('free_single_exact',0):
            out.write_text('');out.with_suffix(out.suffix+'.summary.json').write_text(json.dumps({'total':0,'cross_tab':{},'rules':{}})+'\n');r={'name':s['name'],'status':'COMPLETE','total':0,'seconds':0}
        else:
            with out.with_suffix(out.suffix+'.oracle.stdout').open('wb') as o,out.with_suffix(out.suffix+'.oracle.stderr').open('wb') as e:
                facts=(a.facts_dir or a.capture)/(s['name']+'.facts.jsonl')
                command=['node','--max-old-space-size=8000',str(worker),str(a.oracle),s['root'],str(a.capture/(s['name']+'.sites.jsonl')),str(out)]
                if facts.exists():command.extend(['--facts',str(facts)])
                try:rc=subprocess.run(command,stdout=o,stderr=e,timeout=1500).returncode
                except subprocess.TimeoutExpired:rc=124
            r={'name':s['name'],'status':'COMPLETE' if rc==0 else 'INADMISSIBLE','exit':rc,'seconds':round(time.monotonic()-start,3)}
            if rc==0:
                summary=json.loads(out.with_suffix(out.suffix+'.summary.json').read_text());assert summary['total']==s['free_single_exact'];r['total']=summary['total']
        (a.capture/(s['name']+'.oracle-status.json')).write_text(json.dumps(r,indent=2)+'\n');return r
    with concurrent.futures.ThreadPoolExecutor(2) as pool:
        for r in pool.map(one,pending):statuses.append(r);print(json.dumps(r),flush=True)
    if (a.capture/'summary.json').exists() or not a.watch:break
    time.sleep(5)
aggregate={};samples={};for_rule={};summary_totals=Counter()
for status in statuses:
    if status['status']!='COMPLETE':continue
    corpus='SB' if status['name'].startswith('SB__') else status['name'];aggregate.setdefault(corpus,Counter());for_rule.setdefault(corpus,{})
    rows=[json.loads(l) for l in (a.capture/(status['name']+'.census.jsonl')).read_text().splitlines()]
    for row in rows:
        key='|'.join([row['verdict'],row['callee_form'],row['binding']]);aggregate[corpus][key]+=1;summary_totals[corpus]+=1
        if row['verdict']=='WRONG':
            sk=corpus+'|'+key;samples.setdefault(sk,[])
            if len(samples[sk])<10:samples[sk].append({'root':status['name'],'caller':row['site']['source_span'],'callee':row['target'],'invocation':row.get('invocation'),'reason':row['why']})
        for name,touched in row.get('rules',{}).items():
            for_rule[corpus].setdefault(name,Counter())
            if touched:for_rule[corpus][name][row['verdict']]+=1
summary={'totals':dict(summary_totals),'cross_tab':{c:dict(sorted(v.items())) for c,v in aggregate.items()},'rules':{c:{r:dict(v) for r,v in rs.items()} for c,rs in for_rule.items()},'producers':statuses,'worker_sha256':hashlib.sha256(worker.read_bytes()).hexdigest(),'oracle_sha256':hashlib.sha256(a.oracle.read_bytes()).hexdigest()}
(a.capture/'census-summary.json').write_text(json.dumps(summary,indent=2)+'\n');(a.capture/'wrong-samples.json').write_text(json.dumps(samples,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ['totals','cross_tab','rules']}),flush=True)
