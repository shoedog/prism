"""Extract literal JavaScript registration tables and statically named handlers."""

import argparse
from dataclasses import asdict, dataclass, field
import json
from pathlib import Path
from posixpath import normpath
import re
from typing import Iterator, NotRequired, TypedDict

from tree_sitter import Language, Node, Parser
import tree_sitter_javascript

from .ids import normalize_path


@dataclass(frozen=True)
class RouteSpec:
    callee: str
    method_key: str
    path_key: str
    handler_key: str
    validation_key: str
    mount: str = ""
    handler_under: tuple[str, ...] = ("", "config")
    validation_under: tuple[str, ...] = ("", "config")


@dataclass(frozen=True)
class HandlerSite:
    file: str
    line: int
    name: str


@dataclass(frozen=True)
class Route:
    method: str
    path: str
    handler_text: str | None
    handler_site: HandlerSite | None
    has_validation: bool
    file: str
    line: int
    optional_param: bool = False
    validation_unknown: bool = False


class _Unextracted(TypedDict):
    count: int
    method: NotRequired[str]
    reason: NotRequired[str]
    locations: NotRequired[list[dict[str, str | int]]]


@dataclass
class Extraction:
    routes: list[Route]
    unextracted: list[_Unextracted]
    unparsed: list[str]
    handler_inline: int = 0
    entries: list[dict] = field(default_factory=list)
    outside_registration: list[dict] = field(default_factory=list)


_EXTENSIONS = (".js", ".mjs", ".cjs", ".jsx")
_OPTIONAL_PARAM = re.compile(r"\{[^/{}?]+\?\}")
_ESCAPE = re.compile(
    r"\\(\r\n|x[0-9a-fA-F]{2}|u[0-9a-fA-F]{4}|u\{[0-9a-fA-F]+\}"
    r"|[0-3][0-7]{1,2}|[4-7][0-7]|[\s\S])"
)
_SIMPLE_ESCAPES = {
    "b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t", "v": "\v",
    "\r\n": "", "\r": "", "\n": "", "\u2028": "", "\u2029": "",
}
_FUNCTION_DECLARATIONS = {"function_declaration", "generator_function_declaration"}
_FUNCTIONS = _FUNCTION_DECLARATIONS | {
    "function_expression", "generator_function", "arrow_function", "method_definition",
}
_CLASSES = {"class", "class_declaration"}
_REFERENCES = {
    "identifier", "member_expression", "subscript_expression", "shorthand_property_identifier",
}
_SCOPES = _FUNCTIONS | {
    "program", "statement_block", "catch_clause", "for_statement",
    "for_in_statement", "switch_body",
}


def _children(node: Node) -> Iterator[Node]:
    return (child for child in node.named_children if child.type != "comment")


def _walk(root: Node) -> Iterator[Node]:
    stack = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(list(_children(node))))


def _unwrap(node: Node | None) -> Node | None:
    while node is not None and node.type == "parenthesized_expression":
        node = next(_children(node), None)
    return node


def _text(node: Node) -> str:
    return node.text.decode("utf-8")


def _unescape(match: re.Match[str]) -> str:
    escape = match[1]
    if len(escape) > 1 and escape[0] in "xu":
        digits = escape[1:]
        if escape.startswith("u{"):
            digits = escape[2:-1].lstrip("0") or "0"
        if len(digits) > 6:
            raise ValueError("invalid JavaScript code point")
        return chr(int(digits, 16))
    if escape[0] in "01234567":
        return chr(int(escape, 8))
    return _SIMPLE_ESCAPES.get(escape, escape)


def _string(node: Node | None) -> str | None:
    node = _unwrap(node)
    if node is None or node.type != "string":
        return None
    return _decode_string(_text(node)[1:-1])


def _decode_string(source: str) -> str | None:
    try:
        value = _ESCAPE.sub(_unescape, source)
        return value.encode("utf-16-le", "surrogatepass").decode("utf-16-le", "surrogatepass")
    except ValueError:
        return None


def _key(node: Node | None) -> str | None:
    if node is not None and node.type in {
        "identifier", "property_identifier", "shorthand_property_identifier",
        "shorthand_property_identifier_pattern",
    }:
        return _text(node)
    return _string(node)


def _property(obj: Node | None, name: str) -> Node | None:
    obj = _unwrap(obj)
    if obj is None or obj.type != "object":
        return None
    for item in reversed(list(_children(obj))):
        if item.type == "pair" and _key(item.child_by_field_name("key")) == name:
            return item.child_by_field_name("value")
        if item.type == "method_definition" and _key(item.child_by_field_name("name")) == name:
            return item
        if item.type == "shorthand_property_identifier" and _text(item) == name:
            return item
    return None


