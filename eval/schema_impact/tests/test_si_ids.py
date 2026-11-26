from urllib.parse import unquote

import pytest

from schema_impact.ids import field_id, normalize_path, op_id


def test_normalize_path():
    assert normalize_path("/api/v1/item/{id}/") == "/api/v1/item/{}"
    assert normalize_path("item/{id}", base="/api/v1") == "/api/v1/item/{}"
    assert normalize_path("/", base="/") == "/"
    assert normalize_path("/a//b", base="") == "/a/b"
    assert normalize_path("/f/{id}.json") == "/f/{}.json"
    assert normalize_path("/Entry/:id.json/%2F", base="//Api//") == "/Api/Entry/{}.json/%2F"


def test_op_id():
    assert op_id("get", "/a/{x}/b") == "op:GET /a/{}/b"


def test_field_ids():
    assert (
        field_id("resp:GET /api/tokens:200", ["data", "key"])
        == "field:resp:GET /api/tokens:200#/data/key"
    )
    assert field_id("req:POST /a", ["x"]) == "field:req:POST /a#/x"
    assert field_id("req:POST /a", ["a/b~#*[]"]) == "field:req:POST /a#/a~1b~0%23%2A%5B%5D"


@pytest.mark.parametrize(("dialect", "raw", "expected"), [
    ("gin", "/mugs/*trail", "/mugs/{*0+}"),
    ("express4", "/mugs/*", "/mugs/{*0+}"),
    ("express4", "/mugs/:part?", "/mugs/{?}"),
    ("express5", "/mugs/*trail", "/mugs/{*}"),
    ("express5", "/mugs/{*trail}", "/mugs/{*0+}"),
    ("express5", "/{*trail}", "/{*0+}"),
    ("hapi", "/mugs/{trail*}", "/mugs/{*0+}"),
    ("hapi", "/mugs/{trail*2}", "/mugs/{*2}"),
    ("hapi", "/mugs/wax-{part?}/proof", "/mugs/wax-{?}/proof"),
])
def test_route_capture_spelling(dialect, raw, expected):
    assert normalize_path(raw, dialect=dialect) == expected
    assert normalize_path("/mugs/%7Bpart%7D", dialect=dialect) == "/mugs/%7Bpart%7D"


def test_literal_pointer_segments_round_trip():
    names = ["", "[]", "*", "~1", "/", "%23", "#", "a? b", "é", "e\u0301", "\x00\t\n"]
    encoded = [field_id("req:POST /a", [name]).split("#/", 1)[1] for name in names]
    assert len(set(encoded)) == len(names)
    assert [unquote(part).replace("~1", "/").replace("~0", "~") for part in encoded] == names
