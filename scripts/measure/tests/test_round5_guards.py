"""S3 fold 4: the coordinate rule, the last certificate holes and their pinned shapes.

The graded behaviour is pinned by the self-check goldens (`oracle/selfcheck/*`, run by every
test that prepares the kit). These tests pin what the goldens must keep containing: a control
beside each refusal, both outcomes of the WRONG rule, and the rule text.
"""

import unittest

from measure.corpora import REPO

from . import test_oracle as fixtures

README = REPO / "scripts/measure/README.md"
FORMATS = REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit/FORMATS.md"
OFF = "endpoint-coordinate-off"
MEMBER = "outside-judged-scope"


def rows(case):
    return fixtures.read_jsonl(fixtures.KIT / "selfcheck" / case / "expected.jsonl")


def of(rows_, file, verdict):
    return [r for r in rows_ if r["key"]["from"]["file"] == file and r["verdict"] == verdict]


class CoordinateRule(unittest.TestCase):
    def test_the_four_class_a_shapes_are_undecided_and_each_has_a_value_control(self):
        # Express lib/express.js (two rows), lib/application.js, test/Router.js: the edge's
        # span is the object key beside the value read.
        rows_ = rows("lexical-coordinate")
        off = of(rows_, "wrong36-a.ts", "UNDECIDED")
        self.assertEqual(len(off), 4)  # `app:` twice, `engines:`, `method:`
        self.assertTrue(all(r["why"] == MEMBER for r in off))
        controls = of(rows_, "wrong36-a.ts", "CORRECT")
        self.assertEqual(len(controls), len(off))
        for row in off:
            key = row["key"]
            same_from_same_line = [c for c in controls if c["key"]["from"] == key["from"]
                                   and c["key"]["to"]["line"] == key["to"]["line"]
                                   and c["key"]["to"]["base"] == key["to"]["base"]]
            self.assertEqual(len(same_from_same_line), 1, key)
            # The control is the same edge at the value bytes, after the key.
            self.assertGreater(same_from_same_line[0]["key"]["to"]["start_byte"],
                               key["to"]["start_byte"])

    def test_the_class_b_shapes_are_spelling_mismatch_undecided_beside_a_control(self):
        # The from-span is an identifier X assigned from the claimed base: the producer's alias
        # twin carries X's span, so the oracle cannot call the edge wrong.
        rows_ = rows("lexical-coordinate")
        mismatch = of(rows_, "wrong36-b.ts", "UNDECIDED")
        self.assertEqual(len(mismatch), 4)
        self.assertTrue(all(r["why"] == "endpoint-spelling-mismatch" for r in mismatch))
        self.assertEqual(of(rows_, "wrong36-b.ts", "WRONG"), [])
        control, = of(rows_, "wrong36-b.ts", "CORRECT")
        self.assertEqual(control["key"]["from"]["base"], control["key"]["to"]["base"])

    def test_write_targets_spelled_as_claimed_stay_wrong_beside_a_read_control(self):
        # Express blob.ts `(objectURL = ..)` and bounds.ts `t1 = t2 = ..`: the span is exactly
        # the assignment target and no read of it is on the line.
        rows_ = rows("lexical-coordinate")
        self.assertEqual(len(of(rows_, "wrong-write.ts", "WRONG")), 2)
        self.assertEqual(len(of(rows_, "wrong-write.ts", "CORRECT")), 2)

    def test_one_line_a_shape_and_both_endpoint_faults_have_each_outcome(self):
        rows_ = rows("lexical-coordinate")
        self.assertEqual({r["verdict"] for r in rows_ if r["key"]["from"]["file"] == "oneline.ts"},
                         {"UNDECIDED", "CORRECT"})
        # coord.ts: a role fault with a serving neighbour, one without, and a different
        # binding with and without a neighbour.
        verdicts = sorted((r["key"]["from"]["line"], r["key"]["to"]["line"], r["verdict"])
                          for r in rows_ if r["key"]["from"]["file"] == "coord.ts")
        for outcome in ("UNDECIDED", "WRONG", "CORRECT"):
            self.assertIn(outcome, {v for *_, v in verdicts})

    def test_delete_operands_are_undecided_and_the_void_read_is_correct(self):
        rows_ = rows("lexical-delete")
        self.assertEqual(sorted(r["verdict"] for r in rows_), ["CORRECT"] + ["UNDECIDED"] * 6)
        self.assertCountEqual([r["why"] for r in rows_ if r["verdict"] == "UNDECIDED"],
                              ["delete-operand"] * 4 + [MEMBER] * 2)

    def test_the_argument_slot_follows_the_rule_and_keeps_the_stricter_formats_row(self):
        rows_ = rows("argument-coordinate")
        verdicts = [(r["verdict"], r.get("why")) for r in rows_]
        self.assertEqual(verdicts.count(("UNDECIDED", OFF)), 1)
        self.assertIn(("UNDECIDED", MEMBER), verdicts)
        self.assertEqual(sum(v == "WRONG" for v, _ in verdicts), 3)
        self.assertEqual(sum(v == "CORRECT" for v, _ in verdicts), 3)

    def test_each_boundary_reason_the_goldens_name_is_checked(self):
        # `endpoint-coordinate-off` is a named boundary: a golden must state it.
        text = (fixtures.MEASURE / "oracle/ts_oracle.cjs").read_text()
        self.assertIn(f"'{OFF}'", text)


class RuleText(unittest.TestCase):
    def test_readme_states_the_rule_the_certificate_forms_and_the_formats_exception(self):
        text = " ".join(README.read_text().split())
        for phrase in ("WRONG is returned only for a proved position, role or declaration fault "
                       "at an endpoint spelled as claimed at exact identifier bytes",
                       "UNDECIDED `endpoint-coordinate-off`.",
                       "UNDECIDED `endpoint-spelling-mismatch`, never WRONG",
                       "alias twins carry the alias variable's span",
                       "only the proved formal can serve as a neighbour",
                       "signature formal of an overload are always WRONG",
                       "`N = {..}` replaces every member",
                       "Closed global-scope rule",
                       "no `var` binding of its name that assigns in its own var scope",
                       "a computed-name instance field anywhere in the chain counts"):
            self.assertIn(phrase, text)

    def test_formats_names_the_new_boundary_reason(self):
        self.assertIn(OFF, FORMATS.read_text())


if __name__ == "__main__":
    unittest.main()
