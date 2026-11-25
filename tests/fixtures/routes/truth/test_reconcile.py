"""Handwritten checks for the executed-truth projection."""
from contextlib import redirect_stdout
from copy import deepcopy
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from reconcile import compare_row, read_jsonl, report


def example():
    site = {"file": "app.js", "line": 7}
    callback = {"position": 0, "file": "app.js", "name": "<anon@7:24>", "start_line": 7}
    path = {"raw": "'/:id'", "literal": "/:id"}
    exposure = {"status": "full", "value": "/api/:id", "normalized": "/api/{}"}
    expected = {
        "framework": "express", "site": dict(site), "methods": {"known": ["GET"]},
        "path": dict(path), "paths": [dict(exposure)], "callbacks": [dict(callback)],
    }
    row = {"id": "example", "expect": [expected]}
    truth = {
        "status": "executed",
        "projection": [{
            "call_id": 1, "framework": "express", "site": site, "path": path,
            "paths": [exposure], "methods": {"known": ["GET"]}, "callbacks": [callback],
        }],
        "calls": [{
            "id": 1, "site": {**site, "ordinal": 1}, "args": {"methods": ["GET"]},
            "callbacks": [{"kind": "javascript", "file": "app.js", "name": "<anon@7:24>",
                           "first_line": 7}],
        }],
        "served": [{
            "call_ids": [1], "path": "/api/:id", "declared_methods": ["GET"],
            "effective_methods": ["GET", "HEAD", "OPTIONS"],
        }],
    }
    return row, truth


class ProjectionTests(unittest.TestCase):
    def setUp(self):
        self.row, self.truth = example()
        self.pin = self.row["expect"][0]

    def compare(self):
        return compare_row(self.row, self.truth)

    def assert_fields(self, *fields):
        self.assertEqual([diff.field for diff in self.compare().differences], list(fields))

    def test_full_is_exact_even_with_a_matching_suffix(self):
        self.pin["paths"] = [{"status": "full", "value": "/:id"}]
        self.assert_fields("paths[0].value")
        self.pin["paths"][0]["value"] = "/api/:id"
        self.assert_fields()

    def test_full_preserves_trailing_slash(self):
        self.pin["paths"][0]["value"] = "/api/:id/"
        self.assert_fields("paths[0].value")
        self.pin["paths"][0]["value"] = "/api/:id"
        self.assert_fields()

    def test_pinned_exposure_among_multiple_served_paths(self):
        extra = {**self.truth["served"][0], "path": "/also/:id"}
        self.truth["served"].append(extra)
        self.pin["paths"] = [{"status": "full", "value": "/missing/:id"}]
        self.assert_fields("paths[0].value")
        self.pin["paths"][0]["value"] = "/also/:id"
        self.assert_fields()

    def test_suffix_and_normalized_suffix(self):
        self.pin["paths"] = [{"status": "suffix", "value": "/api", "normalized": "/bad"}]
        self.assert_fields("paths[0].value")
        self.pin["paths"][0]["value"] = "/:id"
        self.assert_fields("paths[0].normalized")
        self.pin["paths"][0]["normalized"] = "/{}"
        self.assert_fields()

    def test_raw_literal_and_normalized_are_separate_pins(self):
        self.pin["path"] = {"raw": "variable", "literal": "/wrong"}
        self.pin["paths"][0]["normalized"] = "/wrong"
        self.assert_fields("path.raw", "path.literal", "paths[0].normalized")
        del self.pin["path"]["literal"]
        self.pin["path"]["raw"] = "'/:id'"
        del self.pin["paths"][0]["normalized"]
        self.assert_fields()

    def test_effective_methods_do_not_satisfy_declared_methods(self):
        self.pin["methods"] = {"known": ["GET", "HEAD", "OPTIONS"]}
        self.assert_fields("methods")
        self.pin["methods"] = {"known": ["GET"]}
        self.assert_fields()

    def test_implicit_methods_allow_any_or_no_pin(self):
        self.truth["calls"][0]["args"]["methods"] = None
        self.truth["served"][0]["declared_methods"] = None
        del self.truth["projection"][0]["methods"]
        self.assert_fields("methods")
        self.pin["methods"] = "any"
        self.assert_fields()
        del self.pin["methods"]
        self.assert_fields()
        self.assertNotIn("methods", self.compare().checked)

    def test_explicit_any_does_not_match_an_explicit_known_set(self):
        self.pin["methods"] = "any"
        self.assert_fields("methods")
        self.truth["projection"][0]["methods"] = {"any": True}
        self.truth["calls"][0]["args"]["methods"] = "ANY"
        self.truth["served"][0]["declared_methods"] = "ANY"
        self.assert_fields()

    def test_callback_file_name_start_line_and_position(self):
        self.pin["callbacks"] = [{
            "position": 1, "file": "other.js", "name": "<anon@7:25>", "start_line": 8,
        }]
        self.assert_fields("callbacks[0].position", "callbacks[0].file",
                           "callbacks[0].name", "callbacks[0].start_line")
        self.pin["callbacks"] = [dict(self.truth["projection"][0]["callbacks"][0])]
        self.assert_fields()

    def test_raw_callback_identity_remains_checkable_without_projection(self):
        del self.truth["projection"][0]["callbacks"]
        self.pin["callbacks"][0]["name"] = "wrong"
        self.assert_fields("callbacks[0].name")
        self.pin["callbacks"][0]["name"] = "<anon@7:24>"
        self.assert_fields()

    def test_empty_callback_pin_detects_an_extra_callback(self):
        self.pin["callbacks"] = []
        self.assert_fields("callbacks.length")
        del self.pin["callbacks"]
        self.assert_fields()

    def test_site_file_and_line(self):
        self.pin["site"] = {"file": "other.js", "line": 8}
        self.assert_fields("site.file", "site.line")
        self.pin["site"] = {"file": "app.js", "line": 7}
        self.assert_fields()

    def test_same_line_requires_ordinal_and_matches_by_it(self):
        second = deepcopy(self.pin)
        second["methods"] = {"known": ["POST"]}
        self.row["expect"].insert(0, second)
        actual = deepcopy(self.truth["projection"][0])
        actual.update(call_id=2, methods={"known": ["POST"]})
        self.truth["projection"].append(actual)
        call = deepcopy(self.truth["calls"][0])
        call.update(id=2, args={"methods": ["POST"]})
        call["site"]["ordinal"] = 2
        self.truth["calls"].append(call)
        self.truth["served"][0]["call_ids"].append(2)
        self.truth["served"][0]["declared_methods"].append("POST")
        self.assert_fields("site.ordinal", "site.ordinal")
        self.pin["site"]["ordinal"] = 1
        second["site"]["ordinal"] = 3
        self.assert_fields("site.ordinal")
        second["site"]["ordinal"] = 2
        self.assert_fields()
        self.pin["methods"] = {"known": ["POST"]}
        second["methods"] = {"known": ["GET"]}
        self.assert_fields("methods", "methods")

    def test_static_fields_are_reported_and_position_still_compares(self):
        self.pin["receiver"] = {"status": "unproven", "reason": "receiver_unproven"}
        self.pin["methods"] = {"status": "unknown", "text": "M"}
        self.pin["paths"] = [{
            "status": "suffix", "value": "/:id", "reason": "variable_prefix", "root": "app",
        }]
        self.pin["callbacks"] = [{
            "position": 1, "status": "unresolved", "text": "wrap(h)", "reason": "wrapper_call",
        }]
        self.row["expect_mounts"] = [{"reason": "mount_child_unproven"}]
        self.assert_fields("callbacks[0].position")
        self.pin["callbacks"][0]["position"] = 0
        result = self.compare()
        self.assert_fields()
        for field in ("receiver", "methods", "paths[0].root", "paths[0].reason",
                      "callbacks[0].reason", "expect_mounts"):
            with self.subTest(field=field):
                self.assertIn(field, result.static_only)

    def test_unserved_static_suffix_and_dynamic_path(self):
        self.pin["paths"] = [{"status": "suffix", "value": "/wrong", "reason": "no_known_mount"}]
        self.assert_fields("paths[0].value")
        self.truth["served"].clear()
        self.truth["projection"][0]["paths"].clear()
        self.pin["paths"] = [{"status": "suffix", "value": "/:id", "reason": "variable_prefix"}]
        self.assert_fields("paths[0].value")
        self.pin["paths"][0]["reason"] = "no_known_mount"
        self.assert_fields()
        self.assertIn("paths[0].value", self.compare().static_only)
        self.pin["paths"] = [{"status": "dynamic", "reason": "non_literal_path"}]
        self.assert_fields()
        self.assertIn("paths[0].status", self.compare().static_only)

    def test_static_candidate_without_a_framework_call(self):
        self.truth["projection"].clear()
        self.truth["calls"].clear()
        self.truth["served"].clear()
        self.assert_fields("registrations")
        self.pin["receiver"] = {"status": "unproven", "reason": "receiver_unproven"}
        self.pin["paths"] = []
        self.assert_fields()
        self.assertIn("registrations", self.compare().static_only)

    def test_empty_expectation_detects_unexpected_registration(self):
        self.row["expect"] = []
        self.assert_fields("registrations")
        self.truth["projection"].clear()
        self.truth["calls"].clear()
        self.truth["served"].clear()
        self.assert_fields()


