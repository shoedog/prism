"""JavaScript/TypeScript/TSX lexical bindings and eager syntax phases."""

from flow import INDEX
from frontend import EXCLUDED_CALLS, literal_boolean
from tree_frontend import (
    CALLABLES, JS_SCOPES, TreeFrontend, children, field, first, kind, span, unwrap, walk,
)

PATTERNS = {
    "array_pattern", "object_pattern", "rest_pattern", "assignment_pattern",
    "object_assignment_pattern", "pair_pattern", "required_parameter", "optional_parameter",
}
TYPE_NODES = {
    "type_annotation", "type_arguments", "type_parameters", "predefined_type",
    "type_identifier", "type_alias_declaration", "interface_declaration",
}
WRAPPERS = {"as_expression", "satisfies_expression", "non_null_expression", "type_assertion"}
BLOCKS = {"statement_block", "program", "else_clause"}
LOOPS = {"for_statement", "for_in_statement", "while_statement", "do_statement"}


def names(node):
    if kind(node) in ("identifier", "shorthand_property_identifier_pattern"):
        yield node
    elif kind(node) in PATTERNS:
        target = (field(node, "left") or field(node, "pattern")
                  or (field(node, "value") if node.type == "pair_pattern" else None))
        if target:
            yield from names(target)
        else:
            for child in children(node):
                yield from names(child)


def truth(node):
    node = unwrap(node)
    syntax = kind(node)
    if syntax in ("true", "array", "object", "regex"):
        return True
    if syntax in ("false", "null"):
        return False
    if syntax == "number":
        token = node.text.decode().replace("_", "").removesuffix("n")
        if token.lower().startswith(("0b", "0o", "0x")):
            return bool(int(token, 0))
        return bool(float(token))
    if syntax == "string" and b"\\" not in node.text:
        return len(node.text) > 2
    operator = field(node, "operator")
    if syntax == "binary_expression" and operator and operator.type in ("&&", "||"):
        return literal_boolean((truth(field(node, key)) for key in ("left", "right")),
                               operator.type == "&&")
    if syntax == "unary_expression" and operator and operator.type == "!":
        value = truth(field(node, "argument"))
        return None if value is None else not value
    return None


