import csv
import json
from pathlib import Path

import pytest

from schema_impact.config import ClientSpec
from schema_impact.consumer_ts import ClientCall, pattern
from schema_impact.match import bind
from schema_impact.schema import load


MATCH_MATRIX = Path(__file__).parent / "fixtures" / "match_matrix.tsv"


def index_with(*operations):
    paths = {}
    for identifier in operations:
        method, path = identifier.removeprefix("op:").split(" ", 1)
        paths.setdefault(path, {})[method.lower()] = {}
    return load(json.dumps({"swagger": "2.0", "paths": paths}).encode())


def call(method, path, reason=None):
    return ClientCall("src/site.ts", 1, method, "", path, "literal", reason, None, None)


def test_match_scoring():
    idx = index_with("op:GET /user/{}", "op:GET /user/me", "op:POST /user/{}")
    assert bind(call("GET", "/user/{}"), idx) == ("bound", ["op:GET /user/{}"], None)
    assert bind(call("GET", "/user/me"), idx) == ("bound", ["op:GET /user/me"], None)
    assert bind(call("DELETE", "/user/{}"), idx) == ("unbound", [], "no_operation")


def test_client_parameter_on_schema_literal_is_never_bound():
    assert bind(call("GET", "/user/{}"), index_with("op:GET /user/me")) == (
        "candidate", ["op:GET /user/me"], "client_parameter_on_literal"
    )


def test_client_literal_on_schema_parameter_binds_when_unique():
    idx = index_with("op:GET /user/{}", "op:GET /user/me")
    assert bind(call("GET", "/user/you"), idx) == ("bound", ["op:GET /user/{}"], None)


def test_pre_match_reason_blocks_matching():
    idx = index_with("op:GET /user/me")
    assert bind(call("GET", "/user/me", "unknown_client_method"), idx) == (
        "unbound", [], "unknown_client_method"
    )


with MATCH_MATRIX.open(newline="") as stream:
    MATRIX = list(csv.DictReader(stream, delimiter="\t"))


@pytest.mark.parametrize("row", MATRIX, ids=[row["id"] for row in MATRIX])
def test_match_matrix(row):
    spec = ClientSpec(strip_prefix="/legacy", add_prefix="/api") if row["id"] == "prefix" else None
    path, url_class = pattern(row["pattern"], spec=spec)
    assert url_class == row["class"]
    expected_ids = row["operation_ids"].split("|") if row["operation_ids"] else []
    assert bind(call(row["method"], path), index_with(*row["operations"].split("|"))) == (
        row["grade"], expected_ids, row["reason"] or None
    )
