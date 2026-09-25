"""P2 census: classify module-level exported variable-declarator initializers in JS/TS files.

Usage: python3 census.py <repo_root> <functions.json> <out.json>
Population = files present in prism's `nav functions` inventory for the same repo (the indexed set that
has >=1 function) UNION every JS/TS file under root not in BUILTIN_SKIP_DIRS (reported separately).
Grammar versions match Cargo.lock (tree-sitter-typescript 0.23.2, tree-sitter-javascript 0.23.1).
"""
import json, os, sys, collections, hashlib
import tree_sitter as ts, tree_sitter_typescript as tst, tree_sitter_javascript as tsj
LANG = {'.ts': ts.Language(tst.language_typescript()), '.tsx': ts.Language(tst.language_tsx()),
        '.js': ts.Language(tsj.language()), '.jsx': ts.Language(tsj.language()),
        '.mjs': ts.Language(tsj.language()), '.cjs': ts.Language(tsj.language())}
SKIP = {'.git', 'target', 'node_modules', 'vendor', 'dist', 'build'}
CASTS = {'as_expression', 'satisfies_expression', 'parenthesized_expression', 'non_null_expression', 'type_assertion'}
FN = {'arrow_function', 'function_expression', 'function'}

def txt(src, n): return src[n.start_byte:n.end_byte].decode('utf8', 'replace')

def callee_name(src, call):
    f = call.child_by_field_name('function')
    if f is None: return '?'
    t = ' '.join(txt(src, f).split())
    return t if len(t) <= 40 else t[:40] + '...'

def unwrap(n):
    casts = []
    while n is not None and n.type in CASTS:
        casts.append(n.type)
        n = n.named_children[0] if n.named_children else None
    return n, casts

def classify(src, v):
    """Return (shape, detail). Shapes are mutually exclusive."""
    v, casts = unwrap(v)
    if v is None: return ('other', 'empty')
    if v.type in FN:
        return ('fn_behind_cast' if casts else 'fn_direct', v.type)
    if v.type == 'call_expression':
        cal = callee_name(src, v)
        args = v.child_by_field_name('arguments')
        if args is None or args.type == 'template_string':
            return ('tagged_template', cal.split('.')[0].split('(')[0])
        named = args.named_children
        fn_args = [a for a in named if a.type in FN]
        call_args = [a for a in named if a.type == 'call_expression']
        id_args = [a for a in named if a.type == 'identifier']
        if casts:
            return ('call_behind_cast', cal)
        if fn_args:
            pos = [i for i, a in enumerate(named) if a.type in FN]
            # function_name Pattern 3 names a fn that is a DIRECT argument of a call whose parent is the declarator.
            if len(fn_args) == 1 and pos == [0] and len(named) == 1:
                return ('call_fn_sole_arg', cal)
            if len(fn_args) == 1 and pos == [0]:
                return ('call_fn_first_arg_plus', cal)
            if len(fn_args) == 1:
                return ('call_fn_later_arg', cal)
            return ('call_fn_multi', cal)
        # nested wrapper: memo(forwardRef(fn)) etc.
        for a in call_args:
            inner_args = a.child_by_field_name('arguments')
            if inner_args is not None and any(x.type in FN for x in inner_args.named_children):
                return ('call_nested_fn', cal + '(' + callee_name(src, a) + ')')
        if len(named) >= 1 and named[0].type == 'identifier':
            return ('call_ident_first_arg', cal)
        return ('call_no_fn_arg', cal)
    if v.type in ('identifier',): return ('identifier', '')
    if v.type in ('member_expression', 'subscript_expression'): return ('member', '')
    if v.type == 'object': return ('object', '')
    if v.type in ('string', 'template_string', 'number', 'true', 'false', 'null', 'undefined', 'regex', 'array'): return ('literal', v.type)
    if v.type in ('new_expression', 'await_expression', 'ternary_expression', 'binary_expression', 'class', 'unary_expression'): return ('expr', v.type)
    return ('other', v.type)

def walk_files(root):
    for dp, dns, fns in os.walk(root):
        dns[:] = sorted(d for d in dns if d not in SKIP and not d.startswith('.'))
        for f in sorted(fns):
            ext = os.path.splitext(f)[1]
            if ext in LANG and not f.startswith('.') and not f.endswith('.d.ts'):
                yield os.path.relpath(os.path.join(dp, f), root)

def main():
    root, fnjson, out = sys.argv[1], sys.argv[2], sys.argv[3]
    inv = json.load(open(fnjson))
    inv = inv if isinstance(inv, list) else inv.get('functions', [])
    indexed = {x['file'] for x in inv}
    rows = []
    manifest = []
    for rel in walk_files(root):
        p = os.path.join(root, rel)
        if os.path.getsize(p) > 2 * 1024 * 1024: continue
        src = open(p, 'rb').read()
        manifest.append((hashlib.sha256(src).hexdigest(), rel))
        tree = ts.Parser(LANG[os.path.splitext(rel)[1]]).parse(src)
        for st in tree.root_node.named_children:
            if st.type != 'export_statement': continue
            decl = st.child_by_field_name('declaration')
            if decl is None or decl.type not in ('lexical_declaration', 'variable_declaration'): continue
            kw = decl.children[0].type if decl.children else '?'
            for d in decl.named_children:
                if d.type != 'variable_declarator': continue
                nm = d.child_by_field_name('name'); v = d.child_by_field_name('value')
                if nm is None or nm.type != 'identifier' or v is None: continue
                shape, detail = classify(src, v)
                rows.append({'file': rel, 'line': d.start_point[0] + 1, 'name': txt(src, nm), 'kw': kw,
                             'shape': shape, 'detail': detail, 'indexed': rel in indexed})
    json.dump({'root': root, 'rows': rows, 'file_manifest': manifest}, open(out, 'w'), indent=0)
    c = collections.Counter((r['shape'], r['detail']) for r in rows if r['shape'] != 'fn_direct')
    by = collections.Counter(r['shape'] for r in rows)
    print('declarators', len(rows), 'by shape', dict(sorted(by.items(), key=lambda x: -x[1])))
    for (s, d), n in c.most_common(40): print(f'  {n:5d} {s:24s} {d}')
if __name__ == '__main__':
    main()
