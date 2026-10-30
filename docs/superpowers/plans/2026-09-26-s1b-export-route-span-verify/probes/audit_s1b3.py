"""S1b-3 row-diff audit against the jsscope auditor's lexical binding (independent of prism).

Usage: python3 audit_s1b3.py <repo_root> <rowdiff.json> <out.json> [--private]
Classes per changed row (base -> proto):
  retargeted_right   Exact multi -> Exact single, and the kept target is the auditor's callable (span match)
  retargeted_other   Exact multi -> single, anything else
  removed_wrong      base had targets; the auditor binds the name in-file to something that is not any of them
                     (a parameter, a destructured or other non-callable declaration, a class, an enum, a
                     namespace, a marker, a duplicate, or a different in-file callable)
  removed_alias      the binding is a declarator (or `using`) whose unwrapped value is outside the closed
                     "holds no function" class, or a destructuring pattern holding a default (owner 2026-09-29): it may hold the base target by value
                     flow, so the removal may lose a right edge. Every such row is listed for a hand audit
  removed_parse      the auditor refuses the binding scope for parse recovery (E6): recall cost, audited by hand
  removed_unbound    the auditor finds no in-file binding and base bound a non-local route (a possible right
                     edge lost): audited by hand. Unbound with only `local_def` targets is removed_wrong
  removed_import     the auditor binds the name to an import: audited by hand
  added_right / added_other   base dropped, proto binds
  relabel            drop reason changed, no edge
--private prints counts only (corpus F); the JSON stays in the private evidence root.
"""
import json, os, sys, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from jsscope import Src, lines, LANG

root, rd, out = sys.argv[1:4]
private = '--private' in sys.argv
srcs = {}


def S(f):
    if f not in srcs:
        ext = os.path.splitext(f)[1]
        srcs[f] = Src(os.path.join(root, f), ext) if ext in LANG else None
    return srcs[f]


def callee_ident(s, sb, eb):
    n = s.node_at(sb, eb)
    while n is not None and not (n.start_byte == sb and n.end_byte == eb):
        n = n.parent
    if n is None:
        return None
    f = n.child_by_field_name('function') if n.type == 'call_expression' else n.child_by_field_name('name')
    return (f, n.type != 'call_expression') if f is not None and f.type == 'identifier' else None


NOFN = {'number', 'string', 'template_string', 'true', 'false', 'null', 'undefined', 'regex'}
UNWRAP = {'parenthesized_expression', 'as_expression', 'satisfies_expression', 'non_null_expression'}


def no_function(v):
    if v.type in NOFN:
        return True
    if v.type == 'array':
        return all(e.type == 'comment' or no_function(e) for e in v.named_children)
    if v.type == 'object':
        return all(m.type == 'comment' or (m.type == 'pair' and m.child_by_field_name('value') is not None
                                           and no_function(m.child_by_field_name('value')))
                   for m in v.named_children)
    return False


def has_default(p):
    stack = [p]
    while stack:
        n = stack.pop()
        if n.type in ('assignment_pattern', 'object_assignment_pattern'):
            return True
        stack.extend(n.named_children)
    return False


def alias_value(ds):
    """The value kind of a single alias declarator, else None. A destructuring pattern holding a default is an alias
    whatever its value (spec r2 sol W1: `{ missing: x = fallback } = {}` binds the default)."""
    if len(ds) != 1 or ds[0].type not in ('variable_declarator', 'assignment_expression'):
        return None
    pat = ds[0].child_by_field_name('left' if ds[0].type == 'assignment_expression' else 'name')
    if pat is not None and pat.type != 'identifier' and has_default(pat):
        return 'pattern_with_default'
    v = ds[0].child_by_field_name('right' if ds[0].type == 'assignment_expression' else 'value')
    while v is not None and (v.type in UNWRAP or v.type in ('type_assertion', 'assignment_expression')):
        v = v.child_by_field_name('right') if v.type == 'assignment_expression' else (
            v.named_children[-1] if v.type == 'type_assertion' else v.named_children[0])
    if v is None or v.type in ('arrow_function', 'function_expression') or no_function(v):
        return None
    return v.type


res = []
cnt = collections.Counter()
for r in json.load(open(rd)):
    f, cn, cl, sf, sb, eb, callee = r['key']
    (bd, bt), (pd, pt) = r['base'], r['proto']
    s = S(sf)
    ident, jsx = (callee_ident(s, sb, eb) if s else None) or (None, False)
    scope, ds = (s.binding(ident, callee) if ident is not None else (None, []))
    v = s.classify(scope, ds, callee, ident, jsx) if scope is not None else {'status': 'global'}
    if v['status'] == 'wrapped_jsx':
        v['status'] = 'callable'
    kinds = sorted({d.type for d in ds})
    if bt and pt:
        span = tuple(v.get('span') or ())
        ok = v['status'] == 'callable' and len(pt) == 1 and (pt[0][2], pt[0][3]) == span and pt[0][0] == sf
        c = 'retargeted_right' if ok else 'retargeted_other'
    elif bt and not pt:
        if alias_value(ds):
            c = 'removed_alias'
        elif scope is None and all(t[5] == 'local_def' for t in bt):
            c = 'removed_wrong'  # unbound at the site: every same-file function is out of scope
        elif scope is None:
            c = 'removed_unbound'
        elif v['status'] == 'parse':
            c = 'removed_parse'
        elif any(k in ('import_statement', 'import_alias') for k in kinds):
            c = 'removed_import'
        elif v['status'] == 'callable' and any(t[0] == sf and (t[2], t[3]) == tuple(v['span']) for t in bt):
            c = 'removed_right'
        else:
            c = 'removed_wrong'
    elif pt and not bt:
        span = tuple(v.get('span') or ())
        c = 'added_right' if v['status'] == 'callable' and (pt[0][2], pt[0][3]) == span else 'added_other'
    else:
        c = 'relabel'
    route = sorted({t[5] for t in bt}) if bt else []
    cnt[(c, ','.join(route))] += 1
    res.append({'key': r['key'], 'class': c, 'auditor': v.get('status'), 'detail': v.get('detail'),
                'decl_kinds': kinds, 'alias_value': alias_value(ds), 'base': r['base'], 'proto': r['proto']})
json.dump(res, open(out, 'w'), indent=0)
for (c, route), n in sorted(cnt.items()):
    print('%5d %-18s %s' % (n, c, route))
if not private:
    for x in res:
        if x['class'] not in ('retargeted_right', 'removed_wrong', 'relabel'):
            print('CHECK', x['class'], x['key'][3], x['key'][6], 'L?', x['auditor'], x['decl_kinds'])
