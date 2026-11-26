"""S3 fold 6: the closed global-scope rule, the property-token rule and their pinned shapes."""

import unittest

from measure.corpora import REPO

from . import test_oracle as fixtures

README = REPO / "scripts/measure/README.md"
OFF = "endpoint-coordinate-off"


def rows(case):
    return fixtures.read_jsonl(fixtures.KIT / "selfcheck" / case / "expected.jsonl")


def files(case, kind):
    return sorted(p.name for p in (fixtures.KIT / "selfcheck" / case / "repo").glob(kind))


class GlobalScope(unittest.TestCase):
    def test_each_reviewer_instance_is_undecided_beside_a_correct_control(self):
        rows_ = rows("certificate-fold6")
        undecided = [r for r in rows_ if r["verdict"] == "UNDECIDED"]
        controls = [r for r in rows_ if r["verdict"] == "CORRECT"]
        self.assertEqual(len(undecided), 11)
        self.assertTrue(all(r["why"] == "ambiguous-dispatch" for r in undecided))
        # One control for the namespace form and one for the function form.
        self.assertEqual(sorted(r["key"]["from"]["file"] for r in controls), ["s1c.ts", "u5.js"])
        used = {r["key"]["from"]["file"] for r in undecided}
        for name in ("s1a.ts", "s1b.ts", "u1.js", "u2.js", "u3.js", "u4a.js", "m1.ts", "s1f.ts"):
            self.assertIn(name, used)

    def test_the_cross_file_instances_have_a_second_file(self):
        names = files("certificate-fold6", "*")
        for second in ("u4b.js", "s1g.ts"):
            self.assertIn(second, names)


class PropertyTokens(unittest.TestCase):
    def test_property_tokens_are_undecided_beside_exact_byte_controls(self):
        rows_ = rows("lexical-fold6")
        off = [r for r in rows_ if r["verdict"] == "UNDECIDED"]
        self.assertEqual(len(off), 7)
        self.assertTrue(all(r["why"] == OFF for r in off))
        self.assertEqual(len([r for r in rows_ if r["verdict"] == "CORRECT"]), 7)
        self.assertEqual({r["key"]["from"]["file"] for r in off},
                         {"l1.ts", "l2.ts", "l3.ts", "l4.ts", "private.ts", "mismatch.ts"})

    def test_a_signature_formal_on_the_implementations_line_stays_wrong(self):
        verdicts = sorted(r["verdict"] for r in rows("argument-overload"))
        self.assertEqual(verdicts, ["CORRECT", "WRONG"])


class RuleText(unittest.TestCase):
    def test_readme_states_the_amendment_and_the_new_rules(self):
        text = " ".join(README.read_text().split())
        for phrase in ("across ALL files of the program and by spelling",
                       "`globalThis.N = {..}`, `var N = {..}` replacing a script namespace",
                       "UNDECIDED `delete-operand`",
                       "different VALUE bindings",
                       "Private names (`#m`) count as occurrences",
                       "also when the overload and its implementation share a line"):
            self.assertIn(phrase, text)


if __name__ == "__main__":
    unittest.main()
