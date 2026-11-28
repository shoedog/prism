"""S3 fold 7: the out-of-model amendment and the bounded lexical and slot items."""

import unittest

from measure.corpora import REPO

from . import test_oracle as fixtures

README = REPO / "scripts/measure/README.md"
FORMATS = REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit/FORMATS.md"
MUTATION = "out-of-model-module-mutation"
SUPPRESSION = "out-of-model-suppression"
MEMBER = "outside-judged-scope"


def rows(case):
    return fixtures.read_jsonl(fixtures.KIT / "selfcheck" / case / "expected.jsonl")


def by_file(case):
    result = {}
    for row in rows(case):
        result.setdefault(row["key"]["from"]["file"], []).append(row)
    return result


def reasons(case, verdict):
    return {name: sorted({r.get("why") for r in found if r["verdict"] == verdict})
            for name, found in by_file(case).items() if any(r["verdict"] == verdict for r in found)}


class ModuleMutation(unittest.TestCase):
    def test_each_instance_is_undecided_beside_an_es_export_control(self):
        undecided = reasons("certificate-fold7-module", "UNDECIDED")
        # sol r5-1 (direct), sol r6-1 (alias of module), Opus r6 W2 P7 and P7c, P7b, Object.assign.
        for name in ("u1.ts", "u2.ts", "u2b.ts", "u3.ts", "u3c.ts", "u4.ts", "u5.ts"):
            self.assertEqual(undecided[name], [MUTATION], name)
        self.assertEqual(sorted(reasons("certificate-fold7-module", "CORRECT")), ["uc1.ts", "uc2.ts"])

    def test_every_write_form_to_the_module_object_is_in_the_belt(self):
        # delete, update, object and array destructuring leaves and a loop head.
        undecided = reasons("certificate-fold7-module", "UNDECIDED")
        for n in range(6, 11):
            self.assertEqual(undecided[f"uw{n}.ts"], [MUTATION], n)

    def test_an_ordinary_commonjs_export_is_also_a_write_to_module_exports(self):
        self.assertEqual(reasons("certificate-fold7-module", "UNDECIDED")["uc3.ts"], [MUTATION])

    def test_the_bare_call_in_a_namespace_block_applies_the_namespace_rules(self):
        # Opus r6 W1: P1a (member written in another file), P1s (suppressed write), P1v.
        undecided = reasons("certificate-fold7-namespace", "UNDECIDED")
        self.assertEqual(undecided["c1.ts"], [MUTATION])
        self.assertEqual(undecided["p1s.ts"], [MUTATION])
        self.assertEqual(undecided["p1v.ts"], [SUPPRESSION])
        self.assertEqual(sorted(reasons("certificate-fold7-namespace", "CORRECT")),
                         ["arrc.ts", "cfca.ts", "ctl.ts", "ctl2c.ts"])
        # A block of the namespace in another file contributes; a variable-held function counts.
        self.assertEqual(undecided["cfa.ts"], [MUTATION])
        self.assertEqual(undecided["arr1.ts"], [MUTATION])

    def test_global_object_writes_reach_script_functions_and_namespace_members_only(self):
        undecided = reasons("certificate-fold7-global", "UNDECIDED")
        self.assertEqual(sorted(undecided), [f"g{i}.ts" for i in range(1, 9)])
        self.assertTrue(all(why == [MUTATION] for why in undecided.values()))
        controls = reasons("certificate-fold7-global", "CORRECT")
        # A module-local function (gc3) is not replaced by a write to the global object, and a
        # block-scoped namesake of another scope or a plain `var f;` is no duplicate.
        # (gc6: a top-level `this` of a module file is not the global object.)
        self.assertEqual(sorted(controls), ["gc1.ts", "gc2.ts", "gc3.ts", "gc4.ts", "gc5.ts", "gc6.ts"])


