"""RP3b: for every site on an evaluation-context position (from position_census.py), join with the base
dump-sites (which rows carry targets, and of what kind) and, for namespace-body sites, ask the r2 auditor where
the name binds: inside the namespace body (safe under T4) or outside it (the walk leaves the body, where TS merge
rules apply and v8 is unsound).

Usage: python3 e_sites_join.py <repo_root> <base-dump-sites.jsonl> <out.json> [--private]
"""
import os, sys, json, collections
sys.path.insert(0, os.environ.get('S1B_PROBES', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')))
import jsscope
from position_census import LANG, SKIP_DIRS, sites, field_of, classify_pair


def main():
    root_dir, dump, out = sys.argv[1:4]
    private = '--private' in sys.argv
    base = {}
    for l in open(dump):
        r = json.loads(l)
        if r.get('record_kind') != 'call_site':
            continue
        s = r['source_span']
        base[(s['file'], s['start_byte'], s['end_byte'], r['callee_text'])] = r
    by_class = collections.defaultdict(collections.Counter)
    ns_detail = collections.Counter()
    examples = collections.defaultdict(list)
    for dp, dns, fns in os.walk(root_dir):
        dns[:] = [d for d in dns if d not in SKIP_DIRS]
        for fn in fns:
            ext = os.path.splitext(fn)[1]
            if ext not in LANG or fn.endswith('.d.ts'):
                continue
            path = os.path.join(dp, fn)
            rel = os.path.relpath(path, root_dir)
            src = None
            tree = None
            try:
                data = open(path, 'rb').read()
            except OSError:
                continue
            import tree_sitter as ts
            tree = ts.Parser(LANG[ext]).parse(data)
            auditor = None
            for site, ident, kind in sites(tree.root_node):
                crossed = {}
                n = ident
                while n.parent is not None:
                    p = n.parent
                    fld = field_of(p, n)
                    c = classify_pair(p, fld, n)
                    if c:
                        crossed[c] = p
                    n = p
                if not crossed:
                    continue
                name = data[ident.start_byte:ident.end_byte].decode('utf8', 'replace')
                key = (rel, site.start_byte, site.end_byte, name)
                row = base.get(key)
                kinds = tuple(sorted({(t['confidence'], t['kind']) for t in (row or {}).get('resolved_targets') or []}))
                status = 'no_base_row' if row is None else ('drop:' + str(row.get('drop')) if not kinds else 'targets:' + ','.join('%s/%s' % k for k in kinds))
                for c in crossed:
                    by_class[c][status] += 1
                if 'namespace_body' in crossed:
                    if auditor is None:
                        auditor = jsscope.Src(path, ext)
                    scope, ds = auditor.binding(auditor.node_at(ident.start_byte, ident.end_byte), name)
                    body = crossed['namespace_body']
                    # the innermost namespace body the path crosses
                    inner = None
                    m = ident
                    while m is not None:
                        if m.type in ('internal_module', 'module'):
                            inner = m.child_by_field_name('body')
                            break
                        m = m.parent
                    if scope is None:
                        where = 'unbound'
                    elif inner is not None and inner.start_byte <= scope.start_byte and scope.end_byte <= inner.end_byte:
                        where = 'binds_inside_namespace_body'
                    else:
                        where = 'walk_leaves_namespace_body'
                    ns_detail[(where, status)] += 1
                    if not private and where == 'walk_leaves_namespace_body' and len(examples[where]) < 25:
                        examples[where].append([rel, ident.start_point[0] + 1, name, status])
    res = {'by_class': {c: dict(v) for c, v in by_class.items()},
           'namespace_detail': [[w, s, n] for (w, s), n in sorted(ns_detail.items())],
           'examples': dict(examples)}
    json.dump(res, open(out, 'w'), indent=1)
    for c, v in sorted(by_class.items()):
        print(c)
        for s, n in v.most_common():
            print('   %-60s %6d' % (s, n))
    if ns_detail:
        print('namespace_body detail (where the name binds, base status):')
        for (w, s), n in sorted(ns_detail.items()):
            print('   %-32s %-50s %6d' % (w, s, n))


if __name__ == '__main__':
    main()
