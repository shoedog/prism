#!/usr/bin/env python3
"""PR-B perf table on a quiet machine: full CPG build (byte dumper) and `nav --cache-dir <lane> dfg-stats`
wall time and peak RSS, base vs head, REPS alternating runs per corpus. Records the RD counters.
Usage: perf_b.py OUT --base B --base-bytes BB --head H --head-bytes HB [--reps 2]"""
import argparse, json, re, subprocess, time
import tempfile
from timed_process import timed_process
from pathlib import Path
H = Path.home()
CORPORA = {'X': H / 'prism-evidence/inputs/excalidraw-0642e72c/source', 'T': H / 'code/bench-repos/TypeScript/src',
           'SB-lodash_4.17.10': H / 'prism-evidence/inputs/secbench-pkgs/prototype-pollution/lodash_4.17.10/src/package'}
def timed(cmd):
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        result = timed_process(cmd,out,err,1800)
        out.seek(0)
        return {**result,'max_rss_mb':round(result['max_rss_bytes']/1e6)}, out.read()
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
            st, out = timed([getattr(a, side), 'nav', '--cache-dir', str(H / 'prism-evidence/js-param-defs/cache'), 'dfg-stats', '--repo', root])
            stats = json.loads(out) if st['exit'] == 0 else {}
            r['dfg_stats_' + side] = {**st, **{k: v for k, v in stats.items() if k.startswith('dfg_rd_') or k == 'dfg_label_exact'}}
        mb = lambda s: min(x['seconds'] for x in r['build'][s])
        r['build_ratio_min'] = round(mb('head') / mb('base'), 3)
        r['rss_ratio_max'] = round(max(x['max_rss_mb'] for x in r['build']['head']) / max(x['max_rss_mb'] for x in r['build']['base']), 3)
        (a.out / 'perf.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: {'build_ratio_min': v['build_ratio_min'], 'rss_ratio_max': v['rss_ratio_max']} for k, v in res.items()}))
if __name__ == '__main__': main()
