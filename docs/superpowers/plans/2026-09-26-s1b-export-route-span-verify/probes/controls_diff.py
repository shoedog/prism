"""Per-scenario diff of two controls_summarize.py outputs. Rows whose only change is drop UnknownName -> JsxIntrinsic
are counted separately (`--hide-intrinsic` suppresses them). Usage: controls_diff.py A/SUMMARY.txt B/SUMMARY.txt"""
import sys, re, difflib
def sections(p):
    out = {}; cur = None
    for l in open(p):
        if l.startswith('== '):
            cur = l.split()[1]; out[cur] = []
        elif cur:
            out[cur].append(l.rstrip('\n'))
    return out
a, b = sections(sys.argv[1]), sections(sys.argv[2])
hide = '--hide-intrinsic' in sys.argv
relabel = 0
for k in sorted(set(a) | set(b)):
    x, y = a.get(k, []), b.get(k, [])
    if x == y:
        continue
    lines = []
    for l1, l2 in zip(x, y):
        if l1 == l2:
            continue
        if l1.replace('drop=UnknownName', 'drop=JsxIntrinsic') == l2:
            relabel += 1
            if hide:
                continue
        lines.append('   - ' + l1.strip() + '\n   + ' + l2.strip())
    if len(x) != len(y):
        lines.append('   (row count %d -> %d)' % (len(x), len(y)))
    if lines:
        print('==', k)
        print('\n'.join(lines))
print('UnknownName->JsxIntrinsic relabels:', relabel)
