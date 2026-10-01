"""S1b-3 spec r2 (owner 2026-09-29): the value-flow cost of dropping parameter-bound callees.

Usage: python3 param_valueflow.py <repo_root> <rowdiff.json> <head-dump-sites.jsonl> <out.json> [--private]
For every removed row (base had targets, proto has none) whose callee the auditor binds to a parameter of an
enclosing function F: F's in-repo call sites are found syntactically (every call expression or JSX element in the
repo whose callee is F's name or `.name`; name-based, so over-counting only makes "single" rarer and the count a
lower bound). If there is exactly one, the argument at the parameter's position (for a destructured object
parameter, that object literal's member of the same name; for a JSX element, the attribute of that name) is read; the row is a RIGHT edge lost when that value is an identifier or a member
whose property names one of the base targets defined in the caller's own file (an identifier must lexically bind,
by the auditor, to that function's declaration), or an inline function whose file and
line span equal a base target's (an imported target is not credited: conservative, a lower bound).
Classes: lost_right (split exact / name_only by the base kind), passed_other, not_single_caller, unreadable.
"""
import json, os, sys, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from jsscope import Src, lines, LANG

root, rd, dump, out = sys.argv[1:5]
private = '--private' in sys.argv
srcs = {}


def S(f):
    if f not in srcs:
        ext = os.path.splitext(f)[1]
        srcs[f] = Src(os.path.join(root, f), ext) if ext in LANG else None
    return srcs[f]


def node_exact(s, sb, eb):
    n = s.node_at(sb, eb)
    while n is not None and not (n.start_byte == sb and n.end_byte == eb):
        n = n.parent
    return n


from census import walk_files
calls = collections.defaultdict(list)            # callee name -> [(file, call or JSX node)]
for rel in walk_files(root):
    src = S(rel)
    if src is None:
        continue
    stack = [src.root]
    while stack:
        n = stack.pop()
        if n.type == 'call_expression':
            c = n.child_by_field_name('function')
            if c is not None and c.type == 'member_expression':
                c = c.child_by_field_name('property')
            if c is not None and c.type in ('identifier', 'property_identifier'):
                calls[src.t(c)].append((rel, n))
        elif n.type in ('jsx_self_closing_element', 'jsx_opening_element'):
            c = n.child_by_field_name('name')
            if c is not None and c.type == 'identifier':
                calls[src.t(c)].append((rel, n))
        stack.extend(n.named_children)


def names_of(s, p):
    o = []
    s.pattern_names(p, o)
    return o


def value_for(s, param, arg, name):
    """The value `name` receives from `arg` through `param` (a parameter node), or None if unreadable."""
    pat = param.child_by_field_name('pattern') or param.child_by_field_name('name') or param
    if pat.type == 'assignment_pattern':
        pat = pat.child_by_field_name('left')
    if arg.type in ('jsx_self_closing_element', 'jsx_opening_element'):
        if pat.type != 'object_pattern':
            return None
        for a in arg.named_children:
            if a.type == 'jsx_attribute' and a.named_children and s.t(a.named_children[0]) == name:
                v = a.named_children[1] if len(a.named_children) > 1 else None
                return v.named_children[0] if v is not None and v.type == 'jsx_expression' and v.named_children else v
        return None
    if pat.type == 'identifier':
        return arg
    if pat.type == 'object_pattern' and arg.type == 'object':
        for m in arg.named_children:
            if m.type == 'shorthand_property_identifier' and s.t(m) == name:
                return m
            if m.type == 'pair' and s.t(m.child_by_field_name('key')).strip('\'"') == name:
                return m.child_by_field_name('value')
    return None


def matches(s, cfile, v, base):
    """The passed value is a base target defined in the caller's own file: named by an identifier or a member's
    property (same file), or the inline function itself (same file and line span)."""
    if v is None:
        return False
    local = [t for t in base if t[0] == cfile]
    if v.type in ('identifier', 'shorthand_property_identifier'):
        # The identifier must lexically bind to that very function in the caller's file (not a shadowing
        # parameter or local of the same name).
        _, ds = s.binding(v, s.t(v))
        spans = set()
        for d in ds:
            spans.add(lines(d))
            val = d.child_by_field_name('value') if d.type == 'variable_declarator' else None
            if val is not None:
                spans.add(lines(val))
        return any(s.t(v) == t[1] and (t[2], t[3]) in spans for t in local)
    if v.type == 'member_expression':
        return any(s.t(v.child_by_field_name('property')) == t[1] for t in local)
    if v.type in ('arrow_function', 'function_expression', 'function'):
        return any(lines(v) == (t[2], t[3]) for t in local)
    return False


