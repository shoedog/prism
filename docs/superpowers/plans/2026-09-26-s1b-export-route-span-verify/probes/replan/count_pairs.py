"""Count the grammar-possible (kind, field) pairs across the pinned grammars (RP3's denominator, 319).

A pair is a concrete named kind with a field (or "<child>" for unfielded children), taken over tree-sitter-javascript 0.23.1 and tree-sitter-typescript 0.23.2 (typescript and tsx), the versions in
Cargo.lock. Usage: python3 count_pairs.py [--list]
"""
import glob, json, os, sys

REG = glob.glob(os.path.expanduser('~/.cargo/registry/src/*/'))[0]
FILES = [REG + 'tree-sitter-javascript-0.23.1/src/node-types.json',
         REG + 'tree-sitter-typescript-0.23.2/typescript/src/node-types.json',
         REG + 'tree-sitter-typescript-0.23.2/tsx/src/node-types.json']
pairs, named, kinds = set(), set(), set()
for p in FILES:
    for t in json.load(open(p)):
        if not t.get('named') or 'subtypes' in t:
            continue
        kinds.add(t['type'])
        for f, v in (t.get('fields') or {}).items():
            pairs.add((t['type'], f))
            if any(x.get('named') for x in v['types']):
                named.add((t['type'], f))
        if t.get('children'):
            pairs.add((t['type'], '<child>'))
            if any(x.get('named') for x in t['children']['types']):
                named.add((t['type'], '<child>'))
print('concrete named kinds', len(kinds), 'pairs', len(pairs), '(of which can hold a named node:', len(named), ')')
assert len(pairs) == 319, 'RP3 denominator changed: re-derive the E-table (SPEC §3.1a)' 
if '--list' in sys.argv:
    for k, f in sorted(pairs):
        print(k, f)
