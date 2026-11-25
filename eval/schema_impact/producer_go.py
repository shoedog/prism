"""Extract Go route evidence and measure its agreement with the schema."""

import argparse
from collections import Counter, defaultdict
from collections.abc import Iterator
from dataclasses import asdict, dataclass
import json
from pathlib import Path
import re

from tree_sitter import Language, Node, Parser
import tree_sitter_go
import yaml

from . import outbound_http
from .ids import normalize_path, op_id
from .schema import SchemaIndex, load


_GO = Language(tree_sitter_go.language())
_ROUTER = re.compile(r"//\s*@Router\s+(\S+)\s+\[([A-Za-z]+)\]\s*")
_METHODS = {"GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"}
_UNEXTRACTED_METHODS = ("Any", "Handle", "Match")
_GO_ESCAPES = dict(zip(b'abfnrtv"\\', b'\a\b\f\n\r\t\v"\\', strict=True))


@dataclass(frozen=True)
class Binding:
    element: str
    file: str
    line: int
    function: str
    receiver_type: str | None
    via: str = "swaggo_annotation"


@dataclass(frozen=True)
class Registration:
    method: str
    path_suffix: str
    handler_name: str
    file: str
    line: int


@dataclass(frozen=True)
class _RegistrationSite:
    registration: Registration
    lines: list[str]


@dataclass(frozen=True)
class _NonliteralSite:
    method: str
    file: str
    line: int
    lines: list[str]


def _is_go_file(file: str) -> bool:
    return file.endswith(".go") and not file.endswith("_test.go")


def _go_sources(repo: Path) -> dict[str, str]:
    return {
        path.relative_to(repo).as_posix(): path.read_bytes().decode("utf-8")
        for path in sorted(repo.rglob("*.go"))
        if _is_go_file(path.name) and path.is_file() and not path.is_symlink()
    }


def _parsed(files: dict[str, str]) -> Iterator[tuple[str, str, Node]]:
    parser = Parser(_GO)
    for file, source in sorted(files.items()):
        if _is_go_file(file):
            yield file, source, parser.parse(source.encode("utf-8")).root_node


def _source_lines(source: str) -> list[str]:
    return [line.removesuffix("\r") for line in source.split("\n")]


def _nodes(root: Node) -> Iterator[Node]:
    stack = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(node.named_children))


def _text(node: Node) -> str:
    return node.text.decode("utf-8")


def _doc_comments(declaration: Node, lines: list[str]) -> Iterator[Node]:
    comment = declaration.prev_named_sibling
    next_row = declaration.start_point.row
    while comment is not None and comment.type == "comment":
        row, column = comment.start_point
        if row + 1 != next_row or lines[row][:column].strip() or not comment.text.startswith(b"//"):
            break
        yield comment
        next_row = row
        comment = comment.prev_named_sibling


def _annotation_groups(file: str, lines: list[str], root: Node) -> Iterator[list[Binding]]:
    for declaration in root.named_children:
        if declaration.type not in {"function_declaration", "method_declaration"}:
            continue
        name = declaration.child_by_field_name("name")
        if name is None or name.is_missing:
            continue
        receiver = declaration.child_by_field_name("receiver")
        receiver_type = None
        if receiver is not None:
            receiver_type = next((_text(n) for n in _nodes(receiver) if n.type == "type_identifier"), None)
        group = []
        for comment in reversed(list(_doc_comments(declaration, lines))):
            match = _ROUTER.fullmatch(_text(comment))
            if match:
                path, method = match.groups()
                group.append(Binding(op_id(method, path), file, comment.start_point.row + 1,
                                     _text(name), receiver_type))
        if group:
            yield group


def annotations_from_sources(files: dict[str, str]) -> list[Binding]:
    return [
        binding
        for file, source, root in _parsed(files)
        for group in _annotation_groups(file, _source_lines(source), root)
        for binding in group
    ]


def annotations(repo: Path) -> list[Binding]:
    return annotations_from_sources(_go_sources(repo))


def _literal_value(node: Node) -> str:
    if node.type == "raw_string_literal":
        return _text(node)[1:-1].replace("\r", "")
    value = bytearray()
    for part in node.named_children:
        if part.type != "escape_sequence":
            value.extend(part.text)
            continue
        escape = part.text[1:]
        if escape[0] in _GO_ESCAPES:
            value.append(_GO_ESCAPES[escape[0]])
        elif escape.startswith(b"x"):
            value.append(int(escape[1:], 16))
        elif escape.startswith((b"u", b"U")):
            value.extend(chr(int(escape[1:], 16)).encode("utf-8"))
        else:
            value.append(int(escape, 8))
    return value.decode("utf-8", errors="surrogateescape")


