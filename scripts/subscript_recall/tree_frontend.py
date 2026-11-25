"""Small Tree-sitter adapters using the already-installed language grammars."""

from collections import Counter, defaultdict
from dataclasses import dataclass
from functools import lru_cache

from tree_sitter import Language, Parser

from flow import INDEX
from frontend import Frontend, merge_owner_locations

CALLABLES = {
    "function_declaration", "method_declaration", "method_definition",
    "function_expression", "arrow_function", "generator_function",
    "generator_function_declaration", "func_literal",
}
JS_SCOPES = {
    "statement_block", "for_statement", "for_in_statement", "catch_clause", "switch_body",
}
GO_SCOPES = {
    "block", "if_statement", "for_statement", "type_switch_statement",
    "expression_switch_statement", "select_statement", "expression_case",
    "type_case", "communication_case", "default_case",
}


@lru_cache(maxsize=None)
def parser(lang):
    if lang == "go":
        from tree_sitter_go import language
    elif lang == "javascript":
        from tree_sitter_javascript import language
    elif lang == "typescript":
        from tree_sitter_typescript import language_typescript as language
    else:
        from tree_sitter_typescript import language_tsx as language
    return Parser(Language(language()))


def children(node):
    return [child for child in node.named_children if child.type != "comment"] if node else []


def field(node, key):
    return node.child_by_field_name(key) if node else None


def fields(node, key):
    return node.children_by_field_name(key) if node else []


def first(node):
    return next(iter(children(node)), None)


def kind(node):
    return node.type if node else ""


def span(node):
    return node.start_byte, node.end_byte


def walk(root, fence=()):
    stack = [root] if root else []
    while stack:
        node = stack.pop()
        yield node
        if node == root or node.type not in fence:
            stack.extend(reversed(children(node)))


def within(scope, point):
    return scope.start_byte <= point.start_byte and point.end_byte <= scope.end_byte


def unwrap(node):
    while kind(node) == "parenthesized_expression":
        node = first(node)
    return node


@dataclass
class Declaration:
    node: object
    scope: object
    visible: int
    tag: str
    annotation: object = None

    @property
    def identity(self):
        return self.node.start_byte


class TreeFile:
    def __init__(self, source, lang):
        self.source = source
        self.lang = lang
        self.tree = parser(lang).parse(source)
        self.owners = {}
        self.locations = {}
        used = Counter()
        self.module_types = {}
        for node in walk(self.tree.root_node, CALLABLES):
            if node.type in ("type_spec", "type_alias") and field(node, "name"):
                self.module_types[self.text(field(node, "name"))] = field(node, "type")
        for node in walk(self.tree.root_node):
            if node.type not in CALLABLES:
                continue
            name = field(node, "name")
            parent = node.parent
            if name is None and kind(parent) in ("variable_declarator", "pair"):
                name = field(parent, "name") or field(parent, "key")
            name = self.text(name) if name else "<anonymous>"
            key = f"{name}@{node.start_point[0] + 1}"
            used[key] += 1
            if used[key] > 1:
                key += f":{node.start_point[1] + 1}"
            self.owners.setdefault(key, []).append(node)

    def text(self, node):
        return self.source[node.start_byte:node.end_byte].decode()

    def compile(self, owner):
        graph, entry = self.build_owner(owner)
        merge_owner_locations(self, owner, graph,
                              lambda n: (n.start_point[0] + 1, n.end_point[0] + 1))
        return graph, entry

    def build_owner(self, owner):
        if self.lang == "go":
            from go_frontend import GoFrontend
            frontend = GoFrontend
        else:
            from js_frontend import JsFrontend
            frontend = JsFrontend
        lower = frontend(self, owner)
        return lower.graph, lower.build()


class TreeFrontend(Frontend):
    def __init__(self, file, owner):
        super().__init__(file.source)
        self.file = file
        self.owner = owner
        self.body = field(owner, "body")
        self.declarations = defaultdict(list)
        self.labels = {}

    span = staticmethod(span)

    def line(self, node):
        return node.start_point[0] + 1

    def statement_line(self, node):
        while node is not None and node != self.owner:
            if (node.type.endswith("_statement") or node.type in
                    ("short_var_declaration", "lexical_declaration", "variable_declaration",
                     "var_spec", "const_spec")):
                return self.line(node)
            node = node.parent
        return self.line(self.owner)

    def node_scope(self, node, scopes):
        node = node.parent
        while node and node != self.owner and node != self.body:
            if node.type in scopes:
                return node
            node = node.parent
        return self.owner

    def add_decl(self, node, scope, visible, tag, annotation=None):
        if node and self.text(node) != "_":
            self.declarations[self.text(node)].append(
                Declaration(node, scope, visible, tag, annotation)
            )

    def declaration(self, root):
        candidates = [d for d in self.declarations.get(self.text(root), ())
                      if within(d.scope, root)
                      and (d.visible <= root.start_byte or d.node == root)]
        if not candidates:
            return None
        return min(candidates, key=lambda d: (
            d.scope.end_byte - d.scope.start_byte, -d.visible, d.node.start_byte,
        ))

    def binding(self, root):
        if root.type in ("this", "super"):
            return None, "this_root" if root.type == "this" else "unsupported_chain"
        decl = self.declaration(root)
        if decl is not None and decl.tag != "type":
            return decl.identity, None
        if self.text(root) in self.outer_names:
            return None, "captured_root"
        return None, "module_root"

    def parts(self, node):
        links = []
        node = unwrap(node)
        while node:
            if node.type == "member_expression":
                prop = field(node, "property")
                if kind(prop) != "property_identifier":
                    return None
                links.append((node, self.text(prop), None))
                node = field(node, "object")
            elif node.type == "selector_expression":
                prop = field(node, "field")
                links.append((node, self.text(prop), None))
                node = field(node, "operand")
            elif node.type in ("subscript_expression", "index_expression"):
                index = field(node, "index")
                if index is None:
                    return None
                links.append((node, INDEX, index))
                node = field(node, "object") or field(node, "operand")
            else:
                break
            node = unwrap(node)
        if kind(node) in ("identifier", "this", "super"):
            return node, links[::-1]
        return None

    def excluded_chain(self, node):
        root, links = self.parts(node)
        return any(field(part, "optional_chain") for part, _, _ in links)

    def operator(self, node):
        operator = field(node, "operator")
        return self.text(operator) if operator else ""

    def label(self, node):
        label = field(node, "label") or first(node)
        return self.text(label) if label else None

    def jump(self, node, end, transfers, is_continue):
        name = self.label(node)
        for label, stop, resume in reversed(transfers):
            if (name == label if name else label is None) and (
                not is_continue or resume is not None
            ):
                return self.emit("continue" if is_continue else "break",
                                 (resume if is_continue else stop,), span(node),
                                 self.graph.events[resume if is_continue else stop].span)
        return self.uncertain(node)

    def collect_outer(self, collect):
        saved = self.declarations
        self.outer_names = set()
        ancestor = self.owner.parent
        while ancestor is not None:
            if ancestor.type in CALLABLES:
                self.declarations = defaultdict(list)
                collect(ancestor)
                self.outer_names.update(self.declarations)
            ancestor = ancestor.parent
        self.declarations = saved

    def sequence(self, nodes, end, context):
        return self.lower_list(nodes, end, lambda n, k: self.stmt(n, k, context))

    def static_receiver(self, node, end, context):
        # The outer lvalue is a Def or kill. Its receiver and indices are evaluated.
        return self.expr(node, end, context=context, target=True)
