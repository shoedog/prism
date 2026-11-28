"""Review gate admission, structured arguments, and population accounting."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import re
import shutil
from unittest.mock import patch

from .gate_fixtures import GateFixture, GateWorkbench, QUIET, read_json, require_gate, section
from .helpers import DEFAULT_VERSION, sha256_of
from .test_population import METRICS, ReviewFixture, spec_for


class GateReviewTests(GateFixture, ReviewFixture):
    def make_arm(self, role, *args, version=DEFAULT_VERSION, **options):
        binary = super().make_arm(role, *args, **options)
        old = repr(DEFAULT_VERSION + f" fixture {role}" + chr(10))
        source = binary.read_text()
        self.assertIn(old, source)
        banner = f"slicing 1.2.3 ({version})\n"
        binary.write_text(source.replace(old, repr(banner)))
        return binary

    def review_plan(self, **options):
        step = dict(id="review", do="review-diff", population=self.population.name,
                    clones=str(self.clones), arms=[dict(name=name, binary=name, args=[])
                                                   for name in ("base", "head")],
                    expect={"ok": {"min": 1}})
        step.update(options)
        return self.plan([step], **self.binaries)

    def gate_review(self, plan, expected=0, *, pin=None):
        self.gate(plan, expected=expected, pin=self.pin(plan) if pin is None else pin)
        return self.step("review")

    def test_review_child_versions_structured_arguments_and_clean_environment(self):
        """n3-9, n2-11, n4-19, gate-024: real review child records pinned arms and literal argv."""
        require_gate("gate_steps")
        self.make_population()
        self.env["PRISM_TEST_INHERITED"] = "must not reach the product"
        self.env["PRISM_EDGES_BYTES"] = "1"
        self.env["GIT_CONFIG_GLOBAL"] = "/dev/null"
        plan = self.review_plan()
        arm = plan["steps"][0]["arms"][1]
        arm["args"] = ["--taint-source", "std::exec"]
        step = self.gate_review(plan)
        self.assertEqual((step["status"], step["metrics"]["ok"]), ("pass", 1))
        review = read_json(self.out / "review/review-receipt.json")
        self.assertEqual((review["gate_sha256"], review["gate_step_id"]), (self.pin(plan), "review"))
        self.assertEqual(review["arms"]["head"]["arm_args"], ["--taint-source", "std::exec"])
        log = self.attempts()
        self.assertEqual([row["role"] for row in log], ["base", "head"])
        self.assertIn("std::exec", log[1]["argv"])
        for row in self.fake_log():
            self.assertFalse(any(key.startswith("PRISM_") for key in row["env"]))
        inputs = json.dumps(self.record("review")["inputs"], sort_keys=True)
        self.assertIn("PRISM_TEST_INHERITED", inputs)
        self.assertIn("PRISM_EDGES_BYTES", inputs)
        self.assertNotIn(self.env["PRISM_TEST_INHERITED"], inputs)

    def test_review_version_pin_is_checked_for_each_arm(self):
        """n3-9/n2-13: fresh review arms with incorrect or invalid pins cannot contribute."""
        require_gate("gate_steps")
        self.make_population()
        for role in ("base", "head"):
            self.next_output(f"invalid-{role}")
            plan = self.review_plan()
            plan["binaries"][role]["expect_version"] = "abc"
            self.gate_review(plan, 2)
            self.assert_no_launch()
            self.next_output(f"mismatch-{role}")
            self.make_arm(role, version="f" * 12)
            failed = self.gate_review(self.review_plan(), 2)
            self.assertEqual(failed["claim_scope"], "none")
            self.assertEqual(self.result()["product_steps"], [])
            self.assertFalse(any(row["role"] == role for row in self.attempts()))
            self.make_arm(role)
        self.next_output("both-matching")
        self.gate_review(self.review_plan())

    def test_review_credit_requires_eligible_lines_and_a_declared_claim(self):
        """f5, n2-12: zero-line review is control; a populated unstated claim is execution."""
        require_gate("gate_credit")
        self.make_population(spec=spec_for({}), edits={"README.md": "no selected source lines\n"})
        step = self.gate_review(self.review_plan())
        self.assertEqual((step["metrics"]["ok"], step["claim_scope"]), (1, "control"))
        self.assertEqual(self.result()["product_steps"], [])
        self.next_output("populated")
        self.make_population()
        plan = self.review_plan()
        step = self.gate_review(plan)
        self.assertEqual((step["metrics"]["ok"], step["claim_scope"]), (1, "product"))
        self.next_output("descriptive")
        plan["steps"][0].pop("expect")
        self.assertEqual(self.gate_review(plan)["claim_scope"], "execution")

    def test_reused_run_binds_population_and_clones_path_is_a_locator(self):
        """gate-011, n4-5: a clone move preserves the decision; population registration does not."""
        require_gate("gate_decision")
        self.make_population()
        runner = GateWorkbench()
        runner.setUp()
        self.addCleanup(runner.doCleanups)
        population = runner.work / self.population.name
        shutil.copyfile(self.population, population)
        plan = self.review_plan()
        plan["steps"].insert(0, runner.run_step("analysis"))
        plan["binaries"]["analysis"] = {"path": str(runner.fake), "expect_version": DEFAULT_VERSION}
        pin = runner.pin(plan)
        previous = runner.prior_run(plan, "analysis", runner.work / "prior", gate_pin=pin,
                                    prediction=runner.prediction)
        plan = runner.reuse(plan, "analysis", previous)
        clones = self.clones.rename(self.clones.with_name("relocated clones"))
        plan["steps"][-1]["clones"] = str(clones)
        runner.next_output("relocated")
        runner.gate(plan, pin=pin)
        self.assertEqual(runner.result()["gate_sha256"], pin)
        self.assertEqual(runner.result()["product_steps"], ["review"])
        self.assertEqual(runner.fake_log(), [])
        self.log.unlink()

        population.write_bytes(population.read_bytes() + b"\n")
        runner.next_output("changed-population")
        new_pin = runner.pin(plan)
        self.assertNotEqual(new_pin, pin)
        runner.gate(plan, expected=2, pin=new_pin)
        runner.assert_no_launch()
        self.assertEqual(self.fake_log(), [])

    def test_duplicate_spec_digests_count_ordinals_and_distinct_diffs_separately(self):
        """n1-28: two successful ordinals for the same spec count as one distinct diff."""
        require_gate("gate_credit")
        self.make_population(ordinals=(1, 2))
        plan = self.review_plan(expect={"ok": {"min": 2}})
        step = self.gate_review(plan)
        self.assertEqual((step["metrics"]["ok"], step["claim_scope"]), (2, "product"))
        population = section(self.report(), "Population and denominators")
        self.assertRegex(population, r"(?i)2\s+of\s+2")
        self.assertRegex(population, r"(?i)(?:1\s+distinct diff|distinct diff[s ]*[:=]\s*1)")

    def test_output_neutral_review_arguments_require_identity_control(self):
        """n4-2: output-neutral options do not distinguish arms under forced JSON."""
        require_gate("gate_credit")
        self.make_population()
        for index, args in enumerate((["--review-min-severity", "suggestion"],
                                      ["--review-min-severity=suggestion"], ["--no-cache"],
                                      ["--cache-dir", "isolated cache"])):
            with self.subTest(args=args):
                plan = self.review_plan()
                plan["binaries"]["head"]["path"] = plan["binaries"]["base"]["path"]
                plan["steps"][0]["arms"][1]["args"] = args
                self.next_output(f"undeclared-{index}")
                self.gate_review(plan, 2)
                self.assert_no_launch()
                self.next_output(f"declared-{index}")
                plan["steps"][0]["identity_control"] = True
                self.assertEqual(self.gate_review(plan)["claim_scope"], "control")
                self.assertEqual(self.result()["product_steps"], [])
        self.next_output("measurement-option")
        plan["steps"][0].pop("identity_control")
        plan["steps"][0]["arms"][1]["args"] = ["--taint-source", "std::exec"]
        self.assertEqual(self.gate_review(plan)["claim_scope"], "product")

    def test_partial_population_names_each_refusal_without_zero_filling(self):
        """gate-014, known-s4-5: the 100 ok / 20 gitlink-refused accounting is explicit."""
        require_gate("gate_steps")
        self.make_population(ordinals=range(1, 121))
        self.git("update-index", "--add", "--cacheinfo", f"160000,{self.commit},vendor/dep")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                 "-c", "commit.gpgsign=false", "commit", "-q", "--amend", "--no-edit")
        bad_commit, bad_tree = self.git("rev-parse", "HEAD"), self.git("rev-parse", "HEAD^{tree}")
        for row in self.registration["diffs"][100:]:
            row.update(commit=bad_commit, tree=bad_tree)
        self.write_population()
        step = self.gate_review(self.review_plan(sample=0, allow_empty=True), 1)
        self.assertEqual((step["child_exit"], step["metrics"]["ok"]), (1, 100))
        refused = list(range(101, 121))
        self.assertEqual(step["metrics"]["refused_ordinals"], refused)
        rows = [json.loads(line) for line in (self.out / "review/per-diff.jsonl").read_bytes().splitlines()]
        self.assertEqual([row["ordinal"] for row in rows if row["status"] == "ok"], list(range(1, 101)))
        self.assertEqual([row["ordinal"] for row in rows if row["status"] == "refused"], refused)
        population = section(self.report(), "Population and denominators")
        self.assertIn("100 of 120", population)
        self.assertIn("gitlink", population)
        for row in rows[100:]:
            self.assertTrue(all(row[key] is None for key in METRICS))
            self.assertIn("gitlink", row["reason"])
            self.assertRegex(population, rf"\b{row['ordinal']}\b")
        self.assertEqual(self.result()["product_steps"], [])
        self.next_output("only-refused")
        step = self.gate_review(self.review_plan(only=[101], allow_empty=True), 1)
        self.assertEqual((step["metrics"]["ok"], step["metrics"]["refused_ordinals"]), (0, [101]))

    def test_review_population_binary_and_row_identity_mismatch_refuse(self):
        """known-s4-3, known-s4-5: gate expectations independently bind the live receipt and row identity."""
        require_gate("gate_steps")
        from measure import review_diff, review_receipt

        self.make_population()
        plan = self.review_plan()
        self.gate_review(plan)
        real_publish = review_diff.publish
        for kind in ("population", "binary", "row-tree"):
            with self.subTest(kind=kind):
                self.next_output(kind)

                def publish(out, stage, registered, arms, rows, size, seed, artifacts, gate, step):
                    arms, rows = deepcopy(arms), deepcopy(rows)
                    if kind == "population":
                        substitute = self.work / "alternate-population.json"
                        substitute.write_bytes(self.population.read_bytes() + b"\n")
                        registered = review_diff.population.load(substitute)
                    elif kind == "binary":
                        arms["head"]["binary"]["sha256"] = "f" * 64
                    else:
                        for arm in rows[0]["arms"].values():
                            arm["input_before_sha256"] = arm["input_after_sha256"] = "f" * 64
                    real_publish(out, stage, registered, arms, rows, size, seed, artifacts, gate, step)
                    review_receipt.admit(out, population=substitute if kind == "population" else self.population)

                with patch.object(review_diff, "publish", side_effect=publish):
                    self.gate_review(plan, 2)
                self.assertEqual(self.result()["result"], "refused")
                self.assertEqual(self.result()["product_steps"], [])

    def test_review_guard_and_quiet_helper_identity_mismatch_refuse(self):
        """known-s4-3, known-s4-5: a live child measuring under a substituted guard input cannot pass."""
        require_gate("gate_steps")
        from measure import review_diff

        self.make_population()
        plan = self.review_plan()
        self.gate_review(plan)
        original_init = review_diff.ReviewExecution.__init__
        for kind in ("guard", "quiet-helper"):
            with self.subTest(kind=kind):
                self.next_output(f"substituted-{kind}")
                other = self.work / kind / "bin/guard"
                other.parent.mkdir(parents=True)
                shutil.copy2(self.guard, other)
                helper = other.parent.parent / "hostquiet/host_quiet.py"
                helper.parent.mkdir()
                shutil.copyfile(self.quiet_helper, helper)
                path = other if kind == "guard" else helper
                with path.open("ab") as fixture:
                    fixture.write(b"\n# not the registered launch input\n")

                def swapped(execution, stage, guard, arms):
                    original_init(execution, stage, other, arms)

                with patch.object(review_diff.ReviewExecution, "__init__", swapped):
                    self.gate_review(plan, 2)
                self.assertEqual(self.result()["result"], "refused")
                self.assertEqual(self.result()["product_steps"], [])

    def test_review_gate_refuses_contradictory_quiet_states(self):
        """known-s4-5: quiet event, reason, and tail must agree across selected ordinals."""
        require_gate("gate_steps")
        from measure import review_diff, review_receipt
        from .test_review_diff import ReviewFoldOneTests

        self.make_population(ordinals=(1, 2))
        plan = self.review_plan()
        live_guard = self.guard.read_bytes()
        self.make_guard({"review": (75, QUIET)})
        step = self.gate_review(plan, 75)
        self.assertEqual(step["status"], "refused")
        self.assertEqual(step["metrics"]["refused_ordinals"], [1, 2])
        real_publish = review_diff.publish
        for kind in ("success-after-quiet", "wrong-reason", "wrong-tail", "no-event",
                     "nonquiet-guard-refused"):
            with self.subTest(kind=kind):
                self.next_output(kind)
                if kind == "success-after-quiet":
                    self.guard.write_bytes(live_guard)

                def publish(out, stage, registered, arms, rows, size, seed, artifacts, gate, step):
                    arms, rows, artifacts = deepcopy(arms), deepcopy(rows), dict(artifacts)
                    if kind == "success-after-quiet":
                        ReviewFoldOneTests.quiet_first_ordinal(self)(rows, {"arms": arms, "artifacts": artifacts})
                    elif kind == "wrong-reason":
                        rows[0]["reason"] = "review-guard-failed"
                    elif kind == "wrong-tail":
                        rows[1]["reason"] = "review-input-changed"
                    elif kind == "no-event":
                        rows[0]["arms"]["base"]["guard_status"] = None
                    else:
                        rows[0]["reason"] = "review-guard-failed"
                        rows[0]["arms"]["base"]["guard_status"] = {
                            "kind": "guard_error", "launched": False, "child_exit": None}
                    real_publish(out, stage, registered, arms, rows, size, seed, artifacts, gate, step)
                    review_receipt.admit(out, population=self.population)

                with patch.object(review_diff, "publish", side_effect=publish):
                    self.gate_review(plan, 2)
                self.assertEqual(self.result()["result"], "refused")
                self.make_guard({"review": (75, QUIET)})

    def test_review_head_launch_requires_usable_base_and_matching_pre_arm_input(self):
        """known-s4-5: a failed head still needs a decodable base envelope and equal pre-arm state."""
        require_gate("gate_steps")
        from measure import receipt, review_diff, review_receipt

        self.make_population()
        self.make_arm("head", exit_code=7)
        plan = self.review_plan()
        step = self.gate_review(plan, 1)
        self.assertEqual((step["status"], step["child_exit"]), ("fail", 1))
        real_publish = review_diff.publish
        for case in ("bad-envelope", "different-pre-arm"):
            with self.subTest(case=case):
                self.next_output(case)

                def publish(out, stage, registered, arms, rows, size, seed, artifacts, gate, step):
                    arm = rows[0]["arms"]
                    if case == "bad-envelope":
                        name = arm["base"]["output"]
                        (stage / name).write_bytes(b"not a findings envelope\n")
                        artifacts[name] = receipt.artifact(stage / name)
                        arm["base"]["output_sha256"] = artifacts[name]["sha256"]
                    else:
                        arm["head"]["input_before_sha256"] = "a" * 64
                    real_publish(out, stage, registered, arms, rows, size, seed, artifacts, gate, step)
                    review_receipt.admit(out, population=self.population)

                with patch.object(review_diff, "publish", side_effect=publish):
                    self.gate_review(plan, 2)
                self.assertEqual(self.result()["result"], "refused")

    def test_persistent_review_executable_change_overrides_child_exit(self):
        """n3-10, known-s4-2: the gate boundary catches persistent review executable drift."""
        require_gate("gate_steps")
        self.make_population()
        action = "p = Path(sys.argv[0]); p.write_bytes(p.read_bytes() + b'# changed' + bytes([10]))"
        binary = self.make_arm("head", after_read=action)
        plan = self.review_plan()
        before = sha256_of(binary)
        self.gate_review(plan, 2)
        self.assertNotEqual(sha256_of(binary), before)
        self.assertEqual(self.result()["result"], "refused")
        self.assertEqual(self.result()["product_steps"], [])

    def test_known_limit_transient_guard_change_during_quiet_version(self):
        """known-limit known-s4-2, known-s4-6: the IC-4 boundary cannot see a restored launch input."""
        require_gate("gate_steps")
        self.make_population()
        path = str(self.quiet_helper)
        for transient in (True, False):
            with self.subTest(transient=transient):
                self.quiet_helper.write_bytes(b"# fixture helper\n")
                self.next_output(f"quiet-drift-{transient}")
                hook = (f"p = Path({path!r}); original = p.read_bytes(); "
                        "p.write_bytes(original + b'# drift' + bytes([10])); "
                        + ("p.write_bytes(original)" if transient else "pass"))
                self.make_guard({"version": (75, QUIET)}, hook=hook)
                self.gate_review(self.review_plan(), 75 if transient else 2)
                self.assertEqual(self.result()["result"], "host_quiet" if transient else "refused")
                self.assertEqual(self.quiet_helper.read_bytes() == b"# fixture helper\n", transient)