def _registration_sites(
    file: str, lines: list[str], root: Node, unextracted: Counter[str],
    nonliteral: dict[str, list[_NonliteralSite]] | None = None,
) -> Iterator[_RegistrationSite]:
    for call in _nodes(root):
        if call.type != "call_expression":
            continue
        callee = call.child_by_field_name("function")
        if callee is None or callee.type != "selector_expression":
            continue
        field = callee.child_by_field_name("field")
        if field is None:
            continue
        method = _text(field)
        if method in _UNEXTRACTED_METHODS:
            unextracted[method] += 1
            continue
        if method not in _METHODS:
            continue
        arguments = call.child_by_field_name("arguments")
        args = [arg for arg in arguments.named_children if arg.type != "comment"] if arguments else []
        if len(args) < 2 or args[0].has_error:
            continue
        handler = None
        # Gin registrations can put middleware before the final handler.
        for node in _nodes(args[-1]):
            if node.type in {"identifier", "field_identifier"}:
                handler = _text(node)
        if handler is None:
            continue
        start, end = call.start_point.row, call.end_point.row
        call_lines = lines[start:end + 1]
        if args[0].type not in {"interpreted_string_literal", "raw_string_literal"}:
            if nonliteral is not None:
                nonliteral[handler].append(_NonliteralSite(method, file, start + 1, call_lines))
            continue
        try:
            suffix = normalize_path(_literal_value(args[0]))
        except ValueError:
            continue  # A Go escape can be syntactically valid but out of range.
        registration = Registration(method, suffix, handler, file, start + 1)
        yield _RegistrationSite(registration, call_lines)


def registrations(repo: Path) -> list[Registration]:
    unextracted: Counter[str] = Counter()
    return [
        site.registration
        for file, source, root in _parsed(_go_sources(repo))
        for site in _registration_sites(file, _source_lines(source), root, unextracted)
    ]


def _entries(node: yaml.Node | None) -> dict[str, tuple[yaml.ScalarNode, yaml.Node]]:
    if not isinstance(node, yaml.MappingNode):
        return {}
    inherited = {}
    explicit = {}
    for key, value in node.value:
        if not isinstance(key, yaml.ScalarNode):
            continue
        if key.tag == "tag:yaml.org,2002:merge":
            sources = value.value if isinstance(value, yaml.SequenceNode) else [value]
            for source in reversed(sources):
                inherited.update(_entries(source))
        else:
            explicit[key.value] = (key, value)
    return inherited | explicit


def _schema_sites(index: SchemaIndex, schema_data: bytes) -> dict[str, tuple[int, str]]:
    if not index.operations:
        return {}
    try:
        text = schema_data.decode("utf-8-sig")
        document = _entries(yaml.compose(text))
    except (UnicodeError, yaml.YAMLError):
        return {}  # Source locations are optional for a schema already accepted by load().
    lines = text.splitlines()
    paths = _entries(document["paths"][1])
    sites = {}
    for element, op in index.operations.items():
        path_key, path_node = paths[op.source_path]
        method = element.split(" ", 1)[0].removeprefix("op:").lower()
        key, _ = _entries(path_node).get(method, (path_key, path_node))
        line = key.start_mark.line
        sites[element] = line + 1, lines[line]
    return sites


