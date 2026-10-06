#!/usr/bin/env python3
"""Run main's harness measure() on the checkpointed copies from sb_joint_checkpoint.cjs.
Records harness and callee-tolerant outcomes (see secbench_callback.py). Usage (repo root):
  python3 .../sb_joint_measure.py --rows DIR/rows.jsonl --pkgs DIR/pkgs --binary BIN --out NEW_DIR"""
import argparse, hashlib, json, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0, str(Path.cwd())); sys.path.insert(0, str(Path(__file__).resolve().parent))
from eval.secbench.run import canonical, measure  # noqa: E402
import secbench_callback as cb  # reuses the lane cache-root binding and tolerant()
def main():
    ap = argparse.ArgumentParser(); ap.add_argument('--rows', type=Path, required=True); ap.add_argument('--pkgs', type=Path, required=True)
    ap.add_argument('--binary', type=Path, required=True); ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--workers', type=int, default=3); ap.add_argument('--timeout', type=int, default=120); a = ap.parse_args()
    if a.out.exists(): raise SystemExit('use a new output directory')
    rows = [json.loads(l) for l in a.rows.read_text().splitlines() if l.strip()]
    a.out.mkdir(parents=True)
    (a.out / 'binding.json').write_bytes(canonical({'binary': str(a.binary.resolve()), 'binary_sha256': hashlib.sha256(a.binary.read_bytes()).hexdigest(),
        'rows_sha256': hashlib.sha256(a.rows.read_bytes()).hexdigest(), 'rows': len(rows)}))
    with (a.out / 'entries.jsonl').open('wb') as s, ThreadPoolExecutor(a.workers) as pool:
        for row in pool.map(lambda r: measure(r, a.binary, a.pkgs, a.out, a.timeout), rows):
            row['callee_tolerant_outcome'] = cb.tolerant(row, a.out); s.write(canonical(row)); s.flush()
    agg = {}
    for l in (a.out / 'entries.jsonl').read_bytes().splitlines():
        r = json.loads(l); k = (r['outcome'], r['callee_tolerant_outcome']); agg['%s|%s' % k] = agg.get('%s|%s' % k, 0) + 1
    print(json.dumps(agg, sort_keys=True))
if __name__ == '__main__': main()
