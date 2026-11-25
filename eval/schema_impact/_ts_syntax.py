"""Syntax shared by the consumer adapter and its URL pattern parser."""

from functools import lru_cache
from pathlib import Path

from tree_sitter import Language, Node, Parser
from tree_sitter_javascript import language as javascript
from tree_sitter_typescript import language_tsx, language_typescript


EXTENSIONS = (".ts", ".tsx", ".js", ".jsx")
FUNCTIONS = {
    "arrow_function", "function_declaration", "function_expression", "generator_function",
    "generator_function_declaration", "method_definition",
}


@lru_cache(maxsize=None)
def _language(extension: str) -> Language:
    capsule = {".ts": language_typescript, ".tsx": language_tsx}.get(extension, javascript)
    return Language(capsule())


def parse(source: bytes, path: Path) -> Node:
    return Parser(_language(path.suffix)).parse(source).root_node


def text(node: Node | None) -> str:
    return node.text.decode("utf-8") if node is not None else ""


def children(node: Node | None) -> list[Node]:
    return [child for child in node.named_children if child.type != "comment"] if node is not None else []


def walk(node: Node):
    pending = [node]
    while pending:
        current = pending.pop()
        yield current
        pending.extend(reversed(children(current)))


def unwrap(node: Node | None) -> Node | None:
    while node is not None:
        if node.type in {"parenthesized_expression", "as_expression", "satisfies_expression",
                         "non_null_expression", "await_expression"}:
            node = next(iter(children(node)), None)
        elif node.type == "type_assertion":
            node = children(node)[-1]
        else:
            break
    return node


def literal(node: Node | None) -> str | None:
    node = unwrap(node)
    if node is None or node.has_error:
        return None
    if node.type == "string" or (
        node.type == "template_string" and all(c.type != "template_substitution" for c in children(node))
    ):
        return text(node)[1:-1]
    return None


def member_chain(node: Node | None) -> tuple[Node | None, tuple[str | None, ...]]:
    members = []
    node = unwrap(node)
    while node is not None and node.type in {"member_expression", "subscript_expression"}:
        prop = node.child_by_field_name("property")
        members.append(text(prop) if prop is not None else literal(node.child_by_field_name("index")))
        node = unwrap(node.child_by_field_name("object"))
    return node, tuple(reversed(members))


def argument(arguments: list[Node], index: int) -> Node | None:
    if 0 <= index < len(arguments) and not any(arg.type == "spread_element" for arg in arguments[:index + 1]):
        return arguments[index]
    return None


def property_value(node: Node | None, key: str) -> tuple[Node | None, bool]:
    """Return the last property value and whether presence or absence is statically known."""
    node = unwrap(node)
    if node is None or node.type != "object":
        return None, False
    for entry in reversed(children(node)):
        if entry.type == "spread_element":
            return None, False
        if entry.type == "shorthand_property_identifier":
            if text(entry) == key:
                return entry, True
            continue
        name = entry.child_by_field_name("key") or entry.child_by_field_name("name")
        if name is None:
            return None, False
        if name.type == "computed_property_name":
            property_name = literal(next(iter(children(name)), None))
            if property_name is None:
                return None, False
        else:
            property_name = literal(name) if name.type == "string" else text(name)
        if property_name == key:
            return entry.child_by_field_name("value") or entry, True
    return None, True
