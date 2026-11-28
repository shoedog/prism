"""The oracle's judged scope is stated positively; shapes outside it are UNDECIDED beside controls."""

import unittest

from measure.corpora import REPO

from . import test_oracle as fixtures

README = REPO / "scripts/measure/README.md"
FORMATS = REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit/FORMATS.md"
SCOPE = "outside-judged-scope"


def rows():
    return fixtures.read_jsonl(fixtures.KIT / "selfcheck" / "judged-scope" / "expected.jsonl")


class JudgedScope(unittest.TestCase):
    def test_every_outside_shape_has_a_judged_control_in_the_same_file(self):
        found = rows()
        outside = [r for r in found if r["verdict"] == "UNDECIDED"]
        self.assertTrue(outside)
        self.assertTrue(all(r["why"] == SCOPE for r in outside))
        controlled = {r["key"]["from"]["file"] for r in found if r["verdict"] == "CORRECT"}
        self.assertEqual({r["key"]["from"]["file"] for r in outside} - controlled, set())
        # The four shapes of the last review: no WRONG anywhere in the case.
        self.assertFalse([r for r in found if r["verdict"] == "WRONG"])
        targets = {r["key"]["to"]["base"] for r in outside}
        self.assertTrue({'"m"', "Foo", "div", "A", "again", "thing"} <= targets)

    def test_both_contracts_state_the_scope_positively_once(self):
        for path in (README, FORMATS):
            with self.subTest(path=path.name):
                text = " ".join(path.read_text().split())
                for phrase in ("Judged scope. The oracle judges a claimed endpoint only when",
                               "binding position or at a reference position",
                               "`oracle/questions/judged_scope.cjs`", "string-keyed element access",
                               "import(\"./mod\").Foo", "intrinsic element"):
                    self.assertIn(phrase, text)
                self.assertNotIn("A member name is a property token", text)


if __name__ == "__main__":
    unittest.main()
