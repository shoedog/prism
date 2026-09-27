"""Grammar closure check for the S1b binding collector (SPEC §3.1).

Reads the pinned grammars' node-types.json (tree-sitter-javascript 0.23.1, tree-sitter-typescript 0.23.2, both the
typescript and tsx grammars; the versions in Cargo.lock) and derives, mechanically, every named node kind that
*could* introduce a binding or a scope for its descendants: a kind is SUSPECT when one of its fields or children can
be a statement list item, a declaration, a binding pattern, a parameter list, a class body or a module body.
Every SUSPECT kind must be explicitly classified in the collector (HANDLED below, mirrored from SPEC §3.1). Every
other non-leaf named kind is mechanically transparent. Exit status 1 and a list when a SUSPECT kind is unclassified.

Usage: python3 grammar_closure.py [--emit-transparent]
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
sys.exit(1 if missing else 0)
