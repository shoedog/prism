"""A1/A4 full-row main parity, in both grammars and both star orders."""
import hashlib, json, subprocess, sys
from pathlib import Path
head, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
expect_wrong = '--expect-wrong' in sys.argv[3:]
base = Path.home()/'code/prism-paths-impl/target/repair-r5/head/prism'
out.mkdir(parents=True, exist_ok=False)
records, baseline = [], {}
for grammar in ['jsx', 'tsx']:
    for kind in ['package', 'alias']:
        for reverse in [False, True]:
            root = out/f'{grammar}-{kind}-{int(reverse)}'
            def write(name, text):
                p = root/name; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(text)
            spec = 'pkg' if kind == 'package' else '@k'
            write('a.'+grammar, f"import {{K}} from '{spec}'; export {{K}};\n")
            write('b.'+grammar, 'export function K(){ return null; }\n')
            branches = ['a', 'b'][::(-1 if reverse else 1)]
            write('barrel.'+grammar, ''.join(f"export * from './{b}';\n" for b in branches))
            write('app.'+grammar, "import {K} from './barrel';\nexport function run() { K(); return <K />; }\n")
            if kind == 'package':
                write('node_modules/pkg/index.d.ts', 'export declare function K(): unknown;\n')
                write('node_modules/pkg/index.js', 'export function K(){ return null; }\n')
            else:
                write('tsconfig.json', json.dumps({'compilerOptions':{'moduleResolution':'node10','allowJs':True,'baseUrl':'.','paths':{'@k':['k2']}}}))
                write('k2.'+grammar, 'export function K(){ return null; }\n')
            rows = {}
            for label, binary in [('base', base), ('head', head)]:
                p = subprocess.run([str(binary),'nav','--no-cache','call-stats','--repo',str(root),'--dump-sites'],capture_output=True,check=True)
                (out/(root.name+'-'+label+'.stderr')).write_bytes(p.stderr)
                all_rows = [json.loads(l) for l in p.stdout.splitlines()]
                rows[label] = [r for r in all_rows if r.get('record_kind') == 'call_site' and r['caller']['file'] == 'app.'+grammar]
                assert len(rows[label]) == 2, rows[label]
                (out/(root.name+'-'+label+'.json')).write_text(json.dumps(rows[label],indent=2)+'\n')
            assert all(r['drop'] == 'UnknownName' and not r['resolved_targets'] for r in rows['base'])
            baseline.setdefault(grammar, rows['base']); assert baseline[grammar] == rows['base']
            if expect_wrong:
                assert all(r['resolved_targets'][0]['function_id']['file'] == 'b.'+grammar for r in rows['head'])
            else:
                assert rows['head'] == rows['base'], (root.name, rows)
            records.append({'case':root.name,'sites':2,'complete_base_parity':rows['head']==rows['base']})
(out/'base-rows.json').write_text(json.dumps(baseline,indent=2)+'\n')
summary = {'cases':len(records),'sites':16,'expect_wrong':expect_wrong,'records':records,'binary_sha256':{k:hashlib.sha256(p.read_bytes()).hexdigest() for k,p in [('base',base),('head',head)]}}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='records'}))
