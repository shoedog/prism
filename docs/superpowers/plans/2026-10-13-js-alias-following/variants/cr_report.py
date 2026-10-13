#!/usr/bin/env python3
"""Aggregate view of a callresult-aggregate.json (counts only)."""
import json, collections, sys
d = json.load(open(sys.argv[1]))
part = sys.argv[2] if len(sys.argv) > 2 else 'product'
by = collections.defaultdict(collections.Counter)
fam = collections.defaultdict(collections.Counter)
for k, v in d['tally'].items():
    b, p, verdict, cls, h = k.split('|')
    if p != part:
        continue
    key = cls + ('' if not cls.startswith('fn') else ' ' + h)
    by[key][verdict] += v
    fam[b][key] += v
vs = ['correct', 'wrong', 'missing', 'unknown_edge', 'unknown_no_edge', 'correct_no_edge']
print(f"sites {d['sites']} (all parts); deps {d['dependencies_known']}; part {part}")
print(f"{'class':42s} {'total':>6s} " + ' '.join(f'{v:>15s}' for v in vs))
for key, c in sorted(by.items(), key=lambda kv: -sum(kv[1].values())):
    print(f"{key:42s} {sum(c.values()):6d} " + ' '.join(f'{c.get(v, 0):15d}' for v in vs))
if len(sys.argv) > 3:
    print('--- by census binding class')
    for b, c in sorted(fam.items(), key=lambda kv: -sum(kv[1].values())):
        print(f"{b} ({sum(c.values())}): " + ', '.join(f'{k} {n}' for k, n in c.most_common(9)))
if d.get('checked'):
    print('--- checker comparison:', json.dumps(d['checked'], sort_keys=True))
