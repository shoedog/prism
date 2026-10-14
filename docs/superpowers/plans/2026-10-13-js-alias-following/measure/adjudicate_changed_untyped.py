#!/usr/bin/env python3
"""Adjudicate CHANGED Exact targets between two site dumps without types, against the syntactic
value-following census (callresult rows: the callee a one-step returned-value follow derives).
  adjudicate_changed_untyped.py <edge-diff-rows.jsonl> <census sites.jsonl> <callresult-rows.jsonl>
For every changed site: what happened (gained / same target new route / target replaced / lost), and
whether the head's target equals the census-derived callee. Prints aggregate counts only."""
import json, sys, collections
diff_p, sites_p, cr_p = sys.argv[1:4]
census = {}
for l in open(sites_p):
    r = json.loads(l); span = (r.get('join_spans') or [r['span']])[0]
    census[(r['file'], span[0], span[1], r['name'])] = r
cr = {}
for l in open(cr_p):
    c = json.loads(l); cr[(c['file'], c['span'][0], c['span'][1], c['name'])] = c
agg = collections.Counter()
for l in open(diff_p):
    r = json.loads(l)
    if r.get('change') == 'record':
        agg[('RECORD ' + ('removed' if r.get('head') is None else 'added'), '', '', '')] += 1; continue
    file, sb, eb, callee, origin = r['key']
    bx = sorted(tuple(t[2:]) for t in r['base_targets'] if t[0] == 'exact'); hx = sorted(tuple(t[2:]) for t in r['head_targets'] if t[0] == 'exact')
    hkinds = '+'.join(sorted(set(t[1] for t in r['head_targets'] if t[0] == 'exact')))
    if not hx and bx: what = 'LOST exact'
    elif hx and not bx: what = 'GAINED exact'
    elif hx == bx: what = 'same target, route or grade changed'
    elif hx and bx: what = 'REPLACED target'
    else: what = 'non-exact change'
    c = cr.get((file, sb, eb, callee)); cen = census.get((file, sb, eb, callee))
    if not hx: verdict = '-'
    elif c is None: verdict = 'site not in the local census (' + ((cen or {}).get('group') or 'no census row') + ')'
    elif c['cls'].startswith('fn') and c.get('target'):
        ok = all(t[0] == c['target']['file'] and t[2] == c['target']['line'] for t in hx)
        verdict = ('equals' if ok else 'DIFFERS from') + f" the census-derived callee (hops {c['hops']})"
    else: verdict = 'census did not derive a callee: ' + c['cls']
    agg[(r['part'], f"{origin}", what + (f" [{hkinds}]" if hx else ''), verdict)] += 1
for k, v in sorted(agg.items(), key=lambda kv: (str(kv[0][0]), -kv[1])):
    print(f"{v:6d}  " + ' | '.join(str(x) for x in k))