class ReportTests(unittest.TestCase):
    def run_report(self, row, truth):
        output = StringIO()
        with redirect_stdout(output):
            status = report({row["id"]: (1, row)}, truth)
        return status, output.getvalue()

    def test_missing_truth_fails_and_successful_truth_agrees(self):
        row, truth = example()
        row["truth_source"] = "executed"
        status, output = self.run_report(row, {})
        self.assertEqual(status, 1)
        self.assertIn("example: missing truth", output)
        status, output = self.run_report(row, {"example": (2, truth)})
        self.assertEqual(status, 0)
        self.assertIn("example (truth line 2): agrees", output)

    def test_hand_row_keeps_refusal_and_rejects_successful_truth(self):
        row, truth = example()
        row["truth_source"] = "hand"
        status, output = self.run_report(row, {"example": (2, truth)})
        self.assertEqual(status, 1)
        self.assertIn("unexpected truth for hand row", output)
        refused = {"status": "hand_required", "error": "untraced mount"}
        row["truth_source"] = "executed"
        status, output = self.run_report(row, {"example": (2, refused)})
        self.assertEqual(status, 1)
        self.assertIn("truth line 2, hand_required", output)
        row["truth_source"] = "hand"
        status, output = self.run_report(row, {"example": (2, refused)})
        self.assertEqual(status, 0)
        self.assertIn("truth line 2 refused: untraced mount", output)

    def test_read_jsonl_rejects_bad_or_duplicate_records_with_line_number(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / "rows.jsonl"
            path.write_text('{"id":"x"}\n{"id":"x"}\n', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, r"rows\.jsonl:2: .*duplicate ID"):
                read_jsonl(path)
            path.write_text('{"id":"x"}\n}\n', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, r"rows\.jsonl:2:"):
                read_jsonl(path)
            path.write_text('{"id":"x"}\n{"id":"y"}\n', encoding="utf-8")
            self.assertEqual(read_jsonl(path)["y"], (2, {"id": "y"}))


if __name__ == "__main__":
    unittest.main()
