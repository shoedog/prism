"""Self-check coverage, key and custody controls for the question closure and built-ins."""

from __future__ import annotations

import copy
import json
import shutil
import unittest
from unittest.mock import patch

from measure import oracle, oracle_inputs
from measure.edges import canonical

from .helpers import write_tree
from . import test_oracle as fixtures


class Custody(unittest.TestCase):
    def setUp(self):
        self.h = fixtures.OracleTests()
        self.addCleanup(self.h.doCleanups)
        self.h.setUp()

    def selfcheck(self, question):
        return self.h.local_command(oracle.selfcheck, ["--typescript", self.h.typescript(),
                                                       "--question", question])

    def test_lane_coverage_is_judged_per_question_set(self):
        h = self.h
        # One lane golden for a built-in relation and property cannot borrow the other
        # three verdicts that the built-in's own cases carry.
        lane = h.work / "single"
        shutil.copytree(fixtures.KIT / "selfcheck/oracle-001", lane / "selfcheck/oracle-001")
        entry = lane / "question.cjs"
        entry.write_text("module.exports = {relations:['argument_binding'], grade() { return "
                         "{verdict:'CORRECT', property:'implementation-slot', why:'proof'}; }};")
        refused = self.selfcheck(entry)
        h.assert_exit(refused, 1)
        self.assertIn("selfcheck-coverage: need all verdicts for argument_binding/implementation-slot",
                      refused.stderr)
        # The same relation and property with a complete lane golden set passes.
        complete = h.question(dict(verdict="CORRECT", property="implementation-slot", why="proof"),
                              relation="argument_binding", prop="implementation-slot", name="complete")
        h.assert_exit(self.selfcheck(complete), 0)

    def test_reserved_input_admission_label_carries_the_inadmissible_verdict(self):
        h = self.h
        question = h.question(dict(verdict="CORRECT", property="custom", why="proof"))
        golden = question.parent / "selfcheck/oracle-004/expected.jsonl"
        rows = fixtures.read_jsonl(golden)
        inadmissible, = (row for row in rows if row["verdict"] == "INADMISSIBLE")
        # With a second property the inadmissible claim is labelled `input-admission`, which is
        # not a graded property. The real property then needs the three graded verdicts.
        inadmissible["property"] = "input-admission"
        golden.write_text(fixtures.json_lines(rows))
        h.assert_exit(self.selfcheck(question), 0)
        # Relabelling a graded verdict of the real property leaves it uncovered and is refused.
        other = question.parent / "selfcheck/oracle-013/expected.jsonl"
        wrong, = fixtures.read_jsonl(other)
        other.write_text(fixtures.json_lines([dict(wrong, property="input-admission")]))
        refused = self.selfcheck(question)
        h.assert_exit(refused, 1)
        self.assertIn("selfcheck-coverage: need all verdicts for custom/custom", refused.stderr)

    def test_selfcheck_key_names_the_question_entry(self):
        h = self.h
        first = h.question(dict(verdict="CORRECT", property="custom", why="proof"))
        second = first.with_name("other.cjs")
        second.write_bytes(first.read_bytes())
        ts = h.typescript()
        one = oracle_inputs.prepare_inputs(ts, first)["key"]
        two = oracle_inputs.prepare_inputs(ts, second)["key"]
        self.assertNotEqual(one, two)
        # Relocating an unchanged closure keeps its key.
        moved = h.work / "moved"
        shutil.copytree(first.parent, moved)
        self.assertEqual(oracle_inputs.prepare_inputs(ts, moved / "question.cjs")["key"], one)
        self.assertEqual(oracle_inputs.prepare_inputs(ts, moved / "other.cjs")["key"], two)

    def test_built_in_modules_run_from_the_verified_staged_copy(self):
        h = self.h
        kit = h.work / "kit"
        shutil.copytree(fixtures.KIT, kit)
        args = h.load_case("oracle-001")
        builtin = kit / "questions/lexical_binding.cjs"
        original = builtin.read_bytes()
        node = shutil.which("node")
        with patch.object(oracle_inputs, "KIT", kit), \
                patch.object(oracle_inputs, "QUALIFICATIONS", kit / "qualifications"), \
                patch.object(oracle, "KIT", kit, create=True):
            h.prepare()
            h.assert_exit(h.oracle(args, out=h.work / "control"), 0)
            # A built-in replaced after it was hashed and restored before the recheck must
            # not grade: the worker runs the bytes Python verified, not the live path.
            h.fake_node(f"""
import subprocess
import sys
path = Path({str(builtin)!r})
before = path.read_bytes()
try:
    path.write_bytes(before + b"\\nthrow new Error('tampered built-in');\\n")
    result = subprocess.call([{node!r}, *sys.argv[1:]])
finally:
    path.write_bytes(before)
raise SystemExit(result)
""")
            outcome = h.oracle(args)
        h.assert_exit(outcome, 0)
        self.assertNotIn("tampered", outcome.stderr)
        self.assertEqual(builtin.read_bytes(), original)
        self.assertTrue((h.out / "oracle-receipt.json").exists())

    def test_qualification_evidence_requires_builder_provenance(self):
        root = self.h.work / "evidence"
        binary = "e" * 64
        raw, = fixtures.read_jsonl(fixtures.CASES / "oracle-022/probe.jsonl")
        good = fixtures.synthetic_evidence(binary, "named-function", raw)

        def check(record):
            blob = canonical(record)
            bundle = dict(schema="prism.measure.qualification/1", builds={binary: {
                "named-function": dict(path="evidence.json", sha256=oracle_inputs.digest(blob))}})
            shutil.rmtree(root, ignore_errors=True)
            write_tree(root, {"bundle.json": canonical(bundle), "evidence.json": blob})
            return oracle_inputs.qualifications(root)[0]

        self.assertEqual(check(good), {binary: ["named-function"]})
        minimal = {key: good[key] for key in ("binary_sha256", "form", "controls")}
        with self.assertRaisesRegex(ValueError, "lacks builder provenance"):
            check(minimal)

        def drop(key):
            return {name: value for name, value in good.items() if name != key}

        for field in ("label", "raw", "source_tree_sha256", "source_state_sha256",
                      "source_file_sha256", "receipt_sha256", "sites_sha256"):
            with self.subTest(missing=field), self.assertRaisesRegex(ValueError, "lacks builder provenance"):
                check(drop(field))
        label = good["label"]
        mutations = (
            dict(good, extra=1),
            dict(good, sites_sha256="not-a-digest"),
            dict(good, label=dict(label, form="named-method")),
            dict(good, label=dict(label, declaration=dict(label["declaration"], name="other"))),
            dict(good, label=dict(label, target_index=1)),
            dict(good, raw=dict(raw, resolved_targets=[dict(raw["resolved_targets"][0],
                                                            confidence="name_only")])),
            dict(good, raw={"record_kind": "call_site"}),
        )
        for number, record in enumerate(mutations):
            with self.subTest(mutation=number), self.assertRaisesRegex(ValueError, "lacks builder provenance"):
                check(record)

    def test_checker_file_digest_is_checked_independently_of_the_tree(self):
        h = self.h
        copied = h.work / "typescript"
        shutil.copytree(h.typescript().parent.parent, copied)
        entry = copied / "lib/typescript.js"
        oracle_inputs.checker_files(entry)
        original = entry.read_bytes()
        entry.write_bytes(original + b"\n")
        # The tree digest also differs, so only the named file digest message separates the checks.
        with self.assertRaisesRegex(oracle_inputs.OracleError, "typescript.js digest differs"):
            oracle_inputs.checker_files(entry)
        entry.write_bytes(original)
        oracle_inputs.checker_files(entry)


if __name__ == "__main__":
    unittest.main()