def _measure(repo: Path, schema_file: Path) -> tuple[list[Binding], dict]:
    try:
        schema_name = schema_file.relative_to(repo).as_posix()
    except ValueError:
        schema_name = schema_file.as_posix()
    schema_data = schema_file.read_bytes()
    index = load(schema_data)
    schema_sites = _schema_sites(index, schema_data)
    lines_by_file = {}
    by_element: dict[str, list[Binding]] = defaultdict(list)
    by_function: dict[str, list[Binding]] = defaultdict(list)
    function_counts: Counter[str] = Counter()
    unextracted: Counter[str] = Counter()
    nonliteral: dict[str, list[_NonliteralSite]] = defaultdict(list)
    bindings = []
    registrations = []
    for file, source, root in _parsed(_go_sources(repo)):
        lines = _source_lines(source)
        lines_by_file[file] = lines
        for group in _annotation_groups(file, lines, root):
            function_counts.update({binding.element for binding in group})
            for binding in group:
                by_element[binding.element].append(binding)
                by_function[binding.function].append(binding)
                bindings.append(binding)
        registrations.extend(_registration_sites(file, lines, root, unextracted, nonliteral))

    by_handler: dict[str, list[_RegistrationSite]] = defaultdict(list)
    for site in registrations:
        by_handler[site.registration.handler_name].append(site)

    rows = []
    row_counts: Counter[int | str] = Counter()

    def add_row(
        kind: int | str, element: str | None, annotations: list[Binding], route_sites: list[_RegistrationSite],
    ) -> None:
        row_counts[kind] += 1
        schema = schema_sites.get(element)
        route = route_sites[0].registration if route_sites else None
        annotation = annotations[0] if annotations else None
        if route is not None and kind != 2:
            file, line, function = route.file, route.line, route.handler_name
        elif annotation is not None:
            file, line, function = annotation.file, annotation.line, annotation.function
        else:
            file, line, function = schema_name, schema[0] if schema else None, None
        context = [site for name in sorted({b.function for b in annotations}) for site in nonliteral[name]]
        row = {
            "row_id": f"f1-{kind}-{row_counts[kind]:04d}", "list": kind,
            "file": file, "line": line, "function": function,
            "evidence": {
                "annotation": [lines_by_file[b.file][b.line - 1] for b in annotations],
                "registration": [line for site in route_sites for line in site.lines],
                "nonliteral_registration": [line for site in context for line in site.lines],
                "schema": [schema[1]] if schema else [],
            },
            "annotation_sites": [asdict(binding) for binding in annotations],
            "registration_sites": [asdict(site.registration) for site in route_sites],
            "nonliteral_registration_sites": [asdict(site) for site in context],
        }
        if element is not None:
            row["element"] = element
        if route is not None and kind != 2:
            row["registration"] = "\n".join(route_sites[0].lines)
        if schema:
            row["schema_site"] = {"file": schema_name, "line": schema[0]}
        rows.append(row)

    covered = set()
    population = agreements = 0
    paths_by_method: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for element in index.operations:
        method, path = element.removeprefix("op:").split(" ", 1)
        paths_by_method[method].append((path, element))
    for site in registrations:
        route = site.registration
        related = by_function[route.handler_name]
        matches = [element for path, element in paths_by_method[route.method] if path.endswith(route.path_suffix)]
        covered.update(matches)
        if not matches:
            add_row(3, None, related, [site])
        elif len(matches) == 1 and len(by_element[matches[0]]) == 1:
            population += 1
            binding = by_element[matches[0]][0]
            if binding.function == route.handler_name:
                agreements += 1
            else:
                add_row("cross_check", binding.element, [binding], [site])
        for binding in related:
            method = binding.element.removeprefix("op:").split(" ", 1)[0]
            if method != route.method:
                add_row(4, binding.element, [binding], [site])

    for element in sorted(index.operations.keys() - covered):
        related = by_element[element]
        routes = [site for name in sorted({b.function for b in related}) for site in by_handler[name]]
        add_row(2, element, related, routes)
    rows.sort(key=lambda row: row["row_id"])

    counts = Counter(function_counts[element] for element in index.operations)
    return bindings, {
        "schema_sha256": index.sha256,
        "operations": {
            "total": len(index.operations), "exactly_one_function": counts[1], "no_function": counts[0],
            "several_functions": len(index.operations) - counts[0] - counts[1],
        },
        "annotations_without_operation": sum(b.element not in index.operations for b in bindings),
        "registrations": len(registrations),
        "unextracted_registration": {method: unextracted[method] for method in _UNEXTRACTED_METHODS},
        "cross_check": {"population": population, "agreements": agreements,
                        "disagreements": population - agreements},
        "residue": rows,
    }


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True, help="Schema path, relative to the repository or absolute")
    parser.add_argument("--out", type=Path, required=True)


def run(args: argparse.Namespace) -> int:
    schema = args.repo / args.schema
    bindings, f1 = _measure(args.repo, schema)
    outbound = outbound_http.count(args.repo)
    args.out.mkdir(parents=True, exist_ok=True)
    for name, rows in (("producer.jsonl", map(asdict, bindings)), ("f1-residue.jsonl", f1["residue"])):
        (args.out / name).write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
    for name, document in (("f1.json", f1), ("outbound-http.json", outbound)):
        (args.out / name).write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    return 0
