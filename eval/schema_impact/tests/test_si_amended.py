import json

import pytest

from schema_impact.schema import diff, load


def schema_index(paths, *, document=None):
    return load(json.dumps({"openapi": "3.1.0", "paths": paths, **(document or {})}).encode())


def json_body(schema):
    return {"content": {"application/json": {"schema": schema}}}


def wire(schema):
    body = json_body(schema)
    return schema_index({"/a": {"post": {"requestBody": body, "responses": {"200": body}}}})


def test_bodyless_success_status_changes_and_default_are_independent():
    body = json_body({"properties": {"text": {"type": "string"}}})
    old = schema_index({"/a": {"head": {"responses": {"204": body, "default": body}}}})
    new = schema_index({"/a": {"head": {"responses": {"205": body, "2XX": {}}}}})
    op = "op:HEAD /a"
    assert old.operations[op].success_statuses == ("204",)
    assert new.operations[op].success_statuses == ("205", "2XX")
    assert old.operations[op].default_response == body
    assert not old.fields and not new.fields
    assert {(change.kind, change.element) for change in diff(old, new)} == {
        ("success_status_removed", "resp:HEAD /a:204"),
        ("success_status_added", "resp:HEAD /a:205"),
        ("success_status_added", "resp:HEAD /a:2XX"),
    }


def test_swagger_media_override_and_ambiguous_inherited_bodies():
    body = {"schema": {"properties": {"stamp": {"type": "string"}}}}
    index = schema_index({"/a": {
        "parameters": [{"name": "first", "in": "body", **body}],
        "post": {"parameters": [{"name": "second", "in": "body", **body}],
                 "produces": ["text/xml"], "responses": {"200": body}},
        "put": {"responses": {"200": body}},
        "patch": {"consumes": [], "responses": {}},
    }}, document={"openapi": None, "swagger": "2.0",
                   "consumes": ["Application/JSON"], "produces": ["application/json"]})
    assert set(index.fields) == {"field:req:PUT /a#/stamp", "field:resp:PUT /a:200#/stamp"}
    assert any(warning.startswith("ambiguous_request_body ") for warning in index.warnings)
    assert any(warning.startswith("non_json_media ") for warning in index.warnings)


def test_object_capacity_and_required_diff():
    schema = {"maxProperties": 1, "properties": {
        "kept": {"type": "string"}, "extra": {"type": "string"},
    }, "additionalProperties": {"properties": {"tag": {"type": "string"}}}}
    before = wire(schema)
    after = wire({**schema, "required": ["kept"]})
    restored = wire({**schema, "required": ["kept"], "maxProperties": 2})
    for root in ("req:POST /a", "resp:POST /a:200"):
        prefix = f"field:{root}#/"
        assert {(change.kind, change.element) for change in diff(before, after)
                if change.element.startswith(prefix)} == {
            ("field_required_added", prefix + "kept"),
            ("field_removed", prefix + "extra"),
            ("field_removed", prefix + "*/tag"),
        }
        assert {prefix + name for name in ("kept", "extra", "*/tag")} <= restored.fields.keys()


def test_directional_capacity_reserves_only_effectively_required_names():
    schema = {"maxProperties": 2, "required": ["secret", "serial"], "properties": {
        "secret": {"type": "string", "writeOnly": True},
        "serial": {"type": "string", "readOnly": True},
        "label": {"type": "string"},
    }}
    index = wire(schema)
    for root, required in (("req:POST /a", "secret"), ("resp:POST /a:200", "serial")):
        shapes = {name: index.fields[f"field:{root}#/{name}"].shape
                  for name in ("secret", "serial", "label")}
        assert {name for name, shape in shapes.items() if "|true|" in shape} == {required}


@pytest.mark.parametrize(("remainder", "required", "fields"), [
    ({"type": "integer"}, [], {"measure": "integer||false|[]|", "other": "||false|[]|"}),
    ({"type": "string"}, [], {"other": "||false|[]|"}),
    ({"type": "string"}, ["measure"], {}),
])
def test_conjunct_remainder_constrains_declared_names(remainder, required, fields):
    index = wire({"allOf": [
        {"required": required, "properties": {"measure": {"type": "number"}}},
        {"properties": {"other": {}}, "additionalProperties": remainder},
    ]})
    assert {key.split("#/")[1]: value.shape for key, value in index.fields.items()} == fields


def test_closed_minimum_counts_feasible_names_and_required_only_inherits_remainder():
    closed = {"properties": {"open": {"type": "string"}, "blocked": False},
              "additionalProperties": False}
    assert not wire({**closed, "minProperties": 2}).fields
    assert len(wire({**closed, "minProperties": 1}).fields) == 2
    assert not wire({"required": ["stamp"], "additionalProperties": False}).fields
    index = wire({"required": ["stamp"], "additionalProperties": {"type": "integer"}})
    assert len(index.fields) == 2
    assert {field.shape for field in index.fields.values()} == {"integer||true|[]|"}


def test_empty_enum_declaration_diff_survives_empty_value_domain():
    before = wire({"properties": {"stamp": {"type": "string"}}})
    after = wire({"properties": {"stamp": {"type": "string", "enum": []}}})
    assert before.fields.keys() == after.fields.keys()
    assert {change.kind for change in diff(before, after)} == {"enum_declaration_changed"}

    impossible = wire({"required": ["stamp"], "properties": {
        "stamp": {"type": "string", "enum": []},
        "other": {"type": "string"},
    }})
    assert impossible.fields.keys() == after.fields.keys()


@pytest.mark.parametrize("terminal", [
    {"type": "object", "maxProperties": 0, "properties": {"unreachable": {}}},
    {"type": "object", "properties": {"unreachable": False}},
    {"type": "array", "maxItems": 0, "items": {"properties": {"unreachable": {}}}},
    {"type": "array", "items": False},
])
def test_depth_limit_does_not_warn_for_an_infeasible_edge(terminal):
    schema = terminal
    for _ in range(16):
        schema = {"properties": {"next": schema}}
    index = wire(schema)
    assert len(index.fields) == 32
    assert not any(warning.startswith("depth_cut ") for warning in index.warnings)


def test_relative_server_requires_retrieval_base_and_preserves_slashes():
    schema = json.dumps({
        "openapi": "3.1.0",
        "servers": [{"url": "../api//v{version}/", "variables": {"version": {"default": "2"}}}],
        "paths": {"/a": {"get": {}}},
    }).encode()
    index = load(schema, retrieval_url="https://example.test/catalog/spec/openapi.json")
    assert set(index.operations) == {"op:GET /catalog/api//v2//a"}
    assert any(warning.startswith("server_variable_unchecked ") for warning in index.warnings)
    withheld = load(schema)
    assert not withheld.operations
    assert any(warning.startswith("server_base_unchecked ") for warning in withheld.warnings)