class Suppression(unittest.TestCase):
    def test_suppression_and_duplicates_are_undecided_beside_controls(self):
        undecided = reasons("certificate-fold7-suppress", "UNDECIDED")
        # r6-2 (function and const under @ts-nocheck), @ts-ignore, @ts-expect-error, @ts-nocheck,
        # a duplicate declaration of the callee name.
        self.assertEqual(sorted(undecided), ["s2.ts", "s6.ts", "u345.ts"])
        self.assertTrue(all(why == [SUPPRESSION] for why in undecided.values()))
        self.assertEqual(len([r for r in rows("certificate-fold7-suppress") if r["verdict"] == "UNDECIDED"]), 5)
        self.assertEqual(sorted(reasons("certificate-fold7-suppress", "CORRECT")), ["c2.ts", "c6.ts", "uc3.ts"])

    def test_declaration_collection_pins_each_kind(self):
        # Removing class, enum or namespace collection from the declaration scan flips its row.
        undecided = reasons("certificate-fold7-pins", "UNDECIDED")
        self.assertEqual(sorted(undecided), ["aCl.ts", "aEn.ts", "aNs.ts", "mSf.ts"])
        self.assertTrue(all(why == [SUPPRESSION] for why in undecided.values()))
        self.assertEqual(sorted(reasons("certificate-fold7-pins", "CORRECT")), ["aCtl.ts", "mSfc.ts"])

    def test_controls_beside_suppressed_instances_moved_to_files_of_their_own(self):
        # The belt is file-wide: a control must not share a file with a suppression comment.
        names = {p.name for p in (fixtures.KIT / "selfcheck/certificate-fold4/repo").glob("*")}
        self.assertTrue({"ns-ctl.ts", "g1-ctl.ts"} <= names)
        controls = [r["key"]["from"]["file"] for r in rows("certificate-fold4") if r["verdict"] == "CORRECT"]
        self.assertIn("ns-ctl.ts", controls)
        self.assertIn("g1-ctl.ts", controls)


class BoundedItems(unittest.TestCase):
    def test_property_tokens_by_syntax_are_outside_the_judged_scope(self):
        # r6-3: `[req.app, app]` with a parameter property and a namespace-exported const.
        off = [r for r in by_file("lexical-fold7")["props.ts"] if r["verdict"] == "UNDECIDED"]
        self.assertEqual(len(off), 2)
        self.assertTrue(all(r["why"] == MEMBER for r in off))
        self.assertEqual(len([r for r in by_file("lexical-fold7")["props.ts"] if r["verdict"] == "CORRECT"]), 2)

    def test_type_only_mentions_are_outside_the_scope_beside_evaluated_controls(self):
        # r6-4: `{ [key]: number }` and `implements ns.T` get no verdict (fold F: type positions are
        # outside the judged scope); the object computed key and `extends ns.C` stay CORRECT.
        verdicts = sorted((r["key"]["to"]["line"], r["verdict"]) for r in by_file("lexical-fold7")["types.ts"])
        self.assertEqual(verdicts, [(4, "UNDECIDED"), (5, "CORRECT"), (7, "UNDECIDED"), (8, "CORRECT"), (9, "UNDECIDED")])

    def test_receiver_edges_are_withheld_for_each_original_receiver_shape(self):
        for name in ("slots.ts", "thisrecv.ts"):
            found = [(r["verdict"], r["why"]) for r in by_file("lexical-fold7")[name]]
            self.assertEqual(found, [("UNDECIDED", MEMBER)] * 2)

    def test_a_constructor_signature_formal_on_the_implementations_line_is_wrong(self):
        # r6-6.
        self.assertEqual(sorted(r["verdict"] for r in rows("argument-fold7")), ["CORRECT", "WRONG"])


class RuleText(unittest.TestCase):
    def test_readme_states_the_amendment_and_where_each_component_applies(self):
        text = " ".join(README.read_text().split())
        for phrase in ("under the static-binding model at exact identifier bytes",
                       "The file-wide checks examine the callee's file F",
                       "UNDECIDED `out-of-model-module-mutation`",
                       "UNDECIDED `out-of-model-suppression`",
                       "The suppression check applies to every callee.",
                       "`const alias = module; alias.exports = {..}`",
                       "UNDECIDED `outside-judged-scope`",
                       "erased computed keys (accessor",
                       "constructors, whose overloads are the containing class's"):
            self.assertIn(phrase, text)

    def test_formats_lists_the_new_named_reasons(self):
        text = FORMATS.read_text()
        for name in (MUTATION, SUPPRESSION, MEMBER, "erased-computed-key"):
            self.assertIn(f"`{name}`", text)

    def test_both_contracts_state_the_scope_and_evidence(self):
        for path in (README, FORMATS):
            with self.subTest(path=path.name):
                text = " ".join(path.read_text().split())
                for phrase in ("41,368", "1,409", "6,651", "`structured-path`",
                               "already UNDECIDED under other reasons", "Judged scope. The oracle judges a claimed endpoint only when",
                               "private name", "JSX attribute name", "`new.target`", "enum member",
                               "shorthand keys", "destructuring elements", "computed-key identifiers",
                               "positive receiver rule", "RULE-R.md",
                               "233 of 19,343", "30 kept and 30 discarded", "summary.md",
                               "can miss indirect calls"):
                    self.assertIn(phrase, text)

    def test_formats_requires_the_proved_formal_as_the_slot_neighbour(self):
        text = " ".join(FORMATS.read_text().split())
        self.assertIn("only the proved formal itself can serve as the neighbour", text)


if __name__ == "__main__":
    unittest.main()
