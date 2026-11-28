"""Round-4 review guards: trace environment keys are validated as written."""

import unittest

from measure.corpora import REPO

from .comparator_fixtures import ComparatorFixture, edge

README = REPO / "scripts/measure/README.md"
ROWS = REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit/FORMATS-OPEN-ROWS.md"


class TraceEnvironmentKeyTests(ComparatorFixture):
    """An invalid environment key is refused even when options and command env agree."""

    def attempt(self, key, expected):
        self.arm("base", [edge()])
        self.arm("head", [edge()], trace=b"", trace_env={key: "1"})
        self.invoke("diff-stats", expected=expected,
                    diagnostic="run binding" if expected else None)

    def test_a_key_that_only_reparses_as_a_valid_name_is_refused(self):
        self.attempt("PRISM_TRACE=undeclared", 2)

    def test_other_invalid_keys_are_refused(self):
        for key in ("prism_trace", "PRISM_", "PRISM_X\n", "OTHER_X", "PRISM_EDGES_BYTES"):
            with self.subTest(key=key):
                self.attempt(key, 2)

    def test_a_declared_name_is_admitted(self):
        self.attempt("PRISM_TEST_TRACE", 0)


class ReadmeAndRowsTests(unittest.TestCase):
    def test_the_digit_limit_is_the_active_setting_with_4300_as_the_default(self):
        text = " ".join(README.read_text().split())
        self.assertIn("4,300 digits is CPython's default", text)
        self.assertIn("follows the interpreter's active integer string conversion limit", text)
        self.assertIn("5,000", text)
        self.assertIn("641-digit", text)

    def test_the_map_table_marks_every_map_closed_or_free_form(self):
        text = README.read_text()
        table = text[text.index("| Map | Keys |"):text.index("Known limits of S2")]
        rows = [line for line in table.splitlines()[2:] if line.startswith("|")]
        self.assertGreaterEqual(len(rows), 10)
        for row in rows:
            self.assertTrue("closed keys" in row or "free-form" in row, row)

    def test_the_two_s2_findings_have_executable_closures(self):
        rows = {line.split("|")[1].strip(): line for line in ROWS.read_text().splitlines()
                if line.startswith("| open-s2-")}
        closures = {
            "open-s2-1": "test_validator_list_is_materialized_inside_validation",
            "open-s2-2": "test_inventory_paths_are_closed_but_raw_edge_and_generic_trace_are_extensible",
        }
        self.assertEqual(set(rows), set(closures))
        for ident, test in closures.items():
            self.assertTrue(rows[ident].rstrip().endswith(f"| WRONG IMMATERIAL | closed: `{test}` |"))


if __name__ == "__main__":
    unittest.main()
