import csv
import json
import re
from pathlib import Path

import pytest

from schema_impact.schema import load


ROOT = Path(__file__).resolve().parents[3]
MATRIX = ROOT / "crates/prism-boundary/tests/fixtures/ids_matrix"
DESIGN = ROOT / "docs/superpowers/plans/2026-11-20-lanes/lane5-schema-impact/DESIGN.md"

# These rows require sources beyond an entry schema and its retrieval URL.
MULTI_SOURCE = {
    "oas-server-variable-distinct-operations": "two schema inventories",
    "s1-oas31-authority-dispatch": "two authority-scoped schema inventories",
    "s1-oas31-external-path-item-server": "external Path Item and its retrieval context",
    "t1-ref-external-relative": "external schema resource",
    "t1-external-pointer-escaped": "external schema resource",
}


def rule_rows():
    ledger = DESIGN.read_text().split("#### E-1 element identity\n", 1)[1]
    ledger = ledger.split("#### E-2 consumer\n", 1)[0].split("| Matrix class", 1)[1]
    rule_ids = set(re.findall(r"`([^`]+)`", ledger))
    with (MATRIX / "rows.tsv").open(newline="") as stream:
        rows = {row["id"]: row for row in csv.DictReader(stream, delimiter="\t")}
    assert len(rows) == 217 and len(rule_ids) == 170
    return [row for key, row in rows.items() if key in rule_ids]


RULE_ROWS = rule_rows()
SCHEMA_ROWS = [row for row in RULE_ROWS if row["fixture"].endswith(".json")]
SINGLE_SCHEMA_ROWS = [row for row in SCHEMA_ROWS if row["id"] not in MULTI_SOURCE]


def test_matrix_partition():
    assert len(RULE_ROWS) == 170
    assert len(SINGLE_SCHEMA_ROWS) == 128
    assert {row["id"] for row in SCHEMA_ROWS} - {row["id"] for row in SINGLE_SCHEMA_ROWS} == MULTI_SOURCE.keys()
    assert sum(row["fixture"].endswith(".go") for row in RULE_ROWS) == 15
    assert sum(row["fixture"].endswith(".js") for row in RULE_ROWS) == 22


@pytest.mark.parametrize("row", SINGLE_SCHEMA_ROWS, ids=lambda row: row["id"])
def test_rule_row_ids(row):
    fixture = MATRIX / row["fixture"]
    context = fixture.with_name("context.json")
    options = json.loads(context.read_text()) if context.is_file() else {}
    expected_ids = row["expected_ids"]
    # The hashed witness table retains the old spelling for these two rows.
    if row["id"] == "field-embedded-brackets":
        expected_ids = expected_ids.replace("tone[]wet", "tone%5B%5Dwet")
    if row["id"] == "field-embedded-star":
        expected_ids = expected_ids.replace("tone*wet", "tone%2Awet")
    index = load(fixture, **options)
    assert index.operations.keys() | index.fields.keys() == set(filter(None, expected_ids.split(";")))
    warnings = {warning.split()[0] for warning in index.warnings}
    superseded = {"unescaped_property_name", "media_type_fallback"}
    if row["id"] in {"oas-operation-server-overrides", "oas-path-server-overrides"}:
        superseded.add("duplicate_operation")
    assert not warnings.intersection(superseded)
    assert set(filter(None, row["expected_warnings"].split(";"))) - superseded <= warnings
