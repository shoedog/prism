"""Track value bindings, lexical shadowing and writes in one source file."""

from dataclasses import dataclass, field

from tree_sitter import Node

from . import _ts_syntax as ts
from ._ts_imports import Import, imports


_BLOCKS = {"statement_block", "for_statement", "for_in_statement", "switch_body", "catch_clause", "class_body"}
_DECLARATIONS = {"class_declaration", "enum_declaration", "abstract_class_declaration"}
_ASSIGNMENTS = {"assignment_expression", "augmented_assignment_expression", "update_expression"}
_BINDING_PATTERNS = {"array_pattern", "object_pattern", "rest_pattern", "formal_parameters"}


@dataclass
class Binding:
    imported: Import | None = None
    initializer: Node | None = None
    constant: bool = False
    written: bool = False


@dataclass
class Scope:
    parent: "Scope | None"
    function: Node | None = None
    names: dict[str, Binding] = field(default_factory=dict)

    def find(self, name: str) -> Binding | None:
        scope = self
        while scope is not None:
            if name in scope.names:
                return scope.names[name]
            scope = scope.parent
        return None


def _identifiers(node: Node | None):
    if node is None:
        return
    if node.type in {"identifier", "type_identifier", "shorthand_property_identifier_pattern"}:
        yield node
    elif node.type in _BINDING_PATTERNS:
        for child in ts.children(node):
            yield from _identifiers(child)
    elif node.type in {"pair_pattern", "object_assignment_pattern", "assignment_pattern",
                       "required_parameter", "optional_parameter"}:
        child = (node.child_by_field_name("value") if node.type == "pair_pattern"
                 else node.child_by_field_name("pattern") or node.child_by_field_name("left"))
        yield from _identifiers(child)


def _names(node: Node | None):
    return map(ts.text, _identifiers(node))


def _function_scope(scope: Scope) -> Scope:
    while scope.function is None and scope.parent is not None:
        scope = scope.parent
    return scope


class Bindings:
    def __init__(self, tree: Node):
        module = Scope(None)
        self.scopes: dict[Node, Scope] = {}
        self.declarations: set[Node] = set()
        self._visit(tree, module)
        for node, scope in self.scopes.items():
            if node.type in _ASSIGNMENTS or (
                node.type == "for_in_statement" and node.child_by_field_name("kind") is None
            ):
                target = node.child_by_field_name("left") or node.child_by_field_name("argument")
                for name in _names(ts.unwrap(target)):
                    binding = scope.find(name)
                    if binding is not None:
                        binding.written = True

    def _visit(self, node: Node, scope: Scope) -> None:
        kind = node.type
        name = node.child_by_field_name("name")
        if kind in ts.FUNCTIONS:
            if name is not None:
                self.declarations.add(name)
            if kind in {"function_declaration", "generator_function_declaration"} and name is not None:
                scope.names[ts.text(name)] = Binding()
            scope = Scope(scope, node)
            if kind in {"function_expression", "generator_function"} and name is not None:
                scope.names[ts.text(name)] = Binding()
            params = node.child_by_field_name("parameters") or node.child_by_field_name("parameter")
            self.declarations.update(_identifiers(params))
            for local in _names(params):
                scope.names[local] = Binding()
        elif kind in _BLOCKS:
            scope = Scope(scope)
            if kind == "catch_clause":
                self.declarations.update(_identifiers(node.child_by_field_name("parameter")))
                for local in _names(node.child_by_field_name("parameter")):
                    scope.names[local] = Binding()
            if kind == "for_in_statement":
                declaration = node.child_by_field_name("kind")
                if declaration is not None:
                    self.declarations.update(_identifiers(node.child_by_field_name("left")))
                    target_scope = _function_scope(scope) if declaration.type == "var" else scope
                    for local in _names(node.child_by_field_name("left")):
                        target_scope.names[local] = Binding()

        self.scopes[node] = scope
        if kind == "import_statement":
            for local, imported in imports(node):
                scope.names[local] = Binding(imported=imported)
        elif kind == "variable_declarator":
            self.declarations.update(_identifiers(name))
            keyword = node.parent.children[0].type
            target_scope = _function_scope(scope) if keyword == "var" else scope
            for local in _names(name):
                initializer = node.child_by_field_name("value") if name.type == "identifier" else None
                target_scope.names[local] = Binding(initializer=initializer, constant=keyword == "const")
        elif kind in _DECLARATIONS and name is not None:
            self.declarations.add(name)
            scope.names[ts.text(name)] = Binding()

        for child in ts.children(node):
            self._visit(child, scope)

    def find(self, node: Node) -> Binding | None:
        return self.scopes[node].find(ts.text(node))

    def reference(self, node: Node) -> bool:
        if node.type not in {"identifier", "shorthand_property_identifier"} or node in self.declarations:
            return False
        parent = node.parent
        while parent.type not in {"program", "statement_block"}:
            if parent.type in {"import_statement", "type_annotation", "type_query", "type_arguments"}:
                return False
            if parent.type == "export_specifier":
                return node == parent.child_by_field_name("name") and parent.parent.parent.child_by_field_name("source") is None
            parent = parent.parent
        return True

    def constant(self, node: Node) -> str | None:
        if node.type != "identifier":
            return None
        binding = self.find(node)
        if binding is not None and binding.constant and not binding.written:
            initializer = binding.initializer
            if initializer is not None:
                defined_in = _function_scope(self.scopes[initializer]).function
                if defined_in is None or defined_in == _function_scope(self.scopes[node]).function:
                    return ts.literal(initializer)
        return None

    def enclosing_function(self, node: Node) -> str | None:
        function = _function_scope(self.scopes[node]).function
        if function is None:
            return None
        name = function.child_by_field_name("name")
        if name is None and function.parent.type == "variable_declarator":
            name = function.parent.child_by_field_name("name")
        return ts.text(name) or None
