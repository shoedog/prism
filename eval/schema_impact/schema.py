"""Read, fingerprint, and compare operation-rooted schema wire trees."""

from collections import defaultdict
from collections.abc import Iterator
from dataclasses import dataclass, field, replace
from hashlib import sha256
from itertools import chain, product
import json
from operator import attrgetter
from pathlib import Path
import re
from typing import Any
from urllib.parse import unquote

import yaml

from . import _schema_constraints as constraints
from .ids import Edge as _Edge, field_id, op_id, server_paths


_SWAGGER_METHODS = {"get", "put", "post", "delete", "options", "head", "patch"}
_SUCCESS = re.compile(r"2[0-9]{2}")
_JSON_MEDIA = "application/json"
_PARTIAL = {
    "$anchor", "$id", "$dynamicRef", "prefixItems", "contains", "if", "then", "else", "not",
    "dependentSchemas", "dependentRequired", "dependencies", "patternProperties", "propertyNames",
    "unevaluatedProperties", "unevaluatedItems",
}
_MAX_DEPTH = 16


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
    success_statuses: tuple[str, ...] = ()
    default_response: Any = None


@dataclass(frozen=True)
class _Shape:
    type: str
    format: str
    required: bool
    enum: tuple[str, ...]
    items_type: str
    enum_declared: bool

    @property
    def signature(self) -> str:
        required = "true" if self.required else "false"
        enum = f"[{','.join(self.enum)}]"
        empty_enum = "|enum" if self.enum_declared and not self.enum else ""
        return f"{self.type}|{self.format}|{required}|{enum}|{self.items_type}{empty_enum}"


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
    forbidden: bool = False
    # Composition indices preserve the origin of overlapping properties.
    allof: tuple[tuple[int, int], ...] = ()

    def child(self, schema: Any) -> "_Node":
        return replace(self, schema=_mapping(schema), forbidden=schema is False)


@dataclass
class _Declaration:
    layers: list[_Node]
    definition: str | None = None
    required: bool = False


def _ref_path(ref: Any) -> _Ref | None:
    if ref == "#":
        return ()
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None
    fragment = unquote(ref[1:])
    if not fragment.startswith("/"):
        return None
    return tuple(part.replace("~1", "/").replace("~0", "~") for part in fragment[1:].split("/"))


def _definition(ref: _Ref) -> str | None:
    match ref:
        case ("definitions", name) | ("components", "schemas", name) | ("$defs", name):
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
    return field_id(root, pointer)


def _first_name(names: list[str | None]) -> str | None:
    return min((name for name in names if name is not None), default=None)


def _make_field(
    declarations: list[_Declaration], items: dict[int, list[_Node]], recursive: str | None,
) -> Field:
    shapes: set[_Shape] = set()
    for declaration in declarations:
        schemas = [layer.schema for layer in declaration.layers]
        effective_type = constraints.type_name(constraints.types(schemas))
        enum = constraints.effective_enum(schemas)
        # Composition-only wrappers contribute through their members.
        shaped = [
            layer for layer in declaration.layers
            if _node_type(layer.schema) or "enum" in layer.schema or "format" in layer.schema
        ]
        for layer in shaped or declaration.layers[:1] or [_Node({})]:
            schema = layer.schema
            item_types = {_node_type(item.schema) for item in items.get(id(layer), [])}
            shapes.add(_Shape(
                effective_type or _node_type(schema),
                schema.get("format", ""),
                declaration.required,
                tuple(sorted({_json(value) for value in enum or []})),
                ",".join(sorted(item_types - {""})),
                enum is not None,
            ))
    ordered = tuple(sorted(shapes, key=attrgetter("signature")))
    signatures = [shape.signature for shape in ordered]
    signature = signatures[0] if len(signatures) == 1 else _json(signatures)
    definition = _first_name([declaration.definition for declaration in declarations])
    return Field(definition, recursive, signature, ordered)


