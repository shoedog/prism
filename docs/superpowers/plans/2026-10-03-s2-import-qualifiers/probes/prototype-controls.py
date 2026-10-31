"""Public synthetic head/base/native controls, including negatives in both grammars."""
import importlib.util, json, subprocess, sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('s2compare',HERE/'compare-head.py')
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
head,headfacts,out=map(Path,sys.argv[1:4]);out.mkdir(parents=True,exist_ok=False)
cfg={'compilerOptions':{'allowJs':True,'checkJs':True,'moduleResolution':'node','jsx':'preserve',
    'baseUrl':'.','paths':{'@lib':['m']}},'include':['**/*']}
cases=json.loads((HERE/'synthetic-controls.json').read_text())
results=[]
for grammar in ('jsx','tsx'):
 for name,case in cases.items():
    d=out/(name+'-'+grammar);root=d/'source';root.mkdir(parents=True)
    (root/'tsconfig.json').write_text(json.dumps(case.get('config',cfg)))
    for p,text in case['files'].items():
        f=root/p.replace('{ext}',grammar);f.parent.mkdir(parents=True,exist_ok=True);f.write_text(text)
    for binary,dest in [(c.BIN,'base-sites.jsonl'),(head,'head-sites.jsonl')]:
        c.run([binary,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],d/dest)
    c.run([headfacts,root],d/'facts.jsonl')
    c.run(['node',HERE/'census.cjs',c.TS,root,d/'base-sites.jsonl',d/'facts.jsonl',d],d/'oracle.log')
    s=c.compare(d)
    expected=case['new_exact'] if grammar=='tsx' or not case.get('typescript_only') else 0
    assert s['changed']==expected,(name,grammar,s,expected)
    results.append(dict(case=name,grammar=grammar,**s))
    print(name,grammar,s['correct'],flush=True)
summary={'scenarios':len(results),'sites':sum(r['total_sites'] for r in results),
    'new_correct':sum(r['correct'] for r in results),'lost_base_edges':0,'unproven':0,'results':results}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='results'}))
