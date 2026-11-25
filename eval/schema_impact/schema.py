"""Read, fingerprint, and compare operation-rooted schema wire trees."""

from collections import defaultdict
from collections.abc import Iterator
from dataclasses import dataclass, field, replace
from enum import Enum
from hashlib import sha256
import json
from operator import attrgetter
from pathlib import Path
import re
from typing import Any
from urllib.parse import unquote, urlsplit

import yaml

from .ids import field_id, op_id


_METHODS = {"get", "put", "post", "delete", "options", "head", "patch", "trace"}
_SUCCESS = re.compile(r"2[0-9]{2}")
_UNESCAPED = re.compile(r"[/~#*]|\[\]")
_COMPOSITIONS = ("allOf", "oneOf", "anyOf")
_WRAPPER_CONSTRAINTS = {
    "type", "properties", "additionalProperties", "items", "required", "enum", "format", "oneOf", "anyOf",
}
_MAX_DEPTH = 16


class _Edge(Enum):
    ITEM = "[]"
    MAP_VALUE = "*"


type _Pointer = tuple[str | _Edge, ...]
type _Ref = tuple[str, ...]


class _SchemaYamlLoader(yaml.SafeLoader):
    # Keep legacy YAML 1.1 booleans and implicit timestamps as strings.
    yaml_implicit_resolvers = {
        initial: [
            (tag, pattern) for tag, pattern in resolvers
            if tag not in {"tag:yaml.org,2002:bool", "tag:yaml.org,2002:timestamp"}
        ]
        for initial, resolvers in yaml.SafeLoader.yaml_implicit_resolvers.items()
    }


_SchemaYamlLoader.add_implicit_resolver(
    "tag:yaml.org,2002:bool", re.compile(r"^(?:true|True|TRUE|false|False|FALSE)$"), "tTfF",
)


def _json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def _hash_shapes(shapes: dict[str, str]) -> str:
    lines = "".join(f"{element}\t{shapes[element]}\n" for element in sorted(shapes))
    return sha256(lines.encode()).hexdigest()


def _mapping(value: Any) -> dict:
    return value if isinstance(value, dict) else {}


@dataclass(frozen=True)
class Operation:
    source_path: str
    operation_id: str | None
    shape: str


@dataclass(frozen=True)
class _Shape:
    type: str
    format: str
    required: bool
    enum: tuple[str, ...]
    items_type: str
    item_types: tuple[str, ...]

    @property
    def signature(self) -> str:
        required = "true" if self.required else "false"
        enum = f"[{','.join(self.enum)}]"
        return f"{self.type}|{self.format}|{required}|{enum}|{self.items_type}"


@dataclass(frozen=True)
class Field:
    definition: str | None
    recursive: str | None
    shape: str
    _shapes: tuple[_Shape, ...] = field(repr=False)


@dataclass(frozen=True)
class Change:
    kind: str
    element: str
    old_shape: str | None = None
    new_shape: str | None = None


@dataclass
class SchemaIndex:
    operations: dict[str, Operation]
    fields: dict[str, Field]
    sha256: str
    info_version: str | None
    content_hash: str
    warnings: list[str]
    _definitions: dict[str, set[str]] = field(repr=False)
    _field_operations: dict[str, str] = field(repr=False)
    _first_responses: dict[str, dict[_Pointer, set[str]]] = field(repr=False)

    def definitions_of(self, op_id: str) -> set[str]:
        return set(self._definitions.get(op_id, ()))

    def response_fields(self, op_id: str, response_root: str) -> list[str]:
        tree = self._first_responses.get(op_id, {})
        root = (response_root,)
        return sorted(tree.get(root, set()) | tree.get((*root, _Edge.ITEM), set()))


@dataclass(frozen=True)
class _Node:
    schema: dict
    refs: frozenset[_Ref] = frozenset()
    definition: str | None = None
    recursive: str | None = None
    stopped: bool = False
    # Composition indices preserve the origin of overlapping properties.
    allof: tuple[tuple[int, int], ...] = ()
    unions: frozenset[tuple[int, int]] = frozenset()

    def child(self, schema: Any) -> "_Node":
        return replace(self, schema=_mapping(schema))


@dataclass
class _Declaration:
    layers: list[_Node]
    definition: str | None = None
    required: bool = False


def _ref_path(ref: Any) -> _Ref | None:
    if ref == "#":
        return ()
    if not isinstance(ref, str) or not ref.startswith("#/"):
        return None
    return tuple(part.replace("~1", "/").replace("~0", "~")
                 for part in unquote(ref[2:]).split("/"))


