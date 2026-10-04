"""Closed identity-use controls: every escape/write keeps the complete base row."""
import importlib.util, json, subprocess, sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('s2compare',HERE/'compare-head.py')
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
head,headfacts,out=map(Path,sys.argv[1:4]);out.mkdir(parents=True,exist_ok=False)
uses = {
    's2-w1': 'const Alias = Q; function replacement() { return 1; } Alias.sm = replacement;',
    'alias-assignment': 'let A; A = Q;',
    'destructure': 'const { sm } = Q;',
    'destructure-yields-identity': 'const { value: A } = { value: Q }; A.sm = other;',
    'argument': 'consume(Q);', 'return': 'function escape() { return Q; }',
    'property': 'const holder = { value: Q };', 'shorthand': 'const holder = { Q };',
    'array': 'const holder = [Q];', 'map': 'new Map([[0, Q]]);',
    'spread-object': 'const holder = { ...Q };', 'spread-argument': 'consume(...Q);',
    'member-write': 'Q.sm = other;', 'computed-write': 'Q[key] = other;',
    'assign': 'Object.assign(Q, {});', 'define-property': "Object.defineProperty(Q, 'sm', {});",
    'reflect': "Reflect.get(Q, 'sm');", 'computed-call': 'Q[key]();',
    'member-value': 'const method = Q.sm;', 'renamed-export': 'export { Q as Renamed };',
    'default-plus-use': 'export default Q; Q.sm();',
}
cases={}
for name, use in uses.items():
 for place in ('provider', 'caller', 'importer', 'forwarder'):
    files={'m.{ext}':'export class C { static sm() { return 0; } }\n',
           'app.{ext}':'import { C as X } from "./m"; export function run() { X.sm(); }\n'}
    if place=='provider': files['m.{ext}']+=use.replace('Q','C')
    elif place=='caller': files['app.{ext}']+=use.replace('Q','X')
    else: files['other.{ext}']='import { C as Q } from "./m"; '+use+(' export { Q };' if place=='forwarder' else '')
    cases[name+'-'+place]=files
for name, use in {
    'source-renamed-export': 'export { C as D } from "./m";',
    'namespace-alias': 'import * as ns from "./m"; const A=ns.C; A.sm=other;',
    'namespace-member-write': 'import * as ns from "./m"; ns.C.sm=other;',
    'dynamic-destructure': 'export async function mutate(){const { C: A } = await import("./m"); A.sm=other;}',
    'dynamic-shorthand': 'export async function mutate(){const { C } = await import("./m"); C.sm=other;}',
    'dynamic-property': 'export async function mutate(){const A = (await import("./m")).C; A.sm=other;}',
    'unproved-owner': 'import { C as Q } from "../m"; consume(Q);',
}.items():
    files={'m.{ext}':'export class C { static sm() { return 0; } }\n',
           'app.{ext}':'import { C as X } from "./m"; export function run() { X.sm(); }\n'}
    files[('outside/' if name=='unproved-owner' else '')+'other.{ext}']=use
    if name=='unproved-owner': files['outside/tsconfig.json']='{"compilerOptions":{"noResolve":true},"include":["**/*"]}'
    cases[name]=files
for name, provider in {
    'object-alias':'export const C={sm(){}}; const A=C; A.sm=other;',
    'class-this-escape':'export class C { static sm() { return this; } }',
    'class-self-member-write':'export class C { static cache; static sm() { C.cache = 1; } }',
    'class-self-member-chain':'export class C { static cache; static sm() { return C.cache.value; } }',
    'object-this-escape':'export const C={sm(){consume(this);}};',
    'module-namespace-escape':'export * as C from "./leaf";',
}.items():
    cases[name]={'m.{ext}':'export { C } from "./provider";', 'provider.{ext}':provider,
        'app.{ext}':'import { C as X } from "./m"; export function run() { X.sm(); }',
        'leaf.{ext}':'export function sm(){};'}
    if name=='module-namespace-escape': cases[name]['other.{ext}']='import { C as Y } from "./m"; const A=Y;'
for name, body in {
    'alias': 'const A=this; A.sm=other;',
    'return': 'return this;',
    'argument': 'consume(this);',
    'write': 'this.sm=other;',
    'computed': 'this[key]();',
    'member-value': 'const method=this.sm;',
}.items():
    for carrier in ('declared', 'module'):
        cases[f'{carrier}-namespace-this-{name}']={
            'm.{ext}':'export { C } from "./provider";',
            'provider.{ext}': ('export namespace C { export function sm() { '+body+' } }'
                if carrier=='declared' else 'export * as C from "./leaf";'),
            'leaf.{ext}':'export function sm() { '+body+' }',
            'app.{ext}':'import { C as X } from "./m"; export function run() { X.sm(); }',
        }
reports=[]
for grammar in ('jsx','tsx'):
 for name, files in cases.items():
    d=out/(name+'-'+grammar);root=d/'source';root.mkdir(parents=True)
    (root/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'allowJs':True,'jsx':'preserve','moduleResolution':'node'},'include':['**/*']}))
    for file, source in files.items():
        p=root/file.replace('{ext}',grammar);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(source)
    for binary,file in [(c.BIN,'base-sites.jsonl'),(head,'head-sites.jsonl')]:
        c.run([binary,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],d/file)
    c.run([headfacts,root],d/'facts.jsonl')
    c.run(['node',HERE/'census.cjs',c.TS,root,d/'base-sites.jsonl',d/'facts.jsonl',d],d/'oracle.log')
    try:
        static=c.compare(d)
    except AssertionError:
        # Enumerate the whole RED population; the detailed static comparison
        # already records unproven rows. GREEN still requires base byte parity.
        static=json.loads((d/'comparison.json').read_text())
    before=[r for r in c.read(d/'base-sites.jsonl') if r.get('record_kind')=='call_site']
    after=[r for r in c.read(d/'head-sites.jsonl') if r.get('record_kind')=='call_site']
    kept=before==after
    reports.append({'case':name,'grammar':grammar,'classification':'PASS' if kept else 'WRONG_E5_IDENTITY_USE','confidence':100,
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
