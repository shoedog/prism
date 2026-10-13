"""READ: reproducible public S2-0 measurement. Never accepts a private root.
Usage: python3 public.py --out NEW_DIRECTORY [--fresh-main REBUILT_BIN] [X installed-X R T]
"""
import argparse
import concurrent.futures
import hashlib
import json
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
HOME_ROOT = Path.home()
TS = HOME_ROOT/'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
BIN = HERE.parents[4]/'target/s2-plan/bin/main-prism'
FACTS = HERE.parents[4]/'target/s2-plan/bin/main-dump_imports'
ROOTS = {'X': HOME_ROOT/'prism-evidence/inputs/excalidraw-0642e72c/source',
         'installed-X': HOME_ROOT/'prism-evidence/inputs/excalidraw-0642e72c-installed/source',
         'R': HOME_ROOT/'code/bench-repos/ruff/playground',
         'T': HOME_ROOT/'code/bench-repos/TypeScript/src'}


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--fresh-main', type=Path)
    parser.add_argument('--reuse-streams', action='store_true', help='rehash all source facts; reuse only this turn\'s complete raw streams')
    parser.add_argument('corpora', nargs='*', metavar='PUBLIC_CORPUS')
    a = parser.parse_args()
    if set(a.corpora) - set(ROOTS):
        parser.error('public corpus labels only: X installed-X R T')
    if a.out.exists() and not a.reuse_streams:
        parser.error('refuse pre-existing evidence directory')
    manifest = json.loads((HERE/'reference-binaries.json').read_text())
    for role, p in [('base',BIN),('facts',FACTS)]:
        assert sha(p)==manifest['binaries'][role]['sha256'], 'reference binary drift'
    def corpus(c):
        start = time.monotonic()
        d = a.out/c
        d.mkdir(parents=True, exist_ok=True)
        root = ROOTS[c]
        def run(args, name):
            with (d/name).open('w') as out, (d/(name+'.stderr')).open('w') as err:
                subprocess.run([str(s) for s in args], stdout=out, stderr=err, check=True, timeout=600)
        if not a.reuse_streams:
            run([BIN,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],'base-sites.jsonl')
            run([FACTS,root],'facts.jsonl')
        run(['node',HERE/'census.cjs',TS,root,d/'base-sites.jsonl',d/'facts.jsonl',d],'oracle.log')
        summary = json.loads((d/'summary.json').read_text())
        receipt = json.loads((d/'receipt.json').read_text())
        receipt.update({'claim':'MEASURED','corpus':c,'base_binary_sha256':sha(BIN),
                        'facts_binary_sha256':sha(FACTS),'runner_sha256':sha(__file__),
                        'elapsed_seconds':round(time.monotonic()-start,3)})
        if a.fresh_main:
            run([a.fresh_main,'nav','--no-cache','call-stats','--repo',root,'--dump-sites'],'fresh-main-sites.jsonl')
            before=(d/'base-sites.jsonl').read_bytes()
            after=(d/'fresh-main-sites.jsonl').read_bytes()
            # Complete stream equality binds all keys, metadata, drops and targets.
            assert before==after, 'reference/current main parity failed'
            receipt.update({'fresh_main_binary_sha256':sha(a.fresh_main),
                            'fresh_main_dump_sha256':sha(d/'fresh-main-sites.jsonl'),
                            'complete_stream_byte_parity':True})
        (d/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
        return c,summary
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        summaries = dict(pool.map(corpus,a.corpora or ROOTS))
    (a.out/'summary.json').write_text(json.dumps({'claim':'MEASURED','corpora':summaries},indent=2)+'\n')
    print(json.dumps({c:{k:s[k] for k in ('total_sites','s2_sites','low_sites','callable_low','positive_filter_ceiling')}
                      for c,s in summaries.items()},sort_keys=True))


if __name__ == '__main__':
    main()
