"""Go lexical bindings and the supported statement/communication phases."""

from frontend import EXCLUDED_CALLS, literal_boolean
from tree_frontend import (
    CALLABLES, GO_SCOPES, TreeFrontend, children, field, fields, first, kind, span, unwrap, walk,
)

BUILTIN_TYPES = {
    "any", "bool", "byte", "comparable", "complex64", "complex128", "error",
    "float32", "float64", "int", "int8", "int16", "int32", "int64", "rune",
    "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
}
CONTAINERS = {"map_type": "map", "array_type": "array", "slice_type": "slice",
              "implicit_length_array_type": "array"}
CASE_KINDS = {"expression_case", "default_case", "communication_case", "type_case"}


def operands(node):
    if kind(node) == "expression_list":
        return children(node)
    return [node] if node else []


def truth(node):
    node = unwrap(node)
    if kind(node) == "true":
        return True
    if kind(node) == "false":
        return False
    operator = field(node, "operator")
    if kind(node) == "binary_expression" and operator and operator.type in ("&&", "||"):
        return literal_boolean((truth(field(node, key)) for key in ("left", "right")),
                               operator.type == "&&")
    if kind(node) == "unary_expression" and operator and operator.type == "!":
        value = truth(field(node, "operand"))
        return None if value is None else not value
    return None


