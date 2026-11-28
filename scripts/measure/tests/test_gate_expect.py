"""Coverage minima, metric paths, and scoped oracle claims."""

from __future__ import annotations

from .gate_fixtures import GateWorkbench, require_gate
from .helpers import write_tree


class GateExpectTests(GateWorkbench):
    def test_successful_child_can_fail_a_gate_threshold(self):
        """gate-006: a threshold failure names the step, metric and observation."""
        require_gate("gate_expect")
        listing = {"id": "inventory", "do": "corpora-list", "expect": {"listed": {"min": 2}}}
        plan = self.plan([listing, self.run_step()], base=self.fake)
        self.gate(plan, expected=1)
        step = self.step("inventory")
        self.assertEqual((step["status"], step["child_exit"], step["metrics"]["listed"]), ("fail", 0, 1))
        self.assertIn("listed", str(step["failures"]))
        self.assertIn("inventory", self.report())
        self.assertEqual(self.step("base")["status"], "not_run")
        self.assertEqual(self.fake_log(), [])

    def test_empty_manifest_coverage_and_run_only_execution(self):
        """gate-013: list and verify require positive coverage or a consumed waiver."""
        require_gate("gate_expect")
        run = self.plan([self.run_step(expect={"commands": {"min": 1}})], base=self.fake)
        self.gate(run)
        self.assertEqual((self.result()["claim_scope"], self.step("base")["claim_scope"],
                          self.step("base")["metrics"]["commands"]), ("execution", "execution", 3))
        self.write_manifest({})
        for operation, metric in (("list", "listed"), ("verify", "verified")):
            for waiver in (False, True):
                with self.subTest(operation=operation, waiver=waiver):
                    self.next_output(f"empty-{operation}-{waiver}")
                    plan = self.plan([{"id": operation, "do": f"corpora-{operation}",
                                       "allow_empty": waiver}])
                    self.gate(plan, expected=0 if waiver else 1)
                    step = self.step(operation)
                    self.assertEqual(step["metrics"][metric], 0)
                    self.assertEqual(step["claim_scope"], "control" if waiver else "none")
                    self.assertEqual(self.result()["product_steps"], [])
                    self.assertIn(metric, self.report())

    def test_dirty_corpora_verify_has_one_failed_result(self):
        """n2-1: verify's one dirty-checkout outcome survives as child 3 with zero verified."""
        require_gate("gate_expect")
        plan = self.plan([{"id": "verify", "do": "corpora-verify", "keys": [self.key]},
                          self.run_step()], base=self.fake)
        self.gate(plan)
        self.assertEqual(self.step("verify")["metrics"]["verified"], 1)
        write_tree(self.source, {"a.ts": "export const dirty = true;\n"})
        self.next_output("dirty")
        self.gate(plan, expected=1)
        self.assertEqual((self.result()["result"], self.result()["exit"]), ("fail", 1))
        self.assertEqual((self.step("verify")["status"], self.step("verify")["child_exit"],
                          self.step("verify")["metrics"]["verified"]), ("fail", 3, 0))
        self.assertEqual(self.step("base")["status"], "not_run")
        self.assertEqual(self.fake_log(), [])

    def test_each_stats_table_requires_numeric_leaves_on_each_arm(self):
        """gate-014, n1-15: other tables and textual metadata cannot cover an empty table."""
        require_gate("gate_expect")
        allow = self.work / "allow.txt"
        allow.write_text("count\n")
        for arm in ("base", "head"):
            base = self.product("base", call={"note": "none"} if arm == "base" else None)
            head = self.product("head", call={"note": "none"} if arm == "head" else None)
            for waiver in (False, True):
                with self.subTest(arm=arm, waiver=waiver):
                    self.next_output(f"{arm}-{waiver}")
                    stats = dict(id="compare", do="diff-stats", base="base", head="head",
                                 which="both", allow=allow.name, allow_empty=waiver)
                    plan = self.pair(base=base, head=head, step=stats)
                    self.gate(plan, pin=self.pin(plan), expected=0 if waiver else 1)
                    tables = self.step("compare")["metrics"]["tables"]
                    self.assertEqual(tables["call"][arm + "_leaves"], 0)
                    self.assertEqual(tables["dfg"][arm + "_leaves"], 1)
                    self.assertEqual(self.step("compare")["claim_scope"], "control" if waiver else "none")
                    self.assertEqual(self.result()["product_steps"], [])

    def test_diff_stats_counts_and_child_failure_override_expect(self):
        """gate-019: keep the two counter table counts when the allow-list fails the child."""
        require_gate("gate_expect")
        allow = self.work / "allow.txt"
        allow.write_text("allowed\n")
        step = dict(id="compare", do="diff-stats", base="base", head="head", allow=allow.name,
                    which="both", expect={"deltas": 2, "forbidden": 1, "tables.dfg.forbidden": 0})
        base = self.product("base", dfg={"allowed": 1}, call={"locked": 1})
        head = self.product("head", dfg={"allowed": 2}, call={"locked": 2})
        plan = self.pair(base=base, head=head, step=step)
        self.gate(plan, pin=self.pin(plan), expected=1)
        observed = self.step("compare")
        self.assertEqual((observed["status"], observed["child_exit"]), ("fail", 1))
        self.assertEqual(observed["metrics"], {
            "base_leaves": 2, "head_leaves": 2, "deltas": 2, "forbidden": 1,
            "tables": {"dfg": {"base_leaves": 1, "head_leaves": 1, "deltas": 1, "forbidden": 0},
                       "call": {"base_leaves": 1, "head_leaves": 1, "deltas": 1, "forbidden": 1}},
        })
        allow.write_text("allowed\nlocked\n")
        step["expect"]["forbidden"] = 0
        self.next_output("allowed-control")
        self.gate(plan, pin=self.pin(plan))

    def test_zero_threshold_never_waives_oracle_global_coverage(self):
        """gate-015: empty, undecided, inadmissible and default WRONG-only all fail."""
        require_gate("gate_expect")
        from .gate_oracle_fixtures import oracle_fixture

        cases = (
            ("empty", [], {}, (0, 0, 0)),
            ("undecided", ["UNDECIDED"], {}, (1, 0, 0)),
            ("inadmissible-zero", ["INADMISSIBLE"], {"expect": {"denominators.wrong": 0}}, (1, 0, 1)),
            ("inadmissible-absence", ["INADMISSIBLE"],
             {"absence": [{"property": "slot"}], "allow_empty": True}, (1, 0, 1)),
            ("wrong", ["WRONG"], {}, (1, 1, 0)),
        )
        with oracle_fixture(self) as oracle:
            for label, verdicts, options, counts in cases:
                with self.subTest(label=label):
                    self.next_output(label)
                    plan = oracle.edges([("binding", "other", verdict) for verdict in verdicts], **options)
                    self.gate(plan, pin=self.pin(plan), expected=1)
                    step = self.step("grade")
                    denom = step["metrics"]["denominators"]
                    self.assertEqual(tuple(denom[key] for key in ("graded", "decided", "inadmissible")), counts)
                    self.assertEqual((step["status"], step["child_exit"]), ("fail", 0))
                    self.assertEqual(self.result()["product_steps"], [])

    def test_relation_property_expectation_needs_decided_coverage(self):
        """gate-017, gate-020: dotted literal scope works, empty and undecided scopes fail."""
        require_gate("gate_expect")
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self) as oracle:
            for label, slot, expected in (("absent", [], 1), ("undecided", ["UNDECIDED"], 1),
                                          ("decided", ["CORRECT"], 0)):
                with self.subTest(label=label):
                    self.next_output(label)
                    plan = oracle.edges([("binding", "other", "CORRECT")]
                                        + [("binding", "slot", value) for value in slot],
                                        expect={"tuples.binding.slot.WRONG": 0})
                    self.gate(plan, pin=self.pin(plan), expected=expected)
                    bucket = self.step("grade")["metrics"]["tuples"]["binding"]["slot"]
                    self.assertEqual((bucket["WRONG"], bucket["CORRECT"]), (0, int(not expected)))
                    self.assertEqual(bucket["UNDECIDED"], int(label == "undecided"))
            self.next_output("escaped")
            plan = oracle.edges([("lane.a", "slot.v1", "CORRECT")],
                                expect={r"tuples.lane\.a.slot\.v1.WRONG": 0,
                                        r"tuples.lane\.a.slot\.v1.CORRECT": 1})
            self.gate(plan, pin=self.pin(plan))
            self.next_output("unescaped")
            plan["steps"][-1]["expect"] = {"tuples.lane.a.slot.v1.WRONG": 0}
            self.gate(plan, pin=self.pin(plan), expected=1)

    def test_confidence_kind_expectation_needs_its_own_decided_rows(self):
        """gate-020: an unrelated decided population cannot satisfy a scoped zero."""
        require_gate("gate_expect")
        from .gate_oracle_fixtures import oracle_fixture

        path = "denominators.populations.name_only.kinds.argument_follow.WRONG"
        with oracle_fixture(self) as oracle:
            for label, extra, expected in (("none", [], 1), ("undecided", ["UNDECIDED"], 1),
                                           ("correct", ["CORRECT"], 0)):
                with self.subTest(label=label):
                    self.next_output(label)
                    plan = oracle.calls([("exact", "local_def", "CORRECT")] +
                                        [("name_only", "argument_follow", verdict) for verdict in extra],
                                        expect={path: 0})
                    self.gate(plan, pin=self.pin(plan), expected=expected)
                    measured = self.step("grade")["metrics"]["denominators"]["populations"]
                    bucket = measured["name_only"]["kinds"]["argument_follow"]
                    self.assertEqual((bucket["WRONG"], bucket["graded"]), (0, len(extra)))

    def test_absence_counts_emitted_undecided_and_cannot_supply_credit(self):
        """gate-015, gate-020: absence waives only global positive minima when graded is zero."""
        require_gate("gate_expect")
        from .gate_oracle_fixtures import oracle_fixture

        scope = {"relation": "binding", "property": "slot"}
        with oracle_fixture(self) as oracle:
            for label, rows, expected in (
                ("empty", [], 0),
                ("unrelated", [("binding", "other", "CORRECT")], 0),
                ("undecided", [("binding", "slot", "UNDECIDED")], 1),
                ("correct", [("binding", "slot", "CORRECT")], 1),
            ):
                with self.subTest(label=label):
                    self.next_output(label)
                    plan = oracle.edges(rows, absence=[scope], allow_empty=True)
                    self.gate(plan, pin=self.pin(plan), expected=expected)
                    self.assertEqual(self.step("grade")["metrics"]["coverage_checks"], [
                        dict(scope=scope, claim="absence", graded=int(bool(expected)),
                             decided=int(label == "correct"), wrong=0, passed=not expected),
                    ])
                    self.assertEqual(self.step("grade")["claim_scope"], "control" if not expected else "none")
                    self.assertEqual(self.result()["product_steps"], [])

    def test_integer_rate_boundary_and_decided_minimum_in_the_declared_scope(self):
        """gate-023: 1/20 passes, 1/19 and unrelated coverage fail the exact rational check."""
        require_gate("gate_expect")
        from .gate_oracle_fixtures import oracle_fixture

        scope = dict(relation="argfollow", confidence="name_only", kind="argument_follow")
        with oracle_fixture(self) as oracle:
            cases = [
                ("boundary", 1, 19, 0, 0, 1, [1, 20], 0),
                ("above-bound", 1, 18, 0, 0, 1, [1, 20], 1),
                ("empty-scope", 0, 0, 0, 200, 1, [1, 20], 1),
                ("all-undecided-scope", 0, 0, 21, 1, 1, [1, 20], 1),
                ("minimum", 1, 19, 1, 0, 21, [1, 20], 1),
                ("zero-bound", 1, 19, 0, 0, 1, [0, 1], 1),
            ]
            for label, wrong, correct, undecided, unrelated, minimum, bound, expected in cases:
                with self.subTest(label=label):
                    self.next_output(label)
                    rows = []
                    for verdict, n in (("WRONG", wrong), ("CORRECT", correct), ("UNDECIDED", undecided)):
                        rows += [("name_only", "argument_follow", verdict)] * n
                    rows += [("exact", "local_def", "CORRECT")] * unrelated
                    plan = oracle.calls(rows, quality=[dict(scope=scope, max_wrong=bound,
                                                            min_decided=minimum)])
                    self.gate(plan, pin=self.pin(plan), expected=expected)
                    checks = self.step("grade")["metrics"]["coverage_checks"]
                    self.assertIn(dict(scope=scope, claim="quality", graded=wrong + correct + undecided,
                                       decided=wrong + correct, wrong=wrong, passed=not expected), checks)
