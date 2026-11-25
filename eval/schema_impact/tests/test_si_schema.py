from hashlib import sha256
import json
from pathlib import Path

from schema_impact.cli import main
from schema_impact.schema import diff, load


FIXTURES = Path(__file__).with_name("fixtures")


def fixture(name):
    return load(FIXTURES / name)


def swagger_response(schema, definitions=None):
    return load(json.dumps({
        "swagger": "2.0",
        "paths": {"/a": {"get": {"responses": {"200": {"schema": schema}}}}},
        "definitions": definitions or {},
    }).encode())


def test_load_mini_swagger():
    idx = fixture("mini.swagger.json")
    operation = "op:GET /api/v1/item/{}"
    assert len(idx.operations) == 4
    assert idx.operations[operation].operation_id == "getItem"
    assert idx.operations[operation].source_path == "/item/{id}"
    title = idx.fields["field:resp:GET /api/v1/item/{}:200#/data/title"]
    assert title.definition == "schema.ItemInfo"
    assert "field:resp:GET /api/v1/item/{}:200#/code" in idx.fields
    assert not any(":400#" in key for key in idx.fields)
    assert idx.definitions_of(operation) == {"schema.Reply", "schema.ItemInfo"}
    assert idx.response_fields(operation, "data") == ["id", "title"]
    assert idx.response_fields("op:GET /api/v1/item", "data") == ["id", "title"]
    assert idx.response_fields("op:GET /missing", "data") == []
    assert idx.info_version == "1.2.3"
    assert idx.sha256 == sha256((FIXTURES / "mini.swagger.json").read_bytes()).hexdigest()
    assert set(key for key in idx.fields if key.startswith("field:req:")) == {
        "field:req:POST /api/v1/item#/title",
    }


def test_recursive_definition_terminates():
    idx = fixture("recursive.json")
    prefix = "field:resp:GET /n:200#/"
    assert idx.fields[prefix + "children"].recursive == "Node"
    assert idx.fields[prefix + "parent"].recursive == "Node"
    assert not any(key.startswith(prefix + "children/[]/") for key in idx.fields)
    assert not any(key.startswith(prefix + "parent/") for key in idx.fields)
    assert idx.definitions_of("op:GET /n") == {"Node"}


def test_recursion_through_container_edges_marks_the_owning_property():
    ref = {"$ref": "#/definitions/Node"}
    idx = swagger_response(ref, {"Node": {"properties": {
        "by_name": {"additionalProperties": ref},
        "groups": {"items": {"items": ref}},
        "label": {"type": "string"},
    }}})
    prefix = "field:resp:GET /a:200#/"
    assert set(idx.fields) == {prefix + name for name in ("by_name", "groups", "label")}
    assert {idx.fields[prefix + name].recursive for name in ("by_name", "groups")} == {"Node"}
    assert idx.fields[prefix + "label"].recursive is None


def test_colliding_paths_with_disjoint_methods_are_both_kept():
    idx = fixture("collision.json")
    assert set(idx.operations) == {"op:GET /a/{}", "op:POST /a/{}"}
    assert idx.warnings == []


def test_true_duplicate_keeps_the_first_sorted_and_warns():
    idx = fixture("duplicate.json")
    assert list(idx.operations) == ["op:GET /a/{}"]
    assert idx.operations["op:GET /a/{}"].source_path == "/a/{id}"
    assert set(idx.fields) == {"field:resp:GET /a/{}:200#/first"}
    assert any("duplicate_operation" in warning for warning in idx.warnings)


def test_diff_kinds():
    changes = diff(fixture("diff_old.json"), fixture("diff_new.json"))
    assert {(change.kind, change.element) for change in changes} == {
        ("op_removed", "op:GET /gone"),
        ("field_removed", "field:resp:GET /a:200#/obsolete"),
        ("field_type_changed", "field:resp:GET /a:200#/value"),
    }


def test_wire_neutral_rename_has_no_change():
    old, new = fixture("rename_old.json"), fixture("rename_new.json")
    assert diff(old, new) == []
    assert old.definitions_of("op:GET /a") == {"schema.A"}
    assert new.definitions_of("op:GET /a") == {"schema.B"}
    assert old.content_hash == new.content_hash
    assert old.sha256 != new.sha256


