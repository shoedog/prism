#!/usr/bin/env python3
"""Controller regression (F STOP 2026-10-05): a collapsed zero-width Use endpoint whose line also
holds same-text NON-reference identifiers (JSX attribute name, member property, object key) must
adjudicate CORRECT; a genuinely mixed binding (two references binding differently) stays UNDECIDED."""
import json, subprocess, sys, tempfile
from pathlib import Path
E=Path.home()/'prism-evidence/js-param-defs/repair-r1'
TS=Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
HEAD=E/'bin/prism-head-r1-bytes'; P=Path(__file__).resolve().parent
def check(name, fname, src, line, expect):
    d=Path(tempfile.mkdtemp(prefix=f'pd-{name}-')); (d/fname).write_text(src)
    raw=subprocess.run([str(HEAD),str(d)],capture_output=True,text=True,check=True).stdout
    rows=[r for r in map(json.loads,raw.splitlines()) if r['from']['parameter'] and r['from']['path']['base']=='x' and r['to']['line']==line]
    assert rows,(name,'no row on line',line)
    v=rows[0];v['to']['start_byte']=v['to']['end_byte']=0
    c={'class':'CONTROL','row':{k:w for k,w in v.items() if k not in ['confidence','doubt','kill_line']},
       'head_label':{k:v.get(k) for k in ['confidence','doubt','kill_line']}}
    (d/'changed.jsonl').write_text(json.dumps(c)+'\n')
    subprocess.run(['node',str(P/'adjudicate.cjs'),str(TS),str(d),str(d/'changed.jsonl'),str(d/'adj.json')],check=True,capture_output=True)
    a=json.loads((d/'adj.json').read_text()); key=[k for k in a if k.startswith('CONTROL|def->use|')]
    ok=any(k.startswith(f'CONTROL|def->use|{expect}') for k in key)
    print(name, key, 'PASS' if ok else 'FAIL'); return ok
res=[check('jsx-attr','case.tsx','const f = x =>\n  <C x={x} />;\n',2,'CORRECT'),
     check('member-prop','case.ts','const f = x =>\n  o.x + x;\n',2,'CORRECT'),
     check('obj-key','case.ts','const f = x =>\n  ({ x: x });\n',2,'CORRECT'),
     check('mixed-refs','case.ts','function outer(x) {\n { let x=other; use(x); } use(x);\n}\n',2,'UNDECIDED')]
sys.exit(0 if all(res) else 1)