class JsFrontend(TreeFrontend):
    def __init__(self, file, owner):
        super().__init__(file, owner)
        self.collect(owner)
        self.collect_outer(self.collect)

    def collect(self, owner):
        for param in children(field(owner, "parameters")) + [field(owner, "parameter")]:
            for name in names(param):
                self.add_decl(name, owner, owner.start_byte, "parameter")
        if owner.type in ("function_expression", "generator_function") and field(owner, "name"):
            self.add_decl(field(owner, "name"), owner, owner.start_byte, "function")
        body = field(owner, "body")
        for node in walk(body, CALLABLES):
            if node.type == "variable_declarator":
                scope = (owner if kind(node.parent) == "variable_declaration"
                         else self.node_scope(node, JS_SCOPES))
                for name in names(field(node, "name")):
                    self.add_decl(name, scope, scope.start_byte, "local")
            elif node.type == "for_in_statement" and field(node, "kind"):
                scope = owner if self.text(field(node, "kind")) == "var" else node
                for name in names(field(node, "left")):
                    self.add_decl(name, scope, scope.start_byte, "local")
            elif node.type == "catch_clause":
                for name in names(field(node, "parameter")):
                    self.add_decl(name, node, node.start_byte, "local")
            elif node.type in ("function_declaration", "generator_function_declaration", "class_declaration"):
                scope = self.node_scope(node, JS_SCOPES)
                self.add_decl(field(node, "name"), scope, scope.start_byte, "local")

    def build(self):
        self.dynamic_scope = any(n.type == "with_statement" or (
            n.type == "call_expression" and field(n, "function")
            and self.text(unwrap(field(n, "function"))) == "eval"
        ) for n in walk(self.body, CALLABLES))
        if self.dynamic_scope:
            self.graph.flags["dynamic_scope"] += 1
        end = self.emit("owner_exit")
        if self.owner.type == "arrow_function" and kind(self.body) != "statement_block":
            return self.expr(self.body, end, "return")
        return self.stmt(self.body, end, ((), ()))

    def binding(self, root):
        binding, flag = super().binding(root)
        return binding, flag or ("dynamic_scope" if self.dynamic_scope else None)

    def targets(self, node, end, context):
        if kind(node) in PATTERNS:
            default = field(node, "right")
            target = field(node, "left") or field(node, "pattern")
            if node.type == "pair_pattern":
                target = field(node, "value")
            if target:
                result = self.targets(target, end, context)
                if default:
                    result = self.branch(None, self.expr(default, result, context=context),
                                         result, span(node))
                return result
            return self.lower_list(children(node), end, lambda n, k: self.targets(n, k, context))
        if kind(node) == "shorthand_property_identifier_pattern":
            decl = self.declaration(node)
            if decl:
                return self.emit("replace", (end,), span(node), (decl.identity,))
            return end
        self.location(node, "def")
        return self.static_receiver(node, self.mutation(node, end), context)

    def expr(self, node, end, role=None, context=((), ()), target=False, deferred=False):
        if node is None or node.type in CALLABLES or node.type in TYPE_NODES:
            return end
        node = unwrap(node)
        sub = lambda n, k: self.expr(n, k, context=context, deferred=deferred)
        loops, _ = context
        syntax = node.type
        if syntax in WRAPPERS:
            value = (children(node)[-1] if syntax == "type_assertion" else first(node))
            if role and self.path(unwrap(value)):
                flag = "ts_as_wrapper" if syntax == "as_expression" else "unsupported_wrapper"
                self.endpoint(unwrap(value), role, flags=(flag,), operand_span=span(node))
            return sub(value, end)
        if syntax in ("identifier", "member_expression") and not target:
            self.location(node, "use")
        if role and not deferred:
            end = self.read(node, role, end)
        if syntax in ("call_expression", "new_expression"):
            callee = field(node, "function") or field(node, "constructor")
            args = children(field(node, "arguments"))
            optional = field(node, "optional_chain") or (
                callee and any(field(n, "optional_chain") for n in walk(callee, CALLABLES))
            )
            callee_path = self.path(callee)
            callee_name = ".".join(callee_path[2]) if callee_path else ""
            role = ("arg" if callee and callee_name not in EXCLUDED_CALLS
                    and not deferred and not optional else None)
            after = self.lower_list(args, end, lambda n, k: self.expr(
                n, k, role, context, deferred=deferred
            ))
            return sub(callee, self.branch(None, after, end, span(node)) if optional else after)
        if syntax in ("assignment_expression", "augmented_assignment_expression"):
            left, right = field(node, "left"), field(node, "right")
            operator = self.operator(node)
            conditional = operator in ("&&=", "||=", "??=")
            compound = syntax == "augmented_assignment_expression"
            rhs_role = None if compound or kind(left) in PATTERNS else "assign_rhs"
            lower_rhs = lambda k: self.expr(right, k, rhs_role, context, deferred=deferred)
            if not conditional:
                start = self.write(left, sub, lower_rhs, end, loops, compound=compound)
                if start is not None:
                    return start
            self.location(left, "def")
            if kind(left) in PATTERNS:
                return lower_rhs(self.targets(left, end, context))
            assigned = lower_rhs(self.mutation(left, end))
            if conditional:
                assigned = self.branch(None, assigned, end, span(node))
            return self.static_receiver(left, assigned, context)
        if syntax == "binary_expression" and self.operator(node) in ("&&", "||", "??"):
            left, right = field(node, "left"), field(node, "right")
            operator = self.operator(node)
            value = truth(left)
            if operator == "??":
                literal = kind(unwrap(left))
                value = None
                if literal == "null":
                    value = True
                elif literal in ("number", "string", "true", "false", "array", "object", "regex"):
                    value = False
            yes, no = (sub(right, end), end) if operator != "||" else (end, sub(right, end))
            return sub(left, self.branch(value, yes, no, span(node)))
        if syntax == "ternary_expression":
            test = field(node, "condition")
            return sub(test, self.branch(truth(test), sub(field(node, "consequence"), end),
                                         sub(field(node, "alternative"), end), span(node)))
        if syntax == "unary_expression" and self.operator(node) == "delete":
            arg = field(node, "argument") or children(node)[-1]
            effect = self.mutation(arg, end, "delete" if INDEX in (self.path(arg, indexed=True)
                                   or (None, None, ()))[2] else "replace")
            if self.path(arg, indexed=True) and self.excluded_chain(arg):
                effect = self.branch(None, effect, end, span(node))
            return self.static_receiver(arg, effect, context)
        if syntax == "update_expression":
            arg = field(node, "argument")
            self.location(arg, "def")
            return self.static_receiver(arg, self.mutation(arg, end), context)
        if syntax in ("await_expression", "yield_expression"):
            return self.lower_list(children(node), self.uncertain(node, end), sub)
        if syntax == "member_expression":
            obj = field(node, "object")
            return sub(obj, end)
        if syntax == "subscript_expression":
            obj, idx = field(node, "object"), field(node, "index")
            indexed = sub(idx, end)
            if field(node, "optional_chain"):
                indexed = self.branch(None, indexed, end, span(node))
            return sub(obj, indexed)
        if syntax in ("jsx_element", "jsx_opening_element", "jsx_self_closing_element",
                      "jsx_fragment", "jsx_attribute"):
            values = [n for n in children(node) if n.type.startswith("jsx_")
                      and n.type not in ("jsx_closing_element", "jsx_text")]
            return self.lower_list(values, end, sub)
        if syntax == "class":
            return self.class_effect(node, end)
        return self.lower_list(children(node), end, sub)

    def class_effect(self, node, end):
        # Class evaluation has unmodelled phases. Inspect computed names and
        # heritage, which evaluate now, while skipping deferred method bodies.
        candidates = list(children(node))
        members = children(field(node, "body"))
        candidates += [field(member, "name") for member in members]
        paths = set()
        for candidate in candidates:
            if kind(candidate) not in ("class_heritage", "computed_property_name", "decorator"):
                continue
            for child in walk(candidate, CALLABLES):
                if child.type not in ("assignment_expression", "augmented_assignment_expression"):
                    continue
                target = self.path(field(child, "left"))
                if target:
                    root, _, path = target
                    binding, refusal = self.binding(root)
                    if not refusal:
                        paths.add((binding, *path[1:]))
        if paths:
            line = self.statement_line(node)
            self.graph.unordered_locations[line].update(paths)
            return self.emit("unordered", (end,), span(node), tuple(sorted(paths)))
        return self.uncertain(node, end)

    def stmt(self, node, end, context, label=None):
        if node is None:
            return end
        loops, transfers = context
        expr = lambda n, k, role=None: self.expr(n, k, role, context)
        children_of = children(node)
        syntax = node.type
        if syntax in BLOCKS:
            return self.sequence(children_of, end, context)
        if syntax in ("expression_statement", "empty_statement"):
            return expr(first(node), end)
        if syntax in ("lexical_declaration", "variable_declaration"):
            return self.sequence(children_of, end, context)
        if syntax == "variable_declarator":
            value, name = field(node, "value"), field(node, "name")
            return expr(value, self.targets(name, end, context),
                        None if kind(name) in PATTERNS else "assign_rhs")
        if syntax in ("return_statement", "throw_statement"):
            return expr(first(node), self.emit(syntax, (), span(node)),
                        "return" if syntax == "return_statement" else None)
        if syntax == "if_statement":
            test = field(node, "condition")
            yes = self.stmt(field(node, "consequence"), end, context)
            no = self.stmt(field(node, "alternative"), end, context)
            return expr(test, self.branch(truth(test), yes, no, span(node)))
        if syntax in LOOPS:
            return self.loop(node, end, context, label)
        if syntax in ("break_statement", "continue_statement"):
            return self.jump(node, end, transfers, syntax == "continue_statement")
        if syntax == "labeled_statement":
            body = field(node, "body") or children_of[-1]
            name = self.label(node)
            if kind(body) in LOOPS:
                return self.stmt(body, end, context, label=name)
            return self.stmt(body, end, (loops, (*transfers, (name, end, None))))
        if syntax == "try_statement":
            tail = self.uncertain(node, end)
            blocked = (loops, self.block_jumps(node, transfers))
            finalizer = field(node, "finalizer")
            if finalizer:
                tail = self.stmt(field(finalizer, "body"), tail, blocked)
                tail = self.uncertain(finalizer, tail)
            body = self.stmt(field(node, "body"), tail, blocked)
            handler = field(node, "handler")
            if not handler:
                return body
            handled = self.stmt(field(handler, "body"), tail, blocked)
            return self.branch(None, body, self.uncertain(handler, handled), span(node))
        if syntax == "switch_statement":
            body = field(node, "body")
            after = self.uncertain(node, end)
            arms = []
            for case in reversed(children(body)):
                value = field(case, "value")
                statements = [n for n in children(case) if n != value]
                case_context = (loops, (*self.block_jumps(node, transfers), (None, after, None)))
                arms.append(expr(value, self.sequence(statements, after, case_context)))
            return expr(field(node, "value"), self.uncertain(node, *arms, end))
        if syntax == "class_declaration":
            return self.class_effect(node, end)
        if syntax in CALLABLES or syntax in TYPE_NODES:
            return end
        return self.uncertain(node, end)

    def block_jumps(self, node, transfers):
        return tuple((name, self.uncertain(node, stop),
                      self.uncertain(node, resume) if resume is not None else None)
                     for name, stop, resume in transfers)

    def loop(self, node, end, context, label):
        loops, transfers = context
        syntax = node.type
        identity = span(node)
        header = self.emit("loop_test", (), identity)
        iteration_context = ((*loops, identity), transfers)
        update = self.expr(field(node, "increment"), header, context=iteration_context)
        jumps = (*transfers, (None, end, update))
        if label:
            jumps = (*jumps, (label, end, update))
        inner = ((*loops, identity), jumps)
        body = self.stmt(field(node, "body"), update, inner)
        if syntax == "for_in_statement":
            target = field(node, "left")
            enter = self.emit("loop_entry", (self.targets(target, body, inner),), identity,
                              identity)
            self.graph.events[header].successors = (self.branch(None, enter, end, identity),)
            collection = self.expr(field(node, "right"), header, context=context)
            if field(node, "value") or any(n.type == "await" for n in node.children):
                return self.uncertain(node, collection, end)
            return collection
        test = field(node, "condition")
        if kind(test) in ("empty_statement", ";"):
            test = None
        enter = self.emit("loop_entry", (body,), identity, identity)
        check = self.branch(truth(test) if test else True, enter, end, identity)
        self.graph.events[header].successors = (self.expr(test, check, context=inner),)
        if syntax == "do_statement":
            return enter
        initializer = field(node, "initializer")
        if kind(initializer) in ("lexical_declaration", "variable_declaration", "empty_statement"):
            return self.stmt(initializer, header, context)
        return self.expr(initializer, header, context=context)
