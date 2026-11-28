"""Report structure, verbatim prediction, and recorded limits."""

from __future__ import annotations

import json
import platform
import re
from unittest.mock import patch

from .gate_fixtures import GateWorkbench, REPORT_SECTIONS, read_json, require_gate, section
from .helpers import REPO, sha256_of, write_tree


class GateReportTests(GateWorkbench):
    def test_report_has_eight_headings_in_order_and_matching_prediction(self):
        """gate-008, gate-010: a passing reused run has exactly the eight ordered headings."""
        require_gate("report")
        plan = self.plan([self.run_step()], base=self.fake)
        pin = self.pin(plan)
        prior = self.prior_run(plan, "base", self.work / "prior", gate_pin=pin,
                               prediction=self.prediction)
        self.next_output("matching")
        self.gate(self.reuse(plan, "base", prior), pin=pin)
        self.assertEqual(tuple(re.findall(r"(?m)^## (.+)$", self.report())), REPORT_SECTIONS)
        self.assertIn("prediction-digest-matched", self.report() + self.diagnostic)
        self.assertEqual(self.result()["prediction_sha256"], sha256_of(self.prediction))

    def test_report_verbatim_prediction_uses_a_longer_fence(self):
        """gate-009: preserve Unicode, CRLF/LF, blank lines and backticks verbatim."""
        require_gate("report")
        prediction = "Prévision \r\n\r\n````\n## inside the prediction\nline with ````` ticks\n".encode()
        self.prediction.write_bytes(prediction)
        self.gate(self.plan([self.run_step()], base=self.fake))
        report = self.report_bytes()
        text = report.split(b"## Prediction\n", 1)[1]
        match = re.search(rb"(?m)^(`{6,})[a-z]*\r?\n", text)
        self.assertIsNotNone(match, text)
        start = match.end()
        self.assertEqual(text[start:start + len(prediction)], prediction)
        self.assertTrue(text[start + len(prediction):].lstrip(b"\r\n").startswith(match[1] + b"\n"))
        self.assertIn(sha256_of(self.prediction).encode(), text[:start])

    def test_report_lists_waivers_line_projection_refusals_and_unrun_steps(self):
        """gate-008, gate-014, gate-018, gate-025, n2-35: Limits describes waivers and unfinished measurements."""
        require_gate("report")
        plan = self.pair(step=self.edge_step(key="line", line_reason="predeclared line control",
                                             allow_empty=True))
        plan["steps"] += [self.run_step("fail"), self.run_step("unfinished", binary="base")]
        failure = self.product("fail", actions={"edges": "sys.exit(7)"})
        plan["binaries"]["fail"] = {"path": str(failure),
                                   "expect_version": plan["binaries"]["base"]["expect_version"]}
        self.gate(plan, pin=self.pin(plan), expected=1)
        limits = section(self.report(), "Limits")
        for text in ("compare", "predeclared line control", "unfinished"):
            self.assertIn(text, limits)
        self.assertRegex(limits, r"(?i)waiv|allow_empty")
        self.assert_ic5(limits)

        self.next_output("refused")
        plan["steps"].pop(3)
        self.classifier.write_text("CLASSES = []\n")
        self.gate(plan, pin=self.pin(plan), expected=2)
        limits = section(self.report(), "Limits")
        self.assertIn("compare", limits)
        self.assertRegex(limits, r"(?i)refus")
        self.assertIn("unfinished", limits)

    def assert_ic5(self, limits):
        self.assertRegex(limits, r"(?is)(first saw|first observed|saw an outcome|when.*outcome)")
        self.assertRegex(limits, r"(?is)(rerun|re-run|repeat)")

    def test_known_limit_pin_allows_repeat_of_same_decision(self):
        """known-limit n2-35/IC-5: registration does not prove timing or prevent reruns."""
        require_gate("report")
        plan = self.pair()
        pin = self.pin(plan)
        self.gate(plan, pin=pin)
        first = (self.out / "decision.json").read_bytes()
        self.next_output("rerun")
        self.gate(plan, pin=pin)
        self.assertEqual((self.out / "decision.json").read_bytes(), first)
        self.assertEqual(self.result()["product_steps"], ["compare"])
        self.assert_ic5(section(self.report(), "Limits"))
        self.next_output("changed-control")
        plan["steps"][-1]["expect"] = {"unattributed": {"max": 1}}
        self.gate(plan, pin=pin, expected=2)
        self.assert_no_launch()

    def test_known_limit_adjacent_guard_config_is_not_bound(self):
        """known-limit n3-21/IC-3: the guard and quiet helper bind; sourced config does not."""
        require_gate("gate_decision")
        config = self.guard.with_name("adjacent-config")
        config.write_text("before")
        hook = f"os.environ['PRISM_FIXTURE_ADJACENT'] = Path({str(config)!r}).read_text()"
        self.make_guard(hook=hook)
        plan = self.pair()
        pin = self.pin(plan)
        self.gate(plan, pin=pin)
        self.assertEqual({row["env"]["PRISM_FIXTURE_ADJACENT"] for row in self.fake_log()}, {"before"})
        config.write_text("after")
        self.next_output("different-config")
        self.gate(plan, pin=pin)
        self.assertEqual({row["env"]["PRISM_FIXTURE_ADJACENT"] for row in self.fake_log()}, {"after"})
        self.assertEqual(self.result()["gate_sha256"], pin)
        self.next_output("different-helper-control")
        self.quiet_helper.write_bytes(b"# changed bound helper\n")
        self.gate(plan, pin=pin, expected=2)
        self.assert_no_launch()

    def test_known_limit_kit_commit_is_recorded_without_binding_reuse(self):
        """known-limit n4-3: independently bound run bytes from another kit commit are recorded."""
        require_gate("report")
        from measure import run

        plan = self.pair()
        pin = self.pin(plan)
        earlier_commit = "a" * 40
        git = run.git

        def old_commit(root, *argv):
            if root == REPO and argv == ("rev-parse", "HEAD"):
                return (earlier_commit + "\n").encode()
            return git(root, *argv)

        with patch.object(run, "git", side_effect=old_commit):
            old = self.prior_run(plan, "base", self.work / "earlier", gate_pin=pin,
                                 prediction=self.prediction)
        self.assertEqual(read_json(old / "receipt.json")["kit_commit"], earlier_commit)
        reused = self.reuse(plan, "base", old)
        self.next_output("new-kit")
        self.gate(reused, pin=pin)
        limits = section(self.report(), "Limits")
        self.assertIn(earlier_commit, limits)
        self.assertIn(self.result()["kit_commit"], self.report())
        self.assertNotEqual(self.result()["kit_commit"], earlier_commit)
        self.assertEqual(self.result()["gate_sha256"], pin)
        self.assertEqual(self.result()["product_steps"], ["compare"])
        self.next_output("wrong-receipt-digest-control")
        reused["steps"][0]["reuse"]["receipt_sha256"] = "b" * 64
        self.gate(reused, pin=pin, expected=2)
        self.assert_no_launch()

    def test_known_limit_runtimes_recorded_and_excluded_from_decision(self):
        """known-limit n4-13: gate and child record runtimes; the decision binds checker content."""
        require_gate("report")
        from .gate_oracle_fixtures import oracle_fixture
        from .gate_oracle_worker import VERSION

        with oracle_fixture(self) as oracle:
            plan = oracle.edges([("binding", "other", "CORRECT")])
            pin = self.pin(plan)
            self.gate(plan, pin=pin)
            runtime = read_json(self.out / "grade/oracle-receipt.json")["inputs"]["runtime"]
            self.assertEqual(runtime, {"node": VERSION, "python": platform.python_version()})
            result = self.result()
            self.assertIn(result["kit_commit"], self.report())
            summary = {key: value for key, value in result.items() if key != "steps"}
            for value in runtime.values():
                self.assertIn(value, json.dumps(summary))
                self.assertNotIn(value, (self.out / "decision.json").read_text())
            limits = section(self.report(), "Limits")
            self.assertRegex(limits, r"(?i)runtime|python|node")
            self.assertRegex(limits, r"(?i)unbound|not bound|outside.*custody")

            # Control: physically move the checker under the immutable pin,
            # then change the relocated bytes and require a prelaunch refusal.
            import shutil

            checker = self.work / "relocated-checker/lib/typescript.js"
            checker.parent.mkdir(parents=True)
            shutil.copyfile(oracle.ts, checker)
            plan["steps"][-1]["typescript"] = str(checker)
            self.next_output("checker-moved")
            self.gate(plan, pin=pin)
            self.next_output("checker-changed")
            write_tree(checker.parent, {checker.name: "// different checker bytes\n"})
            self.gate(plan, pin=pin, expected=2)
            self.assert_no_launch()
