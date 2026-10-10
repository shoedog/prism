#!/usr/bin/env python3
"""Multiset base/head diff of wire or owner/byte-enriched DFG rows (EVALUATION.md §3.5 classes).

Usage: rowdiff.py BASE.jsonl HEAD.jsonl OUT.json [--rows OUT_ROWS.jsonl]

Classes, per row identity = row without its label fields (confidence, doubt, kill_line):
  ADDED       identity count grows and no same-identity base row was relabelled into it
  LOST        identity count shrinks
  RELABELLED  same identity, a base label replaced by a different head label (paired 1:1)
  RE-OWNED    (PR-B) a LOST row and an ADDED row whose identities differ ONLY in the endpoint
              `owner` objects are paired 1:1 and reported as RE-OWNED (with base/head owners and
              labels), never as an independent LOST plus ADDED. Wire records omit owners, so the
              pairing is inert there; call-site rows are checked separately
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
    # PR-B: pair owner-only replacements (identity minus endpoint owners) as RE-OWNED.
    def ownerless(row):
        r = json.loads(json.dumps(row))
        for end in ('from', 'to'):
            if isinstance(r.get(end), dict):
                r[end].pop('owner', None)
        return json.dumps(r, sort_keys=True)
    lost_by = defaultdict(list)
    for i, c in enumerate(changed):
        if c['class'] == 'LOST':
            lost_by[ownerless(c['row'])].append(i)
    reowned = set()
    for i, c in enumerate(changed):
        if c['class'] != 'ADDED':
            continue
        bucket = lost_by.get(ownerless(c['row']))
        if not bucket:
            continue
        j = bucket.pop()
        lost = changed[j]
        reowned.update((i, j))
        counts['ADDED'] -= 1
        counts['LOST'] -= 1
        counts['LOST:' + lost['base_label']['confidence']] -= 1
        lab = c['head_label']
        counts['ADDED:%s:%s' % (lab['confidence'], lab['doubt'])] -= 1
        counts['ADDED:%s->%s' % (c['row']['from']['access'], c['row']['to']['access'])] -= 1
        counts['RE-OWNED'] += 1
        counts['RE-OWNED:%s->%s' % (lost['base_label']['confidence'], lab['confidence'])] += 1
        changed.append({'class': 'RE-OWNED', 'row': c['row'], 'base_row': lost['row'],
                        'base_label': lost['base_label'], 'head_label': lab})
    changed = [c for k, c in enumerate(changed) if k not in reowned]
    counts = Counter({k: v for k, v in counts.items() if v})
    summary = {'base_rows': sum(b.values()), 'head_rows': sum(h.values()),
               'RE-OWNED-rule': 'LOST+ADDED pairs differing only in endpoint owners are RE-OWNED',
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