class _Reader:
    def __init__(self, document: dict):
        self.document = document
        self.openapi = str(document.get("openapi", "")).startswith("3.")
        self.oas31 = str(document.get("openapi", "")).startswith("3.1.")
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
            if "$ref" not in current:
                return node
            value = current["$ref"]
            if schema and self.oas31 and len(current) > 1:
                siblings = {key: raw for key, raw in current.items() if key != "$ref"}
                return node.child({"allOf": [{"$ref": value}, siblings]})
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
                if name is None or "#/" not in location:
                    self.warnings.add(f"recursive {location}")
                return replace(node.child(target), stopped=True,
                               recursive=name or node.definition)
            node = replace(node.child(target), refs=node.refs | {ref},
                           definition=name or node.definition)

    def message(self, raw: Any, location: str) -> dict:
        resolved = self._resolve(_Node(_mapping(raw)), location, schema=False)
        return {} if resolved.stopped else resolved.schema

    def parameters(self, path: dict, operation: dict, location: str) -> dict[tuple, dict]:
        parameters = {}
        for scope in (path, operation):
            for raw in scope.get("parameters", []):
                parameter = self.message(raw, location)
                if parameter:
                    parameters[parameter.get("name"), parameter.get("in")] = parameter
        return parameters

    def _branches(
        self, node: _Node, location: str, active: frozenset[int] = frozenset(),
    ) -> list[list[_Node]]:
        node = self._resolve(node, location, schema=True)
        schema = node.schema
        identity = id(schema)
        if identity in active and not node.stopped:
            self.warnings.add(f"recursive_composition {location}")
            return [[replace(node, stopped=True)]]
        allof = node.allof + ((identity, -1),) if "allOf" in schema else node.allof
        groups = [[[replace(node, allof=allof)]]]
        if node.stopped:
            return groups[0]
        for kind in ("allOf", "oneOf", "anyOf"):
            members = schema.get(kind, [])
            if not members:
                continue
            if kind != "allOf":
                self.warnings.add(f"{kind}_union {location}")
            alternatives = []
            for index, member in enumerate(members):
                child = node.child(member)
                if kind == "allOf":
                    child = replace(child, allof=node.allof + ((identity, index),))
                branches = self._branches(child, location, active | {identity})
                if kind == "allOf":
                    groups.append(branches)
                else:
                    alternatives.extend(branches)
            if alternatives:
                groups.append(alternatives)
        return [list(chain.from_iterable(parts)) for parts in product(*groups)]

    def _conjunction(self, nodes: list[_Node], location: str) -> Iterator[list[_Node]]:
        for parts in product(*(self._branches(node, location) for node in nodes)):
            yield list(chain.from_iterable(parts))

    def _object_edges(
        self, children: list[_Node], root: str, pointer: _Pointer, *, cut: bool,
    ) -> tuple[dict[str, list[_Declaration]] | None, list[_Declaration], dict[str, list[_Declaration]]]:
        schemas = [node.schema for node in children]
        minimum, maximum = constraints.limits(schemas, "minProperties", "maxProperties")
        required = {name for schema in schemas for name in schema.get("required", [])}
        names = required | {name for schema in schemas for name in _mapping(schema.get("properties"))}
        annotation = "readOnly" if root.startswith("req:") else "writeOnly"
        feasible: dict[str, list[_Declaration]] = {}
        empty_enums: dict[str, list[_Declaration]] = {}
        for name in sorted(names):
            key = _field_key(root, (*pointer, name))
            owners = [node for node in children if name in _mapping(node.schema.get("properties"))]
            branches: dict[int, set[int]] = defaultdict(set)
            for owner in owners:
                for group, member in owner.allof:
                    branches[group].add(member)
            if any(len(members) > 1 for members in branches.values()):
                self.warnings.add(f"allof_overlap {key}")
            # A local property takes precedence over this member's remainder.
            values = [node.child(_mapping(node.schema.get("properties")).get(
                name, node.schema.get("additionalProperties", True),
            )) for node in children]
            layers = [values] if cut else list(self._conjunction(values, key))
            if self.openapi and any(layer.schema.get(annotation) is True
                                   for branch in layers for layer in branch):
                required.discard(name)
            definition = _first_name([node.definition for node in owners])
            for branch in layers:
                if any(layer.forbidden for layer in branch):
                    continue
                declaration = _Declaration(branch, definition, name in required)
                value_schemas = [layer.schema for layer in branch]
                if constraints.feasible_types(value_schemas):
                    feasible.setdefault(name, []).append(declaration)
                elif owners and any(layer.schema.get("enum") == [] for layer in branch):
                    empty_enums.setdefault(name, []).append(declaration)

        map_nodes = [node.child(node.schema["additionalProperties"]) for node in children
                     if isinstance(node.schema.get("additionalProperties"), dict)]
        map_key = _field_key(root, (*pointer, _Edge.MAP_VALUE))
        closed = any(schema.get("additionalProperties") is False for schema in schemas)
        map_layers = [] if closed or not map_nodes else (
            [map_nodes] if cut else self._conjunction(map_nodes, map_key)
        )
        maps = [
            _Declaration(branch) for branch in map_layers
            if not any(layer.forbidden for layer in branch)
            and constraints.feasible_types([layer.schema for layer in branch])
        ]
        closed |= bool(map_nodes and not maps)
        if (minimum > maximum or len(required) > maximum or required - feasible.keys()
                or (closed and len(feasible) < minimum)):
            return None, [], empty_enums
        if len(required) == maximum:
            return {name: feasible[name] for name in sorted(required)}, [], empty_enums
        return feasible, maps, empty_enums

    def walk(self, root: str, raw: Any) -> tuple[dict[str, Field], dict[_Pointer, set[str]]]:
        fields: dict[str, Field] = {}
        properties: dict[_Pointer, set[str]] = {}

        def visit(pointer: _Pointer, declarations: list[_Declaration]) -> str | None:
            location = _field_key(root, pointer) if pointer else root
            recursive = [node.recursive for declaration in declarations for node in declaration.layers]
            visible = []
            items_by_layer: dict[int, list[_Node]] = {}
            named: dict[str, list[_Declaration]] = defaultdict(list)
            items: list[_Declaration] = []
            maps: list[_Declaration] = []
            item_pointer = (*pointer, _Edge.ITEM)
            cut = len(pointer) == _MAX_DEPTH
            withheld = False
            feasible_names: set[str] = set()
            for declaration in declarations:
                layers = declaration.layers
                schemas = [node.schema for node in layers]
                if any(node.forbidden for node in layers):
                    continue
                possible = constraints.feasible_types(schemas)
                children = [node for node in layers if not node.stopped]
                unsupported = _PARTIAL.intersection(chain.from_iterable(schemas))
                if any(isinstance(value, (dict, list)) for schema in schemas
                       for value in [schema.get("const"), *schema.get("enum", [])]):
                    unsupported.add("object_or_array_value")
                if unsupported:
                    self.warnings.add(f"partial_schema {location} {','.join(sorted(unsupported))}")
                    visible.append(declaration)
                    continue
                if "object" in possible:
                    edges, remainder, empty_enums = self._object_edges(children, root, pointer, cut=cut)
                    if cut:
                        withheld |= bool(edges or remainder or empty_enums)
                    else:
                        for name, values in chain((edges or {}).items(), empty_enums.items()):
                            named[name].extend(values)
                        feasible_names.update(edges or {})
                        maps.extend(remainder)
                    if edges is None:
                        possible -= {"object"}
                        if not any("type" in schema for schema in schemas):
                            continue
                minimum, maximum = constraints.limits(schemas, "minItems", "maxItems")
                if "array" in possible:
                    owners = [node for node in children if "items" in node.schema]
                    if any(node.schema["items"] is False for node in owners):
                        if minimum > 0:
                            possible -= {"array"}
                    elif owners and maximum > 0 and cut:
                        withheld = True
                    elif owners and maximum > 0:
                        item_nodes = [node.child(node.schema["items"]) for node in owners]
                        branches = list(self._conjunction(item_nodes, _field_key(root, item_pointer)))
                        items.extend(_Declaration(branch) for branch in branches)
                        for owner in owners:
                            items_by_layer[id(owner)] = list(chain.from_iterable(branches))
                if possible or any(schema.get("enum") == [] for schema in schemas):
                    visible.append(declaration)

            if withheld:
                self.warnings.add(f"depth_cut {location}")
            properties[pointer] = feasible_names
            for name, values in named.items():
                visit((*pointer, name), values)
            if items:
                recursive.append(visit(item_pointer, items))
            if maps:
                recursive.append(visit((*pointer, _Edge.MAP_VALUE), maps))

            cycle = _first_name(recursive)
            if visible and pointer and isinstance(pointer[-1], str):
                fields[location] = _make_field(visible, items_by_layer, cycle)
            return cycle

        visit((), [_Declaration(branch) for branch in self._branches(_Node({}).child(raw), root)])
        return fields, properties


