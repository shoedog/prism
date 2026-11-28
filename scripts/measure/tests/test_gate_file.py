"""Gate graph preflight refuses malformed input before any launch."""

from __future__ import annotations

from copy import deepcopy
import json

from .comparator_fixtures import edge
from .gate_fixtures import GateWorkbench, require_gate


class GateFileTests(GateWorkbench):
    def refuses(self, plans, *, pinned=False):
        for index, plan in enumerate(plans):
            with self.subTest(index=index, plan=plan):
                self.next_output(f"refused-{index}")
                self.gate(plan, expected=2, pin=self.pin(plan) if pinned else None)
                self.assertEqual(self.result()["result"], "refused")
                self.assert_no_launch()

    def test_schema_and_closed_fields_preflight(self):
        """gate-007: a malformed graph cannot launch an earlier valid run."""
        require_gate("gate_file")
        valid = self.plan([self.run_step()], base=self.fake)
        variants = [
            {**valid, "schema": "prism.measure.gate/2"},
            {**valid, "unknown": True},
            {**valid, "steps": [self.run_step(), {"id": "end", "do": "corpora-list", "keys": []}]},
            {**valid, "steps": [self.run_step(), {"id": "end", "do": "not-a-command"}]},
            {**valid, "binaries": {"base": {**valid["binaries"]["base"], "unknown": True}}},
        ]
        self.refuses(variants)
        self.next_output("valid")
        self.gate(valid)
        self.assertEqual(self.step("base")["status"], "pass")

    def test_empty_graph_refuses(self):
        """gate-013: a gate cannot certify an empty steps array."""
        require_gate("gate_file")
        self.refuses([self.plan([])])

    def test_step_id_grammar_and_case_insensitive_uniqueness(self):
        """n2-32: output-directory IDs are unique even after case folding."""
        require_gate("gate_file")
        first = self.run_step("Arm", binary="base")
        self.refuses([
            self.plan([first, self.run_step(other, binary="base")], base=self.fake)
            for other in ("Arm", "arm", "", ".", "../outside", "a/b", "has space", "café")
        ])
        self.next_output("valid")
        self.gate(self.plan([first, self.run_step("arm_2-0", binary="base")], base=self.fake))

    def test_references_must_be_earlier_and_of_required_type(self):
        """gate-007, gate-011: missing, forward, self and wrong-kind references are preflight errors."""
        require_gate("gate_file")
        valid = self.pair()
        base, head, diff = valid["steps"]
        broken = [
            [base, head, dict(diff, head="absent")],
            [base, diff, head],
            [base, head, dict(diff, head=diff["id"])],
            [base, {"id": "head", "do": "corpora-list"}, diff],
        ]
        self.refuses([{**valid, "steps": steps} for steps in broken], pinned=True)
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self) as oracle:
            step = oracle.step("ts-edges", run="base", changes="head", side="head")
            self.refuses([{**valid, "steps": [base, head, step]}], pinned=True)

    def test_trusted_input_files_must_be_regular_and_below_gate(self):
        """gate-021/IC-1/IC-7: a matching digest cannot authenticate a linked controller input."""
        require_gate("gate_file")
        plan = self.pair()
        pin = self.pin(plan)
        actual = self.classifier.with_name("actual.py")
        self.classifier.rename(actual)
        self.classifier.symlink_to(actual.name)
        self.gate(plan, expected=2, pin=pin)
        self.assert_no_launch()
        self.classifier.unlink()
        actual.rename(self.classifier)
        self.next_output("regular-file-control")
        self.gate(plan, pin=pin)

    def test_expectation_shape_and_path_grammar(self):
        """gate-017: invalid escapes, wildcard paths and invalid numeric bounds refuse."""
        require_gate("gate_file")
        malformed = [
            {r"by_class.rd\qmove.keys": 1}, {"tables.*": 1}, {"tables..dfg": 1},
            {"commands#comment": 1}, {"": 0}, {"commands": True},
            {"commands": "1"}, {"commands": {}}, {"commands": []},
            {"commands": {"min": 2, "max": 1}},
            {"commands": {"min": 0, "unknown": 2}}, {"commands": {"max": False}},
        ]
        self.refuses([self.plan([self.run_step(), self.run_step("late", binary="base", expect=value)],
                                base=self.fake) for value in malformed])
        self.next_output("non-finite")
        valid = self.plan([self.run_step(expect={"commands": "NOT_A_NUMBER"})], base=self.fake)
        self.gate_path.write_text(json.dumps(valid).replace('"NOT_A_NUMBER"', "NaN"))
        result = self.local_command(["--manifest", self.manifest, "gate", self.gate_path,
                                     "--out", self.out, "--corpora-root", self.work])
        self.assert_exit(result, 2)
        self.assert_no_launch()

    def test_escaped_dotted_class_reads_one_literal_component(self):
        """gate-017: escaped dotted class succeeds and unescaped hierarchy fails by path."""
        require_gate("gate_file")
        self.classifier.write_text('CLASSES = ["rd.move"]\ndef classify(change, context): return "rd.move"\n')
        plan = self.pair(head=self.product("head", edges=[edge(label="moved")]))
        plan["steps"][-1]["expect"] = {r"by_class.rd\.move.keys": 1}
        self.gate(plan, pin=self.pin(plan))
        metrics = self.step("compare")["metrics"]
        self.assertEqual(metrics["by_class"]["rd.move"]["keys"], 1)
        self.next_output("unescaped")
        path = "by_class.rd.move.keys"
        plan["steps"][-1]["expect"] = {path: 1}
        self.gate(plan, pin=self.pin(plan), expected=1)
        self.assertIn(path, str(self.step("compare")["failures"]))

    def test_quality_absence_closed_shapes_and_overlap(self):
        """gate-020, gate-023: reject bad rational shapes, scope fields, and overlapping claims."""
        require_gate("gate_file")
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self) as oracle:
            valid = oracle.edges([("binding", "other", "CORRECT")])
            step = valid["steps"][-1]
            scope = {"relation": "binding", "property": "slot"}
            quality = {"scope": scope, "max_wrong": [0, 1], "min_decided": 1}
            invalid = [
                {"quality": quality}, {"quality": [{**quality, "extra": False}]},
                {"quality": [{**quality, "max_wrong": [0, 0]}]},
                {"quality": [{**quality, "max_wrong": [2, 1]}]},
                {"quality": [{**quality, "max_wrong": [False, 1]}]},
                {"quality": [{**quality, "min_decided": True}]},
                {"quality": [{**quality, "min_decided": 0}]},
                {"quality": [{**quality, "scope": {"unlisted": "slot"}}]},
                {"quality": [{**quality, "scope": {"relation": "unregistered"}}]},
                {"absence": scope}, {"absence": [{"property": False}]},
                {"absence": [{"confidence": "unknown"}]},
                {"absence": [{"kind": "not-a-selected-edge-kind"}]},
                {"absence": [scope], "quality": [quality]},
                {"absence": [scope], "quality": [{**quality, "scope": {"relation": "binding"}}]},
                {"absence": [scope], "expect": {"tuples.binding.slot.WRONG": 0}},
                {"allow_empty": True},
            ]
            for index, options in enumerate(invalid):
                with self.subTest(options=options):
                    plan = deepcopy(valid)
                    plan["steps"][-1] = {**step, **options}
                    self.next_output(f"quality-{index}")
                    self.gate(plan, expected=2, pin=self.pin(plan))
                    self.assert_no_launch()
            self.next_output("non-overlapping")
            step.update(absence=[scope], quality=[{**quality, "scope": {"property": "other"}}])
            self.gate(valid, pin=self.pin(valid))
            self.assertEqual(self.step("grade")["status"], "pass")

    def test_version_pin_validity(self):
        """n2-13: non-token and too-short binary pins refuse before the version probe."""
        require_gate("gate_file")
        plan = self.plan([self.run_step()], base=self.fake)
        variants = []
        for invalid in ("", "a1b2c3", "v", " v1", "v1 two", False, 42, None):
            case = deepcopy(plan)
            case["binaries"]["base"]["expect_version"] = invalid
            variants.append(case)
        self.refuses(variants)
        self.next_output("valid")
        plan["binaries"]["base"]["expect_version"] = "9d1a7d0"
        self.gate(plan)
        self.assertEqual(self.step("base")["status"], "pass")
