"""Audit every changed row of a base -> proto row-diff against the independent jsscope auditor (s1b_census.py JSON).

Usage: python3 audit_rowdiff.py <repo_root> <rowdiff.json> <base-census.json> <out.json> [--private]
Each changed row gets exactly one class:
  relabel_intrinsic        drop UnknownName -> JsxIntrinsic on a lowercase/dashed/namespaced plain JSX tag (no edge)
  removed_wrong            base had targets, proto drops, and the auditor says the base targets were WRONG
  removed_right            base had targets the auditor says were RIGHT, and proto drops them (a recall loss)
  retargeted_right         proto keeps exactly the auditor's expected target out of base's set
  retargeted_other         proto keeps a target the auditor did not expect
  added                    base dropped, proto binds (audited against the auditor where it can)
  other                    anything else
"""
import json, os, sys, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

root, rowdiff, census, out = sys.argv[1:5]
private = '--private' in sys.argv
rows = json.load(open(rowdiff))
cen = {}
for r in json.load(open(census)):
    if 'bytes' in r:
        cen[(r['file'], r['bytes'][0], r['bytes'][1], r['callee'])] = r
srcs = {}


def src(f):
    if f not in srcs:
        srcs[f] = open(os.path.join(root, f), 'rb').read()
    return srcs[f]


def intrinsic(f, sb, eb):
    t = src(f)[sb:eb].decode('utf8', 'replace')
    if not t.startswith('<'):
        return False
    tag = t[1:].split(None, 1)[0].rstrip('/>').split('>')[0]
    return bool(tag) and '.' not in tag and ((tag[0].isascii() and tag[0].islower()) or '-' in tag or ':' in tag)


res = []
cls = collections.Counter()
detail = collections.Counter()
for x in rows:
    k = x['key']  # caller file, caller name, caller start, site file, sb, eb, callee
    bdrop, btg = x['base']
    pdrop, ptg = x['proto']
    site = (k[3], k[4], k[5], k[6])
    c = cen.get(site)
    verdict = c['verdict'] if c else None
    audit = c.get('audit') if c else None
    if not btg and not ptg:
        kind = 'relabel_intrinsic' if pdrop == 'JsxIntrinsic' and intrinsic(k[3], k[4], k[5]) else 'other'
    elif btg and not ptg:
        if pdrop == 'JsxIntrinsic' and intrinsic(k[3], k[4], k[5]):
            kind = 'removed_wrong'
            verdict = verdict or 'WRONG:intrinsic_lowercase_tag'
        elif verdict and verdict.startswith('WRONG'):
            kind = 'removed_wrong'
        elif verdict == 'RIGHT':
            kind = 'removed_right'
        else:
            kind = 'other'
    elif btg and ptg:
        want = audit.get('span') if audit else None
        pt = [(t[0], t[2], t[3]) for t in ptg]
        if want and len(pt) == 1 and pt[0] == (k[3], want[0], want[1]):
            kind = 'retargeted_right'
        elif verdict == 'RIGHT' and sorted(map(tuple, btg)) != sorted(map(tuple, ptg)):
            kind = 'retargeted_other'
        else:
            kind = 'retargeted_other' if want else 'other'
    else:
        kind = 'added'
    cls[kind] += 1
    detail[(kind, bdrop or '-', tuple(sorted({t[5] for t in btg})), pdrop or '-',
            tuple(sorted({t[5] for t in ptg})), verdict or '-')] += 1
    res.append({'class': kind, 'verdict': verdict, 'audit': audit, **x})
json.dump(res, open(out, 'w'), indent=0)
print('classes', dict(cls))
for d, n in sorted(detail.items(), key=lambda kv: -kv[1]):
    print('%5d  %s' % (n, ' | '.join(str(p) for p in d)))
if not private:
    for r in res:
        if r['class'] in ('removed_right', 'retargeted_other', 'added', 'other'):
            print('  ', r['class'], r['key'][3], r['key'][6], r['base'], '->', r['proto'], r['verdict'], r['audit'])
