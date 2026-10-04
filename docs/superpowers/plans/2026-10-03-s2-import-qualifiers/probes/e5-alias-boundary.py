"""Admission control: known object alias writes must keep base even if TS binds the declaration."""
import importlib.util, json, subprocess, sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('s2compare',HERE/'compare-head.py')
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
head,headfacts,out=map(Path,sys.argv[1:4]);out.mkdir(parents=True,exist_ok=False)
reports=[]
for grammar in ('jsx','tsx'):
    d=out/grammar;root=d/'source';root.mkdir(parents=True)
    (root/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'allowJs':True,'jsx':'preserve','moduleResolution':'node'},'include':['**/*']}))
    (root/('m.'+grammar)).write_text('export class C { static sm() { return 0; } }\nconst Alias = C; function replacement() { return 1; } Alias.sm = replacement;\n')
    (root/('app.'+grammar)).write_text('import { C as X } from "./m"; export function run() { X.sm(); }\n')
    for binary,file in [(c.BIN,'base-sites.jsonl'),(head,'head-sites.jsonl')]:
        c.run([binary,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],d/file)
    c.run([headfacts,root],d/'facts.jsonl')
    c.run(['node',HERE/'census.cjs',c.TS,root,d/'base-sites.jsonl',d/'facts.jsonl',d],d/'oracle.log')
    static=c.compare(d)
    before=[r for r in c.read(d/'base-sites.jsonl') if r.get('record_kind')=='call_site']
    after=[r for r in c.read(d/'head-sites.jsonl') if r.get('record_kind')=='call_site']
    kept=before==after
    reports.append({'grammar':grammar,'classification':'PASS' if kept else 'WRONG_E5_OBJECT_ALIAS','confidence':100,
                    'oracle_changed_correct':static['correct'],'expected':before,'actual':after})
runtime=json.loads(subprocess.check_output(['node','--input-type=module','-e',
    'class C { static sm() { return 0; } } const Alias=C; function replacement() { return 1; } Alias.sm=replacement; console.log(JSON.stringify({sameObject:Alias===C,result:C.sm()}));'],text=True))
assert runtime=={'sameObject':True,'result':1},runtime
report={'classification':'PASS' if all(r['classification']=='PASS' for r in reports) else 'WRONG',
        'runtime_identity_control':runtime,'results':reports,'frozen_binaries':{r:__import__('hashlib').sha256(p.read_bytes()).hexdigest() for r,p in [('base',c.BIN),('head',head),('facts',headfacts)]}}
(out/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'classification':report['classification'],'scenarios':len(reports),'runtime':runtime,
                  'new_wrong_admissions':sum(r['oracle_changed_correct'] for r in reports)}))
sys.exit(0 if report['classification']=='PASS' else 1)