def _definition(ref: _Ref) -> str | None:
    match ref:
        case ("definitions", name) | ("components", "schemas", name):
            return name
    return None


def _node_type(schema: dict) -> str:
    if "type" in schema:
        value = schema["type"]
        return _json(sorted(value)) if isinstance(value, list) else str(value)
    if "properties" in schema or "additionalProperties" in schema:
        return "object"
    if "items" in schema:
        return "array"
    return ""


def _field_key(root: str, pointer: _Pointer) -> str:
    return field_id(root, [part.value if isinstance(part, _Edge) else part for part in pointer])


def _first_name(names: list[str | None]) -> str | None:
    return min((name for name in names if name is not None), default=None)


def _make_field(
    declarations: list[_Declaration], items: dict[int, list[_Node]], recursive: str | None,
) -> Field:
    shapes = []
    for declaration in declarations:
        # Composition-only wrappers contribute through their members.
        shaped = [
            layer for layer in declaration.layers
            if _node_type(layer.schema) or "enum" in layer.schema or "format" in layer.schema
        ]
        for layer in shaped or declaration.layers[:1] or [_Node({})]:
            schema = layer.schema
            item_types = tuple(sorted({_node_type(item.schema) for item in items.get(id(layer), [])} - {""}))
            shapes.append(_Shape(
                _node_type(schema),
                schema.get("format", ""),
                declaration.required,
                tuple(sorted({_json(value) for value in schema.get("enum", [])})),
                ",".join(item_types),
                item_types,
            ))
    shapes.sort(key=attrgetter("signature"))
    signatures = [shape.signature for shape in shapes]
    signature = signatures[0] if len(signatures) == 1 else _json(signatures)
    definition = _first_name([declaration.definition for declaration in declarations])
    return Field(definition, recursive, signature, tuple(shapes))


