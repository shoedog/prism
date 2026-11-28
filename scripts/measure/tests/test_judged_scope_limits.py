"""Known limits of the judged scope: each is pinned beside a control, with the verdict the scope requires."""

import json
import unittest

from measure.corpora import REPO

from . import test_oracle as fixtures

REGISTRY = fixtures.KIT / "judged_scope_limits.json"
CASE = fixtures.KIT / "selfcheck" / "judged-scope-limits"
README = REPO / "scripts/measure/README.md"
FORMATS = REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit/FORMATS.md"


def verdict(golden):
    return golden["verdict"] + (" " + golden["why"] if golden.get("why") else "")


class JudgedScopeLimits(unittest.TestCase):
    def setUp(self):
        self.registry = json.loads(REGISTRY.read_text())
        self.goldens = fixtures.read_jsonl(CASE / "expected.jsonl")

    def test_every_row_is_claimed_once_and_today_matches_the_golden(self):
        claimed = []
        for limit in self.registry["limits"] + [self.registry["extra_controls"]]:
            for pinned in limit["pinned"]:
                claimed.append(pinned["row"])
                self.assertEqual(verdict(self.goldens[pinned["row"] - 1]), pinned["today"], limit["id"])
                self.assertNotEqual(pinned["today"], pinned["required"], limit["id"])
            for control in limit["controls"]:
                claimed.append(control["row"])
                self.assertEqual(verdict(self.goldens[control["row"] - 1]), control["verdict"], limit["id"])
        self.assertEqual(sorted(claimed), list(range(1, len(self.goldens) + 1)))

    def test_every_limit_has_a_control_in_the_same_file_and_relation(self):
        for limit in self.registry["limits"]:
            with self.subTest(limit=limit["id"]):
                self.assertTrue(limit["pinned"] and limit["controls"] and limit["note"].strip())
                def where(row):
                    golden = self.goldens[row - 1]
                    return golden["key"]["from"]["file"], golden["relation"]
                self.assertTrue({where(p["row"]) for p in limit["pinned"]}
                                <= {where(c["row"]) for c in limit["controls"]})

    def test_a_control_is_judged_as_the_scope_requires(self):
        for limit in self.registry["limits"]:
            for control in limit["controls"]:
                # A control is never a pinned miss: its verdict is the scope's own answer for its row.
                self.assertNotIn(control["row"], {p["row"] for p in limit["pinned"]})
        # Only the overload exception keeps a decided WRONG against the strict reading.
        decided_wrong = {limit["id"] for limit in self.registry["limits"]
                         for pinned in limit["pinned"]
                         if pinned["today"].startswith("WRONG") and pinned["required"] == "UNDECIDED"}
        self.assertEqual(decided_wrong, {"erased-signature-formals", "overload-formals"})

    def test_incidence_on_the_real_inputs_is_zero_and_documented(self):
        self.assertEqual(self.registry["incidence"],
                         {"Express": "0 of 5,040 decided rows", "Excalidraw": "0 of 41,383 decided rows"})
        for path in (README, FORMATS):
            with self.subTest(path=path.name):
                text = " ".join(path.read_text().split())
                self.assertIn("Known limits of the judged scope", text)
                self.assertIn("0 of 5,040 decided rows", text)
                self.assertIn("0 of 41,383 decided rows", text)
                for limit in self.registry["limits"]:
                    self.assertIn(limit["id"], text)


if __name__ == "__main__":
    unittest.main()
