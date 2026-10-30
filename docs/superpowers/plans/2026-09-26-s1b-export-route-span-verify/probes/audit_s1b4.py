"""R3 audit inventory; deliberately does not infer value flow from lexical binding.

Usage: audit_s1b4.py ROOT ROWDIFF OUT [--private]
An identity-preserving Exact -> NameOnly change loses no target. All removals,
retargets and additions require source/export/value-flow hand audit. The optional
lexical annotation shares jsscope's documented position and alias blind spots;
it never certifies a removed edge as wrong. --private prints aggregates only.
"""
import collections
import json
import os
import sys
from jsscope import Src, LANG


def identities(targets):
    return collections.Counter(tuple(t[:4]) for t in targets)


def main():
    root, rowdiff, output = sys.argv[1:4]
    private = '--private' in sys.argv[4:]
    sources = {}
    results = []
    counts = collections.Counter()
    for row in json.load(open(rowdiff)):
        bd, bt = row['base']
        hd, ht = row['proto']
        removed = list((identities(bt) - identities(ht)).elements())
        gained = list((identities(ht) - identities(bt)).elements())
        if not removed and not gained and bt and ht:
            demotion = (all(t[4:] == ['exact', 'import_qualified'] for t in bt)
                        and all(t[4:] == ['name_only', 'import_qualified'] for t in ht))
            category = 'exact_to_nameonly_edges_kept' if demotion else 'identity_kept_check_grade'
        elif removed:
            category = 'removed_needs_valueflow' if not ht else 'retargeted_needs_valueflow'
        elif gained:
            category = 'added_needs_export_audit'
        else:
            category = 'relabel_no_edges'
        sf, sb, eb = row['key'][3:6]
        annotation = {'status': 'unavailable'}
        ext = os.path.splitext(sf)[1]
        if ext in LANG:
            s = sources.setdefault(sf, None)
            if s is None:
                s = sources[sf] = Src(os.path.join(root, sf), ext)
            n = s.node_at(sb, eb)
            while n is not None and (n.start_byte != sb or n.end_byte != eb):
                n = n.parent
            if n is not None:
                member = n.child_by_field_name('function') if n.type == 'call_expression' else n.child_by_field_name('name')
                q = member.child_by_field_name('object') if member is not None else None
                if q is not None and q.type == 'identifier':
                    name = s.src[q.start_byte:q.end_byte].decode()
                    scope, decls = s.binding(q, name)
                    annotation = {'qualifier': name,
                                  'decl_kinds': [d.type for d in decls],
                                  'scope': scope.type if scope is not None else None,
                                  'not_a_valueflow_proof': True}
        counts[category] += 1
        results.append(dict(row, audit_class=category, removed_identities=removed,
                            gained_identities=gained, lexical_annotation=annotation))
    with open(output, 'w') as f:
        json.dump(results, f, indent=1)
    print(json.dumps(dict(sorted(counts.items())), sort_keys=True))
    if not private:
        for r in results:
            if r['removed_identities'] or r['gained_identities']:
                print('HAND_AUDIT', r['audit_class'], r['key'])


if __name__ == '__main__':
    main()