def test_allof_member_order_and_property_wrapper():
    a, b = fixture("allof_ab.json"), fixture("allof_ba.json")
    assert diff(a, b) == []
    assert a.fields == b.fields
    assert a.operations == b.operations
    assert any("allof_overlap" in warning for warning in a.warnings)
    prefix = "field:resp:GET /b:200#/"
    assert a.fields[prefix + "level"].shape == "integer|int32|false|[]|"
    assert prefix + "mode" not in a.fields


def test_openapi_nearest_servers_and_json_message_selection():
    idx = load(b"""\
openapi: 3.0.3
info: {version: '2.0'}
servers:
  - url: https://{tenant}.example.test/api/{version}/Fixed?skip=1
    variables:
      tenant: {default: test, enum: [test]}
      version: {default: v2, enum: [v2]}
  - url: https://example.test/ignored
paths:
  /items/{name}:
    servers: [{url: /also-ignored}]
    post:
      requestBody:
        $ref: '#/components/requestBodies/Item'
      responses:
        2XX:
          content:
            application/json:
              schema: {properties: {range: {type: boolean}}}
        400:
          $ref: './other.yaml#/never-read'
        201:
          $ref: '#/components/responses/Item'
        2999:
          $ref: './other.yaml#/never-read'
        default:
          $ref: './other.yaml#/never-read'
  /fallback:
    post:
      requestBody:
        content:
          text/plain:
            schema: {properties: {ignored: {type: string}}}
          application/xml:
            schema: {properties: {selected: {type: string}}}
      responses: {}
components:
  requestBodies:
    Item:
      content:
        application/xml:
          schema: {properties: {ignored: {type: string}}}
        application/json:
          schema: {properties: {name: {type: string}}}
  responses:
    Item:
      content:
        application/json:
          schema:
            properties:
              data:
                type: array
                items:
                  $ref: '#/components/schemas/A~1B~0C'
  schemas:
    A/B~C:
      properties:
        name: {type: string}
        flags: {type: object, properties: {nested: {type: boolean}}}
""")
    operation = "op:POST /also-ignored/items/{}"
    assert set(idx.operations) == {operation, "op:POST /api/v2/Fixed/fallback", "op:POST /ignored/fallback"}
    assert idx.info_version == "2.0"
    assert idx.response_fields(operation, "data") == ["flags", "name"]
    assert idx.definitions_of(operation) == {"A/B~C"}
    assert set(idx.fields) == {
        "field:req:POST /also-ignored/items/{}#/name",
        "field:resp:POST /also-ignored/items/{}:201#/data",
        "field:resp:POST /also-ignored/items/{}:201#/data/[]/flags",
        "field:resp:POST /also-ignored/items/{}:201#/data/[]/flags/nested",
        "field:resp:POST /also-ignored/items/{}:201#/data/[]/name",
        "field:resp:POST /also-ignored/items/{}:2XX#/range",
    }
    assert idx.operations[operation].success_statuses == ("201", "2XX")
    assert idx.operations[operation].default_response == {"$ref": "./other.yaml#/never-read"}
    assert any(warning.startswith("non_json_media ") for warning in idx.warnings)
    assert not any("media_type_fallback" in warning for warning in idx.warnings)


def test_yaml_plain_schema_strings_are_preserved():
    idx = load(b"""\
swagger: '2.0'
paths:
  /a:
    get:
      responses:
        200:
          schema:
            properties:
              on: {type: string, enum: [yes, no]}
              day: {type: string, enum: [2026-01-01]}
              ready: {type: boolean, enum: [false, true]}
""")
    prefix = "field:resp:GET /a:200#/"
    assert idx.fields[prefix + "on"].shape == 'string||false|["no","yes"]|'
    assert idx.fields[prefix + "day"].shape == 'string||false|["2026-01-01"]|'
    assert idx.fields[prefix + "ready"].shape == "boolean||false|[false,true]|"


