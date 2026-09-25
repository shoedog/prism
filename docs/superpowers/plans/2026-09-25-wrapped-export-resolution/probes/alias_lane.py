"""P13: size the secondary lanes. (a) default-object member alias: dropped JSX `<A.B>` where A is a relative default
import (or tsconfig-path import, reported separately) and the module's `export default { B: Ident, ... }`.
Usage: python3 alias_lane.py <root> <dump> [paths.json]"""
import json, sys, os, re, collections
sys.path.insert(0, os.path.dirname(__file__))
sys.argv += [] 
from census import LANG, txt, walk_files
import tree_sitter as ts
import project as P
root, dump = sys.argv[1], sys.argv[2]
paths = json.load(open(sys.argv[3])) if len(sys.argv) > 3 else {}
files = set(walk_files(root))
def default_obj(f):
    src = open(os.path.join(root, f), 'rb').read()
    t = ts.Parser(LANG[os.path.splitext(f)[1]]).parse(src)
    for st in t.root_node.named_children:
        if st.type == 'export_statement':
            v = st.child_by_field_name('value')
            if v is not None and v.type == 'object':
                return {txt(src, p.child_by_field_name('key')): p.child_by_field_name('value').type for p in v.named_children if p.type == 'pair'}
    return None
imports = {}
c = collections.Counter()
for l in open(dump):
    r = json.loads(l)
    if r.get('record_kind') != 'call_site' or r.get('drop') != 'UnknownName': continue
    s = r['source_span']; f = s['file']
    b = open(os.path.join(root, f), 'rb').read()[s['start_byte']:s['start_byte'] + 80].decode('utf8', 'replace')
    m = re.match(r'<([A-Za-z_$][\w$]*)\.([A-Za-z_$][\w$]*)', b)
    if not m: continue
    obj, mem = m.groups()
    if f not in imports:
        src = open(os.path.join(root, f), 'rb').read()
        ff = P.FileFacts(f, src, ts.Parser(LANG[os.path.splitext(f)[1]]).parse(src))
        imports[f] = ff.imports
    imp = imports[f].get(obj)
    if not imp: c['jsx_member:obj_not_import'] += 1; continue
    module, member = imp
    cands = P.mod_candidates(module, f, files)
    kind = 'relative'
    if not cands:
        kind = 'nonrelative'; cands = None
        continue_ = True
        c['jsx_member:%s_import:%s' % (kind, member)] += 1
        continue
    if member != 'default': c['jsx_member:relative_named_or_ns:%s' % member] += 1; continue
    d = default_obj(cands[0])
    if d is None: c['jsx_member:relative_default_not_object'] += 1; continue
    c['jsx_member:relative_default_object:%s' % ('ident_value' if d.get(mem) == 'identifier' else d.get(mem))] += 1
for k, n in c.most_common(): print(f'  {n:5d} {k}')
