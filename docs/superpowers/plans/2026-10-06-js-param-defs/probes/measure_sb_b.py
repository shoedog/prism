#!/usr/bin/env python3
"""PR-B SecBench byte rows, disk-lean (planner; static only, never executes package code).
Per authenticated package root: base/head byte dumpers + `call-stats --dump-sites` (timed), a
per-package owner/byte rowdiff (RE-OWNED pairing included), then the full row files are deleted;
changed rows are kept with `file` rewritten to `<class>/<entry>/src/package/<file>` so one
adjudicate.cjs run over packages root resolves them. A failed producer on EITHER side excludes
the package from both aggregates (PR-A F4 rule). Usage:
  measure_sb_b.py OUT --base B --base-bytes BB --head H --head-bytes HB [--jobs 4] [--timeout 300]"""
import argparse, concurrent.futures, hashlib, json, subprocess, time
from collections import Counter
from pathlib import Path
H = Path.home(); PK = H / 'prism-evidence/inputs/secbench-pkgs'; P = Path(__file__).resolve().parent
def run(cmd, out, timeout):
    t = time.monotonic()
    with open(out, 'wb') as o:
        try: rc = subprocess.run([str(c) for c in cmd], stdout=o, stderr=subprocess.DEVNULL, timeout=timeout).returncode
        except subprocess.TimeoutExpired: rc = 124
    return {'exit': rc, 'seconds': round(time.monotonic() - t, 3)}
def main():
    ap = argparse.ArgumentParser(); ap.add_argument('out', type=Path)
    for s in ['base', 'head']: ap.add_argument('--' + s, type=Path, required=True); ap.add_argument('--' + s + '-bytes', type=Path, required=True)
    ap.add_argument('--jobs', type=int, default=4); ap.add_argument('--timeout', type=int, default=300)
    ap.add_argument('--only', nargs='*', help='iteration subset (class/entry); never a gate result'); a = ap.parse_args()
    a.out.mkdir(parents=True); (a.out / 'per').mkdir()
    sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
    (a.out / 'binding.json').write_text(json.dumps({k: {'path': str(getattr(a, k)), 'sha256': sha(getattr(a, k))} for k in ['base', 'base_bytes', 'head', 'head_bytes']}, indent=1) + '\n')
    m = json.loads((PK / 'manifest.json').read_text())
    names = [f"{e['class']}/{e['entry']}" for e in m['entries'] if e['status'] == 'ok']
    if a.only: names = [n for n in names if n in set(a.only)]
    def one(name):
        root = PK / name / 'src/package'; tag = name.replace('/', '__'); p = a.out / 'per' / tag; st = {'name': name}
        for side in ['base', 'head']:
            st[side] = {'bytes': run([getattr(a, side + '_bytes'), root], f'{p}.{side}.bytes.raw', a.timeout),
                        'sites': run([getattr(a, side), 'nav', '--no-cache', 'call-stats', '--repo', root, '--dump-sites'], f'{p}.{side}.sites', a.timeout)}
        st['ok'] = all(v['exit'] == 0 for s in ['base', 'head'] for v in st[s].values())
        if st['ok']:
            st['sites_identical'] = Path(f'{p}.base.sites').read_bytes() == Path(f'{p}.head.sites').read_bytes()
            for side in ['base', 'head']:
                rows = sorted(l for l in Path(f'{p}.{side}.bytes.raw').read_text().splitlines() if l.strip())
                Path(f'{p}.{side}.bytes.jsonl').write_text(''.join(r + '\n' for r in rows)); st[side]['rows'] = len(rows)
            subprocess.run(['python3', str(P / 'rowdiff.py'), f'{p}.base.bytes.jsonl', f'{p}.head.bytes.jsonl', f'{p}.diff.json', '--rows', f'{p}.changed.raw'], stdout=subprocess.DEVNULL, check=True)
            st['diff'] = json.loads(Path(f'{p}.diff.json').read_text())
            with open(f'{p}.changed.jsonl', 'w') as o:
                for l in Path(f'{p}.changed.raw').read_text().splitlines():
                    c = json.loads(l)
                    for key in ('row', 'base_row'):
                        if key in c:
                            for e in ('from', 'to'): c[key][e]['file'] = f"{name}/src/package/{c[key][e]['file']}"
                    o.write(json.dumps(c, sort_keys=True) + '\n')
        for suffix in ['base.bytes.raw', 'head.bytes.raw', 'base.bytes.jsonl', 'head.bytes.jsonl', 'base.sites', 'head.sites', 'changed.raw', 'diff.json']:
            Path(f'{p}.{suffix}').unlink(missing_ok=True)
        return st
    agg, statuses = Counter(), []
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as pool, open(a.out / 'status.jsonl', 'w') as sf:
        for st in pool.map(one, names):
            statuses.append(st); sf.write(json.dumps(st) + '\n'); sf.flush()
    with open(a.out / 'changed.jsonl', 'w') as o:
        for st in statuses:
            if st['ok']:
                o.write((a.out / 'per' / (st['name'].replace('/', '__') + '.changed.jsonl')).read_text())
                for k, v in st['diff'].items():
                    if isinstance(v, int): agg[k] += v
    summary = {'roots': len(names), 'admitted': sum(s['ok'] for s in statuses), 'excluded': sorted(s['name'] for s in statuses if not s['ok']),
               'sites_differ': sorted(s['name'] for s in statuses if s['ok'] and not s['sites_identical']), 'diff': dict(sorted(agg.items()))}
    (a.out / 'summary.json').write_text(json.dumps(summary, indent=1) + '\n'); print(json.dumps(summary))
if __name__ == '__main__': main()
