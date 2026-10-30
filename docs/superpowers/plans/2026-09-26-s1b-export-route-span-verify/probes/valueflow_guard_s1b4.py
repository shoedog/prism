"""Inventory every lost target before making a zero-right-loss claim.

Usage: valueflow_guard_s1b4.py BASE_DUMP HEAD_DUMP OUT_JSON [--private]
This checks complete dumps, not rowdiff.py's intersection or lexical denotation.
If losses is empty, no edge identity was lost, regardless of aliases or parameters.
Otherwise EVERY listed loss needs an independent value-flow/source audit; this
script never calls a lexical refusal proof that the old target was wrong.
"""
import collections
import json
import sys


def load(path):
    rows = {}
    for line in open(path):
        r = json.loads(line)
        if r.get('record_kind') != 'call_site':
            continue
        c, s = r['caller'], r['source_span']
        key = (c['file'], c['name'], c['start_line'], c['end_line'],
               s['file'], s['start_byte'], s['end_byte'], r['callee_text'])
        if key in rows:
            raise ValueError('duplicate site key: inadmissible comparison')
        rows[key] = r
    return rows


def identities(row):
    return collections.Counter((t['function_id']['file'], t['function_id']['name'],
                                t['function_id']['start_line'], t['function_id']['end_line'])
                               for t in row.get('resolved_targets') or [])


def main():
    base, head, output = sys.argv[1:4]
    a, b = load(base), load(head)
    losses = []
    for key, row in sorted(a.items()):
        missing = identities(row) - identities(b.get(key, {}))
        if missing:
            losses.append({'site': key, 'lost_targets': list(missing.elements()),
                           'base': row, 'head': b.get(key),
                           'requires_independent_valueflow_audit': True})
    report = dict(base_sites=len(a), head_sites=len(b),
                  keys_only_base=len(set(a) - set(b)), keys_only_head=len(set(b) - set(a)),
                  lost_target_count=sum(len(r['lost_targets']) for r in losses), losses=losses)
    with open(output, 'w') as f:
        json.dump(report, f, indent=1)
    print(json.dumps({k: v for k, v in report.items() if k != 'losses'}, sort_keys=True))
    if '--private' not in sys.argv[4:]:
        for row in losses:
            print('VALUEFLOW_AUDIT', row['site'], row['lost_targets'])


if __name__ == '__main__':
    main()
