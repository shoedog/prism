"""Step orchestration, wrapper custody and prospective run admission."""

from __future__ import annotations

from copy import deepcopy
import gzip
import json
from pathlib import Path
import shutil
import sys
from unittest.mock import patch

from .comparator_fixtures import edge
from .gate_fixtures import (GateWorkbench, QUIET, artifact, jsonl, read_json, require_gate,
                            save_json)
from .helpers import sha256_of


class GateStepsTests(GateWorkbench):
    def assert_refused_before_launch(self, plan, *, pin=None, retrospective=False):
        self.gate(plan, expected=2, pin=pin, retrospective=retrospective)
        self.assertEqual((self.result()["result"], self.result()["product_steps"]), ("refused", []))
        self.assert_no_launch()

    def test_corpora_run_and_comparison_steps_publish_measured_outputs(self):
        """gate-013, gate-019, gate-024: exercise list, verify, run, edges, and stats via public CLI."""
        require_gate("gate_steps")
        plan = self.pair()
        plan["steps"][:0] = [{"id": "list", "do": "corpora-list", "expect": {"listed": 1}},
                            {"id": "verify", "do": "corpora-verify", "keys": [self.key],
                             "expect": {"verified": 1}}]
        plan["steps"][2]["expect"] = {"commands": 3}
        allow = self.work / "allow.txt"
        allow.write_bytes(b"count\n")
        plan["steps"].append({"id": "stats", "do": "diff-stats", "base": "base", "head": "head",
                              "allow": allow.name, "expect": {"deltas": 0, "forbidden": 0}})
        pin = self.pin(plan)
        self.gate(plan, pin=pin)
        for ident in ("list", "verify", "base", "head", "compare", "stats"):
            self.assertEqual(self.step(ident)["status"], "pass")
            self.assertEqual(self.record(ident)["gate_step_id"], ident)
        self.assertEqual(self.step("base")["metrics"]["commands"], 3)
        self.assertEqual(self.step("compare")["metrics"]["base_edges"], 1)
        self.assertEqual(self.step("stats")["metrics"]["tables"]["dfg"]["base_leaves"], 1)
        record = read_json(self.out / "compare/certificate.json")
        self.assertEqual((record["gate_sha256"], record["gate_step_id"]), (pin, "compare"))

    def test_oracle_edges_and_calls_receive_gate_binding(self):
        """gate-024: both oracle step kinds publish actual child receipts with gate bindings."""
        require_gate("gate_steps")
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self) as oracle:
            for kind in ("edges", "calls"):
                with self.subTest(kind=kind):
                    self.next_output(kind)
                    plan = (oracle.edges([("binding", "other", "CORRECT")]) if kind == "edges"
                            else oracle.calls([("exact", "local_def", "CORRECT")]))
                    pin = self.pin(plan)
                    self.gate(plan, pin=pin)
                    self.assertEqual(self.step("grade")["metrics"]["denominators"]["decided"], 1)
                    record = read_json(self.out / "grade/oracle-receipt.json")
                    self.assertEqual((record["gate_sha256"], record["gate_step_id"]), (pin, "grade"))
                    self.assertIn("oracle-receipt.json", self.record("grade")["artifacts"])
                    events = [json.loads(line) for line in self.guard_events.read_bytes().splitlines()]
                    self.assertEqual(sum(row["command"] == "node" for row in events), 1)

    def test_classifier_child_two_refuses_and_stops_graph(self):
        """gate-018: child CLI 2 is retained and the first failing step stops later work."""
        require_gate("gate_steps")
        self.classifier.write_text('CLASSES = []\n')
        plan = self.pair()
        plan["steps"].append(self.run_step("later", binary="base"))
        self.gate(plan, expected=2, pin=self.pin(plan))
        self.assertEqual((self.result()["result"], self.step("compare")["status"],
                          self.step("compare")["child_exit"]), ("refused", "refused", 2))
        self.assertEqual(self.step("later")["status"], "not_run")
        self.assertEqual(self.step("later")["artifacts"], {})
        self.assertFalse((self.out / "later/step-receipt.json").exists())
        self.assertEqual(len(self.fake_log()), 8)
        wrapper = self.record("compare")
        self.assertEqual(wrapper["inputs"]["files"]["compare/classifier"], sha256_of(self.classifier))
        self.assertEqual(wrapper["artifacts"], {})

    def test_raw_product_exits_are_failures_even_for_two_or_75(self):
        """gate-018: the raw product exit survives run's CLI 4 and cannot become host quiet."""
        require_gate("gate_steps")
        for code in (2, 7, 75):
            with self.subTest(code=code):
                self.next_output(f"child-{code}")
                binary = self.product("base", actions={"call-stats": f"sys.exit({code})"})
                plan = self.plan([self.run_step(), self.run_step("later", binary="base")], base=binary)
                self.gate(plan, expected=1)
                step = self.step("base")
                self.assertEqual((self.result()["result"], step["child_exit"], step["status"]),
                                 ("fail", code, "fail"))
                self.assertEqual(step["guard_status"], {"kind": "child", "launched": True,
                                                        "child_exit": code})
                self.assertEqual(self.step("later")["status"], "not_run")

    def test_quiet_run_wrapper_records_partial_artifacts(self):
        """gate-018, n4-15: a structured quiet refusal yields 75 and a refused partial wrapper."""
        require_gate("gate_steps")
        self.make_guard({"call-stats": (75, QUIET)})
        plan = self.plan([self.run_step(), self.run_step("later", binary="base")], base=self.fake)
        self.gate(plan, expected=75)
        step, wrapper = self.step("base"), self.record("base")
        self.assertEqual((self.result()["result"], step["status"], step["guard_status"]),
                         ("host_quiet", "refused", QUIET))
        self.assertEqual(wrapper["status"], "refused")
        self.assertEqual(wrapper["artifacts"], step["artifacts"])
        self.assertIn("dfg-stats.json", wrapper["artifacts"])
        self.assertEqual(wrapper["artifacts"]["dfg-stats.json"], artifact(self.out / "base/dfg-stats.json"))
        self.assertEqual(self.step("later")["status"], "not_run")
        self.assertFalse((self.out / "later/step-receipt.json").exists())

    def test_oracle_admission_three_and_node_75_have_different_results(self):
        """gate-018: missing self-check is refused; a launched Node 75 is a failure."""
        require_gate("gate_steps")
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self, selfcheck=False) as oracle:
            plan = oracle.edges([("binding", "other", "CORRECT")])
            pin = self.pin(plan)
            self.gate(plan, expected=2, pin=pin)
            self.assertEqual((self.result()["result"], self.step("grade")["child_exit"]), ("refused", 3))
            oracle.selfcheck()
            self.next_output("node-exit")
            (self.work / "node-exit").write_text("75")
            self.gate(plan, expected=1, pin=pin)
            self.assertEqual((self.result()["result"], self.step("grade")["child_exit"]), ("fail", 75))

    def test_persistent_executable_mutation_refuses_even_if_child_failed(self):
        """n3-10: persistent measurement-side replacement is a custody refusal, exit 2."""
        require_gate("gate_steps")
        action = "p = Path(sys.argv[0]); p.write_bytes(p.read_bytes() + b'# changed' + bytes([10]))"
        plan = self.plan([self.run_step(), self.run_step("later", binary="base")],
                         base=self.product("base", actions={"edges": action}))
        before = sha256_of(self.fake)
        self.gate(plan, expected=2)
        self.assertNotEqual(sha256_of(self.fake), before)
        self.assertEqual((self.result()["result"], self.step("base")["status"],
                          self.step("later")["status"]), ("refused", "refused", "not_run"))
        self.assertIn("executable", self.report() + self.diagnostic)
        self.next_output("unchanged-control")
        self.product("base")
        self.gate(plan)
        self.assertEqual(self.result()["result"], "pass")

    def test_reuse_freezes_threshold_order_and_input_dependencies(self):
        """gate-011: prospective reuse cannot revise a frozen graph; retrospective keeps both hashes."""
        require_gate("gate_steps")
        base = self.product("base", edges=[edge(), edge(sb=155), edge(sb=156), edge(sb=157)])
        head = self.product("head", edges=[edge()])
        self.classifier.write_text('CLASSES=["fixture"]\ndef classify(change, context): return None\n')
        plan = self.pair(base=base, head=head)
        pin = self.pin(plan)
        prior = self.prior_run(plan, "head", self.work / "prior-head", gate_pin=pin,
                               prediction=self.prediction)
        plan = self.reuse(plan, "head", prior)
        edits = {}
        edits["threshold"] = deepcopy(plan)
        edits["threshold"]["steps"][-1]["expect"] = {"unattributed": {"max": 3}}
        edits["dependency"] = deepcopy(plan)
        edits["dependency"]["steps"][-1].update(base="head", head="base")
        edits["reordered"] = {**plan, "steps": [plan["steps"][1], plan["steps"][0], plan["steps"][2]]}
        alternate = self.classifier.with_name("alternate.py")
        alternate.write_bytes(self.classifier.read_bytes() + b"# alternate\n")
        edits["classifier"] = deepcopy(plan)
        edits["classifier"]["steps"][-1]["classifier"] = alternate.name
        for label, candidate in edits.items():
            with self.subTest(label=label):
                self.next_output(label)
                self.assert_refused_before_launch(candidate, pin=self.pin(candidate))
        self.next_output("retrospective")
        self.gate(edits["threshold"], expected=1, retrospective=True)
        self.assertEqual(self.result()["result"], "retrospective")
        self.assertEqual(self.result()["claim_scope"], "none")
        self.assertIn(pin, self.report())
        self.assertIn(self.pin(edits["threshold"]), self.report())
        self.next_output("same-plan")
        self.gate(plan, expected=1, pin=pin)
        self.assertEqual(self.step("compare")["metrics"]["unattributed"], 3)
        self.assertEqual({row["arm"] for row in self.fake_log()}, {"base"})

    def test_reuse_checks_binary_digest_version_step_and_options_in_both_modes(self):
        """gate-012: receipts cannot occupy another slot or configuration even retrospectively."""
        require_gate("gate_steps")
        plan = self.pair()
        pin = self.pin(plan)
        prior = self.prior_run(plan, "head", self.work / "prior-head", gate_pin=pin,
                               prediction=self.prediction)
        plan = self.reuse(plan, "head", prior)
        alternate = self.work / "same-version-different-executable"
        shutil.copy2(plan["binaries"]["head"]["path"], alternate)
        with alternate.open("ab") as script:
            script.write(b"\n# distinct executable bytes\n")
        candidates = {}
        candidates["sha"] = deepcopy(plan)
        candidates["sha"]["binaries"]["head"]["path"] = str(alternate)
        candidates["version"] = deepcopy(plan)
        candidates["version"]["binaries"]["head"]["expect_version"] = "abc1234"
        candidates["step"] = deepcopy(plan)
        candidates["step"]["steps"][1]["id"] = "other"
        candidates["step"]["steps"][-1]["head"] = "other"
        candidates["options"] = deepcopy(plan)
        candidates["options"]["steps"][1]["sites"] = True
        for label, candidate in candidates.items():
            for retrospective in (False, True):
                with self.subTest(label=label, retrospective=retrospective):
                    self.next_output(f"{label}-{retrospective}")
                    self.assert_refused_before_launch(candidate, pin=self.pin(candidate),
                                                      retrospective=retrospective)
        self.next_output("matching")
        self.gate(plan, pin=pin)
        self.assertEqual(self.step("head")["status"], "pass")
        self.assertEqual({row["arm"] for row in self.fake_log()}, {"base"})

    def test_corrupt_failed_or_prediction_mismatched_reuse_refuses_before_launch(self):
        """terra-gate-001, terra3-gate-001, sol-gate-001, sol-gate-002: run reader and frozen prediction both apply."""
        require_gate("gate_steps")
        plan = self.pair(head=self.product("head", actions={"edges": "sys.exit(7)"}))
        pin = self.pin(plan)
        failed = self.prior_run(plan, "head", self.work / "failed", gate_pin=pin,
                                prediction=self.prediction, expected=4)
        broken = self.reuse(plan, "head", failed)
        self.next_output("failed-receipt")
        self.assert_refused_before_launch(broken, pin=pin)

        plan = self.pair()
        pin = self.pin(plan)
        prior = self.prior_run(plan, "head", self.work / "prior", gate_pin=pin,
                               prediction=self.prediction)
        reused = self.reuse(plan, "head", prior)
        target = prior / "edges.jsonl.gz"
        stored = target.read_bytes()
        target.write_bytes(stored + b"corruption")
        for retrospective in (False, True):
            self.next_output(f"corrupt-{retrospective}")
            self.assert_refused_before_launch(reused, pin=pin, retrospective=retrospective)
        target.write_bytes(stored)
        self.prediction.write_bytes(b"different prediction\n")
        self.next_output("head-prediction")
        self.assert_refused_before_launch(reused, pin=self.pin(plan))
        self.assertIn("prediction-digest-mismatch", self.report() + self.diagnostic)

    def test_reuse_requires_controller_supplied_digest(self):
        """n1-14: a consistently resealed run cannot authenticate itself."""
        require_gate("gate_steps")
        from measure import receipt
        from measure.corpora import load_manifest

        plan = self.pair()
        pin = self.pin(plan)
        prior = self.prior_run(plan, "head", self.work / "prior", gate_pin=pin,
                               prediction=self.prediction)
        original = self.reuse(plan, "head", prior)
        self.next_output("admitted")
        self.gate(original, pin=pin)
        forged = read_json(prior / "receipt.json")
        stream = prior / "edges.jsonl.gz"
        stream.write_bytes(gzip.compress(jsonl([edge(sb=250)]), mtime=0))
        changed = artifact(stream)
        forged["artifacts"][stream.name] = changed
        command = next(row for row in forged["commands"] if row["id"] == "edges")
        command.update(output_sha256=changed["sha256"], bytes=changed["bytes"])
        save_json(prior / "receipt.json", forged)
        receipt.admit(prior, entries=load_manifest(self.manifest))
        self.next_output("stale-independent-digest")
        self.assert_refused_before_launch(original, pin=pin)
        self.next_output("missing-independent-digest")
        self.assert_refused_before_launch(self.reuse(plan, "head", prior, receipt_sha256=False), pin=pin)

    def test_wrappers_round_trip_and_bind_predecessor_and_artifact_sets(self):
        """gate-024: a reused and a fresh wrapper bind direct child inputs and every output."""
        require_gate("gate_steps")
        plan = self.pair()
        pin = self.pin(plan)
        prior = self.prior_run(plan, "base", self.work / "prior", gate_pin=pin,
                               prediction=self.prediction)
        plan = self.reuse(plan, "base", prior)
        self.next_output("wrapped")
        self.gate(plan, pin=pin)
        for ident in ("base", "head", "compare"):
            wrapper = self.record(ident)
            step = self.step(ident)
            self.assertEqual(set(wrapper), {"gate_sha256", "gate_step_id", "prediction_sha256",
                                            "status", "inputs", "artifacts"})
            self.assertEqual((wrapper["gate_sha256"], wrapper["gate_step_id"],
                              wrapper["prediction_sha256"], wrapper["status"]),
                             (pin, ident, sha256_of(self.prediction), "pass"))
            self.assertEqual(set(wrapper["inputs"]), {"files", "steps"})
            self.assertEqual(wrapper["artifacts"], step["artifacts"])
            self.assertNotIn("step-receipt.json", wrapper["artifacts"])
            for path, expected in wrapper["artifacts"].items():
                self.assertEqual(artifact(self.out / ident / path), expected)
        base, head, compare = map(self.record, ("base", "head", "compare"))
        expected = {name: {"path": f"{name}/step-receipt.json",
                           "sha256": sha256_of(self.out / name / "step-receipt.json")}
                    for name in ("base", "head")}
        self.assertEqual(compare["inputs"]["steps"], expected)
        self.assertEqual(compare["inputs"]["files"]["compare/classifier"], sha256_of(self.classifier))
        self.assertEqual(base["inputs"]["files"]["base/reuse"], sha256_of(prior / "receipt.json"))
        self.assertTrue(any((self.out / "base" / path).read_bytes() == (prior / "receipt.json").read_bytes()
                            for path in base["artifacts"]))
        self.assertEqual(head["inputs"]["files"]["head/binary/head"], sha256_of(Path(plan["binaries"]["head"]["path"])))
        self.assertEqual(head["inputs"]["files"]["head/corpus_input"],
                         read_json(self.out / "head/receipt.json")["corpus"]["input_sha256"])

    def test_damaged_predecessor_wrapper_refuses_dependent_step(self):
        """gate-024: swapped ID, missing role, wrong size and missing required output all refuse."""
        require_gate("gate_steps")
        from measure import cli

        plan = self.pair()
        pin = self.pin(plan)
        run = cli.run
        edits = {
            "swapped-id": lambda row: row.update(gate_step_id="head"),
            "missing-role": lambda row: row["inputs"]["files"].pop("base/manifest"),
            "wrong-size": lambda row: row["artifacts"]["edges.jsonl.gz"].update(bytes=0),
            "unlisted-output": lambda row: row["artifacts"].pop("edges.jsonl.gz"),
        }
        for label, edit in edits.items():
            with self.subTest(label=label):
                self.next_output(label)

                def after_head(*args, **kwargs):
                    status = run(*args, **kwargs)
                    if kwargs["gate_step_id"] == "head":
                        path = self.out / "base/step-receipt.json"
                        row = read_json(path)
                        edit(row)
                        save_json(path, row)
                    return status

                with patch.object(cli, "run", side_effect=after_head):
                    self.gate(plan, pin=pin, expected=2)
                self.assertEqual(self.result()["result"], "refused")
                self.assertFalse((self.out / "compare/certificate.json").exists())

    def test_classifier_only_sees_controller_registered_sidecars(self):
        """gate-021, astra-join-006: verified role files enable attribution; ambient files stay hidden."""
        require_gate("gate_steps")
        from measure import cli

        self.classifier.write_text("""
CLASSES = ["marker"]
def classify(change, context):
    try:
        from marker_helper import EXPECTED
    except ImportError:
        return None
    marker = context.base_run / "marker.txt"
    return "marker" if marker.is_file() and marker.read_text() == EXPECTED else None
""")
        helper = self.work / "marker_helper.py"
        helper.write_text("EXPECTED = 'accepted'\n")
        marker = self.work / "bound-marker.txt"
        marker.write_text("accepted")
        plan = self.pair(base=self.product("base", edges=[edge(), edge(sb=191)]))
        roles = {
            "base": {}, "head": {},
            "helpers": {helper.name: {"path": helper.name, "sha256": sha256_of(helper)}},
        }
        plan["steps"][-1]["classifier_files"] = roles
        pin = self.pin(plan)
        prior = self.prior_run(plan, "base", self.work / "prior", gate_pin=pin,
                               prediction=self.prediction)
        prior_head = self.prior_run(plan, "head", self.work / "prior-head", gate_pin=pin,
                                    prediction=self.prediction)
        ambient = prior / "marker.txt"
        ambient.write_bytes(marker.read_bytes())
        with patch.object(sys, "path", [str(self.work), *sys.path]), patch.dict(sys.modules):
            standalone = self.local_command(["--manifest", self.manifest, "diff-edges", prior, prior_head,
                                             "--classifier", self.classifier, "--out", self.work / "standalone"],
                                            command=cli.main)
        self.assert_exit(standalone, 0)
        self.assertEqual(read_json(self.work / "standalone/certificate.json")["attributed"], 1)

        reused = self.reuse(plan, "base", prior)
        for name, value in (("unlisted-match", "accepted"), ("unlisted-change", "changed")):
            ambient.write_text(value)
            self.next_output(name)
            self.gate(reused, pin=pin, expected=1)
            self.assertEqual(self.step("compare")["metrics"]["attributed"], 0)

        roles["base"]["marker.txt"] = {"path": marker.name, "sha256": sha256_of(marker)}
        pin = self.pin(plan)
        registered = self.prior_run(plan, "base", self.work / "registered", gate_pin=pin,
                                    prediction=self.prediction)
        plan = self.reuse(plan, "base", registered)
        self.next_output("registered-role")
        self.gate(plan, pin=pin)
        self.assertEqual(self.step("compare")["metrics"]["attributed"], 1)
        roles = self.record("compare")["inputs"]["files"]
        self.assertEqual(roles["compare/classifier_files/base/marker.txt"], sha256_of(marker))
        self.assertEqual(roles[f"compare/classifier_files/helpers/{helper.name}"], sha256_of(helper))

        for input_path in (marker, helper):
            original = input_path.read_bytes()
            input_path.write_bytes(original + b"# changed\n")
            self.next_output(f"changed-{input_path.name}")
            self.assert_refused_before_launch(plan, pin=pin)
            input_path.write_bytes(original)
        moved = deepcopy(plan)
        files = moved["steps"][-1]["classifier_files"]
        files["head"] = files["base"]
        files["base"] = {}
        self.next_output("moved-role")
        self.assert_refused_before_launch(moved, pin=pin)

        registered_marker = registered / "marker.txt"
        registered_marker.write_bytes(marker.read_bytes())
        record = read_json(registered / "receipt.json")
        record["artifacts"]["marker.txt"] = artifact(registered_marker)
        save_json(registered / "receipt.json", record)
        self.next_output("undeclared-artifact")
        self.assert_refused_before_launch(self.reuse(plan, "base", registered), pin=pin)
