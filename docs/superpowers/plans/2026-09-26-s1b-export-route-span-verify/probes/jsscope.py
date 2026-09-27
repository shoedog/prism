"""Independent JS/TS lexical-binding auditor for S1b (tree-sitter-python; grammars pinned to Cargo.lock:
typescript 0.23.2, javascript 0.23.1). It does not link prism. r1 fold: rewritten against SPEC §3.1's enumerated
table (ECMA-262 VarScopedDeclarations / LexicallyScopedDeclarations / BoundNames, Annex B.3.3-B.3.4, `with`, the
implicit `arguments`, class static blocks and TS namespace bodies as var scopes, TS value vs type space, `using`,
`import x = ...`, labelled declarations, destructuring defaults, parameter scopes and decorators).

Same author as the prototype: it catches implementation slips, not a shared misreading of the rules. The pinned
controls (probes/controls_gen.py, one JSX and one TSX row per table entry) are the independent check on the rules.

r3 (Fable re-plan, 2026-09-26): folds sol r2 W2 (structural brace tokens), Opus r2 W2 (delimited-child sealing) and
Opus r2 S2 (the `with` tuple). It does NOT yet implement the evaluation-context table of REPLAN-fable.md §3 (computed
member keys, namespace/enum leave rule, `with` object position): those are the implementer's RED rows, and this
auditor still shares v8's position blindness there (recorded so nobody cites it as independent on those forms).

resolve_callable(site, name, jsx) -> dict(status, span, detail, local) with status in:
  callable | wrapped_jsx | wrapped_nonjsx | maycall | not_callable | dup | global | parse | escaped | with
"""
import re
import tree_sitter as ts, tree_sitter_typescript as tst, tree_sitter_javascript as tsj

LANG = {'.ts': ts.Language(tst.language_typescript()), '.tsx': ts.Language(tst.language_tsx()),
        '.js': ts.Language(tsj.language()), '.jsx': ts.Language(tsj.language()),
        '.mjs': ts.Language(tsj.language()), '.cjs': ts.Language(tsj.language())}
FUNCS = {'function_declaration', 'generator_function_declaration', 'function_expression', 'function',
         'generator_function', 'arrow_function', 'method_definition'}
CLASSES = {'class_declaration', 'class', 'abstract_class_declaration'}
FN_VALUES = {'arrow_function', 'function_expression', 'function'}
REACT = {'forwardRef', 'memo', 'React.forwardRef', 'React.memo'}
PASS = {'useCallback', 'React.useCallback'}
WRAPPERS = {'parenthesized_expression', 'as_expression', 'satisfies_expression', 'non_null_expression',
            'type_assertion', 'assignment_expression'}


def lines(n):
    return (n.start_point[0] + 1, n.end_point[0] + 1)


def inside(outer, inner):
    return outer.start_byte <= inner.start_byte and inner.end_byte <= outer.end_byte


