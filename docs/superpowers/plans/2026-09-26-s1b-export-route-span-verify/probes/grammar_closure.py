"""Grammar closure check for the S1b binding collector (SPEC §3.1).

Reads the pinned grammars' node-types.json (tree-sitter-javascript 0.23.1, tree-sitter-typescript 0.23.2, both the
typescript and tsx grammars; the versions in Cargo.lock) and derives, mechanically, every named node kind that
*could* introduce a binding or a scope for its descendants: a kind is SUSPECT when one of its fields or children can
be a statement list item, a declaration, a binding pattern, a parameter list, a class body or a module body.
Every SUSPECT kind must be explicitly classified in the collector (HANDLED below, mirrored from SPEC §3.1). Every
other non-leaf named kind is mechanically transparent. Exit status 1 and a list when a SUSPECT kind is unclassified.

r3 fold (sol r3 W2, Opus r3 S3): two more checks.
  * Table 1b, the kind allowlist: every concrete named kind, leaf or not (B0 checks every named node). With
    --rust <dir>, the runtime `CLASSIFIED` list must equal it exactly (missing or extra kinds fail).
  * Table 2, the evaluation-context table: for every Σ′ kind, every field that can hold a named node ("children"
    for unfielded named children) must have exactly one rule in E_TABLE below, and E_TABLE must name no other
    position. With --rust <dir>, the runtime `E_TABLE` must equal it exactly, rule for rule.

Usage: python3 grammar_closure.py [--emit-transparent] [--rust <src/ast dir> [--kinds-only]]
"""
import json, os, sys, glob

REG = glob.glob(os.path.expanduser('~/.cargo/registry/src/*/'))[0]
FILES = {
    'javascript': REG + 'tree-sitter-javascript-0.23.1/src/node-types.json',
    'typescript': REG + 'tree-sitter-typescript-0.23.2/typescript/src/node-types.json',
    'tsx': REG + 'tree-sitter-typescript-0.23.2/tsx/src/node-types.json',
}
# Kinds whose presence as a direct child makes the parent binding-relevant. `internal_module`/`module` are left out:
# they are their own scopes, and the TS grammar also admits them in expression position, which would otherwise
# mark every expression kind; as statements they are reached through the `statement`/`declaration` supertypes.
BINDING_CHILDREN = {
    'statement', 'declaration', 'variable_declaration', 'lexical_declaration', 'function_declaration',
    'generator_function_declaration', 'class_declaration', 'abstract_class_declaration', 'import_statement',
    'import_alias', 'enum_declaration', 'ambient_declaration', 'statement_block',
    'class_body', 'class_static_block', 'formal_parameters', 'pattern', 'object_pattern', 'array_pattern',
    'assignment_pattern', 'object_assignment_pattern', 'rest_pattern', 'variable_declarator', 'catch_clause',
    'switch_body', 'switch_case', 'switch_default', 'labeled_statement', 'export_statement', 'function_signature',
    'interface_declaration', 'type_alias_declaration', 'decorator', 'required_parameter', 'optional_parameter',
}
# SPEC §3.1 classification (scope kinds, declaration kinds, wrappers, and kinds reviewed as binding-inert).
HANDLED = {
    # scopes (Σ)
    'program', 'function_declaration', 'generator_function_declaration', 'function_expression', 'function',
    'generator_function', 'arrow_function', 'method_definition', 'formal_parameters', 'statement_block',
    'class_static_block', 'for_statement', 'for_in_statement', 'catch_clause', 'switch_body',
    'class_declaration', 'class', 'abstract_class_declaration', 'with_statement', 'internal_module', 'module',
    # declaration forms and their wrappers
    'variable_declaration', 'lexical_declaration', 'variable_declarator', 'import_statement', 'import_alias',
    'enum_declaration', 'ambient_declaration', 'export_statement', 'labeled_statement', 'switch_case',
    'switch_default', 'expression_statement', 'function_signature', 'interface_declaration',
    'type_alias_declaration', 'decorator',
    # patterns (collected by the pattern walker, SPEC §3.1 "binding patterns")
    'object_pattern', 'array_pattern', 'assignment_pattern', 'object_assignment_pattern', 'rest_pattern',
    'pair_pattern', 'required_parameter', 'optional_parameter',
    # statements that nest a statement or block but introduce no binding of their own (Annex B markers apply
    # to function declarations reached through them)
    'if_statement', 'else_clause', 'while_statement', 'do_statement', 'try_statement', 'finally_clause',
    'switch_statement',
    # class members: their bodies are function-like or their own scopes; the member itself binds nothing in
    # an enclosing scope
    'class_body', 'public_field_definition', 'field_definition', 'abstract_method_signature', 'method_signature',
    # TS module-ish wrappers
    'export_clause', 'namespace_export', 'import_clause', 'named_imports', 'namespace_import', 'import_specifier',
    'export_specifier', 'import_require_clause', 'nested_identifier',
    # expression kinds that can hold a pattern only as an assignment target (writes, SPEC B5), never a binding
    'assignment_expression', 'augmented_assignment_expression', 'parenthesized_expression',
    # TS type-level kinds that can nest declarations only in type space (no value binding)
    'object_type', 'interface_body', 'function_type', 'constructor_type', 'type_parameters', 'index_signature',
    'call_signature', 'construct_signature', 'property_signature', 'enum_body', 'tuple_type', 'type_predicate',
    'asserts', 'type_query',
    # identifier fields that are references, not bindings (JSX tag names; resolved by the site walk itself)
    'jsx_opening_element', 'jsx_closing_element', 'jsx_self_closing_element', 'jsx_namespace_name',
    # write forms (SPEC B5)
    'update_expression',
}


