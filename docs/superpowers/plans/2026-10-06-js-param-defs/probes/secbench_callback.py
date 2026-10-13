#!/usr/bin/env python3
"""PR-B SecBench conversions on the 97 `callback_argument_parameter_registration` rows (planner probe).

O1 harness measure() validates anonymous callable/parameter bytes, seeds location
witness/frontier, and omits anonymous callees. Member-only/rest sources can still
lack Defs. The inherited callee_tolerant_outcome field is a compatibility diagnostic;
under O1 it agrees with the harness outcome and supplies no separate credit.
Usage (repository root): python3 .../secbench_callback.py --binary BIN --out NEW_DIR [--workers 4] [--timeout 120]
"""
import argparse, hashlib, json, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0, str(Path.cwd()))
from eval.secbench.run import canonical, measure, classify  # noqa: E402
import eval.secbench.run as harness
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
MECH = 'callback_argument_parameter_registration'

def load_json_gz(path):
    import gzip
    try:
        return json.loads(gzip.open(path).read())
    except Exception:
        return None

def tolerant(row, out):
    if row['outcome'] != 'prism_error':
        return row['outcome']
    raw = out / 'raw' / row['class'] / row['entry']
    w, f = load_json_gz(raw / 'witness.json.gz'), load_json_gz(raw / 'frontier.json.gz')
    if not w or not f or 'error' in w or 'error' in f:
        return 'prism_error'
    try:
        return classify(row, w, {'items': []}, f)[0]
    except (KeyError, ValueError, TypeError):
        return 'prism_error'

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--binary', type=Path, required=True); ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--workers', type=int, default=4); ap.add_argument('--timeout', type=int, default=120)
    a = ap.parse_args()
    if a.out.exists(): raise SystemExit('use a new output directory')
    data = INSPECTION.read_bytes()
    if hashlib.sha256(data).hexdigest() != INSPECTION_SHA: raise SystemExit('inspection hash mismatch')
    keys = set()
    for line in R1_ENTRIES.read_bytes().splitlines():
        r = json.loads(line)
        m = (r.get('error_mechanism') or {}).get('mechanism') or (r.get('first_break') or {}).get('mechanism')
        if m == MECH: keys.add((r['class'], r['entry']))
    rows = [json.loads(l) for l in data.splitlines()]
    rows = [r for r in rows if (r['class'], r['entry']) in keys]
    a.out.mkdir(parents=True)
    (a.out / 'binding.json').write_bytes(canonical({'binary': str(a.binary.resolve()), 'binary_sha256': hashlib.sha256(a.binary.read_bytes()).hexdigest(),
        'inspection_sha256': INSPECTION_SHA, 'select': MECH, 'rows': len(rows), 'timeout': a.timeout, 'workers': a.workers}))
    t = time.time()
    with (a.out / 'entries.jsonl').open('wb') as stream, ThreadPoolExecutor(a.workers) as pool:
        for row in pool.map(lambda r: measure(r, a.binary, PACKAGES, a.out, a.timeout), rows):
            row['callee_tolerant_outcome'] = tolerant(row, a.out)
            stream.write(canonical(row)); stream.flush()
    outcomes, tol, errs = {}, {}, {}
    for line in (a.out / 'entries.jsonl').read_bytes().splitlines():
        r = json.loads(line)
        outcomes[r['outcome']] = outcomes.get(r['outcome'], 0) + 1
        tol[r['callee_tolerant_outcome']] = tol.get(r['callee_tolerant_outcome'], 0) + 1
        if r['outcome'] == 'prism_error':
            k = ','.join(sorted(n for n, v in r['invocations'].items() if v.get('error')))
            errs[k] = errs.get(k, 0) + 1
    print(json.dumps({'rows': len(rows), 'outcomes': outcomes, 'callee_tolerant_outcomes': tol, 'error_invocations': errs, 'seconds': round(time.time() - t)}, sort_keys=True))
if __name__ == '__main__': main()
