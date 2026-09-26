"""Independent JS/TS lexical-binding auditor for S1b (tree-sitter-python; grammars pinned to Cargo.lock:
typescript 0.23.2, javascript 0.23.1). It does NOT link prism; it re-derives, from the ECMAScript scoping rules,
which declaration an identifier at a site statically denotes, and which callable span (if any) that binding holds.

binding(site_node, name) walks the site's ancestors. The first scope that declares `name` decides:
  - function-like node: parameters, a named function expression's own name, `var` hoisted from its body (not
    crossing nested functions), and lexical declarations directly in its body block;
  - statement_block / switch_body / for heads / catch clause: `let`/`const`/`class`/function declarations directly
    in it (module and TS code is strict, so block function declarations are block-scoped);
  - class declaration/expression: the class name inside the class;
  - program: imports, every top-level declaration, and `var` hoisted from top-level blocks.
A scope with 2+ declarations of `name` yields ('dup', ...). The result is a Binding tuple.

callable span of a binding (1-indexed lines, the FunctionId identity prism uses):
  function_declaration -> its node; declarator with arrow/function-expression value -> the value;
  named function expression self name -> the expression; anything else -> None.
A binding is 'written' when any assignment/update/for-in/of target of `name` resolves to the same binding.
"""
import tree_sitter as ts, tree_sitter_typescript as tst, tree_sitter_javascript as tsj

LANG = {'.ts': ts.Language(tst.language_typescript()), '.tsx': ts.Language(tst.language_tsx()),
        '.js': ts.Language(tsj.language()), '.jsx': ts.Language(tsj.language()),
        '.mjs': ts.Language(tsj.language()), '.cjs': ts.Language(tsj.language())}
FUNCS = {'function_declaration', 'generator_function_declaration', 'function_expression', 'function',
         'generator_function', 'arrow_function', 'method_definition'}
FN_VALUES = {'arrow_function', 'function_expression', 'function', 'generator_function'}
BLOCKS = {'statement_block', 'switch_body', 'for_statement', 'for_in_statement', 'for_of_statement',
          'catch_clause', 'class_body'}
CASTS = {'as_expression', 'satisfies_expression', 'parenthesized_expression', 'non_null_expression',
         'type_assertion'}


def lines(n):
    return (n.start_point[0] + 1, n.end_point[0] + 1)


