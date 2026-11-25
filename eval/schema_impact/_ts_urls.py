"""Recover canonical URL paths from the static portions of expressions."""

from collections.abc import Callable
from pathlib import Path

from tree_sitter import Node

from . import _ts_syntax as ts
from .config import ClientSpec
from .ids import normalize_path


Constants = Callable[[Node], str | None]
_IGNORED_ARMS = {"null", "undefined", "false"}
_NON_STRING_KEYS = {"object", "number", "true", "false", "null", "undefined", "regex"}


def pattern(
    expr_text: str, consts: dict[str, str] | None = None, *, spec: ClientSpec | None = None,
) -> tuple[str | None, str]:
    source = ("(" + expr_text + "\n)").encode()
    tree = ts.parse(source, Path("expression.ts"))
    if tree.has_error or len(ts.children(tree)) != 1:
        return None, "dynamic"
    expression = next(iter(ts.children(ts.children(tree)[0])), None)
    constants = consts or {}
    result, url_class, _ = url_pattern(expression, lambda node: constants.get(ts.text(node)), spec, hook_key=True)
    return result, url_class


def url_pattern(
    node: Node | None, const: Constants, spec: ClientSpec | None, *, hook_key: bool,
) -> tuple[str | None, str, str | None]:
    node = ts.unwrap(node)
    if node is None:
        return None, "dynamic", "non_string_key" if hook_key else "dynamic_url"
    if node.type == "ternary_expression":
        left = ts.unwrap(node.child_by_field_name("consequence"))
        right = ts.unwrap(node.child_by_field_name("alternative"))
        for disabled, enabled in ((left, right), (right, left)):
            if ts.text(disabled) in _IGNORED_ARMS or ts.literal(disabled) == "":
                return url_pattern(enabled, const, spec, hook_key=hook_key)
        return None, "conditional", "conditional_url"
    if hook_key and node.type == "array":
        entries = [entry for entry in node.children[1:] if entry.type != "comment"]
        first = ts.unwrap(entries[0]) if entries and entries[0].is_named else None
        if first is None or (
            first.type not in {"string", "template_string"} and const(first) is None
        ):
            return None, "dynamic", "non_string_key"
        node = first
    if hook_key and node.type in _NON_STRING_KEYS:
        return None, "dynamic", "non_string_key"
    if node.type in ts.FUNCTIONS:
        return None, "dynamic", "unconfigured_client_form"

    parts = _parts(node, const)
    if parts is not None:
        path, url_class = _path(parts)
        if path is not None:
            if spec is not None:
                path = spec.add_prefix + path.removeprefix(spec.strip_prefix)
            return normalize_path(path), url_class, None
    return None, "dynamic", "dynamic_url"


def _parts(node: Node | None, const: Constants) -> list[str | None] | None:
    node = ts.unwrap(node)
    value = ts.literal(node)
    if value is not None:
        return [value]
    if node is None:
        return None
    if node.type == "identifier":
        value = const(node)
        return [value] if value is not None else None
    if node.type == "template_string" and not node.has_error:
        parts = []
        for child in ts.children(node):
            if child.type == "template_substitution":
                expr = ts.unwrap(next(iter(ts.children(child)), None))
                parts.append(const(expr) if expr is not None and expr.type == "identifier" else None)
            else:
                parts.append(ts.text(child))
        return parts
    if node.type == "binary_expression" and ts.text(node.child_by_field_name("operator")) == "+":
        left = _parts(node.child_by_field_name("left"), const)
        right = _parts(node.child_by_field_name("right"), const)
        if left is not None or right is not None:
            return (left if left is not None else [None]) + (right if right is not None else [None])
    return None


def _path(parts: list[str | None]) -> tuple[str | None, str]:
    segments: list[list[str | None]] = [[]]
    for part in parts:
        if part is None:
            segments[-1].append(None)
            continue
        path, query, _ = part.partition("?")
        first, *rest = path.split("/")
        if first:
            segments[-1].append(first)
        segments.extend([segment] if segment else [] for segment in rest)
        if query:
            break

    pattern_segments = []
    templated = False
    for index, segment in enumerate(segments):
        if None not in segment:
            pattern_segments.append("".join(segment))
        elif index > 0 and segment == [None]:
            pattern_segments.append("{}")
            templated = True
        else:
            return None, "dynamic"
    return "/".join(pattern_segments), "template" if templated else "literal"
