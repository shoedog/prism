#!/usr/bin/env python3
"""Adjudicate measure_sb_b.py per-package changed rows with adjudicate.cjs (one node process per
package, parallel) and sum the verdict keys. Usage: adjudicate_sb_b.py RUN_DIR [--jobs 4]"""
import argparse, collections, concurrent.futures, json, subprocess
from pathlib import Path
P = Path(__file__).resolve().parent; H = Path.home()
TS = H / 'prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js'
PK = H / 'prism-evidence/inputs/secbench-pkgs'
def main():
    ap = argparse.ArgumentParser(); ap.add_argument('run', type=Path); ap.add_argument('--jobs', type=int, default=4); a = ap.parse_args()
    out = a.run / 'adjudication'; out.mkdir(exist_ok=True)
    files = sorted(f for f in (a.run / 'per').glob('*.changed.jsonl') if f.stat().st_size)
    def one(f):
        tag = f.name[:-len('.changed.jsonl')]
        adj, det = out / f'{tag}.adj.json', out / f'{tag}.details.jsonl'
        r = subprocess.run(['node', '--max-old-space-size=8000', str(P / 'adjudicate.cjs'), str(TS), str(PK), str(f), str(adj), '--details', str(det)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        return tag, r.returncode, (json.loads(adj.read_text()) if r.returncode == 0 else r.stderr.decode()[-500:])
    agg, failed = collections.Counter(), {}
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool:
        for tag, rc, res in pool.map(one, files):
            if rc: failed[tag] = res
            else: agg.update(res)
    summary = {'packages_with_changes': len(files), 'failed': failed, 'aggregate': dict(sorted(agg.items()))}
    (out / 'summary.json').write_text(json.dumps(summary, indent=1) + '\n'); print(json.dumps(summary))
if __name__ == '__main__': main()
