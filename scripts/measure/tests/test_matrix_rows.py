"""Validate the Stage T fixture inventory without running the kit."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

MATRIX = Path(__file__).resolve().parents[1] / "matrix"
CASES = MATRIX / "cases"
FIELDS = {"id", "group", "input", "required", "truth", "source"}
PREFIXES = {
    "run": "run",
    "comparator": "cmp",
    "join": "join",
    "oracle": "oracle",
    "review": "review",
    "gate": "gate",
}
SOURCES = {"probe", "product", "synthetic"}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field {key!r}")
        result[key] = value
    return result


def read_jsonl(path):
    with path.open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, 1):
            try:
                yield json.loads(line, object_pairs_hook=unique_object)
            except ValueError as error:
                raise AssertionError(f"{path.relative_to(MATRIX)}:{number}: {error}") from error


class MatrixRowsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = list(read_jsonl(MATRIX / "rows.jsonl"))

    def test_row_schema_and_unique_ids(self):
        self.assertTrue(self.rows)
        found = set()
        for number, row in enumerate(self.rows, 1):
            with self.subTest(line=number):
                self.assertIsInstance(row, dict)
                self.assertEqual(set(row), FIELDS)
                self.assertIn(row["group"], PREFIXES)
                self.assertIn(row["source"], SOURCES)
                self.assertIsInstance(row["id"], str)
                self.assertIsNotNone(re.fullmatch(PREFIXES[row["group"]] + r"-[0-9]{3}", row["id"]))
                self.assertNotIn(row["id"], found)
                found.add(row["id"])
                self.assertIsInstance(row["input"], dict)
                self.assertEqual(row["input"].get("case"), row["id"])
                self.assertIsInstance(row["required"], str)
                self.assertTrue(row["required"].strip())
                self.assertIsInstance(row["truth"], dict)

    def test_case_directories_match_the_rows_and_contain_files(self):
        referenced = {row["input"]["case"] for row in self.rows}
        actual = {path.name for path in CASES.iterdir() if path.is_dir()}
        self.assertEqual(actual, referenced)
        for name in sorted(referenced):
            with self.subTest(case=name):
                self.assertTrue(any(path.is_file() for path in (CASES / name).rglob("*")))

    def test_case_jsonl_records_parse(self):
        for path in sorted(CASES.rglob("*.jsonl")):
            with self.subTest(file=path.relative_to(CASES)):
                for _ in read_jsonl(path):
                    pass

    def test_product_rows_retain_fixture_source_and_raw_output(self):
        for row in self.rows:
            if row["source"] != "product":
                continue
            with self.subTest(case=row["id"]):
                inputs = row["input"]
                case = (CASES / inputs["case"]).resolve()
                self.assertIsInstance(inputs.get("fixture"), str)
                fixture = (case / inputs["fixture"]).resolve()
                self.assertIn(case, fixture.parents)
                fixture_files = [fixture] if fixture.is_file() else list(fixture.rglob("*"))
                self.assertTrue(any(file.is_file() and file.stat().st_size for file in fixture_files))
                self.assertIsInstance(inputs.get("raw"), list)
                self.assertTrue(inputs["raw"])
                for name in inputs["raw"]:
                    output = (case / name).resolve()
                    self.assertIn(case, output.parents)
                    self.assertTrue(output.is_file() and output.stat().st_size, name)

    def test_all_inventory_disagreements_are_referenced(self):
        cited = set()
        for row in self.rows:
            for number in row["input"].get("inventory", []):
                self.assertIs(type(number), int, row["id"])
                cited.add(number)
        self.assertEqual(cited, set(range(1, 19)))


if __name__ == "__main__":
    unittest.main()
