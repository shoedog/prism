"""READ: calibrate the S2 measurement instrument, not a product prototype.
The cases include positive identity filtering, alias/absence, shadow, E5,
both JSX/TSX grammars, class static/instance members and CommonJS forms.
"""
import gzip
import json
import os
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
TS = Path.home() / 'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
BIN = HERE.parents[4] / 'target/s2-plan/bin/main-prism'
FACTS = HERE.parents[4] / 'target/s2-plan/bin/main-dump_imports'
OUT = HERE.parents[4] / 'target/s2-plan/controls'


def run(args, stdout, stderr):
    with stdout.open('w') as out, stderr.open('w') as err:
        subprocess.run([str(a) for a in args], stdout=out, stderr=err, check=True)


def main():
    results = {}
    for grammar in ('jsx', 'tsx'):
        d = OUT / grammar
        root = d / 'source'
        (root / 'decoy').mkdir(parents=True, exist_ok=True)
        def write(name, text):
            (root / name).write_text(text)
        write('tsconfig.json', json.dumps({'compilerOptions': {
            'allowJs': True, 'checkJs': True, 'target': 'es2020',
            'module': 'commonjs', 'moduleResolution': 'node', 'esModuleInterop': True,
            'jsx': 'preserve'}, 'include': ['**/*']}))
        write(f'm.{grammar}', '''export function f() { return 1; }
export const ns = { f, arrow: () => 2, method() { return 3; } };
export const _ns = ns;
export default ns;
export class C {
  static sm() { return 4; }
  static sf = () => 5;
  im() { return 6; }
  field = () => 7;
}
export function Callable() { return 8; }
Callable.f = f;
function register(value) { return value; }
export const action = register({ f: () => 9 });
''')
        write(f'decoy/m.{grammar}', 'export function f() { return -1; }\n')
        write(f'terminal.{grammar}', 'export function f() { return 10; }\n')
        write(f'barrel.{grammar}', "export * as space from './terminal';\n")
        write(f'instance.{grammar}', f"import {{ C }} from './m';\nexport default new C();\n")
        write('cjs.js', '''function f() { return 11; }
module.exports = { f, nested: { f } };
''')
        write('cjs-other.js', 'exports.f = function f() { return 13; };\n')
        write('eq.ts', '''const Eq = { f() { return 12; } };
export = Eq;
''')
        equals = "import Eq = require('./eq');\n" if grammar == 'tsx' else ''
        eqcall = 'Eq.f();' if grammar == 'tsx' else ''
        write(f'caller.{grammar}', f'''import X, {{ ns, C, Callable, action, _ns as _alias, ns as Δ }} from './m';
import {{ space }} from './barrel';
import instance from './instance';
const Req = require('./cjs');
const OtherReq = require('./cjs-other');
const {{ nested: Destruct }} = require('./cjs');
{equals}export function calls() {{
  X.f(); ns.f(); ns.arrow(); ns.method();
  C.sm(); C.sf(); C.im(); C.field();
  Callable.f(); action.f(); space.f();
  Req.f(); Destruct.f(); X.absent(); {eqcall}
  _alias.f(); Δ.f(); instance.im(); instance.field(); OtherReq.f();
}}
export function shadow(X) {{ X.f(); }}
export function shadowRequire(require) {{ const Hidden = require('./cjs'); Hidden.f(); }}
''')
        # Mixed-language sites reproduced the former fatal site/fact join.
        write('main.rs', 'fn helper() {}\nfn caller() { helper(); }\n')
        caller = root/f'caller.{grammar}'
        caller.write_bytes(('\uFEFF'+caller.read_text().replace('\n','\r\n')).encode('utf8'))
        run([BIN, 'nav', '--no-cache', 'call-stats', '--repo', root, '--dump-sites'], d/'base-sites.jsonl', d/'base.stderr')
        run([FACTS, root], d/'facts.jsonl', d/'facts.stderr')
        run(['node', HERE/'census.cjs', TS, root, d/'base-sites.jsonl', d/'facts.jsonl', d], d/'oracle.stdout', d/'oracle.stderr')
        rows = json.loads((d/'candidates.json').read_text())
        summary = json.loads((d/'summary.json').read_text())
        assert summary['unjoinable_reasons'] == {'site_fact_join':1}, summary
        assert json.loads((d/'unjoinable.json').read_text())[0]['key'][0]=='main.rs'
        def case(q, member):
            matches = [r for r in rows if r['qualifier'] == q and r['member'] == member]
            assert len(matches) == 1, (q, member, matches)
            return matches[0]
        assert case('X', 'f')['terminal']['class'] == 'function'
        assert case('X', 'f')['mechanism'] == 'default_object'
        assert case('ns', 'f')['mechanism'] == 'named_object'
        assert case('ns', 'f')['base_multiple'] and case('ns', 'f')['base_contains_terminal']
        assert case('ns', 'arrow')['terminal']['class'] == 'function_value'
        assert case('ns', 'method')['terminal']['class'] == 'object_method'
        assert case('C', 'sm')['terminal']['class'] == 'class_static_method'
        assert case('C', 'sf')['terminal']['class'] == 'class_static_field'
        assert not case('C', 'im')['callable']
        assert not case('C', 'field')['callable']
        assert case('space', 'f')['mechanism'] == 'reexported_namespace'
        assert case('Req', 'f')['mechanism'] == 'commonjs_module_exports'
        assert case('Destruct', 'f')['binding']['kind'] == 'require_destructure'
        assert case('action', 'f')['mechanism'] == 'named_call_result'
        assert not case('X', 'absent')['callable']
        assert not any(r['qualifier'] == 'Hidden' or r['key'][1] == 'shadow' for r in rows)
        # Assignment-backed members should resolve through the RHS, not be
        # confused with re-exported module namespaces (TS 53608-53612).
        assert case('Callable', 'f')['terminal']['class'] == 'function'
        assert case('Callable', 'f')['mechanism'] == 'named_function_members'
        assert case('_alias', 'f')['terminal']['class'] == 'function'
        assert case('Δ', 'f')['terminal']['class'] == 'function'
        assert case('instance', 'im')['terminal']['class'] == 'class_instance_method'
        assert case('instance', 'field')['terminal']['class'] == 'class_instance_field'
        assert case('OtherReq', 'f')['mechanism'] == 'commonjs_other_exports'
        if grammar == 'tsx':
            assert case('Eq', 'f')['mechanism'] == 'export_equals'
        results[grammar] = {'status': 'PASS', 'candidate_sites': len(rows)}
    # Negative setup case: an empty stream is inadmissible, never zero yield.
    empty = OUT/'empty.jsonl'
    empty.write_text('')
    p = subprocess.run(['node', str(HERE/'census.cjs'), str(TS), str(root), str(empty),
                        str(d/'facts.jsonl'), str(d)], capture_output=True, text=True)
    assert p.returncode != 0 and 'zero-site probe is inadmissible' in p.stderr
    results['empty_stream'] = {'status':'PASS'}
    # Compression is a storage choice and must preserve the census.
    for name in ('base-sites.jsonl', 'facts.jsonl'):
        (d/(name+'.gz')).write_bytes(gzip.compress((d/name).read_bytes(), mtime=0))
    expected = json.loads((d/'summary.json').read_text())
    run(['node', HERE/'census.cjs', TS, root, d/'base-sites.jsonl.gz', d/'facts.jsonl.gz', d], d/'compressed.stdout', d/'compressed.stderr')
    assert json.loads((d/'summary.json').read_text()) == expected
    results['compressed_inputs'] = {'status':'PASS'}
    # The F wrapper is exercised only on this synthetic public fixture.
    wrapper = HERE.parent/'CONTROLLER-s2.sh'
    private_out = OUT/'synthetic-controller-output'
    # Refuse pre-existing evidence in the real wrapper; use a fresh test directory.
    import tempfile
    with tempfile.TemporaryDirectory(prefix='wrapper-', dir=OUT) as sandbox:
        private_out = Path(sandbox)/'new-output'
        env = {**os.environ, 'CORPUS_F_ROOT':str(root), 'PRIVATE_EVIDENCE_ROOT':str(private_out)}
        p = subprocess.run(['bash', str(wrapper), str(BIN), str(FACTS), str(TS)], env=env, capture_output=True, text=True)
        assert p.returncode == 0 and not p.stderr
        aggregate = json.loads(p.stdout)
        assert aggregate['status']=='COMPLETE' and aggregate['low_sites']==expected['low_sites']
        assert set(aggregate) == {'claim','status','corpus','total_sites','source_files','s2_sites','low_sites',
            'callable_low','positive_filter_ceiling','mechanisms','binding_kinds','terminal_classes','exclusions',
            'unjoinable','unjoinable_reasons'}
        p = subprocess.run(['bash', str(wrapper), str(BIN), str(FACTS), str(TS)], env=env, capture_output=True, text=True)
        assert p.returncode != 0 and json.loads(p.stdout)['stage']=='evidence_directory_exists' and not p.stderr
        env['PRIVATE_EVIDENCE_ROOT']=str(Path(sandbox)/'bad-binary-output')
        p = subprocess.run(['bash', str(wrapper), str(TS), str(FACTS), str(TS)], env=env, capture_output=True, text=True)
        assert p.returncode != 0 and json.loads(p.stdout)['stage']=='binary_binding' and not p.stderr
    p = subprocess.run(['bash', str(wrapper)], capture_output=True, text=True)
    assert p.returncode != 0 and json.loads(p.stdout)['stage']=='arguments' and not p.stderr
    results['wrapper_synthetic_only'] = {'status':'PASS', 'negative_cases':3}
    (OUT/'results.json').write_text(json.dumps({'claim':'MEASURED', **results}, indent=2)+'\n')
    print(json.dumps(results))


if __name__ == '__main__':
    main()
