"""Complete public diagnostic controls; never consumes controller private inputs.

Usage: python3 gap-controls.py P1_BIN FACTS_BIN P2_BIN TS_JS
"""
import json, os, subprocess, sys
from pathlib import Path

packet=Path(__file__).resolve().parent
out=Path(os.environ.get('P2_GAP_CONTROL_OUT',str(packet.parents[4]/'target/p2-gap-controls'))).resolve()
assert not out.exists(), 'use a fresh public control directory'
root=out/'source';root.mkdir(parents=True)
p1,facts,p2,ts=map(lambda p:str(Path(p).resolve()),sys.argv[1:])
cases=[]
def write(d,files):
    for file,text in files.items():
        p=d/file;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
for grammar,member in [(g,m) for g in ['jsx','tsx'] for m in [False,True]]:
    shapes={
        'relative-positive':({'entry.ts':"export { real } from './real.js';\n"},'PASS'),
        'nonrelative':({'entry.ts':"export { real } from '@hop';\n"},'NONRELATIVE_HOP'),
        'entry-js-first-pass':({'entry.js':"export { real } from './real.js';\n"},'ENTRY_JS_FIRST_PASS'),
        'hop-js-first-pass':({'entry.ts':"export { real } from './real.js';\n"},'HOP_JS_FIRST_PASS'),
        'package-main':({'entry.ts':"export { real } from './folder';\n",'folder/package.json':'{"main":"real.js"}\n',
                         'folder/real.js':'export function real() { return null; }\n'},'PACKAGE_DIRECTORY_REDIRECT'),
        'index-positive':({'entry.ts':"export { real } from './folder';\n",'folder/index.js':'export function real() { return null; }\n'},'PASS'),
        'explicit-redirect':({'entry.ts':"export { real } from './folder.js';\n",'folder.js/index.js':'export function real() { return null; }\n'},'EXPLICIT_JS_LITERAL_UNAVAILABLE'),
        'directory-literal':({'entry.ts':"export { real } from './folder/';\n",'folder/index.js':'export function real() { return null; }\n'},'DIRECTORY_LITERAL'),
        'file-index-competition':({'entry.ts':"export { real } from './real';\n",'real/index.js':'export function real() { return 2; }\n'},'CANDIDATE_COMPETITION'),
        'depth-three':({'entry.ts':"export { real } from './a.js';\n",'a.js':"export { real } from './b.js';\n",'b.js':"export { real } from './real.js';\n"},'EXPORT_DEPTH_LIMIT'),
        'forward-arrow':({'entry.ts':"import { real } from './real.js'; export { real };\n",'real.js':'export const real = () => null;\n'},'IMPORT_FORWARDABILITY'),
        'star-hole':({'entry.ts':"export * from './real.js'; export * from './missing';\n"},'UNRESOLVED_STAR_BRANCH'),
        'star-competition-negative':({'entry.ts':"export { real } from './real.js'; export { real } from './real.js';\n"},'EXPORT_NAME_CONFLICT'),
        'default-expression':({'entry.ts':"export { default as real } from './real.js';\n",'real.js':'export default () => null;\n'},'EXPORT_NAME_MISSING'),
        'cycle-positive':({'entry.ts':"export * from './real.js'; export * from './a.js';\n",'a.js':"export * from './a.js';\n"},'UNRESOLVED_STAR_BRANCH'),
        'binding-write-negative':({'entry.ts':"export { real } from './real.js';\n",'real.js':'export let real = () => null;\nreal = () => null;\n'},'TERMINAL_SPAN_REQUIRED'),
        'explicit-entry-negative':({'entry.js':"export { real } from './real.js';\n"},'EXPLICIT_EXTENSION_PATH'),
        'declaration-negative':({'entry.ts':"export { real } from './real';\n",'real.d.ts':'export declare function real(): unknown;\n'},None),
        'allow-off-negative':({'entry.ts':"export { real } from './real.js';\n"},'ALLOW_JS_OFF'),
        'missing-negative':({'entry.ts':"export { real } from './missing';\n"},None),
        'ordered-substitution-negative':({'entry.ts':"export { real } from './real.js';\n"},None),
        'ambient-negative':({'entry.ts':"export { real } from './real.js';\n",'ambient.d.ts':"declare module '@lib' { export function real(): unknown; }\n"},None),
        'shadow-negative':({'entry.ts':"export { real } from './real.js';\n"},None),
        'unindexed-package-negative':({'entry.ts':"export { real } from './node_modules/pkg/real.js';\n",'node_modules/pkg/real.js':'export function real() { return null; }\n'},'SOURCE_NOT_INDEXED'),
    }
    for shape,(overrides,expected) in shapes.items():
        d=root/(grammar+'-'+shape+('-member' if member else ''));d.mkdir()
        entry='entry.js' if 'entry.js' in overrides else 'entry.ts'
        literal='entry.js' if shape=='explicit-entry-negative' else entry.removesuffix('.js') if entry.endswith('.js') else entry
        config={'compilerOptions':{'moduleResolution':'node10','allowJs':shape!='allow-off-negative','jsx':'preserve',
                 'paths':{'@lib':['./'+literal],'@hop':['./real']}},'include':['**/*']}
        if shape=='ordered-substitution-negative':config['compilerOptions']['paths']['@lib']=['./missing','./entry.ts']
        app="import { real as Real } from '@lib';\nexport function run() { Real(); return <Real />; }\n"
        if shape=='shadow-negative':app="import { real as Real } from '@lib';\nexport function run(Real) { Real(); return <Real />; }\n"
        files={'tsconfig.json':json.dumps(config)+'\n','app.'+grammar:app,'real.js':'export function real() { return null; }\n'}
        files.update(overrides)
        if member:
            for file,text in list(files.items()):
                if file.endswith('.js') and any(s in text for s in ['function real(', 'const real =', 'let real =']):
                    files[file]=text+"real.displayName = 'Real';\n"
        write(d,files)
        if shape=='ambient-negative':
            # Snapshot ambient fences are global to the repository, not config-local.
            for file in ['tsconfig.json','app.'+grammar,'ambient.d.ts']:
                p=d/file;p.write_text(p.read_text().replace('@lib','@ambient-control'))
        if shape=='entry-js-first-pass':(d/'node_modules/@lib.ts').mkdir(parents=True)
        if shape=='hop-js-first-pass':(d/'real.ts').mkdir()
        cases.append({'case':d.name,'shape':shape,'gate':expected,'member':member})