class _Reader:
    def __init__(self, document: dict):
        self.document = document
        self.warnings: set[str] = set()
        self.definitions: set[str] = set()

    def _lookup(self, ref: _Ref) -> Any:
        target: Any = self.document
        for part in ref:
            if isinstance(target, list) and part.isascii() and part.isdecimal():
                index = int(part)
                target = target[index] if index < len(target) else None
            else:
                target = _mapping(target).get(part)
        return target

    def _resolve(self, node: _Node, location: str, *, schema: bool) -> _Node:
        while True:
            current = node.schema
            if schema and "$ref" not in current:
                members = current.get("allOf", [])
                if (len(members) == 1 and "$ref" in _mapping(members[0])
                        and not _WRAPPER_CONSTRAINTS.intersection(current)):
                    current = members[0]
            if "$ref" not in current:
                return node
            value = current["$ref"]
            ref = _ref_path(value)
            if ref is None:
                self.warnings.add(f"nonlocal_ref {location} {value!r}")
                return replace(node, schema={}, stopped=True)
            target = self._lookup(ref)
            if not isinstance(target, (dict, bool)):
                self.warnings.add(f"unresolved_ref {location} {value!r}")
                return replace(node, schema={}, stopped=True)
            name = _definition(ref) if schema else None
            if name is not None:
                self.definitions.add(name)
            if ref in node.refs:
                return replace(node, schema=_mapping(target), stopped=True,
                               recursive=name or node.definition)
            node = replace(node, schema=_mapping(target), refs=node.refs | {ref},
                           definition=name or node.definition)

    def message(self, raw: Any, location: str) -> dict:
        resolved = self._resolve(_Node(_mapping(raw)), location, schema=False)
        return {} if resolved.stopped else resolved.schema

    def _layers(
        self, node: _Node, location: str, active: frozenset[int] = frozenset(),
    ) -> Iterator[_Node]:
        node = self._resolve(node, location, schema=True)
        schema = node.schema
        if "properties" in schema and not isinstance(schema["properties"], dict):
            self.warnings.add(f"malformed_properties {location}")
        identity = id(schema)
        if identity in active and not node.stopped:
            self.warnings.add(f"recursive_composition {location}")
            return
        allof = node.allof + ((identity, -1),) if "allOf" in schema else node.allof
        yield replace(node, allof=allof)
        if node.stopped:
            return
        for kind in _COMPOSITIONS:
            members = schema.get(kind, [])
            if members and kind != "allOf":
                self.warnings.add(f"{kind}_union {location}")
            for index, member in enumerate(members):
                child = node.child(member)
                if kind == "allOf":
                    child = replace(child, allof=node.allof + ((identity, index),))
                else:
                    child = replace(child, unions=node.unions | {(identity, index)})
                yield from self._layers(child, location, active | {identity})

    def walk(self, root: str, raw: Any) -> tuple[dict[str, Field], dict[_Pointer, set[str]]]:
        fields: dict[str, Field] = {}
        properties: dict[_Pointer, set[str]] = {}

        def visit(pointer: _Pointer, declarations: list[_Declaration]) -> str | None:
            location = _field_key(root, pointer)
            layers = [node for declaration in declarations for node in declaration.layers]
            children = [node for node in layers if not node.stopped]
            recursive = [node.recursive for node in layers]
            items_by_layer: dict[int, list[_Node]] = {}
            item_pointer = (*pointer, _Edge.ITEM)
            if len(pointer) == _MAX_DEPTH:
                if any(node.schema.get("properties") or node.schema.get("items")
                       or isinstance(node.schema.get("additionalProperties"), dict)
                       for node in children):
                    self.warnings.add(f"depth_cut {location}")
            else:
                item_key = _field_key(root, item_pointer)
                items_by_layer = {
                    id(node): list(self._layers(node.child(node.schema["items"]), item_key))
                    for node in children if "items" in node.schema
                }
                visit_properties(pointer, children)
                items = [item for group in items_by_layer.values() for item in group]
                if items:
                    recursive.append(visit(item_pointer, [_Declaration(items)]))

                map_pointer = (*pointer, _Edge.MAP_VALUE)
                map_key = _field_key(root, map_pointer)
                values = [
                    value for node in children
                    if isinstance(node.schema.get("additionalProperties"), dict)
                    for value in self._layers(node.child(node.schema["additionalProperties"]), map_key)
                ]
                objects = [value for value in values if _node_type(value.schema) in ("object", "")]
                if any(value.recursive or _node_type(value.schema) == "object" for value in objects):
                    recursive.append(visit(map_pointer, [_Declaration(objects)]))

            cycle = _first_name(recursive)
            if pointer and isinstance(pointer[-1], str):
                if location in fields:
                    self.warnings.add(f"field_id_collision {location}")
                else:
                    fields[location] = _make_field(declarations, items_by_layer, cycle)
            return cycle

        def visit_properties(pointer: _Pointer, children: list[_Node]) -> None:
            declared: dict[str, list[_Node]] = defaultdict(list)
            for node in children:
                for name in _mapping(node.schema.get("properties")):
                    declared[name].append(node)
            properties[pointer] = set(declared)

            for name, owners in sorted(declared.items()):
                child_pointer = (*pointer, name)
                child_key = _field_key(root, child_pointer)
                if _UNESCAPED.search(name):
                    self.warnings.add(f"unescaped_property_name {child_key}")
                branches: dict[int, set[int]] = defaultdict(set)
                child_declarations = []
                for owner in owners:
                    for group, member in owner.allof:
                        branches[group].add(member)
                    required = any(name in layer.schema.get("required", [])
                                   and layer.unions <= owner.unions for layer in children)
                    nodes = list(self._layers(owner.child(owner.schema["properties"][name]), child_key))
                    child_declarations.append(_Declaration(nodes, owner.definition, required))
                if any(len(members) > 1 for members in branches.values()):
                    self.warnings.add(f"allof_overlap {child_key}")
                visit(child_pointer, child_declarations)

        visit((), [_Declaration(list(self._layers(_Node(_mapping(raw)), root)))])
        return fields, properties


def _base(document: dict, openapi: bool) -> str:
    if not openapi:
        return document.get("basePath", "")
    servers = document.get("servers", [])
    if not servers:
        return ""
    # Strip the authority separately so a template in the hostname or port
    # never affects parsing of the server path.
    url = re.sub(r"^(?:[a-zA-Z][a-zA-Z0-9+.-]*:)?//[^/?#]*", "", servers[0]["url"])
    return "/".join(part for part in urlsplit(url).path.split("/")
                    if not re.fullmatch(r"\{[^{}]+\}", part))


def _content_schema(reader: _Reader, raw: Any, root: str) -> Any:
    content = _mapping(reader.message(raw, root).get("content"))
    if not content:
        return None
    media = "application/json" if "application/json" in content else min(content)
    if media != "application/json":
        reader.warnings.add(f"media_type_fallback {root} {media}")
    return _mapping(content[media]).get("schema")


def _swagger_body(reader: _Reader, path: dict, operation: dict, root: str) -> Any:
    for scope in (operation, path):
        for raw in scope.get("parameters", []):
            parameter = reader.message(raw, root)
            if parameter.get("in") == "body":
                return parameter.get("schema")
    return None