def _content_schema(reader: _Reader, raw: Any, root: str) -> Any:
    content = _mapping(reader.message(raw, root).get("content"))
    for media in content:
        if media.lower() != _JSON_MEDIA:
            reader.warnings.add(f"non_json_media {root} {media}")
    matches = sorted(media for media in content if media.lower() == _JSON_MEDIA)
    if len(matches) == 1:
        return _mapping(content[matches[0]]).get("schema")
    if matches:
        reader.warnings.add(f"ambiguous_json_media {root}")
    return None


def _swagger_schema(reader: _Reader, raw: dict, operation: dict, field: str, root: str) -> Any:
    schema = raw.get("schema")
    if schema is None:
        return None
    media = operation.get(field, reader.document.get(field))
    if media is None:
        reader.warnings.add(f"media_unverified {root}")
        return schema
    if any(value.lower() == _JSON_MEDIA for value in media):
        return schema
    reader.warnings.add(f"non_json_media {root} {','.join(media)}")
    return None


def _request_schema(
    reader: _Reader, parameters: dict[tuple, dict], operation: dict, method: str, root: str,
) -> Any:
    if reader.openapi:
        if method == "trace" or (not reader.oas31 and method not in {"post", "put", "patch"}):
            return None
        return _content_schema(reader, operation.get("requestBody"), root)
    bodies = [p for (_, location), p in parameters.items() if location == "body"]
    if len(bodies) == 1:
        return _swagger_schema(reader, bodies[0], operation, "consumes", root)
    if bodies:
        reader.warnings.add(f"ambiguous_request_body {root}")
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