class GoFrontend(TreeFrontend):
    def __init__(self, file, owner):
        super().__init__(file, owner)
        self.collect(owner)
        self.collect_outer(self.collect)

    def add_go(self, name, scope, visible, tag, annotation=None, reuse=False):
        if kind(name) != "identifier" and tag != "type":
            return
        if reuse and any(d.scope == scope and d.visible <= name.start_byte
                         for d in self.declarations[self.text(name)]):
            return
        self.add_decl(name, scope, visible, tag, annotation)

    def value_type(self, value):
        if kind(value) in ("composite_literal", "type_conversion_expression"):
            return field(value, "type")
        if kind(value) == "call_expression" and field(value, "function"):
            callee = self.text(field(value, "function"))
            if callee == "new":
                return value
            if callee == "make":
                return first(field(value, "arguments"))
        return None

    def collect(self, owner):
        for slot in ("receiver", "parameters", "result"):
            for param in children(field(owner, slot)):
                for name in fields(param, "name"):
                    self.add_go(name, owner, owner.start_byte, "parameter", field(param, "type"))
        for node in walk(field(owner, "body"), CALLABLES):
            syntax = node.type
            if syntax in ("var_spec", "const_spec"):
                scope = self.node_scope(node, GO_SCOPES)
                values = operands(field(node, "value"))
                for i, name in enumerate(fields(node, "name")):
                    annotation = field(node, "type") or (
                        self.value_type(values[i]) if i < len(values) else None
                    )
                    self.add_go(name, scope, node.end_byte, "local", annotation)
            elif syntax in ("type_spec", "type_alias"):
                scope = self.node_scope(node, GO_SCOPES)
                self.add_go(field(node, "name"), scope, node.start_byte, "type", field(node, "type"))
            elif (syntax == "short_var_declaration" or
                  (syntax in ("range_clause", "receive_statement") and
                   any(ch.type == ":=" for ch in node.children))):
                scope = self.node_scope(node, GO_SCOPES)
                values = operands(field(node, "right"))
                for i, name in enumerate(operands(field(node, "left"))):
                    annotation = self.value_type(values[i]) if i < len(values) else None
                    self.add_go(name, scope, node.end_byte, "local", annotation, reuse=True)

    def type_node(self, annotation):
        visited = set()
        while kind(annotation) == "type_identifier":
            name = self.text(annotation)
            if name in visited:
                break
            visited.add(name)
            decl = self.declaration(annotation)
            annotation = (decl.annotation if decl and decl.tag == "type"
                          else self.file.module_types.get(name))
        return annotation

    def annotate_container(self, point, root, links):
        if point.role not in ("write", "aug_write"):
            return
        decl = self.declaration(root)
        annotation = self.type_node(decl.annotation) if decl else None
        if kind(annotation) in ("pointer_type", "call_expression"):
            point.flags.add("pointer_root")
        for _, member, operand in links:
            if operand is not None:
                break
            if kind(annotation) == "struct_type":
                field_list = first(annotation)
                member_decl = next((f for f in children(field_list)
                                    if any(self.text(n) == member for n in fields(f, "name"))), None)
                annotation = self.type_node(field(member_decl, "type"))
            else:
                annotation = None
        point.container_kind = CONTAINERS.get(kind(annotation), "unknown")

    def build(self):
        end = self.emit("owner_exit")
        for node in walk(self.body, CALLABLES):
            if node.type == "labeled_statement":
                self.labels[self.label(node)] = self.emit("label", (), span(node))
        return self.stmt(self.body, end, ((), (), None))

    def is_conversion(self, callee):
        node = unwrap(callee)
        if kind(node) == "identifier":
            name = self.text(node)
            decl = self.declaration(node)
            return (decl.tag == "type" if decl else
                    name in BUILTIN_TYPES or name in self.file.module_types)
        if kind(node) in ("index_expression", "generic_type"):
            return self.is_conversion(field(node, "operand") or field(node, "type") or first(node))
        return False

    def expr(self, node, end, role=None, context=((), (), None), target=False, deferred=False):
        if node is None or node.type in CALLABLES:
            return end
        node = unwrap(node)
        sub = lambda n, k: self.expr(n, k, context=context, deferred=deferred)
        syntax = node.type
        if syntax in ("identifier", "selector_expression") and not target:
            self.location(node, "use")
        if role and not deferred:
            end = self.read(node, role, end)
        if syntax == "call_expression":
            callee = field(node, "function")
            args = children(field(node, "arguments"))
            name = self.text(unwrap(callee)) if callee else ""
            if name in ("delete", "clear") and args and not deferred:
                end = self.mutation(args[0], end, "clear")
            eligible = (not deferred and name not in EXCLUDED_CALLS and
                        not self.is_conversion(callee))
            # Nested evaluations happen before deferred/concurrent invocation.
            end = self.lower_list(args, end, lambda n, k: self.expr(
                n, k, "arg" if eligible else None, context,
            ))
            return self.expr(callee, end, context=context)
        if syntax == "type_conversion_expression":
            return sub(field(node, "operand"), end)
        if syntax == "binary_expression" and self.operator(node) in ("&&", "||"):
            left, right = field(node, "left"), field(node, "right")
            yes, no = (sub(right, end), end) if self.operator(node) == "&&" else (end, sub(right, end))
            return sub(left, self.branch(truth(left), yes, no, span(node)))
        if syntax == "selector_expression":
            return sub(field(node, "operand"), end)
        return self.lower_list(children(node), end, sub)

    def assignments(self, left, right, end, context, op="=", allow_write=True):
        loops, _, _ = context
        expr = lambda n, k, role=None: self.expr(n, k, role, context)
        plain = op in ("=", ":=")
        if allow_write and len(left) == len(right) == 1:
            start = self.write(left[0], expr,
                               lambda k: expr(right[0], k, "assign_rhs" if plain else None),
                               end, loops, compound=not plain)
            if start is not None:
                return start
        assigned = end
        for target in reversed(left):
            self.location(target, "def")
            assigned = self.mutation(target, assigned)
        values = self.lower_list(right, assigned, lambda n, k: expr(
            n, k, "assign_rhs" if plain else None
        ))
        return self.lower_list(left, values, lambda n, k: self.static_receiver(n, k, context))

    def stmt(self, node, end, context, label=None):
        if node is None:
            return end
        loops, transfers, fallthrough = context
        syntax = node.type
        child_nodes = children(node)
        expr = lambda n, k, role=None: self.expr(n, k, role, context)
        if syntax in ("block", "statement_list", "var_declaration", "const_declaration"):
            return self.sequence(child_nodes, end, context)
        if syntax in ("empty_statement", "type_declaration"):
            return end
        if syntax == "expression_statement":
            return expr(first(node), end)
        if syntax in ("assignment_statement", "short_var_declaration"):
            return self.assignments(operands(field(node, "left")), operands(field(node, "right")),
                                    end, context, self.operator(node) or ":=")
        if syntax in ("var_spec", "const_spec"):
            return self.assignments(fields(node, "name"), operands(field(node, "value")),
                                    end, context, allow_write=False)
        if syntax in ("inc_statement", "dec_statement"):
            target = first(node)
            self.location(target, "def")
            return self.static_receiver(target, self.mutation(target, end), context)
        if syntax == "return_statement":
            halt = self.emit("return", (), span(node))
            return self.lower_list(operands(first(node)), halt, lambda n, k: expr(n, k, "return"))
        if syntax == "if_statement":
            test = field(node, "condition")
            yes = self.stmt(field(node, "consequence"), end, context)
            no = self.stmt(field(node, "alternative"), end, context)
            return self.stmt(field(node, "initializer"),
                             expr(test, self.branch(truth(test), yes, no, span(node))), context)
        if syntax == "for_statement":
            return self.loop(node, end, context, label)
        if syntax in ("break_statement", "continue_statement"):
            return self.jump(node, end, transfers, syntax == "continue_statement")
        if syntax == "goto_statement":
            dest = self.labels.get(self.label(node))
            return self.uncertain(node, dest) if dest is not None else self.uncertain(node)
        if syntax == "fallthrough_statement":
            return (self.uncertain(node, fallthrough) if fallthrough is not None
                    else self.uncertain(node))
        if syntax == "labeled_statement":
            body = next((n for n in child_nodes if n != field(node, "label")), None)
            name = self.label(node)
            start = self.stmt(body, end, context, name)
            self.graph.events[self.labels[name]].successors = (start,)
            return self.labels[name]
        if syntax == "expression_switch_statement":
            return self.switch(node, end, context, label)
        if syntax == "type_switch_statement":
            arms = []
            for case in child_nodes:
                if case.type in CASE_KINDS:
                    ctx = (loops, (*transfers, (None, end, None)), None)
                    body = [n for n in children(case) if n not in fields(case, "type")]
                    arms.append(self.sequence(body, end, ctx))
            branch = expr(field(node, "value"), self.uncertain(node, *arms, end))
            return self.stmt(field(node, "initializer"), branch, context)
        if syntax == "select_statement":
            return self.select(node, end, context, label)
        if syntax in ("defer_statement", "go_statement"):
            registration = self.emit(syntax, (end,), span(node))
            return self.expr(first(node), registration, context=context, deferred=True)
        if syntax == "send_statement":
            return expr(field(node, "channel"), expr(field(node, "value"), end))
        return self.uncertain(node, end)

    def loop(self, node, end, context, label):
        loops, transfers, fallthrough = context
        identity = span(node)
        header = self.emit("loop_test", (), identity)
        clause = next((n for n in children(node) if n != field(node, "body")), None)
        inner_loops = (*loops, identity)
        post = self.stmt(field(clause, "update"), header, (inner_loops, transfers, fallthrough))
        jumps = (*transfers, (None, end, post))
        if label:
            jumps = (*jumps, (label, end, post))
        inner = (inner_loops, jumps, fallthrough)
        body = self.stmt(field(node, "body"), post, inner)
        if kind(clause) == "range_clause":
            bind = self.assignments(operands(field(clause, "left")), [], body, inner,
                                    allow_write=False)
            enter = self.emit("loop_entry", (bind,), identity, identity)
            self.graph.events[header].successors = (self.branch(None, enter, end, identity),)
            return self.expr(field(clause, "right"), header, context=context)
        test = field(clause, "condition") if kind(clause) == "for_clause" else clause
        enter = self.emit("loop_entry", (body,), identity, identity)
        check = self.branch(truth(test) if test else True, enter, end, identity)
        self.graph.events[header].successors = (self.expr(test, check, context=inner),)
        return self.stmt(field(clause, "initializer"), header, context)

    def case_transfers(self, end, transfers, label):
        transfers = (*transfers, (None, end, None))
        if label:
            transfers = (*transfers, (label, end, None))
        return transfers

    def switch(self, node, end, context, label):
        loops, transfers, _ = context
        jumps = self.case_transfers(end, transfers, label)
        tag = field(node, "value")
        tests = []
        default = end
        next_body = None
        for case in reversed(children(node)):
            if case.type not in CASE_KINDS:
                continue
            values = field(case, "value")
            statements = [n for n in children(case) if n != values]
            body = self.sequence(statements, end, (loops, jumps, next_body))
            next_body = body
            if case.type == "default_case":
                default = body
            else:
                tests.extend((v, body) for v in reversed(operands(values)))
        check = default
        for value, body in tests:
            value_truth = truth(value)
            tag_truth = truth(tag)
            condition = value_truth if tag is None else (
                value_truth == tag_truth if None not in (value_truth, tag_truth) else None
            )
            check = self.expr(value, self.branch(condition, body, check, span(value)), context=context)
        check = self.expr(tag, check, context=context)
        return self.stmt(field(node, "initializer"), check, context)

    def select(self, node, end, context, label):
        loops, transfers, _ = context
        case_context = (loops, self.case_transfers(end, transfers, label), None)
        choices, eager = [], []
        for case in children(node):
            if case.type not in CASE_KINDS:
                continue
            communication = field(case, "communication")
            statements = [n for n in children(case) if n != communication]
            body = self.sequence(statements, end, case_context)
            if kind(communication) == "receive_statement":
                receive = field(communication, "right")
                eager.append(field(receive, "operand") or receive)
                body = self.assignments(operands(field(communication, "left")), [], body,
                                        case_context, allow_write=False)
            elif kind(communication) == "send_statement":
                eager.extend([field(communication, "channel"), field(communication, "value")])
            choices.append(body)
        # A blocked select offers no outgoing route. Only the selected case binds a target.
        choose = self.emit("select_choice", choices, span(node))
        return self.lower_list(eager, choose, lambda n, k: self.expr(n, k, context=context))
