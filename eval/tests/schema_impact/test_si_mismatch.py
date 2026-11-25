import json

import pytest

from schema_impact import schema
from schema_impact.cli import main


def _call(line, method, pattern, *, grade="unbound", reason="no_operation", operation_ids=()):
    return {
        "file": "client.ts", "line": line, "method": method, "url_text": pattern,
        "pattern": pattern, "url_class": "template" if pattern and "{}" in pattern else "literal",
        "reason": reason if reason == "dynamic_url" else None,
        "response_type": None, "enclosing_function": None, "operation_ids": list(operation_ids),
        "grade": grade, "match_reason": reason,
    }


@pytest.fixture
def census(tmp_path):
    schema_path = tmp_path / "schema.json"
    schema_path.write_text(json.dumps({
        "swagger": "2.0",
        "basePath": "/api",
        "paths": {
            "/stock/{sku}": {"get": {}, "post": {}},
            "/orphan": {"get": {}},
            "/flag/current": {"get": {}},
        },
    }))
    index = schema.load(schema_path)
    producer = tmp_path / "producer"
    producer.mkdir()
    list_2 = {
        "row_id": "f1-2-0001", "list": 2, "element": "op:GET /api/orphan",
        "evidence": {"annotation": ["// @Router /api/orphan [get]"]},
    }
    list_3 = {
        "row_id": "f1-3-0001", "list": 3, "registration": 'r.HEAD("/orphan", Check)',
    }
    list_4 = {
        "row_id": "f1-4-0001", "list": 4, "element": "op:GET /api/orphan",
        "evidence": {"registration": [list_3["registration"]], **list_2["evidence"]},
    }
    cross_check = {"row_id": "f1-cross_check-0001", "list": "cross_check"}
    (producer / "f1.json").write_text(json.dumps({
        "schema_sha256": index.sha256, "residue": [list_2, list_3, list_4, cross_check],
    }))
    consumer = tmp_path / "consumer"
    consumer.mkdir()
    calls = [
        _call(1, "GET", "/api/unknown"),
        _call(2, "DELETE", "/api/stock/blue"),
        _call(3, "GET", None, reason="dynamic_url"),
        _call(4, "GET", "/api/orphan", grade="bound", reason=None, operation_ids=("op:GET /api/orphan",)),
        _call(5, "GET", "/api/flag/{}", grade="candidate", reason="client_parameter_on_literal",
              operation_ids=("op:GET /api/flag/current",)),
        _call(6, "POST", "/api/flag/{}"),
    ]
    (consumer / "consumer.jsonl").write_text("".join(json.dumps(call) + "\n" for call in calls))
    (consumer / "f2.json").write_text(json.dumps({
        "schema_sha256": index.sha256, "denominator": len(calls),
    }))
    return producer, consumer, schema_path, index


def test_standing_keeps_the_four_lists_disjoint_for_client_calls(census):
    from schema_impact import mismatch

    producer, consumer, _, index = census
    lists = mismatch.standing(producer, consumer, index)
    assert {key: len(rows) for key, rows in lists.items()} == {"1": 1, "2": 1, "3": 1, "4": 3}
    assert lists["1"][0]["line"] == 1
    assert lists["1"][0]["match_reason"] == "no_operation"
    assert lists["2"][0]["element"] == "op:GET /api/orphan"
    assert lists["3"][0]["registration"] == 'r.HEAD("/orphan", Check)'
    producer_method, literal, template = lists["4"]
    assert producer_method["f1_row_id"] == "f1-4-0001"
    assert producer_method["evidence"]["annotation"] == ["// @Router /api/orphan [get]"]
    assert literal["line"] == 2
    assert literal["other_method_operations"] == [
        {"element": "op:GET /api/stock/{}", "source_path": "/stock/{sku}"},
        {"element": "op:POST /api/stock/{}", "source_path": "/stock/{sku}"},
    ]
    assert template["line"] == 6
    assert template["other_method_operations"] == [
        {"element": "op:GET /api/flag/current", "source_path": "/flag/current"},
    ]
    rows = [row for key, group in lists.items() for row in group if row["list"] == int(key)]
    assert len({row["row_id"] for row in rows}) == len(rows) == 6


@pytest.mark.parametrize(("method", "pattern"), [(None, "/api/orphan"), ("GET", None)])
def test_standing_needs_both_method_and_path_for_client_disagreement(census, method, pattern):
    from schema_impact import mismatch

    producer, consumer, _, index = census
    census_path = consumer / "consumer.jsonl"
    with census_path.open("a", encoding="utf-8") as stream:
        stream.write("\n" + json.dumps(_call(7, method, pattern)) + "\n")
    receipt_path = consumer / "f2.json"
    receipt = json.loads(receipt_path.read_text())
    receipt["denominator"] += 1
    receipt_path.write_text(json.dumps(receipt))

    lists = mismatch.standing(producer, consumer, index)
    assert [row["line"] for row in lists["1"]] == [1, 7]
    assert len(lists["4"]) == 3


@pytest.mark.parametrize("receipt", ["f1.json", "f2.json"])
def test_standing_rejects_receipt_with_another_schema(census, receipt):
    from schema_impact import mismatch

    producer, consumer, _, index = census
    path = (producer if receipt == "f1.json" else consumer) / receipt
    document = json.loads(path.read_text())
    document["schema_sha256"] = "0" * 64
    path.write_text(json.dumps(document))
    with pytest.raises(ValueError, match=rf"{receipt}.*schema"):
        mismatch.standing(producer, consumer, index)


def test_mismatch_cli_writes_all_groups(census, tmp_path):
    producer, consumer, schema_path, _ = census
    out = tmp_path / "run" / "out"
    assert main([
        "mismatch", "--producer", str(producer), "--consumer", str(consumer),
        "--schema", str(schema_path), "--out", str(out),
    ]) == 0
    lists = json.loads((out / "f8.json").read_text())
    assert {key: len(rows) for key, rows in lists.items()} == {"1": 1, "2": 1, "3": 1, "4": 3}
