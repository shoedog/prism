"""RP4: re-check the rows the narrow E6 rule kept (broad refused) under Opus r2 W2's delimited-child sealing rule.

A parse error is sealed only when it lies inside a *delimited child* of a sealing node (function with a braced body,
class, static block) that is strictly inside the binding scope and does not contain the site: the `body`
(`statement_block` / `class_body`, braces) or the `parameters` (`formal_parameters`, parentheses). An error in the
header (name, type parameters, return type, heritage) is not sealed.

Usage: python3 rp4_delimited.py <repo_root> <vs-default.json> [--private]
"""
import os, sys, json
sys.path.insert(0, os.environ.get('S1B_PROBES', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')))
import jsscope
from jsscope import FUNCS, CLASSES, inside

DELIM = {'statement_block': ('{', '}'), 'class_body': ('{', '}'), 'formal_parameters': ('(', ')')}


def delimited_children(node):
    for f in ('body', 'parameters'):
        c = node.child_by_field_name(f)
        if c is not None and c.type in DELIM:
            o, cl = DELIM[c.type]
            if c.child_count >= 2 and c.child(0).type == o and c.child(c.child_count - 1).type == cl \
                    and not c.child(0).is_missing and not c.child(c.child_count - 1).is_missing:
                yield c


def sealing(u):
    return u.type in CLASSES or u.type == 'class_static_block' or (
        u.type in FUNCS and u.child_by_field_name('body') is not None
        and u.child_by_field_name('body').type == 'statement_block')


def sealed_delimited(scope, site):
    if not scope.has_error:
        return True, None
    stack = [scope]
    while stack:
        n = stack.pop()
        if n.is_error or n.is_missing:
            u = n.parent
            while u is not None and u.id != scope.id and not sealing(u):
                u = u.parent
            if u is None or u.id == scope.id or (site is not None and inside(u, site)):
                return False, ('unsealed', n.start_point[0] + 1)
            if not any(inside(d, n) for d in delimited_children(u)):
                return False, ('header_error', n.start_point[0] + 1, u.type)
        stack.extend(c for c in n.children if c.has_error or c.is_missing)
    return True, None


def main():
    root, vs = sys.argv[1:3]
    private = '--private' in sys.argv
    rows = json.load(open(vs))
    kept = fell = 0
    srcs = {}
    for r in rows:
        k = r['key']
        f, sb, eb, name = k[3], k[4], k[5], k[6]
        ext = os.path.splitext(f)[1]
        if f not in srcs:
            srcs[f] = jsscope.Src(os.path.join(root, f), ext)
        a = srcs[f]
        site = a.node_at(sb, eb)
        callee = site.child_by_field_name('function') if site.type == 'call_expression' else site.child_by_field_name('name')
        scope, ds = a.binding(callee if callee is not None else site, name)
        ok_narrow = a.sealed(scope, site) if scope is not None else None
        ok_delim, why = sealed_delimited(scope, site) if scope is not None else (None, None)
        if ok_delim:
            kept += 1
        else:
            fell += 1
        if not private:
            print('%-45s L%-5d %-30s narrow=%s delimited=%s %s' % (f, site.start_point[0] + 1, name, ok_narrow, ok_delim, why or ''))
    print('rows', len(rows), 'kept under delimited rule', kept, 'fall (now refused)', fell)


if __name__ == '__main__':
    main()
