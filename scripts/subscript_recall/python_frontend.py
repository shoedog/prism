"""Python source lowering using the stdlib AST and UTF-8 byte columns."""

import ast
from collections import Counter

from flow import INDEX
from frontend import EXCLUDED_CALLS, Frontend, literal_boolean, merge_owner_locations

CALLABLES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)
COMPREHENSIONS = (ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)


def walk(node, fences=()):
    todo = [node]
    while todo:
        item = todo.pop()
        yield item
        if not isinstance(item, fences):
            todo.extend(reversed(list(ast.iter_child_nodes(item))))


def bindings(nodes):
    local, redirected = {}, {}
    for node in nodes:
        for part in walk(node, CALLABLES + COMPREHENSIONS + (ast.ClassDef,)):
            if isinstance(part, ast.Name) and isinstance(part.ctx, (ast.Store, ast.Del)):
                local.setdefault(part.id, part)
            elif isinstance(part, (ast.Global, ast.Nonlocal)):
                redirected.update(dict.fromkeys(part.names,
                                                 "module_root" if isinstance(part, ast.Global)
                                                 else "captured_root"))
            elif isinstance(part, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                local.setdefault(part.name, part)
            elif isinstance(part, ast.alias):
                local.setdefault(part.asname or part.name.split(".")[0], part)
            elif isinstance(part, ast.ExceptHandler) and part.name:
                local.setdefault(part.name, part)
            elif type(part).__name__ in ("MatchAs", "MatchStar", "MatchMapping"):
                name = getattr(part, "name", None) or getattr(part, "rest", None)
                if name:
                    local.setdefault(name, part)
    return local, redirected


def truth(node):
    if isinstance(node, ast.Constant):
        return bool(node.value)
    if isinstance(node, ast.BoolOp):
        return literal_boolean((truth(value) for value in node.values), isinstance(node.op, ast.And))
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
        value = truth(node.operand)
        return None if value is None else not value
    return None


class PythonFile:
    def __init__(self, source):
        self.source = source
        self.tree = ast.parse(source)
        self.starts = [3 if source.startswith(b"\xef\xbb\xbf") else 0]
        self.starts.extend(i + 1 for i, byte in enumerate(source) if byte == 10)
        self.owners = {}
        self.parents = {}
        self.locations = {}
        used = Counter()
        for node in walk(self.tree):
            for child in ast.iter_child_nodes(node):
                self.parents[child] = node
            if not isinstance(node, CALLABLES):
                continue
            name = getattr(node, "name", "<anonymous>")
            # The census counts the first token of an async function as its start.
            key = f"{name}@{node.lineno}"
            used[key] += 1
            if used[key] > 1:
                key += f":{node.col_offset + 1}"
            self.owners.setdefault(key, []).append(node)

    def compile(self, owner):
        graph, entry = self.build_owner(owner)
        merge_owner_locations(self, owner, graph, lambda n: (n.lineno, n.end_lineno))
        return graph, entry

    def build_owner(self, owner):
        lower = PythonFrontend(self, owner)
        return lower.graph, lower.build()


class PythonFrontend(Frontend):
    def __init__(self, parsed, owner):
        super().__init__(parsed.source)
        self.parsed = parsed
        self.owner = owner
        nodes = owner.body if not isinstance(owner, ast.Lambda) else [owner.body]
        self.locals, self.redirected = bindings(nodes)
        params = owner.args
        arguments = params.posonlyargs + params.args + params.kwonlyargs
        arguments += [a for a in (params.vararg, params.kwarg) if a]
        self.locals.update((a.arg, a) for a in arguments)
        self.outer = set()
        ancestor = parsed.parents.get(owner)
        while ancestor is not None:
            if isinstance(ancestor, CALLABLES):
                body = ancestor.body if not isinstance(ancestor, ast.Lambda) else [ancestor.body]
                self.outer.update(bindings(body)[0])
                self.outer.update(arg.arg for arg in walk(ancestor.args) if isinstance(arg, ast.arg))
            ancestor = parsed.parents.get(ancestor)

    def span(self, node):
        starts = self.parsed.starts
        return (starts[node.lineno - 1] + node.col_offset,
                starts[node.end_lineno - 1] + node.end_col_offset)

    def line(self, node):
        return node.lineno

    def statement_line(self, node):
        current = node
        while current is not None and current is not self.owner:
            if isinstance(current, ast.stmt):
                return current.lineno
            current = self.parsed.parents.get(current)
        return self.line(node)

    def binding(self, root):
        name = root.id
        if name in self.redirected:
            return None, self.redirected[name]
        decl = self.locals.get(name)
        if decl is not None:
            return name, None
        return None, "captured_root" if name in self.outer else "module_root"

    def parts(self, node):
        links = []
        while isinstance(node, (ast.Attribute, ast.Subscript)):
            if isinstance(node, ast.Attribute):
                links.append((node, node.attr, None))
            else:
                links.append((node, INDEX, node.slice))
            node = node.value
        if isinstance(node, ast.Name):
            return node, links[::-1]
        return None

    def excluded_chain(self, node):
        _, links = self.parts(node)
        return any(isinstance(index, (ast.Slice, ast.Tuple, ast.Starred))
                   for _, _, index in links)

    def build(self):
        end = self.emit("owner_exit")
        if isinstance(self.owner, ast.Lambda):
            return self.expr(self.owner.body, end, "return")
        return self.suite(self.owner.body, end, ())

    def targets(self, node, end, loops):
        if isinstance(node, (ast.Tuple, ast.List)):
            return self.lower_list(node.elts, end, lambda n, k: self.targets(n, k, loops))
        if isinstance(node, ast.Starred):
            return self.targets(node.value, end, loops)
        self.location(node, "def")
        return self.expr(node, self.mutation(node, end), loops=loops, target=True)

    def expr(self, node, end, role=None, loops=(), target=False):
        if node is None or isinstance(node, CALLABLES):
            return end
        if isinstance(node, COMPREHENSIONS):
            # Only the leftmost iterable is evaluated outside the nested scope.
            return self.expr(node.generators[0].iter, self.uncertain(node, end), loops=loops)
        if isinstance(node, (ast.Name, ast.Attribute)) and not target:
            self.location(node, "use")
        end = self.read(node, role, end) if role else end
        sub = lambda n, k: self.expr(n, k, loops=loops)
        if isinstance(node, ast.Call):
            callee = self.text(node.func)
            if (isinstance(node.func, ast.Attribute) and node.func.attr == "clear"
                    and not node.args and not node.keywords and self.path(node.func.value)):
                end = self.mutation(node.func.value, end, "clear")
            arg_role = None if callee in EXCLUDED_CALLS else "arg"
            # Python binds all positional arguments (including stars) before keywords.
            args = node.args + [kw.value for kw in node.keywords]
            for arg in reversed(args):
                value_role = arg_role if not isinstance(arg, ast.Starred) and (
                    arg in node.args or any(kw.value is arg and kw.arg is not None
                                            for kw in node.keywords)
                ) else None
                end = self.expr(arg, end, value_role, loops)
            return sub(node.func, end)
        if isinstance(node, ast.NamedExpr):
            self.location(node.target, "def")
            return self.expr(node.value, self.mutation(node.target, end), "assign_rhs", loops)
        if isinstance(node, ast.IfExp):
            return sub(node.test, self.branch(truth(node.test), sub(node.body, end),
                                              sub(node.orelse, end), self.span(node)))
        if isinstance(node, ast.BoolOp):
            cursor = sub(node.values[-1], end)
            for value in reversed(node.values[:-1]):
                yes, no = (cursor, end) if isinstance(node.op, ast.And) else (end, cursor)
                cursor = sub(value, self.branch(truth(value), yes, no, self.span(value)))
            return cursor
        if isinstance(node, ast.Compare):
            cursor = sub(node.comparators[-1], end)
            for comparator in reversed(node.comparators[:-1]):
                cursor = sub(comparator, self.branch(None, cursor, end, self.span(comparator)))
            return sub(node.left, cursor)
        if isinstance(node, ast.Dict):
            operands = [n for pair in zip(node.keys, node.values) for n in pair if n is not None]
            return self.lower_list(operands, end, sub)
        if isinstance(node, (ast.Yield, ast.YieldFrom, ast.Await)):
            return sub(node.value, self.uncertain(node, end))
        return self.lower_list(ast.iter_child_nodes(node), end, sub)

    def suite(self, nodes, end, loops):
        return self.lower_list(nodes, end, lambda n, k: self.stmt(n, k, loops))

    def stmt(self, node, end, loops):
        expr = lambda n, k, role=None: self.expr(n, k, role, loops)
        suite = lambda nodes, k: self.suite(nodes, k, loops)
        if isinstance(node, CALLABLES):
            # The nested body is separate. Defaults and decorators execute in the outer scope.
            end = self.emit("replace", (end,), self.span(node), (node.name,))
            values = [*node.decorator_list, *node.args.defaults,
                      *(default for default in node.args.kw_defaults if default is not None)]
            return self.lower_list(values, end, expr)
        if isinstance(node, ast.Expr):
            return expr(node.value, end)
        if isinstance(node, ast.Return):
            return expr(node.value, self.emit("return", (), self.span(node)), "return")
        if isinstance(node, ast.Raise):
            halt = self.emit("raise", (), self.span(node))
            return expr(node.exc, expr(node.cause, halt))
        if isinstance(node, ast.If):
            return expr(node.test, self.branch(truth(node.test), suite(node.body, end),
                                               suite(node.orelse, end), self.span(node)))
        if isinstance(node, (ast.While, ast.For, ast.AsyncFor)):
            header = self.emit("loop_test", (), self.span(node))
            identity = self.span(node)
            body_loops = (*loops, (identity, end, header))
            body = self.suite(node.body, header, body_loops)
            exhausted = suite(node.orelse, end)
            if isinstance(node, ast.While):
                enter = self.emit("loop_entry", (body,), identity, identity)
                next_phase = expr(node.test, self.branch(truth(node.test), enter, exhausted, identity))
                self.graph.events[header].successors = (next_phase,)
                return header
            bind = self.targets(node.target, body, body_loops)
            enter = self.emit("loop_entry", (bind,), identity, identity)
            next_phase = self.branch(None, enter, exhausted, identity)
            if isinstance(node, ast.AsyncFor):
                next_phase = self.uncertain(node, next_phase)
            self.graph.events[header].successors = (next_phase,)
            return expr(node.iter, header)
        if isinstance(node, (ast.Break, ast.Continue)):
            if not loops:
                return self.uncertain(node, end)
            identity, stop, resume = loops[-1]
            return self.emit(type(node).__name__.lower(),
                             (stop if isinstance(node, ast.Break) else resume,),
                             self.span(node), identity)
        if isinstance(node, (ast.Assign, ast.AnnAssign, ast.AugAssign)):
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            if node.value is None:
                return end
            compound = isinstance(node, ast.AugAssign)
            if len(targets) == 1:
                start = self.write(targets[0], expr,
                                   lambda k: expr(node.value, k, None if compound else "assign_rhs"),
                                   end, tuple(identity for identity, _, _ in loops),
                                   compound=compound, rhs_first=not compound)
                if start is not None:
                    return start
            after = self.lower_list(targets, end, lambda n, k: self.targets(n, k, loops))
            if compound:
                return self.expr(targets[0], expr(node.value, self.mutation(targets[0], end)),
                                 loops=loops, target=True)
            unpacking = any(isinstance(t, (ast.Tuple, ast.List)) for t in targets)
            return expr(node.value, after, None if unpacking else "assign_rhs")
        if isinstance(node, ast.Delete):
            def delete(target, cursor):
                if isinstance(target, (ast.Tuple, ast.List)):
                    return self.lower_list(target.elts, cursor, delete)
                kind = "delete" if isinstance(target, ast.Subscript) else "replace"
                return self.expr(target, self.mutation(target, cursor, kind), loops=loops, target=True)
            return self.lower_list(node.targets, end, delete)
        if isinstance(node, (ast.Pass, ast.Global, ast.Nonlocal)):
            return end
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            for alias in reversed(node.names):
                name = alias.asname or alias.name.split(".")[0]
                if name in self.locals:
                    end = self.emit("replace", (end,),
                                    self.span(alias if hasattr(alias, "lineno") else node), (name,))
            return end
        if isinstance(node, ast.Assert):
            failure = expr(node.msg, self.emit("raise", (), self.span(node)))
            return expr(node.test, self.branch(truth(node.test), end, failure, self.span(node)))
        if isinstance(node, ast.Try) or type(node).__name__ == "TryStar":
            blocked = self.block_jumps(node, loops)
            suite = lambda nodes, k: self.suite(nodes, k, blocked)
            exit_phase = self.uncertain(node, end)
            final = self.uncertain(node, suite(node.finalbody, exit_phase))
            normal = suite(node.body, suite(node.orelse, final))
            if not node.handlers:
                return normal
            handlers = [suite(handler.body, final) for handler in node.handlers]
            return self.branch(None, normal, self.uncertain(node, *handlers), self.span(node))
        if isinstance(node, (ast.With, ast.AsyncWith)):
            tail = self.suite(node.body, self.uncertain(node, end), self.block_jumps(node, loops))
            for manager in reversed(node.items):
                if manager.optional_vars:
                    tail = self.targets(manager.optional_vars, tail, loops)
                tail = expr(manager.context_expr, tail)
            return self.uncertain(node, tail) if isinstance(node, ast.AsyncWith) else tail
        if type(node).__name__ == "Match":
            arms = [expr(case.guard, suite(case.body, end)) for case in node.cases]
            return expr(node.subject, self.uncertain(node, *arms, end))
        # Unlisted statements or scope transitions have no assumed completion.
        return self.uncertain(node, end)

    def block_jumps(self, node, loops):
        return tuple((identity, self.uncertain(node, stop), self.uncertain(node, resume))
                     for identity, stop, resume in loops)
