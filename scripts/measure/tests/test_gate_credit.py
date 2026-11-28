"""Scope and product-step credit with positive and negative controls."""

from __future__ import annotations

from copy import deepcopy
import shutil

from .comparator_fixtures import edge
from .gate_fixtures import GateWorkbench, require_gate


class GateCreditTests(GateWorkbench):
    def assert_credit(self, scope, product_steps):
        self.assertEqual((self.result()["claim_scope"], self.result()["product_steps"]),
                         (scope, product_steps))

    def test_same_id_and_aliases_are_identity_controls_only(self):
        """gate-016: self-reference and aliases of an intact receipt cannot be a differential."""
        require_gate("gate_credit")
        single = self.plan([self.run_step(), self.edge_step(head="base")], base=self.fake)
        self.gate(single, pin=self.pin(single), expected=2)
        self.assert_credit("none", [])
        self.next_output("declared")
        single["steps"][-1]["identity_control"] = True
        self.gate(single, pin=self.pin(single))
        self.assert_credit("control", [])

        alias = self.work / "same-run-link"
        target = self.prior_run(single, "base", self.work / "prior", gate_pin=self.pin(single),
                                prediction=self.prediction)
        alias.symlink_to(target, target_is_directory=True)
        pair = self.pair(head=self.fake, head_options={"binary": "base"})
        pair = self.reuse(self.reuse(pair, "base", target), "head", alias)
        pair["steps"][-1]["allow_empty"] = True
        self.next_output("same-prior")
        self.gate(pair, pin=self.pin(pair), expected=2)
        self.assert_credit("none", [])
        self.assert_no_launch()

    def test_fresh_same_digest_and_trace_only_are_identity_controls(self):
        """gate-026, f3: distinct paths, step IDs and trace settings cannot grant product scope."""
        require_gate("gate_credit")
        copy = self.work / "copy-of-base"
        shutil.copy2(self.fake, copy)
        for mode in ("edges", "stats"):
            for trace in (False, True):
                step = (self.edge_step() if mode == "edges" else
                        {"id": "compare", "do": "diff-stats", "base": "base", "head": "head",
                         "expect": {"deltas": 0}})
                plan = self.pair(head=copy, step=step,
                                 head_options={"trace_env": ["PRISM_FIXTURE_TRACE=1"]} if trace else None)
                step["allow_empty"] = True
                for declared in (False, True):
                    with self.subTest(mode=mode, trace=trace, declared=declared):
                        self.next_output(f"{mode}-{trace}-{declared}")
                        if declared:
                            step["identity_control"] = True
                        self.gate(plan, pin=self.pin(plan), expected=0 if declared else 2)
                        self.assert_credit("control" if declared else "none", [])
                        if declared:
                            self.assertEqual(self.step("compare")["claim_scope"], "control")
        plan = self.pair(head=copy)
        replacement = self.product("head")
        self.next_output("genuinely-distinct")
        overrides = {"head": replacement}
        self.gate(plan, pin=self.pin(plan, overrides=overrides), overrides=overrides)
        self.assert_credit("product", ["compare"])
        self.assertEqual(self.step("compare")["metrics"]["changed_keys"], 0)

    def test_trace_switch_that_changes_edges_never_earns_credit(self):
        """f3: a trace-only switch does not distinguish arms even if it changes the edge stream."""
        require_gate("gate_credit")
        replacement = edge(sb=200)
        from .gate_fixtures import jsonl

        action = "if os.getenv('PRISM_FIXTURE_TRACE'): sys.stdout.buffer.write(" + repr(jsonl([replacement])) + ")"
        binary = self.product("base", actions={"edges": action})
        plan = self.pair(base=binary, head=binary, head_options={"trace_env": ["PRISM_FIXTURE_TRACE=1"]})
        self.gate(plan, pin=self.pin(plan), expected=2)
        self.assert_credit("none", [])
        self.next_output("declared")
        plan["steps"][-1]["identity_control"] = True
        self.gate(plan, pin=self.pin(plan))
        self.assertEqual(self.step("compare")["metrics"]["changed_keys"], 1)
        self.assert_credit("control", [])

    def test_empty_edge_waiver_never_enters_product_steps(self):
        """gate-014, gate-025: empty arms fail or consume a waiver; populated equal arms can earn credit."""
        require_gate("gate_credit")
        plan = self.pair(base=self.product("base", edges=[]), head=self.product("head", edges=[]))
        self.gate(plan, pin=self.pin(plan), expected=1)
        self.assert_credit("none", [])
        self.assertEqual(self.step("compare")["metrics"]["base_edges"], 0)
        plan["steps"][-1]["allow_empty"] = True
        self.next_output("control")
        self.gate(plan, pin=self.pin(plan))
        self.assert_credit("control", [])
        self.assertEqual(self.step("compare")["claim_scope"], "control")
        self.assertIn("compare", self.report())
        mixed = deepcopy(plan)
        mixed["binaries"].update({name: {"path": str(self.product(name)),
                                         "expect_version": mixed["binaries"]["base"]["expect_version"]}
                                  for name in ("goodbase", "goodhead")})
        mixed["steps"].extend([self.run_step("goodbase"), self.run_step("goodhead"),
                               self.edge_step(id="populated", base="goodbase", head="goodhead",
                                              allow_empty=True)])
        self.next_output("mixed")
        self.gate(mixed, pin=self.pin(mixed))
        self.assert_credit("product", ["populated"])
        self.assertEqual(self.step("populated")["metrics"]["changed_keys"], 0)
        self.assertEqual(self.step("populated")["metrics"]["head_edges"], 1)

        self.next_output("failed-child")
        self.classifier.write_text('CLASSES = []\n')
        self.gate(plan, pin=self.pin(plan), expected=2)
        self.assert_credit("none", [])

    def test_descriptive_steps_have_execution_scope(self):
        """n2-12, n4-7: a changed counter with no allow-list is descriptive and has zero forbidden."""
        require_gate("gate_credit")
        counter = {"id": "compare", "do": "diff-stats", "base": "base", "head": "head"}
        plan = self.pair(head=self.product("head", dfg={"count": 2}), step=counter)
        self.gate(plan, pin=self.pin(plan))
        self.assertEqual((self.step("compare")["metrics"]["deltas"],
                          self.step("compare")["metrics"]["forbidden"]), (1, 0))
        self.assert_credit("execution", [])
        self.next_output("edge-no-claim")
        edges = self.edge_step()
        edges.pop("expect")
        plan = self.pair(step=edges)
        self.gate(plan, pin=self.pin(plan))
        self.assert_credit("execution", [])

    def test_metadata_only_is_not_counter_parity_evidence(self):
        """n1-15: schema metadata alone earns no parity credit on either table."""
        require_gate("gate_credit")
        stats = dict(id="compare", do="diff-stats", base="base", head="head",
                     expect={"deltas": 0})
        schema = {"schema_version": 1}
        for name, left, right, expected in (
            ("metadata", schema, schema, 1),
            ("one-sided", {"schema_version": 1, "count": 7}, schema, 1),
            ("counter", {"schema_version": 1, "count": 7}, {"schema_version": 1, "count": 7}, 0),
        ):
            with self.subTest(name=name):
                self.next_output(name)
                # Admit one delta per table in the one-sided case so the
                # coverage minimum, rather than the threshold, distinguishes it.
                stats["expect"]["deltas"] = 2 if name == "one-sided" else 0
                base = self.product("base", dfg=left, call=left)
                head = self.product("head", dfg=right, call=right)
                plan = self.pair(base=base, head=head, step=stats)
                self.gate(plan, pin=self.pin(plan), expected=expected)
                self.assert_credit("none" if expected else "product", [] if expected else ["compare"])

    def test_oracle_explicit_wrong_total_is_a_negative_control(self):
        """gate-015, n1-16: a matching positive WRONG expectation earns only control."""
        require_gate("gate_credit")
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self) as oracle:
            plan = oracle.edges([("binding", "other", "WRONG")], expect={"denominators.wrong": 1})
            self.gate(plan, pin=self.pin(plan))
            self.assertEqual(self.step("grade")["metrics"]["denominators"]["wrong"], 1)
            self.assert_credit("control", [])

    def test_oracle_quality_requires_nonvacuous_bound_and_correct_in_scope(self):
        """n3-11, n2-12: a vacuous bound and absent claims cannot earn product credit."""
        require_gate("gate_credit")
        from .gate_oracle_fixtures import oracle_fixture

        with oracle_fixture(self) as oracle:
            for label, verdict, bound, earns in (("wrong-vacuous", "WRONG", [1, 1], False),
                                                   ("correct-vacuous", "CORRECT", [1, 1], False),
                                                   ("correct-bounded", "CORRECT", [0, 1], True),
                                                   ("no-claim", "CORRECT", None, False)):
                with self.subTest(label=label):
                    self.next_output(label)
                    options = {} if bound is None else {
                        "quality": [{"scope": {}, "max_wrong": bound, "min_decided": 1}]}
                    plan = oracle.edges([("binding", "other", verdict)], **options)
                    self.gate(plan, pin=self.pin(plan))
                    self.assertEqual(self.result()["product_steps"], ["grade"] if earns else [])
                    if earns or bound is None:
                        self.assertEqual(self.step("grade")["claim_scope"],
                                         "product" if earns else "execution")
                    else:
                        self.assertIn(self.step("grade")["claim_scope"], ("execution", "control"))