# A kind can also bind (or write) a plain identifier through one of these fields.
BINDING_FIELDS = {'name', 'parameter', 'left', 'pattern', 'alias', 'label', 'argument', 'decorator'}


def load(path):
    kinds, supers = {}, {}
    for t in json.load(open(path)):
        if not t.get('named'):
            continue
        if 'subtypes' in t:
            supers[t['type']] = {s['type'] for s in t['subtypes'] if s.get('named')}
        kids = set()
        for fname, f in (t.get('fields') or {}).items():
            kids |= {x['type'] for x in f['types'] if x.get('named')}
            if fname in BINDING_FIELDS and any(x['type'] == 'identifier' for x in f['types']):
                kids.add('identifier@' + fname)
        if t.get('children'):
            kids |= {x['type'] for x in t['children']['types'] if x.get('named')}
            if any(x['type'] == 'identifier' for x in t['children']['types']):
                kids.add('identifier@child')
        kinds[t['type']] = kids
    return kinds, supers


def expand(types, supers, seen=None):
    out = set()
    for t in types:
        out.add(t)
        if t in supers:
            out |= expand(supers[t], supers)
    return out


allk, suspect, transparent = set(), {}, set()
for g, p in FILES.items():
    kinds, supers = load(p)
    for k, kids in kinds.items():
        if k in supers:
            continue
        allk.add(k)
        exp = expand(kids, supers)
        hit = (exp & BINDING_CHILDREN) | {x for x in exp if x.startswith('identifier@')}
        if hit:
            suspect.setdefault(k, set()).update(hit)
        elif kids:
            transparent.add(k)
transparent -= set(suspect)
missing = sorted(set(suspect) - HANDLED)
print('named concrete kinds', len(allk), 'suspect', len(suspect), 'transparent non-leaf', len(transparent),
      'unclassified suspect', len(missing))
for k in missing:
    print('  UNCLASSIFIED', k, sorted(suspect[k])[:6])
if '--emit-transparent' in sys.argv:
    print(' '.join(sorted(transparent)))

# ---- Table 1b: every concrete named kind (leaves included) ------------------------------------------------------
ALL_KINDS = set()
for p in FILES.values():
    for t in json.load(open(p)):
        if t.get('named') and 'subtypes' not in t:
            ALL_KINDS.add(t['type'])

# ---- Table 2: the evaluation-context table (SPEC §3.1a) ------------------------------------------------------
FUNCTIONS = ['function_declaration', 'generator_function_declaration', 'function_expression', 'generator_function']
E_TABLE = {('program', 'children'): 'Inside'}
for f in FUNCTIONS:
    E_TABLE.update({(f, 'body'): 'Inside', (f, 'parameters'): 'Param', (f, 'name'): 'Unlisted',
                    (f, 'return_type'): 'Unlisted', (f, 'type_parameters'): 'Unlisted'})