class Src:
    def __init__(self, path, ext):
        self.src = open(path, 'rb').read()
        self.tree = ts.Parser(LANG[ext]).parse(self.src)
        self.root = self.tree.root_node

    def t(self, n):
        return self.src[n.start_byte:n.end_byte].decode('utf8', 'replace')

    # ---- declarations -------------------------------------------------------------------------------------
    def pattern_names(self, p, out):
        if p is None:
            return
        k = p.type
        if k in ('identifier', 'shorthand_property_identifier_pattern'):
            out.append((self.t(p), p))
        elif k in ('object_pattern', 'array_pattern', 'rest_pattern', 'assignment_pattern',
                   'object_assignment_pattern', 'pair_pattern', 'required_parameter', 'optional_parameter'):
            if k == 'pair_pattern':
                self.pattern_names(p.child_by_field_name('value'), out)
            elif k in ('assignment_pattern', 'object_assignment_pattern'):
                self.pattern_names(p.child_by_field_name('left'), out)
            elif k in ('required_parameter', 'optional_parameter'):
                self.pattern_names(p.child_by_field_name('pattern'), out)
            else:
                for c in p.named_children:
                    self.pattern_names(c, out)

    def decl_kind(self, d):
        """lexical_declaration / variable_declaration keyword."""
        if d.type == 'variable_declaration':
            return 'var'
        for c in d.children:
            if c.type in ('const', 'let', 'using', 'await'):
                return c.type
        return self.t(d).split(None, 1)[0]

    def var_hoisted(self, node, out, top=True):
        """`var` declarators under node, not crossing nested functions/classes' method bodies."""
        for c in node.named_children:
            if c.type in FUNCS or c.type in ('class_declaration', 'class'):
                continue
            if c.type == 'variable_declaration':
                for d in c.named_children:
                    if d.type == 'variable_declarator':
                        names = []
                        self.pattern_names(d.child_by_field_name('name'), names)
                        for nm, _ in names:
                            out.append((nm, ('var', d)))
            if c.type in ('for_in_statement', 'for_of_statement'):
                # `for (var x of ...)`: tree-sitter puts `var` as a keyword child and left as identifier
                kw = [x for x in c.children if x.type == 'var']
                if kw:
                    names = []
                    self.pattern_names(c.child_by_field_name('left'), names)
                    for nm, _ in names:
                        out.append((nm, ('var_forin', c)))
            self.var_hoisted(c, out, False)

    def lexical_in(self, block, out):
        """Lexical (block-scoped) declarations directly in a block-like node."""
        kids = block.named_children
        if block.type == 'switch_body':
            kids = [s for case in block.named_children for s in case.named_children]
        for c in kids:
            d = c
            if c.type == 'export_statement':
                d = c.child_by_field_name('declaration')
                if d is None:
                    continue
            if d.type in ('function_declaration', 'generator_function_declaration'):
                n = d.child_by_field_name('name')
                if n is not None:
                    out.append((self.t(n), ('function_decl', d)))
            elif d.type in ('class_declaration', 'abstract_class_declaration'):
                n = d.child_by_field_name('name')
                if n is not None:
                    out.append((self.t(n), ('class', d)))
            elif d.type == 'lexical_declaration':
                kw = self.decl_kind(d)
                for x in d.named_children:
                    if x.type == 'variable_declarator':
                        names = []
                        self.pattern_names(x.child_by_field_name('name'), names)
                        for nm, _ in names:
                            out.append((nm, (kw, x)))
            elif d.type in ('enum_declaration', 'internal_module', 'module', 'import_alias'):
                n = d.child_by_field_name('name')
                if n is not None:
                    out.append((self.t(n).split('.')[0], ('ts_other', d)))
            elif d.type == 'ambient_declaration':
                names = []
                for x in d.named_children:
                    if x.type in ('function_signature', 'function_declaration'):
                        n = x.child_by_field_name('name')
                        if n is not None:
                            out.append((self.t(n), ('ambient', x)))
                    elif x.type in ('lexical_declaration', 'variable_declaration'):
                        for y in x.named_children:
                            if y.type == 'variable_declarator':
                                names = []
                                self.pattern_names(y.child_by_field_name('name'), names)
                                for nm, _ in names:
                                    out.append((nm, ('ambient', y)))
            elif d.type == 'import_statement':
                if any(ch.type == 'type' for ch in d.children):
                    continue
                for cl in d.named_children:
                    if cl.type != 'import_clause':
                        continue
                    for x in cl.named_children:
                        if x.type == 'identifier':
                            out.append((self.t(x), ('import', d)))
                        elif x.type == 'namespace_import':
                            for y in x.named_children:
                                if y.type == 'identifier':
                                    out.append((self.t(y), ('import', d)))
                        elif x.type == 'named_imports':
                            for sp in x.named_children:
                                if sp.type != 'import_specifier' or any(ch.type == 'type' for ch in sp.children):
                                    continue
                                a = sp.child_by_field_name('alias') or sp.child_by_field_name('name')
                                out.append((self.t(a), ('import', d)))

    def params(self, fn, out):
        ps = fn.child_by_field_name('parameters')
        if ps is None:
            p = fn.child_by_field_name('parameter')
            if p is not None:
                out.append((self.t(p), ('param', fn)))
            return
        for p in ps.named_children:
            names = []
            self.pattern_names(p, names)
            for nm, _ in names:
                out.append((nm, ('param', fn)))

    def scope_decls(self, scope):
        out = []
        k = scope.type
        if k in FUNCS:
            self.params(scope, out)
            if k in ('function_expression', 'function', 'generator_function'):
                n = scope.child_by_field_name('name')
                if n is not None:
                    out.append((self.t(n), ('fe_self', scope)))
            body = scope.child_by_field_name('body')
            if body is not None and body.type == 'statement_block':
                self.var_hoisted(body, out)
                self.lexical_in(body, out)
        elif k == 'program':
            self.var_hoisted(scope, out)
            self.lexical_in(scope, out)
        elif k in ('class_declaration', 'class', 'abstract_class_declaration'):
            n = scope.child_by_field_name('name')
            if n is not None:
                out.append((self.t(n), ('class_self', scope)))
        elif k == 'catch_clause':
            p = scope.child_by_field_name('parameter')
            names = []
            self.pattern_names(p, names)
            for nm, _ in names:
                out.append((nm, ('catch', scope)))
        elif k in ('for_statement', 'for_in_statement', 'for_of_statement'):
            init = scope.child_by_field_name('initializer') or scope.child_by_field_name('left')
            if init is not None and init.type == 'lexical_declaration':
                self.lexical_in_decl(init, out)
            elif k != 'for_statement' and any(c.type in ('let', 'const') for c in scope.children):
                names = []
                self.pattern_names(scope.child_by_field_name('left'), names)
                for nm, _ in names:
                    out.append((nm, ('let_forin', scope)))
        elif k in ('statement_block', 'switch_body'):
            p = scope.parent
            if not (k == 'statement_block' and p is not None and p.type in FUNCS):
                self.lexical_in(scope, out)
        return out

    def lexical_in_decl(self, d, out):
        kw = self.decl_kind(d)
        for x in d.named_children:
            if x.type == 'variable_declarator':
                names = []
                self.pattern_names(x.child_by_field_name('name'), names)
                for nm, _ in names:
                    out.append((nm, (kw, x)))

    def binding(self, site, name):
        """-> (scope_node, [decl, ...]) for the nearest scope declaring name; (None, []) when global."""
        n = site
        cache = getattr(self, '_cache', None)
        if cache is None:
            cache = self._cache = {}
        while n is not None:
            key = n.id
            if key not in cache:
                cache[key] = self.scope_decls(n)
            ds = [d for nm, d in cache[key] if nm == name]
            if ds:
                return n, ds
            n = n.parent
        return None, []

    # ---- values ---------------------------------------------------------------------------------------------
    def callable_of(self, decl):
        """-> ('plain', fnnode) | ('wrapped', fnnode, callee_text) | (None, reason)."""
        kind, node = decl
        if kind == 'function_decl':
            return ('plain', node)
        if kind == 'fe_self':
            return ('plain', node)
        if kind in ('const', 'let', 'var'):
            v = node.child_by_field_name('value')
            if v is None:
                return (None, 'no_value')
            u = v
            while u is not None and u.type in CASTS:
                u = u.named_children[0] if u.named_children else None
            if v.type in FN_VALUES:
                return ('plain', v)
            if u is not None and u.type in FN_VALUES:
                return (None, 'fn_behind_cast')
            if v.type == 'call_expression':
                args = v.child_by_field_name('arguments')
                fn = v.child_by_field_name('function')
                if args is not None:
                    for a in args.named_children:
                        if a.type in FN_VALUES and a.child_by_field_name('name') is None:
                            return ('wrapped', a, ' '.join(self.t(fn).split())[:40] if fn else '?')
                return (None, 'call')
            return (None, v.type)
        return (None, kind)

    def written(self, scope, decl_node, name):
        """Any write whose target name resolves (from the write site) to the same scope."""
        hits = []

        def walk(n):
            if n.type in ('assignment_expression', 'augmented_assignment_expression', 'update_expression',
                          'for_in_statement', 'for_of_statement'):
                tgt = n.child_by_field_name('left') or n.child_by_field_name('argument')
                if tgt is None and n.named_children:
                    tgt = n.named_children[0]
                names = []
                if tgt is not None:
                    if tgt.type == 'parenthesized_expression' and tgt.named_children:
                        tgt = tgt.named_children[0]
                    self.pattern_names(tgt, names)
                for nm, node in names:
                    if nm == name:
                        sc, _ = self.binding(node, name)
                        if sc is not None and sc.id == scope.id:
                            # a `for (var x ...)` / `let x` declaration head is not a write of an existing binding
                            if not (n.type in ('for_in_statement', 'for_of_statement')
                                    and any(c.type in ('var', 'let', 'const') for c in n.children)):
                                hits.append(n)
            for c in n.named_children:
                walk(c)
        walk(scope)
        return hits

    def resolve_callable(self, site, name, jsx):
        """Full audit: -> dict(status, span, detail). status in
        callable | wrapped_jsx | wrapped_nonjsx | not_callable | written | dup | global | intrinsic."""
        scope, ds = self.binding(site, name)
        if scope is None:
            return {'status': 'global'}
        if len(ds) > 1:
            return {'status': 'dup', 'detail': [d[0] for d in ds]}
        d = ds[0]
        c = self.callable_of(d)
        if c[0] is None:
            return {'status': 'not_callable', 'detail': '%s:%s' % (d[0], c[1])}
        if d[0] in ('let', 'var', 'function_decl') and self.written(scope, d[1], name):
            return {'status': 'written', 'detail': d[0], 'span': lines(c[1])}
        if c[0] == 'wrapped':
            return {'status': 'wrapped_jsx' if jsx else 'wrapped_nonjsx', 'span': lines(c[1]),
                    'detail': c[2]}
        return {'status': 'callable', 'span': lines(c[1]), 'detail': d[0]}

    def node_at(self, sb, eb):
        return self.root.descendant_for_byte_range(sb, eb)