def _property_sources(obj: Node, containers: tuple[str, ...]) -> Iterator[Node]:
    for container in containers:
        source = _unwrap(_property(obj, container)) if container else obj
        if source is not None and source.type == "object":
            yield source


def _nested_property(obj: Node, name: str, containers: tuple[str, ...]) -> Node | None:
    for source in _property_sources(obj, containers):
        value = _property(source, name)
        if value is not None:
            return value
    return None


def _validation(obj: Node, spec: RouteSpec) -> tuple[bool, bool]:
    unknown = False
    for source in _property_sources(obj, spec.validation_under):
        if _property(source, spec.validation_key) is not None:
            return True, False
        unknown = unknown or any(child.type == "spread_element" for child in _children(source))
    return False, unknown


def _member_key(node: Node) -> str | None:
    if node.type == "member_expression":
        return _key(node.child_by_field_name("property"))
    if node.type == "subscript_expression":
        return _string(node.child_by_field_name("index"))
    return None


def _member_value(owner: Node, name: str) -> Node | None:
    if owner.type not in _CLASSES:
        return _property(owner, name)
    body = owner.child_by_field_name("body")
    if body is None:
        return None
    for member in reversed(list(_children(body))):
        if member.type == "method_definition" and _key(member.child_by_field_name("name")) == name:
            if any(child.type == "static" for child in member.children):
                return member
    return None


@dataclass(frozen=True)
class _Import:
    source: str
    name: str
    require: Node | None = None


_Binding = Node | _Import | None


@dataclass
class _Unit:
    file: str
    root: Node
    bindings: dict[Node, dict[str, _Binding]] = field(default_factory=dict)
    esm: dict[str, Node] = field(default_factory=dict)
    cjs: dict[str, Node] = field(default_factory=dict)
    cjs_object: Node | None = None

    def bind(self, scope: Node, name: Node, value: _Binding) -> None:
        bindings = self.bindings.setdefault(scope, {})
        key = _text(name)
        bindings[key] = None if key in bindings else value


def _binding_names(pattern: Node) -> Iterator[Node]:
    stack = [pattern]
    while stack:
        node = stack.pop()
        if node.type in {"identifier", "shorthand_property_identifier_pattern"}:
            yield node
        elif node.type in {"array_pattern", "object_pattern", "rest_pattern", "formal_parameters"}:
            stack.extend(reversed(list(_children(node))))
        elif node.type in {"pair_pattern", "assignment_pattern", "object_assignment_pattern"}:
            child = node.child_by_field_name("value" if node.type == "pair_pattern" else "left")
            if child is not None:
                stack.append(child)


def _scope(node: Node, kinds: set[str]) -> Node:
    parent = node.parent
    while parent is not None:
        if parent.type in kinds:
            return parent
        parent = parent.parent
    raise ValueError("declaration outside a JavaScript scope")


def _lookup(unit: _Unit, node: Node, name: str) -> tuple[bool, _Binding]:
    scope = node.parent
    while scope is not None:
        bindings = unit.bindings.get(scope)
        if bindings is not None and name in bindings:
            return True, bindings[name]
        scope = scope.parent
    return False, None


def _imports(unit: _Unit, statement: Node) -> None:
    source = _string(statement.child_by_field_name("source"))
    for node in _walk(statement):
        if node.type == "import_specifier":
            exported = node.child_by_field_name("name")
            local = node.child_by_field_name("alias") or exported
            name = _key(exported)
            if local is not None:
                binding = _Import(source, name) if source is not None and name is not None else None
                unit.bind(unit.root, local, binding)
        elif node.type in {"import_clause", "namespace_import"}:
            # Unsupported imports still introduce local bindings.
            for child in _children(node):
                if child.type == "identifier":
                    unit.bind(unit.root, child, None)


def _require(value: Node | None) -> tuple[str, Node] | None:
    if value is None or value.type != "call_expression":
        return None
    callee = value.child_by_field_name("function")
    args = value.child_by_field_name("arguments")
    if callee is None or callee.type != "identifier" or _text(callee) != "require" or args is None:
        return None
    arguments = list(_children(args))
    source = _string(arguments[0]) if len(arguments) == 1 else None
    return (source, callee) if source is not None else None