res, cnt = [], collections.Counter()
for r in json.load(open(rd)):
    f, cn, cl, sf, sb, eb, callee = r['key']
    (bd, bt), (pd, pt) = r['base'], r['proto']
    if not bt or pt:
        continue
    s = S(sf)
    site = node_exact(s, sb, eb) if s else None
    ident = None
    if site is not None:
        ident = site.child_by_field_name('function') if site.type == 'call_expression' else site.child_by_field_name('name')
    if ident is None or ident.type != 'identifier':
        continue
    scope, ds = s.binding(ident, callee)
    FN = ('arrow_function', 'function_declaration', 'function_expression', 'function', 'method_definition',
          'generator_function', 'generator_function_declaration')
    if not ds:
        continue
    if ds[0].type == 'formal_parameters':
        fn = ds[0].parent
    elif ds[0].type == 'identifier' and ds[0].parent is not None and ds[0].parent.type == 'arrow_function':
        fn = ds[0].parent                                  # a single unparenthesized arrow parameter
    elif ds[0].type in FN:
        fn = ds[0]
    else:
        continue
    params = (fn.child_by_field_name('parameters').named_children if fn.child_by_field_name('parameters')
              else [fn.child_by_field_name('parameter')] if fn.child_by_field_name('parameter') else [])
    params = [p for p in params if p is not None and p.type != 'comment']
    idx = next((i for i, p in enumerate(params) if callee in names_of(s, p)), None)
    kind = 'exact' if bt[0][4] == 'exact' else 'name_only'
    if idx is None:
        continue                                           # not a parameter (a function's own name, `arguments`)
    else:
        fname = s.fn_name(fn)
        cs = calls.get(fname, []) if fname else []
        if len(cs) != 1:
            # Supplementary (not the owner's rule): every call site that supplies this argument passes a
            # base target, and at least one does.
            vals = []
            for cfile, call in cs:
                src2 = S(cfile)
                if call.type == 'call_expression':
                    a2 = call.child_by_field_name('arguments')
                    a2 = [a for a in (a2.named_children if a2 is not None else []) if a.type != 'comment']
                    if idx < len(a2):
                        vals.append((src2, cfile, value_for(src2, params[idx], a2[idx], callee)))
                elif idx == 0:
                    v2 = value_for(src2, params[idx], call, callee)
                    if v2 is not None:
                        vals.append((src2, cfile, v2))
            ok = bool(vals) and len(cs) > 0 and all(matches(a, b, v2, bt) for a, b, v2 in vals)
            c = 'multi_caller_all_suppliers_pass_target' if ok else 'not_single_caller'
        else:
            cfile, call = cs[0]
            cs_src = S(cfile)
            if call.type == 'call_expression':
                args = call.child_by_field_name('arguments')
                args = [a for a in (args.named_children if args is not None else []) if a.type != 'comment']
                v = value_for(cs_src, params[idx], args[idx], callee) if idx < len(args) else None
            else:
                v = value_for(cs_src, params[idx], call, callee) if idx == 0 else None
            if v is None:
                c = 'unreadable'
            elif matches(cs_src, cfile, v, bt):
                c = 'lost_right_' + kind
            else:
                c = 'passed_other'
    cnt[c] += 1
    res.append({'key': r['key'], 'class': c, 'base': bt})
json.dump(res, open(out, 'w'), indent=0)
for c, n in sorted(cnt.items()):
    print('%5d %s' % (n, c))
if not private:
    for x in res:
        if x['class'].startswith('lost_right'):
            print('LOST', x['key'][3], x['key'][1], x['key'][6], '->', x['base'][0][0], x['base'][0][1], x['base'][0][2])