class Src:
    def __init__(self, path, ext):
        self.src = open(path, 'rb').read()
        self.tree = ts.Parser(LANG[ext]).parse(self.src)
        self.root = self.tree.root_node
        self._idx = {}

    def t(self, n):
        return self.src[n.start_byte:n.end_byte].decode('utf8', 'replace')

    # ---- patterns ---------------------------------------------------------------------------------------
    def pattern_names(self, p, out):
        if p is None:
            return
        k = p.type
        if k in ('identifier', 'shorthand_property_identifier_pattern'):
            out.append(self.t(p))
        elif k in ('pair_pattern',):
            self.pattern_names(p.child_by_field_name('value'), out)
        elif k in ('assignment_pattern', 'object_assignment_pattern'):
            self.pattern_names(p.child_by_field_name('left'), out)
        elif k in ('required_parameter', 'optional_parameter'):
            self.pattern_names(p.child_by_field_name('pattern'), out)
        elif k in ('as_expression', 'satisfies_expression', 'non_null_expression', 'parenthesized_expression'):
            if p.named_children:
                self.pattern_names(p.named_children[0], out)
        elif k == 'type_assertion':
            if p.named_children:
                self.pattern_names(p.named_children[-1], out)
        elif k in ('object_pattern', 'array_pattern', 'rest_pattern', 'formal_parameters'):
            for c in p.named_children:
                self.pattern_names(c, out)

    def params(self, fn):
        out = []
        ps = fn.child_by_field_name('parameters')
        if ps is not None:
            self.pattern_names(ps, out)
        p = fn.child_by_field_name('parameter')
        if p is not None:
            self.pattern_names(p, out)
        return out

    @staticmethod
    def using(stmt):
        if stmt.type != 'expression_statement' or not stmt.named_children:
            return None
        e = stmt.named_children[0]
        if e.type == 'await_expression' and e.named_children:
            e = e.named_children[0]
        if e.type == 'assignment_expression' and any(not c.is_named and c.type == 'using' for c in e.children):
            return e
        return None

    # ---- declarations -----------------------------------------------------------------------------------
    def walk(self, node, direct, hoist, taint, out):
        for ch in node.named_children:
            k = ch.type
            nm = ch.child_by_field_name('name')
            own = self.t(nm).split('.')[0] if nm is not None else None
            if k in FUNCS or k in CLASSES or k in ('class_static_block', 'enum_declaration', 'decorator',
                                                    'internal_module', 'module'):
                decl_kinds = ('function_declaration', 'generator_function_declaration', 'class_declaration',
                              'abstract_class_declaration', 'internal_module', 'module', 'enum_declaration')
                if own and direct and k in decl_kinds:
                    out.append((own, ch))
                elif own and hoist and k.endswith('function_declaration'):
                    out.append((own, node))                     # Annex B marker
                continue
            if k == 'variable_declarator':
                ns = []
                self.pattern_names(ch.child_by_field_name('name'), ns)
                for n in ns:
                    out.append((n, node if ('' in taint or n in taint) else ch))
                continue
            if k == 'variable_declaration' and not hoist:
                continue
            if k == 'lexical_declaration' and not direct:
                continue
            if k in ('interface_declaration', 'type_alias_declaration'):
                continue
            if k in ('import_statement', 'import_alias'):
                if direct:
                    for n in self.import_names(ch):
                        out.append((n, ch))
                continue
            if k == 'function_signature':
                if direct and own and node.type == 'ambient_declaration':
                    out.append((own, ch))
                continue
            if k == 'for_in_statement' and hoist:
                if any(c.type == 'var' for c in ch.children):
                    ns = []
                    self.pattern_names(ch.child_by_field_name('left'), ns)
                    for n in ns:
                        out.append((n, ch))
            if k == 'catch_clause':
                ns = list(taint)
                self.pattern_names(ch.child_by_field_name('parameter'), ns)
                b = ch.child_by_field_name('body')
                if b is not None:
                    self.walk(b, False, hoist, set(ns), out)
                continue
            if k == 'with_statement':
                b = ch.child_by_field_name('body')
                if b is not None:
                    self.walk(b, False, hoist, {''}, out)
                continue
            u = self.using(ch)
            if u is not None and direct:
                ns = []
                self.pattern_names(u.child_by_field_name('left'), ns)
                for n in ns:
                    out.append((n, u))
                continue
            transparent = k in ('export_statement', 'expression_statement', 'labeled_statement', 'switch_case',
                                'switch_default', 'ambient_declaration', 'lexical_declaration',
                                'variable_declaration')
            self.walk(ch, direct and transparent, hoist, taint, out)

    def import_names(self, node):
        if node.type == 'import_alias':
            return [self.t(node.named_children[0])] if node.named_children else []
        if any(c.type == 'type' for c in node.children):
            return []
        out, stack = [], [node]
        while stack:
            n = stack.pop()
            for c in n.named_children:
                if c.type == 'identifier':
                    out.append(self.t(c))
                elif c.type == 'import_specifier':
                    if any(x.type == 'type' for x in c.children):
                        continue
                    a = c.child_by_field_name('alias') or c.child_by_field_name('name')
                    out.append(self.t(a))
                elif c.type in ('import_clause', 'namespace_import', 'named_imports', 'import_require_clause'):
                    stack.append(c)
        return out

    def is_scope(self, n):
        k = n.type
        if k in ('program', 'formal_parameters', 'for_statement', 'for_in_statement', 'catch_clause',
                 'switch_body', 'with_statement') or k in FUNCS or k in CLASSES:
            return True
        return k == 'statement_block' and not (n.parent is not None and n.parent.type in FUNCS)

    def scope_decls(self, s):
        if s.id in self._idx:
            return self._idx[s.id]
        out = []
        k = s.type
        if k == 'catch_clause':
            ns = []
            self.pattern_names(s.child_by_field_name('parameter'), ns)
            out += [(n, s) for n in ns]
        elif k == 'for_in_statement':
            if any(c.type in ('let', 'const') for c in s.children):
                ns = []
                self.pattern_names(s.child_by_field_name('left'), ns)
                out += [(n, s) for n in ns]
        elif k == 'for_statement':
            init = s.child_by_field_name('initializer')
            if init is not None and init.type == 'lexical_declaration':
                self.walk(init, True, False, set(), out)
        elif k in CLASSES:
            nm = s.child_by_field_name('name')
            if nm is not None:
                out.append((self.t(nm), s))
        elif k == 'formal_parameters':
            f = s.parent
            out += [(n, s) for n in self.params(f)]
            if f.type != 'arrow_function':
                out.append(('arguments', s))
            if f.type in ('function_expression', 'function', 'generator_function'):
                nm = f.child_by_field_name('name')
                if nm is not None:
                    out.append((self.t(nm), f))
        elif k in FUNCS:
            at = s.child_by_field_name('parameters') or s.child_by_field_name('parameter') or s
            out += [(n, at) for n in self.params(s)]
            if k != 'arrow_function':
                out.append(('arguments', at))
            if k in ('function_expression', 'function', 'generator_function'):
                nm = s.child_by_field_name('name')
                if nm is not None:
                    out.append((self.t(nm), s))
            b = s.child_by_field_name('body')
            if b is not None and b.type == 'statement_block':
                self.walk(b, True, True, set(), out)
        elif k == 'statement_block':
            var_scope = s.parent is not None and s.parent.type in ('class_static_block', 'internal_module', 'module')
            self.walk(s, True, var_scope, set(), out)
        elif k in ('switch_body', 'program'):
            self.walk(s, True, k == 'program', set(), out)
        self._idx[s.id] = out
        return out

    @staticmethod
    def up(n):
        if n.type == 'formal_parameters':
            return n.parent.parent if n.parent is not None else None
        if n.type == 'decorator':
            u = n.parent
            while u is not None and u.type not in CLASSES:
                u = u.parent
            return u.parent if u is not None else None
        return n.parent

    def binding(self, site, name):
        n = site.parent
        while n is not None:
            if n.type == 'with_statement':
                return n, [n]                                # r3 fold (Opus r2 S2): a node, not a tuple
            if self.is_scope(n):
                ds = [d for nm, d in self.scope_decls(n) if nm == name]
                if ds:
                    return n, ds
            n = self.up(n)
        return None, []

    # ---- checks -----------------------------------------------------------------------------------------
    def escaped(self, scope):
        stack = [scope]
        while stack:
            n = stack.pop()
            if n.type in ('identifier', 'shorthand_property_identifier_pattern') and '\\' in self.t(n):
                return True
            stack.extend(n.named_children)
        return False

    def braces_intact(self):
        # r3 fold (sol r2 W2): only structural brace *tokens* count, never brace characters inside string,
        # template, regex or comment text of an ERROR node.
        if not hasattr(self, '_braces'):
            ok, stack = True, [self.root]
            while stack and ok:
                n = stack.pop()
                if n.is_missing and n.type in ('{', '}'):
                    ok = False
                if n.is_error and any(not c.is_named and c.type in ('{', '}') for c in n.children):
                    ok = False
                stack.extend(c for c in n.children if c.has_error or c.is_missing)
            self._braces = ok
        return self._braces

    @staticmethod
    def delimited_children(u):
        # r3 fold (Opus r2 W2): the children whose first and last tokens are paired delimiters.
        for f, (o, c) in (('body', ('{', '}')), ('parameters', ('(', ')'))):
            ch = u.child_by_field_name(f)
            if ch is not None and ch.child_count >= 2 and ch.children[0].type == o and \
                    ch.children[-1].type == c and not ch.children[0].is_missing and not ch.children[-1].is_missing:
                yield ch

    def sealed(self, scope, site):
        if not scope.has_error:
            return True
        if not self.braces_intact():
            return False
        stack = [scope]
        while stack:
            n = stack.pop()
            if n.is_error or n.is_missing:
                u = n.parent
                while u is not None and u.id != scope.id and not (
                        u.type in CLASSES or u.type == 'class_static_block' or (
                        u.type in FUNCS and (u.child_by_field_name('body') is not None and
                                             u.child_by_field_name('body').type == 'statement_block'))):
                    u = u.parent
                if u is None or u.id == scope.id or (site is not None and inside(u, site)):
                    return False
                # r3 fold (Opus r2 W2): a header error (name, type parameters, return type, heritage) is not
                # sealed; only an error inside a delimited child of the sealing node is.
                if not any(inside(d, n) for d in self.delimited_children(u)):
                    return False
            stack.extend(c for c in n.children if c.has_error or c.is_missing)
        return True

    def written(self, scope, name):
        stack = [scope]
        while stack:
            n = stack.pop()
            k = n.type
            is_using = any(not c.is_named and c.type == 'using' for c in n.children)
            if (k in ('assignment_expression', 'augmented_assignment_expression', 'update_expression')
                    and not is_using) or (k == 'for_in_statement' and not any(
                    c.type in ('var', 'let', 'const') for c in n.children)):
                tgt = n.child_by_field_name('left') or n.child_by_field_name('argument')
                ns = []
                self.pattern_names(tgt, ns)
                if name in ns:
                    s, _ = self.binding(tgt, name)
                    if s is not None and (s.id == scope.id or s.type == 'with_statement'):
                        return True
            stack.extend(n.named_children)
        return False

    def fn_name(self, fn):
        nm = fn.child_by_field_name('name')
        if nm is not None:
            return self.t(nm)
        p = fn.parent
        if p is None:
            return None
        if p.type == 'variable_declarator':
            return self.t(p.child_by_field_name('name'))
        if p.type == 'pair':
            return self.t(p.child_by_field_name('key'))
        if p.type == 'assignment_expression':
            left = p.child_by_field_name('left')
            if left.type == 'member_expression':
                return self.t(left.child_by_field_name('property'))
            if left.type == 'identifier':
                return self.t(left)
        if p.type == 'arguments' and p.parent is not None and p.parent.type == 'call_expression' \
                and p.parent.parent is not None and p.parent.parent.type == 'variable_declarator':
            return self.t(p.parent.parent.child_by_field_name('name'))
        return None

    def resolve_callable(self, site, name, jsx):
        scope, ds = self.binding(site, name)
        if scope is None:
            return {'status': 'global'}
        return self.classify(scope, ds, name, site, jsx)

    def module_callable(self, name, jsx):
        ds = [d for nm, d in self.scope_decls(self.root) if nm == name]
        if not ds:
            return {'status': 'absent'}
        return self.classify(self.root, ds, name, None, jsx)

    def classify(self, scope, ds, name, site, jsx):
        if self.escaped(scope):
            return {'status': 'escaped'}
        if not self.sealed(scope, site):
            return {'status': 'parse'}
        if scope.type == 'with_statement':
            return {'status': 'with'}
        if len(ds) > 1:
            return {'status': 'dup'}
        d = ds[0]
        fe_self = d.type in ('function_expression', 'function', 'generator_function') and \
            d.child_by_field_name('name') is not None and self.t(d.child_by_field_name('name')) == name
        using = d.type == 'assignment_expression'
        declarator = d.type in ('variable_declarator',) or using
        function = d.type.endswith('function_declaration')
        if (fe_self or declarator or function) and self.written(scope, name):
            return {'status': 'maycall', 'detail': 'written'}
        if function or fe_self:
            fn = d
        elif declarator:
            v = d.child_by_field_name('right' if using else 'value')
            while v is not None and v.type in WRAPPERS:
                v = v.child_by_field_name('right') if v.type == 'assignment_expression' else (
                    v.named_children[-1] if v.type == 'type_assertion' else v.named_children[0])
            if v is None:
                return {'status': 'not_callable', 'detail': 'no_value'}
            if v.type in FN_VALUES:
                fn = v
            elif v.type == 'call_expression':
                callee = ' '.join(self.t(v.child_by_field_name('function')).split())
                args = v.child_by_field_name('arguments')
                fargs = [a for a in (args.named_children if args is not None else []) if a.type in FN_VALUES]
                if not using and callee in REACT | PASS and fargs and d.parent.type == 'lexical_declaration' \
                        and self.t(d.parent).startswith('const'):
                    inner = fargs[0]
                    st = 'callable' if callee in PASS else ('wrapped_jsx' if jsx else 'wrapped_nonjsx')
                    return {'status': st, 'span': lines(inner), 'local': self.fn_name(inner), 'detail': callee}
                if fargs:
                    return {'status': 'maycall', 'detail': callee}
                return {'status': 'not_callable', 'detail': 'call'}
            else:
                return {'status': 'not_callable', 'detail': v.type}
        else:
            return {'status': 'not_callable', 'detail': d.type}
        return {'status': 'callable', 'span': lines(fn), 'local': self.fn_name(fn), 'detail': d.type}

    def node_at(self, sb, eb):
        return self.root.descendant_for_byte_range(sb, eb)