def test_swagger_path_body_parameter_and_response_cut():
    idx = load(b"""\
swagger: '2.0'
paths:
  /a:
    parameters:
      - $ref: '#/parameters/Body'
      - in: formData
        name: ignored
        schema: {properties: {ignored: {type: string}}}
    post:
      parameters:
        - in: body
          name: body
          schema: {properties: {override: {type: string}}}
      responses:
        200: {description: absent}
        201: {schema: {properties: {data: {properties: {late: {type: string}}}}}}
        2XX: {schema: {properties: {ignored: {type: string}}}}
    put:
      responses: {}
parameters:
  Body:
    in: body
    name: body
    schema: {properties: {inherited: {type: string}}}
""")
    assert idx.response_fields("op:POST /a", "data") == []
    assert set(idx.fields) == {
        "field:req:POST /a#/override",
        "field:req:PUT /a#/inherited",
        "field:resp:POST /a:201#/data",
        "field:resp:POST /a:201#/data/late",
    }


def test_union_maps_and_escaped_property_names():
    idx = swagger_response({
        "properties": {
            "data": {
                "oneOf": [{"properties": {"a*b": {"type": "string"}}}],
                "anyOf": [{"properties": {"c/d": {"type": "string"}}}],
                "additionalProperties": {"$ref": "#/definitions/Entry"},
            },
            "scalars": {"additionalProperties": {"type": "integer"}},
        },
    }, {"Entry": {"properties": {"name": {"type": "string"}}}})
    assert idx.response_fields("op:GET /a", "data") == ["a*b", "c/d"]
    assert set(idx.fields) == {
        "field:resp:GET /a:200#/data",
        "field:resp:GET /a:200#/data/a%2Ab",
        "field:resp:GET /a:200#/data/c~1d",
        "field:resp:GET /a:200#/data/*/name",
        "field:resp:GET /a:200#/scalars",
    }
    assert not any("unescaped_property_name" in warning for warning in idx.warnings)
    assert any("oneOf" in warning for warning in idx.warnings)
    assert any("anyOf" in warning for warning in idx.warnings)


def test_depth_cut_and_ref_is_local():
    nested = {"$ref": "./external.json#/Forbidden"}
    for _ in range(17):
        nested = {"properties": {"next": nested}}
    idx = swagger_response(nested)
    assert len(idx.fields) == 16
    assert any("depth_cut" in warning for warning in idx.warnings)
    assert not any("external.json" in warning for warning in idx.warnings)

    idx = swagger_response({"properties": {"next": {"$ref": "./external.json#/Forbidden"}}})
    assert "field:resp:GET /a:200#/next" in idx.fields
    assert idx.definitions_of("op:GET /a") == set()
    assert any("external.json" in warning for warning in idx.warnings)


def test_diff_added_required_enum_format_and_items():
    old = swagger_response({"properties": {
        "modes": {"type": "string", "enum": ["c", "b", "a"]},
        "number": {"type": "integer", "format": "int32"},
        "items": {"type": "array", "items": {"$ref": "#/definitions/Item"}},
    }}, {"Item": {"type": "integer"}})
    new = swagger_response({
        "required": ["number"],
        "properties": {
            "modes": {"type": "string", "enum": ["b", "c"]},
            "number": {"type": "integer", "format": "int64"},
            "items": {"type": "array", "items": {"$ref": "#/definitions/Item"}},
            "extra": {"type": "boolean"},
        },
    }, {"Item": {"type": "string"}})
    prefix = "field:resp:GET /a:200#/"
    assert old.fields[prefix + "modes"].shape == 'string||false|["a","b","c"]|'
    assert old.fields[prefix + "items"].shape == "array||false|[]|integer"
    assert old.definitions_of("op:GET /a") == {"Item"}
    changes = diff(old, new)
    assert {(change.kind, change.element) for change in changes} == {
        ("field_added", prefix + "extra"),
        ("field_type_changed", prefix + "items"),
        ("field_type_changed", prefix + "number"),
        ("field_required_added", prefix + "number"),
        ("enum_values_changed", prefix + "modes"),
    }
    required = next(change for change in changes if change.kind == "field_required_added")
    assert required.old_shape == old.fields[prefix + "number"].shape
    assert required.new_shape == new.fields[prefix + "number"].shape
    added = diff(load(b'{"swagger":"2.0","paths":{}}'), old)
    assert {change.kind for change in added} == {"op_added", "field_added"}


def test_cli_prints_sorted_ids(capsys):
    path = FIXTURES / "mini.swagger.json"
    index = load(path)
    assert main(["ids", str(path)]) == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == [*sorted(index.operations), *sorted(index.fields)]
    assert captured.err == ""
