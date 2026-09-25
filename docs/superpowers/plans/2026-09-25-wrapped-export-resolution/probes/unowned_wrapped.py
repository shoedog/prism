"""P14b: the wrapper-specific unowned sub-lane: call/JSX nodes inside a function that is NOT named by
function_name Pattern 3 because it sits under a nested wrapper call (memo(forwardRef(fn)), Object.assign(forwardRef(fn), ..))
or behind a cast (`forwardRef(fn) as T`), in a top-level declarator. Usage: python3 unowned_wrapped.py <root>"""
import sys, os, collections
sys.path.insert(0, os.path.dirname(__file__))
from census import LANG, walk_files, txt, CASTS, FN
import tree_sitter as ts
root = sys.argv[1]
c = collections.Counter(); decls = collections.Counter()
def calls_in(n):
    k = 0; st = [n]
    while st:
        x = st.pop()
        if x.type in ('call_expression', 'jsx_self_closing_element', 'jsx_opening_element'): k += 1
        st.extend(x.children)
    return k
for rel in walk_files(root):
    p = os.path.join(root, rel)
    if os.path.getsize(p) > 2 * 1024 * 1024: continue
    src = open(p, 'rb').read()
    t = ts.Parser(LANG[os.path.splitext(rel)[1]]).parse(src)
    for st in t.root_node.named_children:
        d = st.child_by_field_name('declaration') if st.type == 'export_statement' else st
        if d is None or d.type not in ('lexical_declaration', 'variable_declaration'): continue
        for v in d.named_children:
            if v.type != 'variable_declarator': continue
            val = v.child_by_field_name('value')
            if val is None: continue
            cast = False
            while val is not None and val.type in CASTS:
                cast = True; val = val.named_children[0] if val.named_children else None
            if val is None or val.type != 'call_expression': continue
            args = val.child_by_field_name('arguments')
            if args is None: continue
            for a in args.named_children:
                if a.type in FN and cast:
                    c['behind_cast'] += calls_in(a); decls['behind_cast'] += 1
                if a.type == 'call_expression':
                    ia = a.child_by_field_name('arguments')
                    for b in (ia.named_children if ia is not None else []):
                        if b.type in FN:
                            key = '%s(%s(fn))' % (' '.join(txt(src, val.child_by_field_name('function')).split())[:20], ' '.join(txt(src, a.child_by_field_name('function')).split())[:20])
                            c[key] += calls_in(b); decls[key] += 1
print('call/JSX nodes owned by nothing due to wrapper nesting/cast:', sum(c.values()), 'declarators', sum(decls.values()))
for k, n in c.most_common(12): print(f'  {n:5d} calls in {decls[k]} decls  {k}')
