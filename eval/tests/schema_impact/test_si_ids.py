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
        field_id("resp:GET /api/tokens:200", ["data", "[]", "key"])
        == "field:resp:GET /api/tokens:200#/data/[]/key"
    )
    assert field_id("req:POST /a", ["x"]) == "field:req:POST /a#/x"
    assert field_id("req:POST /a", ["a/b~#*[]"]) == "field:req:POST /a#/a/b~#*[]"