def _require_bindings(pattern: Node, value: Node | None) -> dict[Node, _Import]:
    required = _require(value) if pattern.type == "object_pattern" else None
    if required is None:
        return {}
    source, callee = required
    bindings = {}
    for item in _children(pattern):
        if item.type == "pair_pattern":
            name = _key(item.child_by_field_name("key"))
            local = item.child_by_field_name("value")
        elif item.type == "shorthand_property_identifier_pattern":
            name, local = _text(item), item
        else:
            continue
        if name is not None and local is not None and local.type in {
            "identifier", "shorthand_property_identifier_pattern",
        }:
            bindings[local] = _Import(source, name, callee)
    return bindings


def _bind_variable(unit: _Unit, declaration: Node) -> None:
    pattern = declaration.child_by_field_name("name")
    if pattern is None:
        return
    kinds = _FUNCTIONS | {"program"} if declaration.parent.type == "variable_declaration" else _SCOPES
    scope = _scope(declaration, kinds)
    value = _unwrap(declaration.child_by_field_name("value"))
    if pattern.type == "identifier":
        unit.bind(scope, pattern, value)
        return
    imported = _require_bindings(pattern, value)
    for name in _binding_names(pattern):
        unit.bind(scope, name, imported.get(name))


def _index_bindings(unit: _Unit) -> None:
    for node in _walk(unit.root):
        if node.type == "import_statement":
            _imports(unit, node)
        if node.type in _FUNCTION_DECLARATIONS | {"class_declaration"}:
            name = node.child_by_field_name("name")
            if name is not None:
                unit.bind(_scope(node, _SCOPES), name, node)
        if node.type in _FUNCTIONS | {"catch_clause"}:
            params = node.child_by_field_name("parameters") or node.child_by_field_name("parameter")
            if params is not None:
                for name in _binding_names(params):
                    unit.bind(node, name, None)
            if node.type in {"function_expression", "generator_function"}:
                name = node.child_by_field_name("name")
                if name is not None:
                    unit.bind(node, name, node)
        if node.type == "variable_declarator":
            _bind_variable(unit, node)


def _index_esm(unit: _Unit, statement: Node) -> None:
    if statement.child_by_field_name("source") is not None:
        return
    if any(child.type == "default" for child in statement.children):
        return
    declaration = statement.child_by_field_name("declaration")
    if declaration is not None:
        if declaration.type in _FUNCTION_DECLARATIONS | _CLASSES:
            name = declaration.child_by_field_name("name")
            if name is not None:
                unit.esm[_text(name)] = declaration
        elif declaration.type in {"lexical_declaration", "variable_declaration"}:
            for item in _children(declaration):
                pattern = item.child_by_field_name("name")
                if pattern is not None:
                    for binding in _binding_names(pattern):
                        unit.esm[_text(binding)] = binding
    for node in _children(statement):
        if node.type == "export_clause":
            for item in _children(node):
                name = item.child_by_field_name("name")
                exported = _key(item.child_by_field_name("alias") or name)
                if name is not None and exported is not None:
                    unit.esm[exported] = name


def _identifier_is(node: Node | None, name: str) -> bool:
    return node is not None and node.type == "identifier" and _text(node) == name


def _module_exports(node: Node | None) -> bool:
    return (
        node is not None and _member_key(node) == "exports"
        and _identifier_is(node.child_by_field_name("object"), "module")
    )


def _index_exports(unit: _Unit) -> None:
    exports_alias = True
    for statement in _children(unit.root):
        if statement.type == "export_statement":
            _index_esm(unit, statement)
        if statement.type != "expression_statement":
            continue
        assignment = next(_children(statement), None)
        if assignment is None or assignment.type != "assignment_expression":
            continue
        left = assignment.child_by_field_name("left")
        right = _unwrap(assignment.child_by_field_name("right"))
        if left is None or right is None:
            continue
        if _module_exports(left):
            unit.cjs_object = right
            unit.cjs.clear()
            exports_alias = False
        elif _identifier_is(left, "exports"):
            exports_alias = False
        else:
            name = _member_key(left)
            owner = left.child_by_field_name("object")
            if name is not None and (
                _module_exports(owner) or (exports_alias and _identifier_is(owner, "exports"))
            ):
                unit.cjs[name] = right


@dataclass(frozen=True)
class _Value:
    unit: _Unit
    node: Node
    name: str