(out/'manifest.json').write_text(json.dumps(cases,indent=2)+'\n')
env=dict(os.environ,CORPUS_F_ROOT=str(root),PRIVATE_EVIDENCE_ROOT=str(out/'evidence'),TS_JS=ts)
with (out/'aggregate.json').open('w') as f:
    r=subprocess.run(['bash',str(packet/'CONTROLLER-p2-gap.sh'),p1,facts,p2],env=env,stdout=f)
assert r.returncode==0, 'public wrapper failed; inspect private public-control logs'
aggregate=json.loads((out/'aggregate.json').read_text());rows=json.loads((out/'evidence/gap-private-rows.json').read_text())
natives=json.loads((out/'evidence/F-native-rows.json').read_text());by_case={c['case']:c for c in cases}
checks=[]
for c in cases:
    ns=[r for r in natives if r['key'][0].split('/')[0]==c['case']]
    rs=[r for r in rows if r['key'][0].split('/')[0]==c['case']]
    outside={'explicit-entry-negative','unindexed-package-negative','allow-off-negative',
             'declaration-negative','missing-negative','ordered-substitution-negative','shadow-negative'}
    if c['shape'] in outside:
        assert not rs,(c,rs)
        assert not any(n['reason']=='JS_EXPORT_HOP' and n['native_callable'] and n['ownership_agrees'] for n in ns),(c,ns)
    elif c['gate'] and c['gate']!='PASS':
        assert len(rs)==2,(c,rs,[n['reason'] for n in ns])
        assert all(r['gate']==c['gate'] for r in rs),(c,rs)
    elif c['gate']=='PASS':
        assert not rs,(c,rs)
        assert all(n['native_callable'] and n['ownership_agrees'] for n in ns),(c,ns)
    checks.append({'shape':c['shape'],'grammar':c['case'].split('-')[0],'classified_rows':len(rs),'pass':True})
