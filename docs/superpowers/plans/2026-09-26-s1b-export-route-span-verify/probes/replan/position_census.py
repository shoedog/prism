"""RP3: evaluation-position census for unqualified JS/TS call and JSX-tag sites (tree-sitter-python, grammars
pinned as in Cargo.lock). Independent of prism.

For each site identifier, walk to the root and record every (parent_kind, field_of_child) pair crossed. Report:
  - the distinct pairs and their site counts (the data-derived positive allowlist);
  - per E-position class (parameter env, computed key, decorator, heritage, namespace body, enum body, `with`,
    static block, TDZ heads), the number of sites whose path crosses it;
  - optionally, the join with a base->v8d rowdiff.json: how many changed rows cross each class.

Usage: python3 position_census.py <repo_root> <out.json> [--rowdiff <rowdiff.json>] [--private]
"""
import os, sys, json, collections
import tree_sitter as ts, tree_sitter_typescript as tst, tree_sitter_javascript as tsj

LANG = {'.ts': ts.Language(tst.language_typescript()), '.tsx': ts.Language(tst.language_tsx()),
        '.mts': ts.Language(tst.language_typescript()), '.cts': ts.Language(tst.language_typescript()),
        '.js': ts.Language(tsj.language()), '.jsx': ts.Language(tsj.language()),
        '.mjs': ts.Language(tsj.language()), '.cjs': ts.Language(tsj.language())}
FUNCS = {'function_declaration', 'generator_function_declaration', 'function_expression', 'function',
         'generator_function', 'arrow_function', 'method_definition'}
SKIP_DIRS = {'node_modules', '.git', 'dist', 'build', 'target', '.next', 'coverage'}


def classify_pair(parent, field, child):
    """The evaluation-context class of a (parent kind, child field) position, or None when structural."""
    k = parent.type
    if k in FUNCS and field in ('parameters', 'parameter'):
        return 'param_env'
    if child.type == 'computed_property_name':
        return 'computed_key:' + k
    if child.type == 'decorator' or k == 'decorator':
        return 'decorator'
    if k == 'class_heritage' or child.type == 'class_heritage':
        return 'heritage'
    if k in ('internal_module', 'module') and field == 'body':
        return 'namespace_body'
    if k in ('enum_body', 'enum_assignment'):
        return 'enum_body'
    if k == 'with_statement':
        return 'with_' + (field or 'child')
    if k == 'class_static_block':
        return 'static_block'
    if k == 'for_in_statement' and field == 'right':
        return 'for_in_right'
    if k == 'for_statement' and field in ('initializer', 'condition', 'increment'):
        return 'for_head'
    if k == 'catch_clause' and field == 'parameter':
        return 'catch_param'
    return None


def field_of(parent, child):
    # tree-sitter-python: find the field name under which `child` sits in `parent`.
    for i in range(parent.child_count):
        if parent.child(i).id == child.id:
            return parent.field_name_for_child(i)
    return None


def sites(root):
    stack = [root]
    while stack:
        n = stack.pop()
        if n.type == 'call_expression':
            f = n.child_by_field_name('function')
            if f is not None and f.type == 'identifier':
                yield n, f, 'call'
        elif n.type in ('jsx_opening_element', 'jsx_self_closing_element'):
            nm = n.child_by_field_name('name')
            if nm is not None and nm.type == 'identifier':
                yield n, nm, 'jsx'
        stack.extend(n.children)


def main():
    root_dir, out = sys.argv[1], sys.argv[2]
    rowdiff = None
    if '--rowdiff' in sys.argv:
        rowdiff = json.load(open(sys.argv[sys.argv.index('--rowdiff') + 1]))
    private = '--private' in sys.argv
    pairs = collections.Counter()
    classes = collections.Counter()
    class_sites = collections.defaultdict(list)
    total = 0
    files = 0
    site_index = {}
    for dp, dns, fns in os.walk(root_dir):
        dns[:] = [d for d in dns if d not in SKIP_DIRS]
        for fn in fns:
            ext = os.path.splitext(fn)[1]
            if ext not in LANG or fn.endswith('.d.ts'):
                continue
            path = os.path.join(dp, fn)
            rel = os.path.relpath(path, root_dir)
            try:
                src = open(path, 'rb').read()
            except OSError:
                continue
            tree = ts.Parser(LANG[ext]).parse(src)
            files += 1
            for site, ident, kind in sites(tree.root_node):
                total += 1
                crossed = set()
                n = ident
                while n.parent is not None:
                    p = n.parent
                    fld = field_of(p, n)
                    pairs[(p.type, fld or '<child>')] += 1
                    c = classify_pair(p, fld, n)
                    if c:
                        crossed.add(c)
                    n = p
                key = (rel, site.start_byte, site.end_byte, src[ident.start_byte:ident.end_byte].decode('utf8', 'replace'))
                site_index[key] = sorted(crossed)
                for c in crossed:
                    classes[c] += 1
                    if len(class_sites[c]) < 20 and not private:
                        class_sites[c].append([rel, site.start_point[0] + 1, key[3]])
    any_e = sum(1 for v in site_index.values() if v)
    res = {'files': files, 'sites': total, 'sites_on_e_positions': any_e,
           'classes': dict(sorted(classes.items())), 'distinct_pairs': len(pairs),
           'pairs': sorted(([k[0], k[1], v] for k, v in pairs.items()), key=lambda x: -x[2]),
           'examples': {} if private else dict(class_sites)}
    if rowdiff is not None:
        joined = collections.Counter()
        joined_cls = collections.Counter()
        missing = 0
        rows_on_e = []
        for r in rowdiff:
            k = r['key']
            if r['proto'][0] == 'JsxIntrinsic' and not r['base'][1]:
                continue  # intrinsic relabels: S1b-1, no edge
            key = (k[3], k[4], k[5], k[6])
            cr = site_index.get(key)
            if cr is None:
                missing += 1
                continue
            joined['changed_rows_matched'] += 1
            if cr:
                joined['changed_rows_on_e_positions'] += 1
                for c in cr:
                    joined_cls[c] += 1
                if not private and len(rows_on_e) < 40:
                    rows_on_e.append({'key': k, 'classes': cr, 'base': r['base'], 'proto': r['proto']})
        res['rowdiff_join'] = {'matched': joined['changed_rows_matched'], 'unmatched': missing,
                               'on_e_positions': joined['changed_rows_on_e_positions'],
                               'by_class': dict(sorted(joined_cls.items())), 'rows': rows_on_e}
    json.dump(res, open(out, 'w'), indent=1)
    print('files', files, 'sites', total, 'on E-positions', any_e, 'distinct pairs', len(pairs))
    for c, n in sorted(classes.items()):
        print('  %-40s %6d' % (c, n))
    if rowdiff is not None:
        j = res['rowdiff_join']
        print('rowdiff join: matched', j['matched'], 'unmatched', j['unmatched'], 'on E-positions', j['on_e_positions'])
        for c, n in sorted(j['by_class'].items()):
            print('  %-40s %6d' % (c, n))


if __name__ == '__main__':
    main()
