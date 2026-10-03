"""Fresh complete base/P1/P2 public dumps; no private corpus or Git writes."""
import gzip, hashlib, json, subprocess, sys
from pathlib import Path
P1 = Path.home()/'code/prism-paths-impl/target/repair-r5/head/prism'
OLD = Path.home()/'code/prism-paths-plan/target/paths-plan/extension-priority/evidence/public/base'
ROOTS = {'X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c/source',
         'installed-X':Path.home()/'prism-evidence/inputs/excalidraw-0642e72c-installed/source',
         'R':Path.home()/'code/bench-repos/ruff/playground',
         'T':Path.home()/'code/bench-repos/TypeScript/src'}

PROBES = Path(__file__).resolve().parent
REPO = PROBES.parents[4]
OUT = REPO/'target/p2-plan/p2-public'
HEAD = Path(sys.argv[1]).resolve()
BASE = Path('/Users/wesleyjinks/prism-evidence/paths/reviews/evidence-P1-r4-sol61/r4/base-prism')
sys.path.insert(0, str(PROBES.parent/'probes'))
from rowdiff import load
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
summary = {}
for name, root in ROOTS.items():
    d = OUT/name; d.mkdir(parents=True, exist_ok=True)
    files = {}
    for label, binary in [('base',BASE), ('p1',P1), ('p2',HEAD)]:
        dest = d/(label+'.jsonl')
        with dest.open('w') as stdout, (d/(label+'.stderr')).open('w') as stderr:
            subprocess.run([str(binary),'nav','--no-cache','call-stats','--repo',str(root),'--dump-sites'], stdout=stdout, stderr=stderr, check=True)
        files[label] = dest
    rows = {label:load(dest) for label,dest in files.items()}
    assert rows['base'].keys() == rows['p1'].keys() == rows['p2'].keys()
    assert files['p1'].read_bytes() == files['p2'].read_bytes(), 'P1 public rows changed'
    gains = sum(rows['base'][k] != rows['p2'][k] for k in rows['base'])
    assert gains == (3121 if name in ['X','installed-X'] else 0)
    # Retained certificates are admitted only after live input rehash and complete row parity.
    old = Path('/Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/yield')
    assert gzip.decompress((old/(name+'-head.jsonl.gz')).read_bytes()) == files['p2'].read_bytes()
    inputs = json.loads((REPO/'target/p2-plan/public'/name/'oracle-native-inputs.json').read_text())
    for file,digest in inputs.items(): assert sha(root/file) == digest, 'native input drift'
    facts = OLD/(('X' if name == 'installed-X' else name)+'-import-facts.jsonl')
    factrows = [json.loads(line) for line in facts.read_text().splitlines()]
    for r in factrows:
        expected = bytes(r['hash']).hex() if isinstance(r['hash'],list) else r['hash']
        assert sha(root/r['file']) == expected, 'fact input drift'
    summary[name] = {'sites':len(rows['p2']), 'vs_base':gains, 'vs_p1':0, 'p1_byte_identical':True,
                     'native_inputs_rehashed':len(inputs), 'facts_rehashed':len(factrows),
                     'p1_gain_certification':'retained native certificates, live input and full row parity',
                     'dump_sha256':{label:sha(dest) for label,dest in files.items()},
                     'binary_sha256':{label:sha(binary) for label,binary in [('base',BASE),('p1',P1),('p2',HEAD)]}}
    for dest in files.values():
        with gzip.open(str(dest)+'.gz','wb') as f: f.write(dest.read_bytes())
        dest.unlink()
    (OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(name, json.dumps(summary[name]), flush=True)
