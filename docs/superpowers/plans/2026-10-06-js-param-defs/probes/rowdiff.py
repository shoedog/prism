#!/usr/bin/env python3
"""Multiset base/head diff of `nav dfg-stats --edges` rows (EVALUATION.md §3.5 classes).

Usage: rowdiff.py BASE.jsonl HEAD.jsonl OUT.json [--rows OUT_ROWS.jsonl]

Classes, per row identity = row without its label fields (confidence, doubt, kill_line):
  ADDED       identity count grows and no same-identity base row was relabelled into it
  LOST        identity count shrinks
  RELABELLED  same identity, a base label replaced by a different head label (paired 1:1)
  RE-OWNED    not observable on dfg rows (rows carry no owner field); reported as n/a
Aggregate-only stdout; --rows writes the changed rows (private for F).
"""
import json
import sys
from collections import Counter, defaultdict

LABEL = ('confidence', 'doubt', 'kill_line')


def load(path):
    rows = Counter()
    with open(path, 'rb') as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows[line] += 1
    return rows


def split(raw):
    row = json.loads(raw)
    ident = {k: v for k, v in row.items() if k not in LABEL}
    label = {k: row.get(k) for k in LABEL}
    return json.dumps(ident, sort_keys=True), json.dumps(label, sort_keys=True), row


def main():
    base, head, out = sys.argv[1:4]
    rows_out = sys.argv[sys.argv.index('--rows') + 1] if '--rows' in sys.argv else None
    b, h = load(base), load(head)
    by_ident = defaultdict(lambda: [Counter(), Counter()])
    for raw, n in b.items():
        i, l, _ = split(raw)
        by_ident[i][0][l] += n
    for raw, n in h.items():
        i, l, _ = split(raw)
        by_ident[i][1][l] += n
    changed = []
    counts = Counter()
    for ident, (bl, hl) in sorted(by_ident.items()):
        if bl == hl:
            continue
        only_b = bl - hl
        only_h = hl - bl
        lost = list(only_b.elements())
        added = list(only_h.elements())
        pairs = min(len(lost), len(added))
        for k in range(pairs):
            counts['RELABELLED'] += 1
            counts['RELABELLED:%s->%s' % (json.loads(lost[k])['confidence'], json.loads(added[k])['confidence'])] += 1
            changed.append({'class': 'RELABELLED', 'row': json.loads(ident), 'base_label': json.loads(lost[k]), 'head_label': json.loads(added[k])})
        for l in lost[pairs:]:
            counts['LOST'] += 1
            counts['LOST:' + json.loads(l)['confidence']] += 1
            changed.append({'class': 'LOST', 'row': json.loads(ident), 'base_label': json.loads(l)})
        for l in added[pairs:]:
            counts['ADDED'] += 1
            lab = json.loads(l)
            counts['ADDED:%s:%s' % (lab['confidence'], lab['doubt'])] += 1
            r = json.loads(ident)
            counts['ADDED:%s->%s' % (r['from']['access'], r['to']['access'])] += 1
            changed.append({'class': 'ADDED', 'row': r, 'head_label': lab})
    summary = {'base_rows': sum(b.values()), 'head_rows': sum(h.values()), 'RE-OWNED': 'n/a (dfg rows carry no owner)',
               **dict(sorted(counts.items()))}
    with open(out, 'w') as fh:
        json.dump(summary, fh, indent=1, sort_keys=True)
    if rows_out:
        with open(rows_out, 'w') as fh:
            for c in changed:
                fh.write(json.dumps(c, sort_keys=True) + '\n')
    print(json.dumps(summary, sort_keys=True))


if __name__ == '__main__':
    main()
