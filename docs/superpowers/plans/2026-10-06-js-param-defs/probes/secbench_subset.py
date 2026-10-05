#!/usr/bin/env python3
"""Same-harness SecBench re-measurement of a row subset on an arbitrary binary (planner probe).

The committed harness (`python3 -m eval.secbench`) authenticates one pinned SUT binary; a
base/head comparison of an unpinned prototype needs the identical per-entry `measure()` from
main's harness run on BOTH binaries over the same authenticated inspection. This driver does
only that: it never edits packages, never executes package code, and records the binary hash.

Usage (from the repository root):
  python3 docs/.../probes/secbench_subset.py --binary BIN --out NEW_DIR \
      [--select targets|eligible] [--workers 4] [--timeout 120]
"""
import argparse
import hashlib
import json
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path.cwd()))
from eval.secbench.run import canonical, measure  # noqa: E402
import eval.secbench.run as harness
# Preserve measure() and its interpretation; bind every SUT launch to the
# lane-authorized cache root, including witness and seed queries.
_original_invoke = harness.invoke

def _lane_invoke(binary, args, *rest, **kwargs):
    args = list(args)
    if '--cache-dir' in args:
        args[args.index('--cache-dir') + 1] = str(Path.home() / 'prism-evidence/js-param-defs/cache')
    return _original_invoke(binary, args, *rest, **kwargs)

harness.invoke = _lane_invoke

EVIDENCE = Path.home() / 'prism-evidence'
INSPECTION = EVIDENCE / 'meas/secbench/repair-r1/measured/inspection.jsonl'
INSPECTION_SHA = 'ba2f57c6c077205c3e51e3e4c2489465a24267bc0f3b220739bc3b64bea4ee2c'
R1_ENTRIES = EVIDENCE / 'meas/secbench/repair-r1/measured/entries.jsonl'
PACKAGES = EVIDENCE / 'inputs/secbench-pkgs'
# PR-A demonstrated targets: R1 ws2 rest conversions (6) and r2-opus-A F2 unparenthesised arrows (3).
TARGETS = {
    ('prototype-pollution', 'assign-deep_1.0.0'), ('prototype-pollution', 'decal_2.1.3'),
    ('prototype-pollution', 'deep-override_1.0.0'), ('prototype-pollution', 'mixin-deep_2.0.0'),
    ('prototype-pollution', 'think-helper_1.1.0'), ('prototype-pollution', 'viking04-merge_1.0.0'),
}
ARROW_ENTRY_PREFIXES = ('port-killer_', 'is-svg_', 'portprocesses_')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--binary', type=Path, required=True)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--select', choices=('targets', 'eligible'), default='targets')
    ap.add_argument('--workers', type=int, default=4)
    ap.add_argument('--timeout', type=int, default=120)
    ap.add_argument('--only-entry', help='supplemental same-environment control; exact package entry')
    args = ap.parse_args()
    if args.out.exists():
        raise SystemExit('use a new output directory')
    data = INSPECTION.read_bytes()
    if hashlib.sha256(data).hexdigest() != INSPECTION_SHA:
        raise SystemExit('inspection hash mismatch')
    rows = [json.loads(line) for line in data.splitlines()]
    if args.select == 'targets':
        rows = [r for r in rows if (r['class'], r['entry']) in TARGETS or r['entry'].startswith(ARROW_ENTRY_PREFIXES)]
    else:
        rows = [r for r in rows if r.get('gt_status') == 'available']
    if args.only_entry:
        rows = [r for r in rows if r['entry'] == args.only_entry]
        if not rows: raise SystemExit('no exact control entry selected')
    args.out.mkdir(parents=True)
    binding = {'binary': str(args.binary.resolve()), 'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
               'inspection_sha256': INSPECTION_SHA, 'select': args.select, 'rows': len(rows),
               'timeout': args.timeout, 'workers': args.workers, 'only_entry': args.only_entry}
    (args.out / 'binding.json').write_bytes(canonical(binding))
    started = time.time()
    with (args.out / 'entries.jsonl').open('wb') as stream, ThreadPoolExecutor(args.workers) as pool:
        for row in pool.map(lambda r: measure(r, args.binary, PACKAGES, args.out, args.timeout), rows):
            stream.write(canonical(row))
            stream.flush()
    outcomes = {}
    for line in (args.out / 'entries.jsonl').read_bytes().splitlines():
        row = json.loads(line)
        outcomes[row['outcome']] = outcomes.get(row['outcome'], 0) + 1
    print(json.dumps({'rows': len(rows), 'outcomes': outcomes, 'seconds': round(time.time() - started)}, sort_keys=True))


if __name__ == '__main__':
    main()
