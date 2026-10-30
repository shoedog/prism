"""Public F-shaped controls, independently bound by pinned ProjectService/checker."""
import itertools, json, os, subprocess, sys
from pathlib import Path
from compare import load
probes=Path(__file__).resolve().parent
out=Path('target/p2-plan/p2-controls').resolve();root=out/'source';root.mkdir(parents=True,exist_ok=True)
head=Path(sys.argv[1]).resolve()
p1=Path.home()/'code/prism-paths-impl/target/repair-r5/head/prism'
facts=Path.home()/'code/prism-paths-impl/target/repair-r1/base/dump_imports'
ts=Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
manifest=[]
for grammar,route,spelling,member,competitor in itertools.product(['jsx','tsx'],['named','star','forward'],['implicit','explicit'],[False,True],[None,'ts','d.ts']):
    case=f'{grammar}-{route}-{spelling}-{int(member)}-{competitor or "absent"}'
    d=root/case;d.mkdir(parents=True,exist_ok=True)
    suffix='js' if route=='star' else 'jsx'
    spec='./real'+('.'+suffix if spelling=='explicit' else '')
    if route=='named': barrel=f"export {{ real }} from '{spec}';\n"
    elif route=='star': barrel=f"export * from '{spec}';\n"
    else: barrel=f"import {{ real }} from '{spec}'; export {{ real }};\n"
    (d/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'paths':{'@lib':['./entry.ts']}},'include':['**/*']})+'\n')
    (d/'entry.ts').write_text("export { real } from './barrel.js';\n")
    (d/'barrel.js').write_text(barrel)
    (d/('real.'+suffix)).write_text('export function real() { return null; }\n'+("real.displayName = 'Real';\n" if member else ''))
    if competitor: (d/('real.'+competitor)).write_text('export declare function real(): unknown;\n' if competitor=='d.ts' else 'export function real() { return null; }\n')
    (d/('app.'+grammar)).write_text("import { real as Real } from '@lib';\nexport function run() { Real(); return <Real />; }\n")
    manifest.append({'case':case,'grammar':grammar,'route':route,'spelling':spelling,'member':member,'competitor':competitor,'gain':competitor is None})
for grammar,member,route in itertools.product(['jsx','tsx'],[False,True],['named','star']):
    case=f'{grammar}-arrow-{route}-{int(member)}';d=root/case;d.mkdir(parents=True,exist_ok=True)
    (d/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'paths':{'@lib':['./entry.ts']}},'include':['**/*']})+'\n')
    (d/'entry.ts').write_text("export { real } from './real.js';\n" if route=='named' else "export * from './real.js';\n")
    (d/'real.js').write_text('export const real = () => null;\n'+("real.displayName = 'Real';\n" if member else ''))
    (d/('app.'+grammar)).write_text("import { real as Real } from '@lib';\nexport function run() { Real(); return <Real />; }\n")
    manifest.append({'case':case,'grammar':grammar,'route':route,'member':member,'gain':True})
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
# Exercise the exact private wrapper on our synthetic root; stdout must stay aggregate-only.
env=dict(os.environ,CORPUS_F_ROOT=str(root),PRIVATE_EVIDENCE_ROOT=str(out/'evidence'),TS_JS=str(ts))
with (out/'aggregate.json').open('w') as f:
    subprocess.run(['bash',str(probes/'CONTROLLER-p2.sh'),str(p1),str(facts),str(head)],env=env,stdout=f,check=True)
a=load(out/'evidence/p1-sites.jsonl');b=load(out/'evidence/p2-sites.jsonl')
changed=0
for case in manifest:
    ks=[k for k in a if k[0].split('/')[0]==case['case']]
    assert len(ks)==2,(case,len(ks))
    for k in ks:
        assert (a[k]!=b[k]) == case['gain'], (case,k,a[k],b[k])
        changed+=a[k]!=b[k]
agg=json.loads((out/'aggregate.json').read_text())
assert agg['prototype']['changed']==changed
assert agg['prototype']['classes']=={'CORRECT_STATIC_BINDING':changed}
assert str(root) not in (out/'aggregate.json').read_text()
summary={'scenarios':len(manifest),'sites':len(a),'changed':changed,'classes':agg['prototype']['classes'],'aggregate_only_stdout':True}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary))