def _parse(data: bytes) -> tuple[dict, bool]:
    try:
        source = json.loads(data)
    except json.JSONDecodeError:
        source = yaml.load(data, Loader=_SchemaYamlLoader)
    document = _mapping(source)
    openapi = str(document.get("openapi", "")).startswith("3.")
    if not openapi and document.get("swagger") != "2.0":
        raise ValueError("Expected a Swagger 2.0 or OpenAPI 3 schema")
    return document, openapi


def load(path_or_bytes: str | Path | bytes) -> SchemaIndex:
    data = path_or_bytes if isinstance(path_or_bytes, bytes) else Path(path_or_bytes).read_bytes()
    document, openapi = _parse(data)
    base = _base(document, openapi)
    reader = _Reader(document)
    operations: dict[str, Operation] = {}
    fields: dict[str, Field] = {}
    definitions: dict[str, set[str]] = {}
    field_operations: dict[str, str] = {}
    first_responses: dict[str, dict[_Pointer, set[str]]] = {}

    for source_path, raw in sorted(document.get("paths", {}).items()):
        if not source_path.startswith("/"):
            continue
        path = reader.message(raw, source_path)
        for method in sorted(_METHODS & path.keys()):
            identifier = op_id(method, source_path, base)
            if identifier in operations:
                first = operations[identifier].source_path
                reader.warnings.add(f"duplicate_operation {identifier} {first!r} {source_path!r}")
                continue
            reader.definitions.clear()
            operation = _mapping(path[method])
            suffix = identifier.removeprefix("op:")
            request = f"req:{suffix}"
            if openapi:
                body = _content_schema(reader, operation.get("requestBody"), request)
            else:
                body = _swagger_body(reader, path, operation, request)
            own_fields, _ = reader.walk(request, body)
            responses = {str(key): value for key, value in operation.get("responses", {}).items()}
            statuses = sorted(key for key in responses
                              if _SUCCESS.fullmatch(key) or (openapi and key == "2XX"))
            for position, status in enumerate(statuses):
                root = f"resp:{suffix}:{status}"
                if openapi:
                    schema = _content_schema(reader, responses[status], root)
                else:
                    schema = reader.message(responses[status], root).get("schema")
                response, properties = reader.walk(root, schema)
                own_fields.update(response)
                if position == 0:
                    first_responses[identifier] = properties
            operation_shape = _hash_shapes({key: value.shape for key, value in own_fields.items()})
            operations[identifier] = Operation(source_path, operation.get("operationId"), operation_shape)
            definitions[identifier] = reader.definitions.copy()
            fields.update(own_fields)
            field_operations.update(dict.fromkeys(own_fields, identifier))

    version = _mapping(document.get("info")).get("version")
    content_hash = _hash_shapes({key: value.shape for key, value in (operations | fields).items()})
    return SchemaIndex(operations, fields, sha256(data).hexdigest(),
                       str(version) if version is not None else None, content_hash,
                       sorted(reader.warnings), definitions, field_operations, first_responses)


def diff(old: SchemaIndex, new: SchemaIndex) -> list[Change]:
    changes = []
    removed = old.operations.keys() - new.operations.keys()
    for key in sorted(removed):
        changes.append(Change("op_removed", key, old_shape=old.operations[key].shape))
    for key in sorted(new.operations.keys() - old.operations.keys()):
        changes.append(Change("op_added", key, new_shape=new.operations[key].shape))
    for key in sorted(old.fields.keys() | new.fields.keys()):
        if old._field_operations.get(key) in removed:
            continue
        before, after = old.fields.get(key), new.fields.get(key)
        if before is None:
            changes.append(Change("field_added", key, new_shape=after.shape))
        elif after is None:
            changes.append(Change("field_removed", key, old_shape=before.shape))
        elif before.shape != after.shape:
            previous_types = {(s.type, s.format, s.items_type) for s in before._shapes}
            current_types = {(s.type, s.format, s.items_type) for s in after._shapes}
            if previous_types != current_types:
                changes.append(Change("field_type_changed", key, before.shape, after.shape))
            if any(s.required for s in after._shapes) and not any(s.required for s in before._shapes):
                changes.append(Change("field_required_added", key, before.shape, after.shape))
            if {s.enum for s in before._shapes} != {s.enum for s in after._shapes}:
                changes.append(Change("enum_values_changed", key, before.shape, after.shape))
    return changes
