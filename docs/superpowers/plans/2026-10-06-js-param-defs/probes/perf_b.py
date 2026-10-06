#!/usr/bin/env python3
"""PR-B perf table on a quiet machine: full CPG build (byte dumper) and `nav --no-cache dfg-stats`
wall time and peak RSS, base vs head, REPS alternating runs per corpus. Records the RD counters.
Usage: perf_b.py OUT --base B --base-bytes BB --head H --head-bytes HB [--reps 2]"""
import argparse, json, re, subprocess, time
from pathlib import Path
H = Path.home()
CORPORA = {'X': H / 'prism-evidence/inputs/excalidraw-0642e72c/source', 'T': H / 'code/bench-repos/TypeScript/src',
           'SB-lodash_4.17.10': H / 'prism-evidence/inputs/secbench-pkgs/prototype-pollution/lodash_4.17.10/src/package'}
def timed(cmd):
    t = time.monotonic(); p = subprocess.run(['/usr/bin/time', '-l'] + [str(c) for c in cmd], capture_output=True)
    m = re.search(rb'(\d+)\s+maximum resident set size', p.stderr)
    return {'exit': p.returncode, 'seconds': round(time.monotonic() - t, 2), 'max_rss_mb': round(int(m.group(1)) / 1e6) if m else None}, p.stdout
def main():
    ap = argparse.ArgumentParser(); ap.add_argument('out', type=Path)
    for s in ['base', 'head']: ap.add_argument('--' + s, type=Path, required=True); ap.add_argument('--' + s + '-bytes', type=Path, required=True)
    ap.add_argument('--reps', type=int, default=2); a = ap.parse_args(); a.out.mkdir(parents=True, exist_ok=True)
    res = {}
    for name, root in CORPORA.items():
        r = res[name] = {'build': {'base': [], 'head': []}}
        for _ in range(a.reps):
            for side in ['base', 'head']:
                st, _out = timed([getattr(a, side + '_bytes'), root]); r['build'][side].append(st)
        for side in ['base', 'head']:
            st, out = timed([getattr(a, side), 'nav', '--no-cache', 'dfg-stats', '--repo', root])
            stats = json.loads(out) if st['exit'] == 0 else {}
            r['dfg_stats_' + side] = {**st, **{k: v for k, v in stats.items() if k.startswith('dfg_rd_') or k == 'dfg_label_exact'}}
        mb = lambda s: min(x['seconds'] for x in r['build'][s])
        r['build_ratio_min'] = round(mb('head') / mb('base'), 3)
        r['rss_ratio_max'] = round(max(x['max_rss_mb'] for x in r['build']['head']) / max(x['max_rss_mb'] for x in r['build']['base']), 3)
        (a.out / 'perf.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: {'build_ratio_min': v['build_ratio_min'], 'rss_ratio_max': v['rss_ratio_max']} for k, v in res.items()}))
if __name__ == '__main__': main()
