"""P4 yield projection: attribute every currently-dropped (UnknownName) call site to the terminal
declaration shape its import would reach, mirroring R4c's relative-module + export-chain rules.

Usage: python3 project.py <repo_root> <dump-sites.jsonl> <out.json>
Mirrors (by reading, not linking): call_graph::js_ts_relative_module_candidates (relative only, ext order,
index files; first candidate wins), js_exports::resolve_one (MAX_REEXPORT_DEPTH=2, star barrels never carry
default, duplicate names poison). This is a PROJECTION; the prototype run (P6) is the measurement.
"""
import json, os, sys, collections
import tree_sitter as ts, tree_sitter_typescript as tst, tree_sitter_javascript as tsj
sys.path.insert(0, os.path.dirname(__file__))
from census import LANG, SKIP, classify, walk_files, txt, unwrap, FN

def mod_candidates(module, caller, indexed):
    module = module.strip()
    if not (module.startswith('./') or module.startswith('../')): return None
    parts = caller.rsplit('/', 1)[0].split('/') if '/' in caller else []
    rel = module
    while rel.startswith('../'):
        if not parts: return None
        parts.pop(); rel = rel[3:]
    if rel.startswith('./'): rel = rel[2:]
    if not rel: return None
    base = '/'.join(parts)
    out = []
    for ext in ['.js', '.jsx', '.mjs', '.cjs', '.ts', '.tsx', '']:
        c = f'{base}/{rel}{ext}' if base else f'{rel}{ext}'
        if c in indexed: out.append(c)
    for ix in ['index.js', 'index.jsx', 'index.mjs', 'index.cjs', 'index.ts', 'index.tsx']:
        c = f'{base}/{rel}/{ix}' if base else f'{rel}/{ix}'
        if c in indexed: out.append(c)
    return out

def strip_q(s): return s[1:-1] if len(s) >= 2 and s[0] in '"\'' else s

class FileFacts:
    def __init__(self, rel, src, tree):
        self.imports = {}      # local -> (module, member)
        self.named = {}        # exported -> ('local', name) | ('reexport', module, imported)
        self.conflicted = set()
        self.star = []
        self.decls = {}        # top-level local name -> (shape, detail, via)
        self.wrapper_imports = {}  # local ident -> module (for provenance of wrapper callee)
        root = tree.root_node
        def ins(k, v):
            if k in self.conflicted: return
            if k in self.named: del self.named[k]; self.conflicted.add(k)
            else: self.named[k] = v
        for st in root.named_children:
            if st.type == 'import_statement':
                srcn = st.child_by_field_name('source')
                if srcn is None: continue
                module = strip_q(txt(src, srcn))
                if 'import type' in txt(src, st)[:12]: continue
                for cl in st.named_children:
                    if cl.type != 'import_clause': continue
                    for c in cl.named_children:
                        if c.type == 'identifier':
                            self.imports[txt(src, c)] = (module, 'default')
                        elif c.type == 'named_imports':
                            for sp in c.named_children:
                                if sp.type != 'import_specifier': continue
                                if txt(src, sp).startswith('type '): continue
                                n = sp.child_by_field_name('name'); a = sp.child_by_field_name('alias')
                                self.imports[txt(src, a or n)] = (module, txt(src, n))
                        elif c.type == 'namespace_import':
                            ids = [x for x in c.named_children if x.type == 'identifier']
                            if ids: self.imports[txt(src, ids[0])] = (module, '*')
            decl_node = st
            is_export = st.type == 'export_statement'
            if is_export:
                srcn = st.child_by_field_name('source')
                clause = next((c for c in st.named_children if c.type == 'export_clause'), None)
                if clause is not None:
                    for sp in clause.named_children:
                        if sp.type != 'export_specifier': continue
                        n = sp.child_by_field_name('name'); a = sp.child_by_field_name('alias')
                        if n is None: continue
                        exp = txt(src, a or n)
                        if srcn is not None: ins(exp, ('reexport', strip_q(txt(src, srcn)), txt(src, n)))
                        else: ins(exp, ('local', txt(src, n)))
                    continue
                if srcn is not None and '*' in txt(src, st).split('from')[0]:
                    if ' as ' not in txt(src, st).split('from')[0]:
                        self.star.append(strip_q(txt(src, srcn)))
                    continue
                val = st.child_by_field_name('value')
                if val is not None:
                    if val.type == 'identifier': ins('default', ('local', txt(src, val)))
                    else: ins('default', ('expr', val.type))
                    continue
                decl_node = st.child_by_field_name('declaration')
                if decl_node is None: continue
            if decl_node.type in ('function_declaration', 'generator_function_declaration'):
                n = decl_node.child_by_field_name('name')
                if n is not None:
                    self.decls[txt(src, n)] = ('function_decl', '', 'export' if is_export else 'local')
                    if is_export:
                        ins('default' if txt(src, st).startswith('export default') else txt(src, n), ('local', txt(src, n)))
            elif decl_node.type == 'class_declaration':
                n = decl_node.child_by_field_name('name')
                if n is not None:
                    self.decls[txt(src, n)] = ('class', '', 'export' if is_export else 'local')
                    if is_export: ins('default' if txt(src, st).startswith('export default') else txt(src, n), ('class', txt(src, n)))
            elif decl_node.type in ('lexical_declaration', 'variable_declaration'):
                kw = decl_node.children[0].type if decl_node.children else '?'
                for d in decl_node.named_children:
                    if d.type != 'variable_declarator': continue
                    n = d.child_by_field_name('name'); v = d.child_by_field_name('value')
                    if n is None or n.type != 'identifier' or v is None: continue
                    shape, detail = classify(src, v)
                    self.decls[txt(src, n)] = (shape, ' '.join(detail.split())[:40], ('export' if is_export else 'local') + ':' + kw)
                    if is_export and shape in ('fn_direct',):
                        ins(txt(src, n), ('local', txt(src, n)))
                    elif is_export:
                        # the gap: recorded by prism as skipped; we tag it for attribution
                        ins(txt(src, n), ('skipped_decl', txt(src, n)))