def _imported_unit(units: dict[str, _Unit], unit: _Unit, source: str) -> _Unit | None:
    if not source.startswith(("./", "../")):
        return None
    path = normpath(f"{Path(unit.file).parent.as_posix()}/{source}")
    candidates = (
        path,
        *(path + ext for ext in _EXTENSIONS),
        *(f"{path}/index{ext}" for ext in _EXTENSIONS),
    )
    return next((units[file] for file in candidates if file in units), None)


def _resolve(
    units: dict[str, _Unit], unit: _Unit, node: Node | None, name: str = "",
    active: frozenset[Node] = frozenset(),
) -> _Value | None:
    node = _unwrap(node)
    if node is None or node in active or len(active) >= 64:
        return None
    active = active | {node}
    if node.type in _FUNCTIONS | _CLASSES | {"object"}:
        if node.type == "method_definition" and any(
            child.type in {"get", "set"} for child in node.children
        ):
            return None
        return _Value(unit, node, _key(node.child_by_field_name("name")) or name)
    if node.type in {"identifier", "shorthand_property_identifier", "shorthand_property_identifier_pattern"}:
        name = _text(node)
        _, binding = _lookup(unit, node, name)
        if not isinstance(binding, _Import):
            return _resolve(units, unit, binding, name, active)
        imported = _imported_unit(units, unit, binding.source)
        if imported is None:
            return None
        if binding.require is None:
            target = imported.esm.get(binding.name)
        else:
            if _lookup(unit, binding.require, "require")[0]:
                return None
            target = imported.cjs.get(binding.name)
            if target is None:
                obj = _resolve(units, imported, imported.cjs_object, active=active)
                if obj is not None:
                    target = _member_value(obj.node, binding.name)
                    imported = obj.unit
        return _resolve(units, imported, target, binding.name, active)
    member = _member_key(node)
    if member is not None:
        owner = _resolve(units, unit, node.child_by_field_name("object"), active=active)
        if owner is not None:
            target = _member_value(owner.node, member)
            return _resolve(units, owner.unit, target, member, active)
    return None


def _handler_site(units: dict[str, _Unit], unit: _Unit, handler: Node | None) -> HandlerSite | None:
    expression = _unwrap(handler)
    if expression is None or expression.type not in _REFERENCES:
        return None
    value = _resolve(units, unit, expression)
    if value is None or value.node.type not in _FUNCTIONS:
        return None
    return HandlerSite(value.unit.file, value.node.start_point.row + 1, value.name)


def _entries(root: Node, registration: str) -> Iterator[tuple[Node, str | None]]:
    for call in _walk(root):
        if call.type != "call_expression":
            continue
        callee = _unwrap(call.child_by_field_name("function"))
        if callee is not None and callee.type == "member_expression":
            callee = callee.child_by_field_name("property")
        if callee is None or callee.type not in {"identifier", "property_identifier"}:
            continue
        if _text(callee) != registration:
            continue
        args = call.child_by_field_name("arguments")
        arrays = [_unwrap(arg) for arg in _children(args)] if args is not None else []
        arrays = [arg for arg in arrays if arg is not None and arg.type == "array"]
        if not arrays:
            yield call, "registration call has no literal array argument"
        for array in arrays:
            tokens = [child for child in array.children[1:-1] if child.type != "comment"]
            for item in tokens[::2]:
                entry = _unwrap(item)
                if entry is not None and entry.type == "object":
                    yield entry, None
                else:
                    yield item, "registration item is not an object declaration"


def _methods(node: Node | None) -> list[str]:
    node = _unwrap(node)
    if node is None:
        return []
    if node.type == "array":
        tokens = [child for child in node.children[1:-1] if child.type != "comment"]
        items = tokens[::2]  # Elements alternate with separators; a comma here denotes a hole.
        if any(item.type == "," for item in items):
            return []
    else:
        items = [node]
    methods = [_string(item) for item in items]
    if any(method is None for method in methods):
        return []
    return [method.upper() for method in methods if method is not None]


