"""READ: Fresh public P1 dumps and TS oracles; no builds, installs or Git writes.
Usage: python3 public.py [X installed-X R T ...]
"""
import gzip, hashlib, json, subprocess, sys, time
from pathlib import Path

HERE = Path(__file__).resolve().parents[5]
PROBES = Path(__file__).resolve().parent
OUT = HERE / 'target/p2-plan/public'
TS = Path.home() / 'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
P1 = Path.home() / 'code/prism-paths-impl/target/repair-r5/head/prism'
OLD = Path.home() / 'code/prism-paths-plan/target/paths-plan/extension-priority/evidence/public/base'
R5 = Path.home() / 'code/prism-paths-impl/target/repair-r5/yield'
ROOTS = {'X': Path.home() / 'prism-evidence/inputs/excalidraw-0642e72c/source',
         'installed-X': Path.home() / 'prism-evidence/inputs/excalidraw-0642e72c-installed/source',
         'R': Path.home() / 'code/bench-repos/ruff/playground',
         'T': Path.home() / 'code/bench-repos/TypeScript/src'}

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

for corpus in sys.argv[1:] or ROOTS:
    root = ROOTS[corpus]
    d = OUT / corpus
    d.mkdir(parents=True, exist_ok=True)
    facts = OLD / f'{"X" if corpus == "installed-X" else corpus}-import-facts.jsonl'
    fact_rows = [json.loads(line) for line in facts.read_text().splitlines()]
    mismatches = []
    for f in fact_rows:
        actual = sha(root/f['file'])
        # READ: retained extraction uses SHA-256 source-byte hashes.
        expected = f['hash']
        if isinstance(expected, list): expected = bytes(expected).hex()
        if actual != expected: mismatches.append(f['file'])
    assert not mismatches, ('stale facts', mismatches)
    sites = d/'p1-sites.jsonl'
    start = time.monotonic()
    with sites.open('w') as stdout, (d/'p1.stderr').open('w') as stderr:
        subprocess.run([str(P1), 'nav', '--no-cache', 'call-stats', '--repo', str(root), '--dump-sites'], stdout=stdout, stderr=stderr, check=True)
    raw = sites.read_bytes()
    old = gzip.decompress((R5/f'{corpus}-head.jsonl.gz').read_bytes())
    count = sum(json.loads(line).get('record_kind') == 'call_site' for line in raw.splitlines())
    assert count > 0
    receipt = {'claim':'MEASURED', 'corpus':corpus, 'sites':count, 'p1_binary_sha256':sha(P1),
               'fresh_dump_seconds':time.monotonic()-start, 'dump_sha256':sha(sites),
               'r5_byte_parity':raw == old, 'retained_facts':str(facts),
               'facts_sha256':sha(facts), 'source_files_rehashed':len(fact_rows),
               'facts_extraction':'READ retained extraction; all source-byte hashes checked live'}
    (d/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt), flush=True)
    prefix = d/'oracle'
    with (d/'oracle.log').open('w') as log:
        subprocess.run(['node', str(PROBES.parent/'probes/oracle.cjs'), str(TS), str(root), str(sites), str(facts), str(prefix)], stdout=log, stderr=subprocess.STDOUT, check=True)
    with (d/'native.log').open('w') as log:
        subprocess.run(['node', str(PROBES/'native.cjs'), str(TS), str(root), str(prefix)+'-alias-sites.json', str(facts), str(prefix)], stdout=log, stderr=subprocess.STDOUT, check=True)
    summary = json.load(open(str(prefix)+'-native-summary.json'))
    summary['total_sites'] = count
    (d/'summary.json').write_text(json.dumps(summary, indent=2)+'\n')
    print(corpus, json.dumps(summary), flush=True)