MODE = 'dropped'
def main():
    global MODE
    import os as _os
    MODE = _os.environ.get('MODE', 'dropped')
    root, dump, out = sys.argv[1], sys.argv[2], sys.argv[3]
    files = list(walk_files(root))
    indexed = set(files)
    facts = {}
    for rel in files:
        p = os.path.join(root, rel)
        if os.path.getsize(p) > 2 * 1024 * 1024: continue
        src = open(p, 'rb').read()
        facts[rel] = FileFacts(rel, src, ts.Parser(LANG[os.path.splitext(rel)[1]]).parse(src))

    def resolve(file, name, hops, seen):
        if (file, name) in seen: return ('cycle',)
        seen = seen | {(file, name)}
        f = facts.get(file)
        if f is None: return ('no_file',)
        if name in f.conflicted: return ('conflicted',)
        t = f.named.get(name)
        if t is not None:
            if t[0] in ('local', 'skipped_decl', 'class'): return ('terminal', file, t[1], t[0])
            if t[0] == 'expr': return ('default_expr', t[1])
            if t[0] == 'reexport':
                if hops + 1 > 2: return ('depth',)
                c = mod_candidates(t[1], file, indexed)
                if not c: return ('unresolved_module',)
                return resolve(c[0], t[2], hops + 1, seen)
        if name == 'default' or not f.star: return ('no_export',)
        if hops + 1 > 2: return ('depth',)
        hits = set()
        for m in f.star:
            c = mod_candidates(m, file, indexed)
            if not c: continue
            r = resolve(c[0], name, hops + 1, seen)
            if r[0] == 'terminal': hits.add(r)
            elif r[0] == 'conflicted': return r
        if len(hits) == 1: return hits.pop()
        return ('barrel_conflict',) if hits else ('no_export',)

    global LOCAL_ROOTS, WORKSPACE_SCOPES
    LOCAL_ROOTS = {f.split('/')[0] for f in files if '/' in f}
    WORKSPACE_SCOPES = set()
    for dp, dns, fns in os.walk(root):
        dns[:] = [d for d in dns if d not in SKIP and not d.startswith('.')]
        if 'package.json' in fns:
            try:
                nm = json.load(open(os.path.join(dp, 'package.json'))).get('name') or ''
                if nm.startswith('@'): WORKSPACE_SCOPES.add(nm.split('/')[0])
            except Exception: pass
    paths = json.load(open(sys.argv[4])) if len(sys.argv) > 4 else {}
    def to_file(base):
        base = base.lstrip('./') if base.startswith('./') else base
        if base in indexed: return base
        for ext in ['.js', '.jsx', '.mjs', '.cjs', '.ts', '.tsx']:
            if base + ext in indexed: return base + ext
        for ix in ['index.js', 'index.jsx', 'index.mjs', 'index.cjs', 'index.ts', 'index.tsx']:
            if base + '/' + ix in indexed: return base + '/' + ix
        return None
    def alias_resolve(module):
        # tsconfig `paths` semantics (exact key first, then longest wildcard prefix); ASSUMPTION-grade, projection only.
        if module in paths: return to_file(paths[module][0][2:] if paths[module][0].startswith('./') else paths[module][0])
        best = None
        for k, v in paths.items():
            if k.endswith('*') and module.startswith(k[:-1]) and (best is None or len(k) > len(best[0])):
                best = (k, v)
        if best is None: return None
        tgt = best[1][0].replace('*', module[len(best[0]) - 1:])
        return to_file(tgt[2:] if tgt.startswith('./') else tgt)
    rows = []
    for line in open(dump):
        r = json.loads(line)
        if r.get('record_kind') != 'call_site': continue
        if MODE == 'resolved':
            if not any(t['kind'] == 'import_member' for t in r.get('resolved_targets') or []): continue
        elif r.get('drop') != 'UnknownName': continue
        cf = r['caller']['file']; name = r['callee_text']; stext = r.get('source_callee_text') or name
        f = facts.get(cf)
        if f is None: continue
        if '.' in stext and stext != name:
            q = stext.split('.')[0]
            imp = f.imports.get(q)
            lane = 'member_callee'
            if imp and imp[1] == 'default' and mod_candidates(imp[0], cf, indexed):
                lane = 'member_on_default_import'
            rows.append({'file': cf, 'line': r['source_span']['line'], 'callee': stext, 'lane': lane}); continue
        imp = f.imports.get(name)
        if imp is None: continue  # not an import binding: other lanes (locals, globals, externals)
        module, member = imp
        if member == '*': continue
        cands = mod_candidates(module, cf, indexed)
        if not cands:
            # external/bare/alias/`.` module: not R4c territory today; tagged so the bound is visible
            if module.startswith('.'):
                cls = 'relative_unresolved'
            elif module.startswith('@/') or module.startswith('~') or module.startswith('src/') or module.split('/')[0] in LOCAL_ROOTS:
                cls = 'alias_like'
            elif module.startswith('@') and module.split('/')[0] in WORKSPACE_SCOPES:
                cls = 'workspace_pkg'
            else:
                cls = 'bare_pkg'
            row = {'file': cf, 'line': r['source_span']['line'], 'callee': name, 'member': member, 'module': module, 'lane': 'nonrelative:' + cls}
            tf = alias_resolve(module)
            if tf is not None:
                res = resolve(tf, member, 0, frozenset())
                if res[0] == 'terminal':
                    d = facts[res[1]].decls.get(res[2])
                    row.update({'latent': True, 'terminal_file': res[1], 'terminal_local': res[2], 'export_kind': res[3],
                                'shape': d[0] if d else 'undeclared', 'detail': d[1] if d else '', 'via': d[2] if d else ''})
                else:
                    row.update({'latent': True, 'latent_chain': res[0]})
            rows.append(row)
            continue
        res = resolve(cands[0], member, 0, frozenset())
        row = {'file': cf, 'line': r['source_span']['line'], 'callee': name, 'member': member, 'module_file': cands[0]}
        if res[0] == 'terminal':
            tfile, local, kind = res[1], res[2], res[3]
            d = facts[tfile].decls.get(local)
            row.update({'terminal_file': tfile, 'terminal_local': local, 'export_kind': kind,
                        'shape': d[0] if d else 'undeclared', 'detail': d[1] if d else '', 'via': d[2] if d else ''})
            row['lane'] = 'terminal'
        else:
            row['lane'] = 'chain:' + res[0]
        rows.append(row)
    json.dump(rows, open(out, 'w'), indent=0)
    lanes = collections.Counter(r['lane'] for r in rows)
    print('dropped import-bound/member sites', len(rows), dict(lanes))
    sh = collections.Counter((r.get('export_kind'), r.get('shape'), r.get('detail'), r.get('via')) for r in rows if r['lane'] == 'terminal')
    for k, n in sh.most_common(40): print(f'  {n:5d} {k}')
    lat = collections.Counter((r.get('export_kind'), r.get('shape'), r.get('detail')) for r in rows if r.get('latent') and r.get('shape'))
    print('latent (non-relative import, resolvable via tsconfig paths):', sum(lat.values()), 'chain-failed:', sum(1 for r in rows if r.get('latent_chain')))
    for k, n in lat.most_common(25): print(f'  {n:5d} {k}')
if __name__ == '__main__':
    main()
