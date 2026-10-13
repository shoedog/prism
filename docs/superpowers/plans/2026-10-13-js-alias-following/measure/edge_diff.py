#!/usr/bin/env python3
"""Site-level diff of two `prism nav call-stats --dump-sites` outputs (aggregate counts to stdout,
per-site rows to a file with mode 600).
  edge_diff.py <base.sites.jsonl> <head.sites.jsonl> <rows-out.jsonl> [census sites.jsonl]
A site is keyed by (file, start_byte, end_byte, callee_text, origin). Its state is the sorted list of
(confidence, kind, target file, target name, target start_line) plus the drop reason."""
import json, sys, collections, os, re
base_p, head_p, out_p = sys.argv[1:4]
tests = None
if len(sys.argv) > 4:
    # census sites.jsonl: one row per site with its file's product/test flag
    tests = {}
    for line in open(sys.argv[4]):
        r = json.loads(line)
        tests[r['file']] = bool(r['test'])
TEST_RE = re.compile(r'(^|/)(__tests__|__mocks__|tests?|e2e|cypress|fixtures?)/|\.(test|spec|stories)\.[cm]?[jt]sx?$')
def is_test(f):
    if tests is not None and f in tests:
        return tests[f]
    return bool(TEST_RE.search(f))
def load(p):
    sites = collections.defaultdict(list)
    for line in open(p):
        if not line.startswith('{'):
            continue
        try:
            o = json.loads(line)
        except ValueError:
            continue
        if o.get('record_kind') != 'call_site' or not o.get('source_span'):
            continue
        s = o['source_span']
        key = (s['file'], s['start_byte'], s['end_byte'], o.get('callee_text'), o.get('origin'))
        tg = sorted((t['confidence'], t['kind'], t['function_id']['file'], t['function_id']['name'], t['function_id']['start_line']) for t in o.get('resolved_targets') or [])
        sites[key].append((tg, o.get('drop'), s['line']))
    return sites
def summary(states):
    tg = sorted(t for st in states for t in st[0])
    drops = sorted(set(st[1] for st in states if st[1]))
    if not tg:
        return 'none(' + '+'.join(drops) + ')' if drops else 'none'
    kinds = sorted(set(f"{c}:{k}" for c, k, *_ in tg))
    return '+'.join(kinds)
b, h = load(base_p), load(head_p)
agg = collections.Counter()
rows = []
for key in sorted(set(b) | set(h), key=lambda k: tuple('' if x is None else str(x) for x in k)):
    bs, hs = b.get(key), h.get(key)
    part = 'test' if is_test(key[0]) else 'product'
    if bs is None or hs is None:
        agg[(part, 'RECORD ' + ('added' if bs is None else 'removed'), summary(hs or bs))] += 1
        rows.append({'key': key, 'base': bs, 'head': hs, 'change': 'record'})
        continue
    bt = sorted(t for st in bs for t in st[0]); ht = sorted(t for st in hs for t in st[0])
    bd = sorted(set(st[1] for st in bs if st[1])); hd = sorted(set(st[1] for st in hs if st[1]))
    if bt == ht and bd == hd:
        agg[(part, 'same', '')] += 1
        continue
    agg[(part, summary(bs) + ' -> ' + summary(hs), '')] += 1
    rows.append({'key': key, 'line': bs[0][2], 'base_targets': bt, 'head_targets': ht, 'base_drop': bd, 'head_drop': hd, 'part': part,
                 'change': summary(bs) + ' -> ' + summary(hs)})
fd = os.open(out_p, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
with os.fdopen(fd, 'w') as f:
    for r in rows:
        f.write(json.dumps(r) + '\n')
print(f"sites base {sum(len(v) for v in b.values())} head {sum(len(v) for v in h.values())}; changed rows {len(rows)}")
for (part, change, extra), n in sorted(agg.items(), key=lambda kv: (kv[0][0], -kv[1])):
    print(f"{n:8d}  {part:7s} {change} {extra}")