E_TABLE.update({
    ('arrow_function', 'body'): 'Inside', ('arrow_function', 'parameters'): 'Param',
    ('arrow_function', 'parameter'): 'Param', ('arrow_function', 'return_type'): 'Unlisted',
    ('arrow_function', 'type_parameters'): 'Unlisted',
    ('method_definition', 'body'): 'Inside', ('method_definition', 'parameters'): 'Param',
    ('method_definition', 'name'): 'Outside', ('method_definition', 'decorator'): 'Outside',
    ('method_definition', 'children'): 'Unlisted', ('method_definition', 'return_type'): 'Unlisted',
    ('method_definition', 'type_parameters'): 'Unlisted',
    ('formal_parameters', 'children'): 'Inside', ('statement_block', 'children'): 'Inside',
    ('class_static_block', 'body'): 'Inside',
    ('for_statement', 'initializer'): 'Inside', ('for_statement', 'condition'): 'Inside',
    ('for_statement', 'increment'): 'Inside', ('for_statement', 'body'): 'Inside',
    ('for_in_statement', 'left'): 'Inside', ('for_in_statement', 'right'): 'Inside',
    ('for_in_statement', 'value'): 'Inside', ('for_in_statement', 'body'): 'Inside',
    ('catch_clause', 'parameter'): 'Inside', ('catch_clause', 'body'): 'Inside', ('catch_clause', 'type'): 'Unlisted',
    ('switch_body', 'children'): 'Inside',
    ('with_statement', 'object'): 'Outside', ('with_statement', 'body'): 'WithBody',
    ('internal_module', 'body'): 'Leave', ('internal_module', 'name'): 'Unlisted',
    ('module', 'body'): 'Leave', ('module', 'name'): 'Unlisted',
    ('enum_declaration', 'body'): 'Leave', ('enum_declaration', 'name'): 'Unlisted',
    ('enum_body', 'children'): 'Inside', ('enum_body', 'name'): 'Inside',
    ('decorator', 'children'): 'Decorator',
})
for c in ('class_declaration', 'class', 'abstract_class_declaration'):
    E_TABLE.update({(c, 'body'): 'Inside', (c, 'children'): 'Inside', (c, 'decorator'): 'Outside',
                    (c, 'name'): 'Unlisted', (c, 'type_parameters'): 'Unlisted'})
SIGMA = sorted({k for k, _ in E_TABLE})
positions = set()
for p in FILES.values():
    for t in json.load(open(p)):
        if t.get('named') and t['type'] in SIGMA:
            for f, v in (t.get('fields') or {}).items():
                if any(x.get('named') for x in v['types']):
                    positions.add((t['type'], f))
            if t.get('children') and any(x.get('named') for x in t['children']['types']):
                positions.add((t['type'], 'children'))
e_missing = sorted(positions - set(E_TABLE))
e_extra = sorted(set(E_TABLE) - positions)
print('sigma-prime kinds', len(SIGMA), 'positions', len(positions), 'E-table rows', len(E_TABLE),
      'missing', len(e_missing), 'extra', len(e_extra), 'all named kinds', len(ALL_KINDS))
for m in e_missing:
    print('  E MISSING', m)
for m in e_extra:
    print('  E EXTRA', m)
bad = bool(missing or e_missing or e_extra)

# ---- runtime equality (--rust <dir>): the Rust constants must equal the derived tables --------------------------
if '--rust' in sys.argv:
    import re
    d = sys.argv[sys.argv.index('--rust') + 1]
    rs = ''.join(open(os.path.join(d, f)).read() for f in sorted(os.listdir(d)) if f.startswith('js_binding'))
    m = re.search(r'const CLASSIFIED: &str = "(.*?)";', rs, re.S)
    runtime_kinds = set(m.group(1).replace('\\', ' ').split()) if m else set()
    # A rustfmt-wrapped row spans lines, so the pattern allows whitespace between the tuple's parts.
    rt_e = {(k, f): r for k, f, r in re.findall(r'\(\s*"(\w+)",\s*"(\w+)",\s*Pos::(\w+),?\s*\)', rs)}
    kd = (sorted(ALL_KINDS - runtime_kinds), sorted(runtime_kinds - ALL_KINDS))
    # --kinds-only (S1b-2a, before the runtime E_TABLE exists in S1b-3): compare CLASSIFIED only.
    ed = [] if '--kinds-only' in sys.argv else sorted(set(E_TABLE.items()) ^ set(rt_e.items()))
    print('runtime CLASSIFIED missing', len(kd[0]), 'extra', len(kd[1]), '| runtime E_TABLE rows', len(rt_e),
          'differences', len(ed))
    for x in kd[0]:
        print('  KIND MISSING', x)
    for x in kd[1]:
        print('  KIND EXTRA', x)
    for x in ed:
        print('  E DIFF', x)
    bad = bad or bool(kd[0] or kd[1] or ed)
sys.exit(1 if bad else 0)
