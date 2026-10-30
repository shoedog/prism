"""Edge-level taxonomy of an audited row-diff (audit_rowdiff.py output). Prints aggregates only (safe for corpus F).

Usage: python3 taxonomy.py <audit.json>
Per changed row, each base target edge is either kept, re-targeted away, or removed. Every removed edge gets one
semantic class, decided by the auditor's reason and, for call-wrapped declarators, the wrapper callee:
  W  statically wrong: the name never denotes that callable (shadowing, intrinsic tag, out-of-scope or over-named
     candidate, loader/subscription return, styled factory callback, the extra targets of a multi-target row)
  M  may-call: the binding holds a different callable that may run the target later (throttle/debounce/memoize,
     rendering HOCs, test mocks), or holds the target only on some paths (written / later-assigned bindings)
  P  pass-through: the wrapper returns the argument itself (React useCallback)
r1 fold: owner E5 keeps may-call rows at base, so M marks an E5 violation (it must be 0), and `useCallback` rows
(P) are admitted callables (E4). M is decided by the auditor (a written binding, or a call-wrapped declarator with a
direct function argument), not by the declaration keyword.
"""
import json, sys, collections

PASS = {'useCallback', 'React.useCallback'}
MAYCALL = {'throttle', 'debounce', 'throttleRAF', 'memoize', 'memoizeOne', 'Utils.memoize', 'ts.memoize',
           'useStableCallback', 'withInternalFallback', 'vi.fn', 'jest.fn'}


def klass(row):
    v = row['verdict'] or ''
    if v.startswith('MAYCALL'):
        return 'M'
    return 'W'


rows = json.load(open(sys.argv[1]))
edges = collections.Counter()
rowc = collections.Counter()
for r in rows:
    b, p = r['base'][1], r['proto'][1]
    bset = {tuple(t[:4]) for t in b}
    pset = {tuple(t[:4]) for t in p}
    rowc[r['class']] += 1
    if r['class'] == 'relabel_intrinsic':
        continue
    edges[('kept', r['class'])] += len(bset & pset)
    edges[('added', r['class'])] += len(pset - bset)
    removed = bset - pset
    if r['class'] == 'retargeted_right':
        edges[('removed', 'W', 'retarget_extra')] += len(removed)
    elif r['class'] in ('removed_right', 'removed_right_parse'):
        edges[('removed', 'RIGHT', 'parse_recovery_or_other')] += len(removed)
    else:
        k = klass(r)
        edges[('removed', k, (r['verdict'] or '').split(':', 1)[-1] + ':' + str((r.get('audit') or {}).get('detail'))
               if k != 'W' else (r['verdict'] or '').split(':', 1)[-1])] += len(removed)
print('rows', dict(rowc))
tot = collections.Counter()
for k, n in sorted(edges.items(), key=lambda kv: (kv[0][0], str(kv[0][1:]))):
    print('%6d  %s' % (n, ' | '.join(map(str, k))))
    tot[k[0] if k[0] != 'removed' else 'removed_' + k[1]] += n
print('edge totals', dict(tot))