def load(path_or_bytes: str | Path | bytes, *, retrieval_url: str | None = None) -> SchemaIndex:
    data = path_or_bytes if isinstance(path_or_bytes, bytes) else Path(path_or_bytes).read_bytes()
    document, openapi = _parse(data)
    reader = _Reader(document)
    methods = _SWAGGER_METHODS | {"trace"} if openapi else _SWAGGER_METHODS
    operations: dict[str, Operation] = {}
    fields: dict[str, Field] = {}
    definitions: dict[str, set[str]] = {}
    field_operations: dict[str, str] = {}
    first_responses: dict[str, dict[_Pointer, set[str]]] = {}

    for source_path, raw in sorted(document.get("paths", {}).items()):
        if not source_path.startswith("/"):
            continue
        path = reader.message(raw, source_path)
        for method in sorted(methods & path.keys()):
            operation = _mapping(path[method])
            parameters = reader.parameters(path, operation, source_path)
            labels = {name for (name, location), parameter in parameters.items()
                      if openapi and location == "path" and parameter.get("style") == "label"
                      and _mapping(parameter.get("schema")).get("type") in
                      ("string", "number", "integer", "boolean")}
            bases = {document.get("basePath", "")}
            if openapi:
                servers = operation.get("servers", path.get("servers", document.get("servers", [])))
                bases = server_paths(servers, retrieval_url, reader.warnings)
            responses = {str(key): value for key, value in operation.get("responses", {}).items()}
            statuses = tuple(sorted(key for key in responses
                                    if _SUCCESS.fullmatch(key) or (openapi and key == "2XX")))
            for base in sorted(bases):
                identifier = op_id(method, source_path, base, dialect="schema", labels=labels)
                if identifier in operations:
                    first = operations[identifier].source_path
                    reader.warnings.add(f"duplicate_operation {identifier} {first!r} {source_path!r}")
                    continue
                reader.definitions.clear()
                suffix = identifier.removeprefix("op:")
                request = f"req:{suffix}"
                body = _request_schema(reader, parameters, operation, method, request)
                own_fields, _ = reader.walk(request, body)
                for position, status in enumerate(statuses):
                    root = f"resp:{suffix}:{status}"
                    if method == "head" or status in {"204", "205"}:
                        continue
                    if openapi:
                        schema = _content_schema(reader, responses[status], root)
                    else:
                        raw_response = reader.message(responses[status], root)
                        schema = _swagger_schema(reader, raw_response, operation, "produces", root)
                    response, properties = reader.walk(root, schema)
                    own_fields.update(response)
                    if position == 0:
                        first_responses[identifier] = properties
                shapes = {key: value.shape for key, value in own_fields.items()}
                shapes.update(dict.fromkeys((f"resp:{suffix}:{status}" for status in statuses), ""))
                operations[identifier] = Operation(
                    source_path, operation.get("operationId"), _hash_shapes(shapes),
                    statuses, responses.get("default"),
                )
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
    for key in sorted(old.operations.keys() & new.operations.keys()):
        before = set(old.operations[key].success_statuses)
        after = set(new.operations[key].success_statuses)
        root = f"resp:{key.removeprefix('op:')}"
        changes.extend(Change("success_status_removed", f"{root}:{status}") for status in sorted(before - after))
        changes.extend(Change("success_status_added", f"{root}:{status}") for status in sorted(after - before))
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
            was_required = any(s.required for s in before._shapes)
            is_required = any(s.required for s in after._shapes)
            if was_required != is_required:
                kind = "field_required_added" if is_required else "field_required_removed"
                changes.append(Change(kind, key, before.shape, after.shape))
            if {s.enum_declared for s in before._shapes} != {s.enum_declared for s in after._shapes}:
                changes.append(Change("enum_declaration_changed", key, before.shape, after.shape))
            if {s.enum for s in before._shapes} != {s.enum for s in after._shapes}:
                changes.append(Change("enum_values_changed", key, before.shape, after.shape))
    return changes
