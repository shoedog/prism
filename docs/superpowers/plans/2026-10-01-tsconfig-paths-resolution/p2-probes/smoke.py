"""READ: Exercise the measurement instruments on synthetic public inputs in both grammars.
These are probe checks, not P2 product controls or a production prototype.
"""
import json, os, subprocess
from pathlib import Path

PROBES = Path(__file__).resolve().parent
HERE = PROBES.parents[4]
OUT = HERE/'target/p2-plan/smoke'
ROOT = OUT/'source'
ROOT.mkdir(parents=True, exist_ok=True)
TS = Path.home()/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
P1 = Path.home()/'code/prism-paths-impl/target/repair-r5/head/prism'
FACTS = Path.home()/'code/prism-paths-impl/target/repair-r1/base/dump_imports'

expected = {}
for grammar in ['ts', 'tsx']:
    for kind in ['named', 'star', 'forward', 'explicit-js', 'js-to-ts', 'declaration', 'allow-off', 'shadow', 'star-hole', 'wrapped', 'explicit-path-js']:
        case = f'{grammar}-{kind}'
        d=ROOT/case; d.mkdir(exist_ok=True)
        cfg={'compilerOptions':{'moduleResolution':'node10','allowJs':kind!='allow-off','paths':{'@p':['./barrel.'+grammar]}}}
        if kind=='explicit-path-js': cfg['compilerOptions']['paths']['@p']=['./leaf.js']
        (d/'tsconfig.json').write_text(json.dumps(cfg))
        barrel = 'export { leaf } from "./leaf";\n'
        if kind=='star': barrel='export * from "./leaf";\n'
        if kind=='star-hole': barrel='export * from "./leaf"; export * from "./missing";\n'
        if kind=='forward': barrel='import { leaf } from "./leaf"; export { leaf };\n'
        if kind in ['explicit-js','js-to-ts']: barrel='export { leaf } from "./leaf.js";\n'
        (d/f'barrel.{grammar}').write_text(barrel)
        (d/'leaf.js').write_text('export function leaf() { return 1; }\n')
        if kind=='js-to-ts': (d/'leaf.ts').write_text('export function leaf() { return 2; }\n')
        if kind=='declaration': (d/'leaf.d.ts').write_text('export declare function leaf(): number;\n')
        if kind=='explicit-path-js': (d/'leaf.d.ts').write_text('export declare function leaf(): number;\n')
        source='import { leaf } from "@p";\nexport function call() { return leaf(); }\n'
        if kind=='shadow': source='import { leaf } from "@p";\nexport function call(leaf: () => number) { return leaf(); }\n'
        if kind=='wrapped':
            (d/'react.d.ts').write_text('declare module "react" { export function memo<T>(x: T): T; }\n')
            (d/'leaf.js').write_text('import { memo } from "react"; export const leaf = memo(function body() { return null; });\n')
            if grammar=='tsx': source='import { leaf as Leaf } from "@p";\nexport function call() { return <Leaf />; }\n'
        (d/f'caller.{grammar}').write_text(source)
        expected[case] = kind

env=dict(os.environ, CORPUS_F_ROOT=str(ROOT), PRIVATE_EVIDENCE_ROOT=str(OUT/'private'), TS_JS=str(TS))
with (OUT/'aggregate.json').open('w') as stdout, (OUT/'wrapper.stderr').open('w') as stderr:
    subprocess.run(['bash',str(PROBES/'CONTROLLER-p2.sh'),str(P1),str(FACTS)],env=env,stdout=stdout,stderr=stderr,check=True)
aggregate=json.load(open(OUT/'aggregate.json'))
rows=json.load(open(OUT/'private/F-native-rows.json'))
assert len(rows)==22, len(rows)
checks=[]
for r in rows:
    case=r['key'][0].split('/')[0];kind=expected[case]
    if kind in ['named','star','forward','explicit-js']:
        assert r['old_recoverable'] and r['terminal_agrees'] and r['ownership_agrees'],r
        assert r['js_hop_count']==1 and r['allow_js'] and r['hop_stops']==0,r
    elif kind=='js-to-ts':
        assert r['old_recoverable'] and r['terminal']['file'].endswith('/leaf.ts'),r
        assert r['js_hop_count']==0,r
    elif kind=='declaration':
        assert not r['old_recoverable'] and not r['native_callable'],r
        assert not r['terminal_agrees'],r
    elif kind=='allow-off':
        assert not r['allow_js'],r
    elif kind=='shadow':
        assert r['site_import_proof']!='import' and not r['native_callable'],r
    elif kind=='star-hole':
        assert r['old_recoverable'] and r['js_hop_count']==1 and r['hop_stops']>=1,r
    elif kind=='wrapped':
        assert r['terminal']['class']=='wrapped_function',r
        if case.startswith('tsx-'): assert r['old_recoverable'] and r['terminal_agrees'] and r['native_callable'],r
        else: assert not r['old_recoverable'] and not r['native_callable'],r
    elif kind=='explicit-path-js':
        assert r['entry']['target'].endswith('/leaf.js') and r['entry']['js_family'],r
        assert not r['entry']['js_secondary'] and r['terminal_agrees'],r
    checks.append({'case':case,'kind':kind,'status':'PASS'})
assert str(ROOT) not in (OUT/'aggregate.json').read_text()
assert 'caller.' not in (OUT/'aggregate.json').read_text()
assert 'leaf.' not in (OUT/'aggregate.json').read_text()
(OUT/'checks.json').write_text(json.dumps({'claim':'MEASURED','checks':checks,'passed':len(checks),'failed':0,'aggregate_only_stdout':True},indent=2)+'\n')
print(json.dumps({'claim':'MEASURED','passed':len(checks),'failed':0,'aggregate_only_stdout':True}))