def extract(repo: str | Path, spec: RouteSpec) -> Extraction:
    """Return routes and diagnostics in repository path and source order."""
    root = Path(repo).resolve(strict=True)
    if not root.is_dir():
        raise NotADirectoryError(root)
    parser = Parser(Language(tree_sitter_javascript.language()))
    units = {}
    unparsed = []
    for path in sorted(root.rglob("*"), key=lambda path: path.as_posix()):
        if path.suffix not in _EXTENSIONS or path.is_symlink() or not path.is_file():
            continue
        file = path.relative_to(root).as_posix()
        try:
            source = path.read_bytes()
            source.decode("utf-8")
        except (OSError, UnicodeError):
            unparsed.append(file)
            continue
        tree = parser.parse(source)
        if tree.root_node.has_error:
            unparsed.append(file)
            continue
        unit = units[file] = _Unit(file, tree.root_node)
        _index_bindings(unit)
        _index_exports(unit)

    rows = []
    entries = []
    outside = []
    template_locations: list[dict[str, str | int]] = []
    handler_inline = 0
    for unit in units.values():
        registrations = list(_entries(unit.root, spec.callee))
        accepted = {node for node, _ in registrations}
        for node in _walk(unit.root):
            if (node.type == "object" and node not in accepted
                    and _property(node, spec.method_key) is not None
                    and _property(node, spec.path_key) is not None):
                outside.append({"file": unit.file, "line": node.start_point.row + 1,
                                "byte_span": [node.start_byte, node.end_byte], "text": _text(node)})
        for entry, issue in registrations:
            record = {
                "file": unit.file, "line": entry.start_point.row + 1, "end_line": entry.end_point.row + 1,
                "byte_span": [entry.start_byte, entry.end_byte], "text": _text(entry),
                "reason": issue, "operation_ids": [], "original_path": None,
            }
            entries.append(record)
            if issue:
                continue
            if any(child.type == "spread_element" or
                   any(key is not None and key.type == "computed_property_name"
                       for key in (child.child_by_field_name("key"), child.child_by_field_name("name")))
                   for child in _children(entry)):
                record["reason"] = "registration entry has a spread or computed key"
            path_node = _unwrap(_property(entry, spec.path_key))
            if path_node is not None and path_node.type == "template_string":
                if any(child.type == "template_substitution" for child in _children(path_node)):
                    template_locations.append({"file": unit.file, "line": path_node.start_point.row + 1})
                    record["reason"] = "route path template contains a substitution"
                    continue
                source = _text(path_node)[1:-1].replace("\r\n", "\n").replace("\r", "\n")
                path = _decode_string(source)
            else:
                path = _string(path_node)
            methods = _methods(_property(entry, spec.method_key))
            if path is None or not methods:
                record["reason"] = ("route path is missing or not literal" if path is None else
                                    "route method is missing, not literal, or an empty or holed array")
                continue
            handler = _nested_property(entry, spec.handler_key, spec.handler_under)
            handler_text = _text(handler) if handler is not None else None
            site = _handler_site(units, unit, handler)
            expression = _unwrap(handler)
            if expression is not None and expression.type in {"arrow_function", "function_expression"}:
                handler_inline += len(methods)
            has_validation, validation_unknown = _validation(entry, spec)
            optional = bool(_OPTIONAL_PARAM.search(path) or _OPTIONAL_PARAM.search(spec.mount))
            normalized = normalize_path(path, spec.mount)
            record["original_path"] = path
            record["operation_ids"] = [f"op:{method} {normalized}" for method in methods]
            rows.extend(
                Route(method, normalized, handler_text, site, has_validation,
                      unit.file, entry.start_point.row + 1, optional, validation_unknown)
                for method in methods
            )
    wildcards = sum(row.method == "*" for row in rows)
    unextracted: list[_Unextracted] = [{"method": "*", "count": wildcards}] if wildcards else []
    if template_locations:
        unextracted.append({
            "reason": "template_path_with_substitution",
            "count": len(template_locations),
            "locations": template_locations,
        })
    return Extraction(rows, unextracted, unparsed, handler_inline, entries, outside)


def routes(repo: str | Path, spec: RouteSpec) -> list[Route]:
    return extract(repo, spec).routes


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--spec", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)


def run(args: argparse.Namespace) -> int:
    spec = RouteSpec(**json.loads(args.spec.read_text(encoding="utf-8")))
    report = extract(args.repo, spec)
    args.out.mkdir(parents=True, exist_ok=True)
    with (args.out / "routes.jsonl").open("w", encoding="utf-8") as output:
        for route in report.routes:
            output.write(json.dumps(asdict(route)) + "\n")
    summary = {
        "routes": len(report.routes),
        "with_handler_site": sum(route.handler_site is not None for route in report.routes),
        "with_validation": sum(route.has_validation for route in report.routes),
        "with_validation_unknown": sum(route.validation_unknown for route in report.routes),
        "handler_inline": report.handler_inline,
        "unextracted": sum(detail["count"] for detail in report.unextracted),
        "unparsed": len(report.unparsed),
        "unextracted_details": report.unextracted,
        "unparsed_files": report.unparsed,
    }
    (args.out / "routes-summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return 0
