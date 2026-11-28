"""Prediction custody, the independent decision pin, and decision.json bytes."""

from __future__ import annotations

from copy import deepcopy
import os
import shutil

from .comparator_fixtures import edge
from .gate_fixtures import GateWorkbench, canonical, digest, require_gate
from .helpers import sha256_of


class GateDecisionTests(GateWorkbench):
    def test_missing_or_non_utf8_prediction_even_in_retrospective(self):
        """gate-004: the named prediction must exist and decode in both modes."""
        require_gate("gate_decision")
        plan = self.plan([self.run_step()], base=self.fake)
        for label, content in (("missing", None), ("non-utf8", b"broken \xff")):
            for retrospective in (False, True):
                with self.subTest(label=label, retrospective=retrospective):
                    self.next_output(f"{label}-{retrospective}")
                    if content is None:
                        self.prediction.unlink(missing_ok=True)
                    else:
                        self.prediction.write_bytes(content)
                    self.gate(plan, expected=2, retrospective=retrospective)
                    self.assertEqual(self.result()["result"], "refused")
                    if content is None:
                        self.assertIn("prediction-missing", self.report() + self.diagnostic)
                    self.assert_no_launch()

    def test_reuse_compares_prediction_bytes_not_mtime(self):
        """gate-001, gate-010, terra-gate-002, astra-gate-002: mtime is inert and CRLF changes custody."""
        require_gate("gate_decision")
        plan = self.plan([self.run_step()], base=self.fake)
        pin = self.pin(plan)
        prior = self.prior_run(plan, "base", self.work / "old", gate_pin=pin,
                               prediction=self.prediction)
        plan = self.reuse(plan, "base", prior)
        original = self.prediction.read_bytes()
        stamp = self.prediction.stat()
        os.utime(self.prediction, ns=(stamp.st_atime_ns, stamp.st_mtime_ns + 2_000_000_000))
        self.next_output("same-bytes")
        self.gate(plan, pin=pin)
        self.assertIn("prediction-digest-matched", self.report() + self.diagnostic)
        self.assert_no_product_execution()
        stamp = self.prediction.stat()
        self.prediction.write_bytes(original.replace(b"\n", b"\r\n"))
        os.utime(self.prediction, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        self.assertEqual(self.prediction.stat().st_mtime_ns, stamp.st_mtime_ns)
        for retrospective in (False, True):
            self.next_output(f"changed-bytes-{retrospective}")
            self.gate(plan, expected=2, retrospective=retrospective)
            self.assertIn("prediction-digest-mismatch", self.report() + self.diagnostic)
            self.assert_no_launch()

    def assert_no_product_execution(self):
        self.assertEqual(self.fake_log(), [])
        self.assertFalse(self.guard_events.exists())

    def test_missing_old_prediction_digest_refuses_prospective(self):
        """gate-002, gate-005: an intact standalone run with no prediction is legacy evidence."""
        require_gate("gate_decision")
        plan = self.plan([self.run_step()], base=self.fake)
        plan = self.reuse(plan, "base", self.prior_run(plan, "base", self.work / "legacy"))
        self.next_output("prospective")
        self.gate(plan, expected=2)
        self.assertIn("prediction-digest-missing", self.report() + self.diagnostic)
        self.assert_no_launch()
        self.next_output("retrospective")
        self.gate(plan, expected=1, retrospective=True)
        self.assertEqual(self.result()["result"], "retrospective")
        self.assertEqual(self.result()["claim_scope"], "none")
        self.assertEqual(self.result()["product_steps"], [])
        self.assert_no_product_execution()

    def test_prediction_rechecked_between_steps_and_at_close(self):
        """gate-003, sol2-gate-001: a real final or intermediate child changes prediction bytes."""
        require_gate("gate_decision")
        original = self.prediction.read_bytes()
        mutator = f"Path({str(self.prediction)!r}).write_bytes(b'changed prediction\\n')"
        for final in (False, True):
            with self.subTest(final=final):
                self.prediction.write_bytes(original)
                binary = self.product("base", actions={"edges": mutator})
                steps = [self.run_step()]
                if not final:
                    steps.append(self.run_step("later", binary="base"))
                self.next_output(f"changed-{final}")
                self.gate(self.plan(steps, base=binary), expected=2)
                self.assertIn("prediction-changed", self.report() + self.diagnostic)
                self.assertEqual(self.prediction.read_bytes(), b"changed prediction\n")
                self.assertEqual(self.result()["result"], "refused")
                self.assertEqual([row["command"] for row in self.fake_log()],
                                 ["version", "dfg-stats", "call-stats", "edges"])
                if not final:
                    self.assertEqual(self.step("later")["status"], "not_run")

    def test_decision_bytes_and_locator_independence_with_real_reuse(self):
        """gate-010, astra-gate-001, sol2-gate-002, n4-5: exact canonical decision survives moves."""
        require_gate("gate_decision")
        plan = self.pair()
        plan["lane"] = "décision"
        original = canonical(self.projection(plan))
        pin = digest(original)
        self.gate(plan, pin=pin)
        self.assertEqual((self.out / "decision.json").read_bytes(), original)
        self.assertEqual(self.result()["gate_sha256"], pin)
        self.assertEqual(self.result()["decision_pin"], pin)
        old = self.out / "base"
        moved = self.work / "relocated-receipt"
        shutil.copytree(old, moved, symlinks=True)
        executable = self.work / "relocated-binary"
        shutil.copy2(self.fake, executable)
        relocated_guard = self.guard.with_name("moved guard")
        shutil.copy2(self.guard, relocated_guard)
        self.prediction = self.prediction.rename(self.prediction.with_name("copied-prediction.md"))
        relocated = self.reuse(plan, "base", moved)
        relocated["prediction"] = self.prediction.name
        for step in relocated["steps"][:2]:
            step["memguard"] = str(relocated_guard)
        self.next_output("relocated")
        self.gate(relocated, pin=pin, overrides={"base": executable})
        self.assertEqual((self.out / "decision.json").read_bytes(), original)
        self.assertEqual(self.result()["gate_sha256"], pin)
        self.assertEqual({row["arm"] for row in self.fake_log()}, {"head"})

    def test_manifest_local_and_corpora_root_are_locators(self):
        """n2-14: the effective manifest binds the declared content fields, ignoring local."""
        require_gate("gate_decision")
        plan = self.plan([{"id": "inventory", "do": "corpora-list"}, self.run_step()], base=self.fake)
        original = canonical(self.projection(plan))
        pin = digest(original)
        self.gate(plan, pin=pin)
        self.assertEqual((self.out / "decision.json").read_bytes(), original)
        moved = self.work / "relocated-corpora"
        moved.mkdir()
        self.source.rename(moved / "moved-corpus")
        self.entry["local"] = ["moved-corpus"]
        self.write_manifest()
        self.next_output("moved-corpus")
        self.gate(plan, roots=[moved], pin=pin)
        self.assertEqual((self.out / "decision.json").read_bytes(), original)
        self.entry["history"] = not self.entry["history"]
        self.write_manifest()
        self.next_output("changed-registration")
        self.gate(plan, pin=pin, expected=2)
        self.assert_no_launch()

    def test_fresh_differential_requires_registered_pin(self):
        """gate-022: an edited all-fresh gate and edited prediction cannot reuse an old pin."""
        require_gate("gate_decision")
        base = self.product("base", edges=[edge(), edge(sb=155), edge(sb=156), edge(sb=157)])
        head = self.product("head", edges=[edge()])
        self.classifier.write_text('CLASSES=["counted"]\ndef classify(change, context): return None\n')
        plan = self.pair(base=base, head=head)
        pin = self.pin(plan)
        self.gate(plan, pin=pin, expected=1)
        self.assertEqual(self.step("compare")["metrics"]["unattributed"], 3)
        changed = deepcopy(plan)
        changed["steps"][-1]["expect"]["unattributed"] = {"max": 3}
        for label, test_pin in (("missing", None), ("stale", pin), ("malformed", "Z" * 64)):
            self.next_output(label)
            self.gate(changed, pin=test_pin, expected=2)
            self.assert_no_launch()
        self.prediction.write_bytes(b"Changed after measurement.\n")
        self.next_output("also-edited-prediction")
        self.gate(changed, pin=pin, expected=2)
        self.assert_no_launch()
        self.next_output("retrospective")
        self.gate(changed, retrospective=True, expected=1)
        self.assertEqual((self.result()["result"], self.result()["product_steps"]), ("retrospective", []))
        self.assertEqual(self.result()["prediction_sha256"], sha256_of(self.prediction))
