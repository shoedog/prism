#!/usr/bin/env python3
"""PR-B nav byte-identity control (SPEC-prB D7). Static only; never executes corpus code.
For a seeded sample of lines that open or sit inside anonymous JS/TS callables, run
nodes-at / ego / callers / callees (`--location`) plus repo-map on base and head and compare
stdout bytes and exit codes. Usage: nav_identity.py BASE HEAD ROOT OUT [--lines N] [--seed S]
"""
import argparse, hashlib, json, random, re, subprocess
from pathlib import Path
CACHE = Path.home() / 'prism-evidence/js-param-defs/cache'
SKIP = {'node_modules', '.git', 'dist', 'build'}
def lines(root, n, seed):
    cands = []
    for f in sorted(root.rglob('*')):
        if not f.is_file() or f.suffix not in ('.js', '.jsx', '.ts', '.tsx', '.mjs', '.cjs') or set(f.relative_to(root).parts) & SKIP:
            continue
        if any(p.startswith('.') for p in f.relative_to(root).parts):
            continue
        try:
            text = f.read_text(errors='replace').splitlines()
        except OSError:
            continue
        inside = 0
        for i, t in enumerate(text, 1):
            if re.search(r'=>|function\s*\(', t):
                cands.append((str(f.relative_to(root)), i)); inside = 3
            elif inside:
                cands.append((str(f.relative_to(root)), i)); inside -= 1
    random.Random(seed).shuffle(cands)
    return sorted(cands[:n])
def run(binary, args):
    p = subprocess.run([str(binary), 'nav', '--cache-dir', str(CACHE), *args], capture_output=True, timeout=600)
    return p.returncode, p.stdout
def main():
    ap = argparse.ArgumentParser(); ap.add_argument('base', type=Path); ap.add_argument('head', type=Path)
    ap.add_argument('root', type=Path); ap.add_argument('out', type=Path); ap.add_argument('--lines', type=int, default=100); ap.add_argument('--seed', type=int, default=20261006)
    a = ap.parse_args(); a.out.mkdir(parents=True, exist_ok=True)
    sample = lines(a.root, a.lines, a.seed)
    queries = [['repo-map', '--repo', str(a.root), '--format', 'json']]
    for f, n in sample:
        loc = f'{f}:{n}'
        queries += [['nodes-at', '--repo', str(a.root), '--location', loc, '--format', 'json'],
                    ['ego', '--repo', str(a.root), '--location', loc, '--format', 'json'],
                    ['callers', '--repo', str(a.root), '--location', loc, '--format', 'json'],
                    ['callees', '--repo', str(a.root), '--location', loc, '--format', 'json']]
    captures = {}
    for side in ['base', 'head']:
        captures[side] = []
        for i, q in enumerate(queries):
            result = run(getattr(a, side), q)
            (a.out / f'{i:04d}.{side}.json').write_bytes(result[1])
            captures[side].append(result)
    (a.out / 'queries.json').write_text(json.dumps(queries, indent=1) + '\n')
    diffs, total = [], 0
    for i, q in enumerate(queries):
        b, h = captures['base'][i], captures['head'][i]
        total += 1
        if b != h:
            diffs.append({'index': i, 'query': q[0], 'location': q[4] if len(q) > 4 else None, 'base_rc': b[0], 'head_rc': h[0],
                          'base_sha': hashlib.sha256(b[1]).hexdigest(), 'head_sha': hashlib.sha256(h[1]).hexdigest()})
    summary = {'root': str(a.root), 'lines': len(sample), 'queries': total, 'differing': len(diffs), 'diffs': diffs[:50],
               'base': hashlib.sha256(a.base.read_bytes()).hexdigest(), 'head': hashlib.sha256(a.head.read_bytes()).hexdigest()}
    (a.out / 'nav-identity.json').write_text(json.dumps(summary, indent=1) + '\n')
    print(json.dumps({k: v for k, v in summary.items() if k != 'diffs'}))
if __name__ == '__main__': main()
