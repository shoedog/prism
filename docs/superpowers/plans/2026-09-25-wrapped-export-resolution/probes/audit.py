"""P7 precision audit: every new Exact edge from the prototype must land on the FIRST argument function of an
`export const X = <react wrapper>(fn...)` declarator, with the wrapper callee imported from "react".
Usage: python3 audit.py <repo_root> <rowdiff.json>"""
import json, sys, os, collections
import tree_sitter as ts
sys.path.insert(0, os.path.dirname(__file__))
from census import LANG, txt
root, diff = sys.argv[1], sys.argv[2]
rows = json.load(open(diff))
cache = {}
def decls(f):
    if f in cache: return cache[f]
    src = open(os.path.join(root, f), 'rb').read()
    tree = ts.Parser(LANG[os.path.splitext(f)[1]]).parse(src)
    react_named, react_obj = {}, set()
    out = {}
    for st in tree.root_node.named_children:
        if st.type == 'import_statement' and txt(src, st.child_by_field_name('source')).strip('\'"') == 'react':
            for cl in st.named_children:
                if cl.type != 'import_clause': continue
                for c in cl.named_children:
                    if c.type == 'identifier': react_obj.add(txt(src, c))
                    if c.type == 'namespace_import': react_obj.update(txt(src, x) for x in c.named_children if x.type == 'identifier')
                    if c.type == 'named_imports':
                        for sp in c.named_children:
                            n = sp.child_by_field_name('name'); a = sp.child_by_field_name('alias')
                            if n is not None: react_named[txt(src, a or n)] = txt(src, n)
        if st.type == 'export_statement':
            d = st.child_by_field_name('declaration')
            if d is None or d.type != 'lexical_declaration': continue
            for v in d.named_children:
                if v.type != 'variable_declarator': continue
                val = v.child_by_field_name('value')
                if val is None or val.type != 'call_expression': continue
                cal = val.child_by_field_name('function'); args = val.child_by_field_name('arguments').named_children
                if cal.type == 'identifier': w = react_named.get(txt(src, cal))
                elif cal.type == 'member_expression' and txt(src, cal.child_by_field_name('object')) in react_obj: w = txt(src, cal.child_by_field_name('property'))
                else: w = None
                out[txt(src, v.child_by_field_name('name'))] = (w, args[0].type if args else None, args[0].start_point[0] + 1 if args else None, args[0].end_point[0] + 1 if args else None, len(args), d.children[0].type)
    cache[f] = out
    return out
ok = collections.Counter(); bad = []
for r in rows:
    (tfile, tname, ts_, te, conf, kind), = r['proto'][1]
    callee = r['key'][6]
    found = None
    for x, (w, k, s, e, n, kw) in decls(tfile).items():
        if (s, e) == (ts_, te): found = (x, w, k, n, kw)
    if found and found[1] in ('forwardRef', 'memo') and found[2] in ('arrow_function', 'function_expression') and found[4] == 'const':
        ok[(found[1], found[3])] += 1
    else:
        bad.append((r['key'], r['proto'], found))
print('audited', len(rows), 'verified', sum(ok.values()), dict(ok), 'unverified', len(bad))
for b in bad[:20]: print('  BAD', b)
