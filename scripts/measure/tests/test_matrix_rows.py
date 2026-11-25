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

TRUTH_KEYS = {
    "run": {"exit", "binary_ran", "receipt_status"},
    "comparator": {"exit", "changes", "unattributed"},
    "join": {"exit", "changes", "unattributed", "records"},
    "oracle": {"verdicts"},
    "review": {"presence_changes", "payload_changes", "ambiguous_bags", "rule_transitions"},
    "gate": {"exit", "reason"},
}
OPERATION_TRUTH_KEYS = {
    ("run", "output admission"): {"exit"},
    ("run", "python syntax"): {"parses_as_python_39"},
    ("comparator", "diff-stats"): {"exit", "deltas"},
    ("review", "row accounting"): {"row_accounting"},
    ("review", "population spec"): {"spec"},
}
FILE_INPUTS = {
    "base", "base_transport", "classifier", "evidence", "external", "file",
    "generator", "head", "head_transport", "setup", "trace",
}


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

    def test_determined_rows_have_required_truth_keys(self):
        for row in self.rows:
            with self.subTest(case=row["id"]):
                truth = row["truth"]
                if "undetermined" in truth:
                    self.assertIsInstance(truth["undetermined"], str)
                    self.assertTrue(truth["undetermined"].strip())
                    continue
                group = row["group"]
                operation = row["input"].get("operation")
                required = OPERATION_TRUTH_KEYS.get((group, operation), TRUTH_KEYS[group])
                if group == "comparator" and truth.get("exit") == 2:
                    required = {"exit"}
                missing = required - truth.keys()
                self.assertFalse(missing, f"missing truth keys: {sorted(missing)}")

    def test_named_inputs_exist_under_the_case_directory(self):
        for row in self.rows:
            inputs = row["input"]
            case = (CASES / inputs["case"]).resolve()
            for field in sorted(inputs.keys() & (FILE_INPUTS | {"repo"})):
                with self.subTest(case=row["id"], field=field):
                    name = inputs[field]
                    self.assertIsInstance(name, str)
                    self.assertFalse(Path(name).is_absolute(), name)
                    path = (case / name).resolve()
                    self.assertIn(case, path.parents, name)
                    if field == "repo":
                        self.assertTrue(path.is_dir(), name)
                    else:
                        self.assertTrue(path.is_file(), name)

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
        # Stage T has zero product rows. Keep this check for future captures.
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
