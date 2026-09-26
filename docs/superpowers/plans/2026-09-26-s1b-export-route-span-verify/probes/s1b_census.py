"""S1b census + independent audit of every resolved JS/TS row on the S1b routes.

Usage: python3 s1b_census.py <repo_root> <dump-sites.jsonl> <out.json> [--private]
For each call-site row whose caller is a JS/TS file and which has resolved targets, classify by route
(import_member / import_qualified / local_def, plus every kind for lowercase-JSX sites) and audit the targets
against jsscope.py's lexical binding (an independent re-derivation; it does not link prism).

Verdicts: RIGHT (targets == the one callable the name statically denotes), WRONG:<reason>, or
UNSURE:<reason> (the auditor cannot decide; listed for manual audit). --private suppresses file paths and
identifiers in the printed report (corpus F); the JSON keeps them and stays in the private evidence root.
"""
import json, os, sys, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from jsscope import Src, lines, FN_VALUES
from project import FileFacts, mod_candidates
from census import walk_files
import tree_sitter as ts
from jsscope import LANG

JS = ('.js', '.jsx', '.mjs', '.cjs', '.ts', '.tsx')
REACT = {'forwardRef', 'memo', 'React.forwardRef', 'React.memo'}


def main():
    root, dump, out = sys.argv[1], sys.argv[2], sys.argv[3]
    private = '--private' in sys.argv
    files = {}

    def S(rel):
        if rel not in files:
            p = os.path.join(root, rel)
            files[rel] = Src(p, os.path.splitext(rel)[1]) if os.path.exists(p) else None
        return files[rel]

    indexed = set(walk_files(root))
    facts = {}

    def F(rel):
        if rel not in facts:
            s = S(rel)
            facts[rel] = FileFacts(rel, s.src, s.tree) if s else None
        return facts[rel]

    def resolve_export(file, name, hops=0, seen=frozenset()):
        """Mirror of js_exports::resolve_one (depth 2, star barrels, poison) -> (file, local) | None."""
        if (file, name) in seen:
            return None
        seen = seen | {(file, name)}
        f = F(file)
        if f is None or name in f.conflicted:
            return None
        t = f.named.get(name)
        if t is not None:
            if t[0] in ('local', 'skipped_decl'):
                return (file, t[1])
            if t[0] == 'reexport':
                if hops + 1 > 2:
                    return None
                c = mod_candidates(t[1], file, indexed)
                return resolve_export(c[0], t[2], hops + 1, seen) if c else None
            return None
        if name == 'default' or not f.star or hops + 1 > 2:
            return None
        hits = set()
        for m in f.star:
            c = mod_candidates(m, file, indexed)
            if c:
                r = resolve_export(c[0], name, hops + 1, seen)
                if r:
                    hits.add(r)
        return hits.pop() if len(hits) == 1 else None

    def module_callable(file, local, jsx):
        s = S(file)
        ds = [d for nm, d in s.scope_decls(s.root) if nm == local]
        if len(ds) != 1:
            return {'status': 'dup' if ds else 'absent', 'detail': [d[0] for d in ds]}
        d = ds[0]
        c = s.callable_of(d)
        if c[0] is None:
            return {'status': 'not_callable', 'detail': '%s:%s' % (d[0], c[1])}
        if d[0] in ('let', 'var', 'function_decl') and s.written(s.root, d[1], local):
            return {'status': 'written', 'detail': d[0], 'span': lines(c[1])}
        if c[0] == 'wrapped':
            react = c[2] in REACT
            st = ('wrapped_jsx' if jsx else 'wrapped_nonjsx') + ('' if react else '_nonreact')
            return {'status': st, 'span': lines(c[1]), 'detail': c[2]}
        return {'status': 'callable', 'span': lines(c[1]), 'detail': d[0]}

    rows = []
    for line in open(dump):
        r = json.loads(line)
        if r.get('record_kind') != 'call_site':
            continue
        tg = r.get('resolved_targets') or []
        cf = r['caller']['file']
        if not tg or not cf.endswith(JS):
            continue
        kind = tg[0]['kind']
        conf = tg[0]['confidence']
        s = S(cf)
        sp = r['source_span']
        node = s.node_at(sp['start_byte'], sp['end_byte']) if s else None
        while node is not None and (node.start_byte, node.end_byte) != (sp['start_byte'], sp['end_byte']):
            node = node.parent
        if node is None:
            rows.append({'kind': kind, 'verdict': 'UNSURE:no_site_node', 'row': r})
            continue
        jsx = node.type in ('jsx_self_closing_element', 'jsx_opening_element')
        tag = node.child_by_field_name('name') if jsx else None
        ident = None
        qual = None
        if jsx:
            if tag is not None and tag.type == 'identifier':
                ident = tag
            elif tag is not None and tag.type == 'member_expression':
                qual = tag.child_by_field_name('object')
        else:
            fn = node.child_by_field_name('function')
            if fn is not None and fn.type == 'identifier':
                ident = fn
            elif fn is not None and fn.type == 'member_expression':
                qual = fn.child_by_field_name('object')
        lower_intrinsic = jsx and tag is not None and tag.type == 'identifier' and s.t(tag)[:1].islower() \
            and s.t(tag)[:1].isascii()
        name = r['callee_text']
        targets = sorted((t['function_id']['file'], t['function_id']['name'], t['function_id']['start_line'],
                          t['function_id']['end_line']) for t in tg)
        rec = {'origin': r.get('origin'), 'kind': kind, 'conf': conf, 'n': len(tg), 'jsx': jsx, 'lower': bool(lower_intrinsic),
               'file': cf, 'line': sp['line'], 'callee': name, 'targets': targets,
               'bytes': [sp['start_byte'], sp['end_byte']], 'caller': r['caller']['name'],
               'caller_start': r['caller']['start_line']}
        if lower_intrinsic:
            rec['verdict'] = 'WRONG:intrinsic_lowercase_tag'
        elif kind == 'local_def':
            if ident is None:
                rec['verdict'] = 'UNSURE:no_ident'
            else:
                a = s.resolve_callable(ident, name, jsx)
                if a['status'] == 'wrapped_jsx':
                    react = a.get('detail') in REACT
                    a['status'] = 'wrapped_jsx' + ('' if react else '_nonreact')
                if a['status'] == 'wrapped_nonjsx' and a.get('detail') not in REACT:
                    a['status'] = 'wrapped_nonjsx_nonreact'
                rec['audit'] = a
                want = [(cf, name, a['span'][0], a['span'][1])] if 'span' in a else None
                if a['status'] in ('callable', 'wrapped_jsx'):
                    ok = [t for t in targets if (t[0], t[2], t[3]) == (cf, a['span'][0], a['span'][1])]
                    rec['verdict'] = 'RIGHT' if len(targets) == 1 and ok else (
                        'WRONG:multi_incl_right' if ok else 'WRONG:wrong_target')
                else:
                    rec['verdict'] = 'WRONG:' + a['status']
        elif kind == 'import_member':
            t = targets[0]
            a = module_callable(t[0], t[1], jsx)
            rec['audit'] = a
            if a['status'] in ('callable', 'wrapped_jsx') and len(targets) == 1 and a['span'] == (t[2], t[3]):
                rec['verdict'] = 'RIGHT'
            elif a['status'] in ('callable', 'wrapped_jsx'):
                rec['verdict'] = 'WRONG:wrong_target'
            else:
                rec['verdict'] = 'WRONG:' + a['status']
        elif kind == 'import_qualified':
            q = s.t(qual) if qual is not None else None
            imp = F(cf).imports.get(q) if q else None
            rec['qualifier_import'] = imp
            if imp is None:
                rec['verdict'] = 'UNSURE:qualifier_not_esm_import'
                # CommonJS require binding?
                rec['qualifier_text'] = q
            else:
                module, member = imp
                c = mod_candidates(module, cf, indexed)
                if not c:
                    rec['verdict'] = 'WRONG:nonrelative_or_unresolved_module(%s)' % (
                        '*' if member == '*' else 'default' if member == 'default' else 'named')
                elif member != '*':
                    rec['verdict'] = 'WRONG:qualifier_is_%s_import' % ('default' if member == 'default'
                                                                        else 'named')
                else:
                    term = resolve_export(c[0], name)
                    if term is None:
                        rec['verdict'] = 'WRONG:no_export'
                    else:
                        a = module_callable(term[0], term[1], jsx)
                        rec['audit'] = a
                        want = (term[0], a.get('span'))
                        if a['status'] in ('callable', 'wrapped_jsx'):
                            ok = [t for t in targets if (t[0], t[2], t[3]) == (term[0],) + tuple(a['span'])]
                            rec['verdict'] = 'RIGHT' if len(targets) == 1 and ok else (
                                'WRONG:multi_incl_right' if ok else 'WRONG:wrong_target')
                        else:
                            rec['verdict'] = 'WRONG:' + a['status']
        else:
            continue
        rows.append(rec)
    summ = collections.Counter()
    for x in rows:
        summ[(x['kind'], x.get('conf'), x.get('origin'), 'jsx' if x.get('jsx') else 'call', x['verdict'])] += 1
    json.dump(rows, open(out, 'w'), indent=0, default=list)
    for k, v in sorted(summ.items(), key=lambda kv: (kv[0][0], -kv[1])):
        print('%5d  %s' % (v, ' | '.join(str(p) for p in k)))
    if not private:
        for x in rows:
            if not x['verdict'].startswith('RIGHT'):
                print('  ', x['verdict'], x['kind'], '%s:%s' % (x['file'], x['line']), x['callee'],
                      x['targets'][:3], x.get('audit'))


if __name__ == '__main__':
    main()