# Adversarial probe checks: evidence reuse, missing arguments, fact drift and
# export-table drift must yield no MEASURED stdout. No binary/source mutation.
def invoke(args,e):
    return subprocess.run(args,env=e,capture_output=True,text=True)
r=invoke(['bash',str(packet/'CONTROLLER-p2-gap.sh'),p1,facts,p2],env)
assert r.returncode and json.loads(r.stdout)=={'status':'INADMISSIBLE','stage':'evidence_directory_exists'}
r=invoke(['bash',str(packet/'CONTROLLER-p2-gap.sh')],env)
assert r.returncode and json.loads(r.stdout)=={'status':'INADMISSIBLE','stage':'arguments'}
facts_path=out/'evidence/import-facts.jsonl';original=facts_path.read_text()
frows=[json.loads(l) for l in original.splitlines()];frows[0]['hash']='0'*64
facts_path.write_text(''.join(json.dumps(r)+'\n' for r in frows))
r=invoke(['node',str(packet/'gap.cjs'),ts,str(root),str(out/'evidence')],env)
facts_path.write_text(original)
assert r.returncode and not r.stdout,'fact drift must refuse'
side_path=out/'evidence/gap-sidecar.json';original=side_path.read_text();side=json.loads(original)
for allow,files in side['path_exports'].items():
    for file,names in files.items():
        if 'forward-arrow' in file and 'real' in names:
            raise AssertionError('forward-arrow must not project')
    # Remove a root used by an unrecovered star-hole row, forcing replay disagreement.
    for file in list(files):
        if 'star-hole' in file:del files[file]
side_path.write_text(json.dumps(side))
r=invoke(['node',str(packet/'gap.cjs'),ts,str(root),str(out/'evidence')],env)
side_path.write_text(original)
assert r.returncode and not r.stdout,'export drift must refuse'
subprocess.run(['node',str(packet/'gap-guard-controls.cjs'),str(out/'evidence')],stdout=subprocess.DEVNULL,check=True)
guard=json.loads((out/'evidence/gap-guard-checks.json').read_text())
# A wrong binary and a changed dump must not reuse an earlier certificate.
bad_env=dict(env,PRIVATE_EVIDENCE_ROOT=str(out/'binary-drift-evidence'))
r=invoke(['bash',str(packet/'CONTROLLER-p2-gap.sh'),p1,facts,p1],bad_env)
assert r.returncode and json.loads(r.stdout)=={'status':'INADMISSIBLE','stage':'source_and_binary_binding'}
dump=out/'evidence/p2-sites.jsonl';original=dump.read_text()
dump.write_text(original+'\n')
r=invoke(['node',str(packet/'gap.cjs'),ts,str(root),str(out/'evidence')],env)
dump.write_text(original)
assert r.returncode and not r.stdout,'certificate/dump drift must refuse'
# Output strings and keys come only from a fixed catalog plus fixed counters.
serialized=json.dumps(aggregate)
for secret in [str(root),'app.jsx','app.tsx','real.js','@lib','@hop']:
    assert secret not in serialized, 'identifier leaked'
assert aggregate['population']['p2_unrecovered']==len(rows)
summary={'status':'PASS','scenarios':len(cases),'shape_checks':len(checks),'adversarial_checks':6,
         'guard_contract_checks':guard['guard_contract_checks'],
         'diagnosed_rows':len(rows),'classes':{k:v['rows'] for k,v in aggregate['classes'].items()},
         'aggregate_only_stdout':True}
(out/'checks.json').write_text(json.dumps(checks,indent=2)+'\n')
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary))
