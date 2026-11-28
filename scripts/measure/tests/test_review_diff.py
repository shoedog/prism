"""S4 review comparisons, accounting and receipt custody through the CLI."""

from __future__ import annotations

from contextlib import ExitStack
import json
import os
from pathlib import Path
import random
import subprocess
import sys
import unittest
from unittest.mock import patch

from .helpers import DEFAULT_VERSION, executable, fixture_digest, sha256_of
from .test_matrix_rows import CASES
from .test_population import (
    AFTER, EMPTY, METRICS, QUIET, STANDARD_SPEC, ReviewFixture, envelope_bytes,
)

ROW_FIELDS = {"ordinal", "repo", "commit", "status", "reason", "arms", *METRICS,
              "rule_transitions", "anchors"}
ARM_FIELDS = {"status", "exit", "guard_status", "output", "output_sha256", "stderr", "stderr_sha256",
              "input_before_sha256", "input_after_sha256"}
SAMPLE_FIELDS = {"ordinal", "repo", "commit", *METRICS}
TOTAL_FIELDS = {"schema", "population_sha256", "scope", "population", "selected", "ok",
                "failed_ordinals", "refused_ordinals", "excluded_ordinals", *METRICS,
                "sample_size", "seed", "arms"}
RECEIPT_FIELDS = {"schema", "population_sha256", "arms", "gate_sha256", "gate_step_id", "artifacts"}
SHARED_ARGS = ["--algorithm", "taint", "--resolution", "scoped"]


def finding(**changes):
    return {"file": "src/a.ts", "line": 3, "algorithm": "taint", "category": "taint_sink",
            "description": "fixture finding", **changes}


def single(*findings):
    return {**EMPTY, "findings": list(findings)}


class ReviewChecks(ReviewFixture):
    def matrix_arms(self, ident, inputs=None):
        case = CASES / ident
        for role in ("base", "head"):
            filename = inputs[role] if inputs else role + ".json"
            path = case / filename
            self.make_arm(role, stdout=path.read_bytes())

    def pair(self, base, head):
        self.make_arm("base", single(*base))
        self.make_arm("head", single(*head))

    def assert_counts(self, record, presence=0, payload=0, ambiguous=0):
        self.assertEqual({key: record[key] for key in METRICS},
                         dict(zip(METRICS, (presence, payload, ambiguous))))

    def assert_streams(self, row, role, *, stdout, stderr, name=None):
        arm = row["arms"][name or role]
        for field, suffix, raw in (("output", "stdout", stdout), ("stderr", "stderr", stderr)):
            relative = f"arms/{row['ordinal']}/{role}.{suffix}"
            self.assertEqual(arm[field], relative)
            path = self.out / relative
            self.assertEqual(path.read_bytes(), raw)
            self.assertEqual(arm[field + "_sha256"], sha256_of(path))
            self.assertEqual(self.read_json("review-receipt.json")["artifacts"][relative],
                             {"bytes": len(raw), "sha256": sha256_of(path)})

    def check_drift_between_arms(self):
        from measure import inputs, review_diff

        self.make_population()
        self.keep_dirty_source()
        before = self.source_state()
        real_bind = inputs.bind_tree
        mutated = []

        def change_after_base_postcheck(root, *args, **kwargs):
            binding = real_bind(root, *args, **kwargs)
            attempts = self.attempts()
            if (not mutated and len(attempts) == 1 and attempts[0]["role"] == "base"
                    and Path(root).resolve() == Path(attempts[0]["root"]).resolve()):
                (Path(root) / "src/a.ts").write_bytes(b"drift between arms\n")
                mutated.append(fixture_digest(Path(root)))
            return binding

        # Inject at the existing digest seam, after returning the measured base bytes.
        with ExitStack() as stack:
            for module in (inputs, review_diff):
                if getattr(module, "bind_tree", None) is real_bind:
                    stack.enter_context(patch.object(module, "bind_tree", change_after_base_postcheck))
            self.review(expected=1)
        changed_digest, = mutated
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("refused", "review-input-changed"))
        base, head = (row["arms"][role] for role in ("base", "head"))
        self.assertEqual(base["input_before_sha256"], self.pinned_digest)
        self.assertEqual(base["input_after_sha256"], self.pinned_digest)
        self.assertEqual(base["status"], "ok")
        self.assertEqual(head["input_before_sha256"], changed_digest)
        self.assert_unlaunched(head)
        self.assert_null_metrics(row)
        self.assertEqual([attempt["role"] for attempt in self.attempts()], ["base"])
        self.assertEqual(self.source_state(), before)


class ReviewDiffTests(ReviewChecks):
    def test_identical_payloads_cancel_with_multiplicity_and_order(self):
        self.make_population()
        self.matrix_arms("review-005")
        self.review()
        row, = self.read_rows()
        self.assert_counts(row)
        self.assertEqual(row["anchors"], [])

        self.out = self.work / "one-copy-removed"
        head = json.loads((CASES / "review-005/head.json").read_bytes())
        del head["findings"][0]
        self.make_arm("head", head)
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, presence=1)
        residual, = row["anchors"]
        self.assertEqual((residual["added"], residual["removed"]), (0, 1))

    def test_lost_related_line_is_a_payload_change(self):
        self.make_population()
        self.matrix_arms("review-001")
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=1)
        anchor, = row["anchors"]
        pair, = anchor["pairs"]
        self.assertEqual((pair["count"], pair["fields"], pair["text_only"]),
                         (1, ["related_lines"], False))
        self.assertEqual(pair["from"]["related_lines"], [1, 2])
        self.assertEqual(pair["to"]["related_lines"], [2])

    def test_added_rule_id_is_a_rule_transition_not_a_presence_change(self):
        self.make_population()
        self.matrix_arms("review-002")
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=1)
        self.assertEqual(row["rule_transitions"], [
            {"from": ["sql-query"], "to": ["input-validation", "sql-query"],
             "count": 1, "from_state": "list", "to_state": "list"},
        ])

    def test_findings_read_once_per_arm(self):
        self.make_population()
        kept = finding()
        lost = finding(line=4)
        inactive = finding(file="unused.ts", line=99)
        base = {"algorithms_run": ["Taint"], "all_findings": [kept, lost], "findings": [inactive]}
        head = {**single(kept), "all_findings": [lost, inactive]}
        self.make_arm("base", base)
        self.make_arm("head", head)
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, presence=1)
        anchor, = row["anchors"]
        self.assertEqual(anchor["base_bag"], [lost])
        self.assertEqual(anchor["head_bag"], [])

    def test_selected_array_must_exist_and_be_usable(self):
        self.make_population()
        for index, envelope in enumerate((
                {"algorithms_run": ["Taint"], "findings": []},
                {"results": [{"algorithm": "Taint", "findings": []}], "all_findings": {}},
                {"algorithm": "Taint", "findings": None, "all_findings": []})):
            with self.subTest(envelope=envelope):
                self.out = self.work / f"bad-envelope-{index}"
                self.make_arm("head", envelope)
                self.review(expected=1)
                row, = self.read_rows()
                self.assertEqual(row["status"], "failed")
                self.assertEqual(row["arms"]["head"]["exit"], 0)
                self.assert_null_metrics(row)

    def test_rule_metadata_and_ordered_nested_array_are_payload(self):
        self.make_population()
        left = finding(rules=[{"id": "A", "cwe": "CWE-1"}],
                       diagrams=[{"kind": "flow", "text": ["a", "b"]}])
        right = finding(rules=[{"cwe": "CWE-2", "id": "A"}],
                        diagrams=[{"text": ["b", "a"], "kind": "flow"}])
        self.pair([left], [right])
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=1)
        self.assertEqual(row["rule_transitions"], [])
        anchor, = row["anchors"]
        pair, = anchor["pairs"]
        self.assertEqual(pair["fields"], ["diagrams", "rules"])
        self.assertEqual(pair["from"], left)
        self.assertEqual(pair["to"], right)

    def test_rule_states_distinguish_missing_null_and_empty(self):
        self.make_population()
        self.pair([finding(), finding(line=4, rules=None)],
                  [finding(rules=None), finding(line=4, rules=[])])
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=2)
        self.assertCountEqual(row["rule_transitions"], [
            {"from": None, "to": None, "count": 1, "from_state": "missing", "to_state": "null"},
            {"from": None, "to": [], "count": 1, "from_state": "null", "to_state": "list"},
        ])

    def test_ambiguous_surplus_preserves_bags_without_transitions(self):
        self.make_population()
        left = [finding(rules=["shared"]), finding(rules=["A"]), finding(rules=["B"])]
        right = [finding(rules=["C"]), finding(rules=["shared"])]
        self.pair(left, right)
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, presence=1, payload=1, ambiguous=1)
        self.assertEqual(row["rule_transitions"], [])
        anchor, = row["anchors"]
        self.assertEqual((anchor["removed"], anchor["added"], anchor["ambiguous"]), (1, 0, True))
        self.assertCountEqual(anchor["base_bag"], left[1:])
        self.assertEqual(anchor["head_bag"], right[:1])
        self.assertEqual(anchor["pairs"], [])

    def test_homogeneous_larger_bag_supports_a_pair_and_a_surplus(self):
        self.make_population()
        shared, old, new = finding(), finding(severity="warning"), finding(severity="info")
        self.pair([old, shared], [new, shared, new])
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, presence=1, payload=1)
        anchor, = row["anchors"]
        self.assertEqual((anchor["removed"], anchor["added"], anchor["ambiguous"]), (0, 1, False))
        self.assertEqual(anchor["pairs"], [
            {"from": old, "to": new, "count": 1, "fields": ["severity"], "text_only": False},
        ])

    def test_single_text_field_is_text_only(self):
        self.make_population()
        self.pair([finding(message="old"), finding(line=4)],
                  [finding(message="new"), finding(line=4, description="new")])
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=2)
        pairs = [pair for anchor in row["anchors"] for pair in anchor["pairs"]]
        self.assertCountEqual([pair["fields"] for pair in pairs], [["description"], ["message"]])
        self.assertTrue(all(pair["text_only"] is True for pair in pairs))

    def test_failed_arm_fails_its_row_and_totals_name_it(self):
        self.make_population(ordinals=(2,))
        self.make_arm("head", single(finding()), exit_code=7)
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["ordinal"], row["status"]), (2, "failed"))
        self.assertTrue(row["reason"])
        self.assert_null_metrics(row)
        self.assertEqual(row["arms"]["head"]["exit"], 7)
        self.assertEqual(row["arms"]["head"]["guard_status"],
                         {"kind": "child", "launched": True, "child_exit": 7})
        self.assertEqual(self.read_json("totals.json")["failed_ordinals"], [2])
        self.assertEqual(self.read_rows("sample.jsonl"), [])

    def test_worktree_checked_before_and_after_both_arms(self):
        self.make_population()
        self.review()
        row, = self.read_rows()
        for arm in row["arms"].values():
            self.assertEqual(arm["status"], "ok")
            for field in ("input_before_sha256", "input_after_sha256"):
                self.assertEqual(arm[field], self.pinned_digest)

    def test_post_arm_drift_fails_row_and_keeps_launched_streams(self):
        """review-019: valid output and zero exit cannot override a changed scratch tree."""
        self.make_population()
        self.keep_dirty_source()
        before = self.source_state()
        self.make_arm("base", after_read="(root / 'src/a.ts').write_bytes(b'drift\\n')")
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-input-changed"))
        self.assertEqual(row["arms"]["base"]["input_before_sha256"], self.pinned_digest)
        after = self.work / "drifted"
        after.mkdir()
        (after / "src").mkdir()
        (after / "src/a.ts").write_bytes(b"drift\n")
        (after / "views.py").write_bytes((self.source / "views.py").read_bytes())
        self.assertEqual(row["arms"]["base"]["input_after_sha256"], fixture_digest(after))
        self.assert_unlaunched(row["arms"]["head"])
        self.assert_streams(row, "base", stdout=envelope_bytes(EMPTY), stderr=b"base diagnostic\n")
        self.assertEqual([item["role"] for item in self.attempts()], ["base"])
        self.assert_null_metrics(row)
        self.assertEqual(self.source_state(), before)

    def test_head_drift_after_launch_fails_the_row(self):
        self.make_population()
        self.make_arm("head", after_read="(root / 'src/a.ts').write_bytes(b'head drift\\n')")
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-input-changed"))
        base, head = (row["arms"][role] for role in ("base", "head"))
        self.assertEqual(base["input_after_sha256"], self.pinned_digest)
        self.assertEqual(head["input_before_sha256"], self.pinned_digest)
        self.assertRegex(head["input_after_sha256"], r"^[0-9a-f]{64}$")
        self.assertNotEqual(head["input_after_sha256"], self.pinned_digest)
        self.assert_streams(row, "head", stdout=envelope_bytes(EMPTY), stderr=b"head diagnostic\n")
        self.assert_null_metrics(row)

    def test_spec_file_drift_cannot_earn_comparison_coverage(self):
        """open-n3-8: both arms must measure the registered spec, even outside the worktree."""
        self.make_population()
        edited_spec = json.dumps({"files": [{"file_path": "src/a.ts", "modify_type": "Modified",
                                             "diff_lines": [1, 2]}]})
        self.make_arm("head", before_read=f"""
if spec_path is not None:
    try:
        spec_path.write_bytes({edited_spec.encode("utf-8")!r})
    except PermissionError:
        pass
""")
        result = self.review(expected=None)
        row, = self.read_rows()
        attempts = self.attempts()
        self.assertEqual(attempts[0]["spec"], json.dumps(STANDARD_SPEC))
        if result.returncode == 0:
            self.assert_measured_spec(STANDARD_SPEC)
            self.assertEqual(row["status"], "ok")
        else:
            self.assertEqual(result.returncode, 1)
            self.assertIn(row["status"], {"failed", "refused"})
            self.assert_null_metrics(row)

    def test_replacing_head_binary_cannot_borrow_original_identity(self):
        """open-n1-13: replacing head between ordinals cannot misattribute its streams."""
        self.make_population(ordinals=(1, 2))
        original = self.binaries["head"]
        original_hash = sha256_of(original)
        replacement = self.make_arm("head", {**EMPTY, "fixture_build": "replacement"})
        replacement_bytes = replacement.read_bytes()
        self.make_arm("head")
        self.assertEqual(sha256_of(original), original_hash)
        self.make_arm("base", after_read=f"""
with open({str(self.log)!r}, encoding="utf-8") as prior:
    if any(item["role"] == "base" and item["root"] is not None
           for item in map(json.loads, prior)):
        Path({str(original)!r}).write_bytes({replacement_bytes!r})
""")
        result = self.review(expected=None)
        first, second = self.read_rows()
        self.assertEqual((first["ordinal"], first["status"]), (1, "ok"))
        self.assertNotEqual(sha256_of(original), original_hash)
        self.assertEqual(self.read_json("review-receipt.json")["arms"]["head"]["binary"]["sha256"],
                         original_hash)
        self.assert_streams(first, "head", stdout=envelope_bytes(EMPTY), stderr=b"head diagnostic\n")
        if result.returncode == 0:
            self.assertEqual(second["status"], "ok")
            self.assert_streams(second, "head", stdout=envelope_bytes(EMPTY),
                                stderr=b"head diagnostic\n")
        else:
            self.assertEqual(result.returncode, 1)
            self.assertIn(second["status"], {"refused", "failed"})
            self.assert_null_metrics(second)
            arm = second["arms"]["head"]
            if arm["guard_status"] and arm["guard_status"]["launched"]:
                self.assert_streams(second, "head", stdout=envelope_bytes(EMPTY),
                                    stderr=b"head diagnostic\n")
            else:
                self.assert_unlaunched(arm)

    def test_review_receipt_round_trip_binds_positional_arms_and_raw_streams(self):
        """review-017/018: failed and unlaunched arms have distinct evidence."""
        from measure import review_diff

        self.make_population()
        left = self.make_arm("base", exit_code=7, stdout=b"", stderr=b"partial\x00diagnostic\xff")
        right = self.binaries["head"]
        self.review(arms=[f"z-control={left}::--flag::two words",
                          f"a-candidate={right}::--other=value=x"], expected=1)
        row, = self.read_rows()
        self.assertEqual(row["status"], "failed")
        self.assertEqual(set(row["arms"]), {"z-control", "a-candidate"})
        self.assert_unlaunched(row["arms"]["a-candidate"])
        self.assert_streams(row, "base", name="z-control", stdout=b"",
                            stderr=b"partial\x00diagnostic\xff")
        receipt = self.read_json("review-receipt.json")
        self.assertEqual(set(receipt), RECEIPT_FIELDS)
        self.assertEqual(receipt["schema"], "prism.measure.review-receipt/1")
        self.assertEqual(receipt["population_sha256"], sha256_of(self.population))
        self.assertIsNone(receipt["gate_sha256"])
        self.assertIsNone(receipt["gate_step_id"])
        self.assertEqual(receipt["arms"], self.read_json("totals.json")["arms"])
        self.assertEqual(set(receipt["arms"]), {"base", "head"})
        launched, = self.attempts()
        self.assertEqual(launched["role"], "base")
        for role, name, binary, args in (
                ("base", "z-control", left, ["--flag", "two words"]),
                ("head", "a-candidate", right, ["--other=value=x"])):
            arm = receipt["arms"][role]
            self.assertEqual(set(arm), {"name", "binary", "shared_args", "arm_args", "commands"})
            self.assertEqual((arm["name"], arm["shared_args"], arm["arm_args"]),
                             (name, SHARED_ARGS, args))
            self.assertEqual(arm["binary"]["path"], str(binary.resolve()))
            self.assertEqual(arm["binary"]["sha256"], sha256_of(binary))
            self.assertEqual(set(arm["binary"]), {"path", "sha256", "version"})
            if role == "base":
                self.assertEqual(arm["binary"]["version"], f"{DEFAULT_VERSION} fixture base")
                self.assertEqual(arm["commands"], [
                    {"ordinal": 1, "argv": [launched["binary"], *launched["argv"]]},
                ])
            else:
                self.assertIn(arm["binary"]["version"], (None, f"{DEFAULT_VERSION} fixture head"))
                self.assertEqual(arm["commands"], [])

        self.assertEqual(set(receipt["artifacts"]), {
            "per-diff.jsonl", "totals.json", "sample.jsonl", "arms/1/base.stdout", "arms/1/base.stderr",
        })
        for name, artifact in receipt["artifacts"].items():
            path = self.out / name
            self.assertEqual(artifact, {"bytes": path.stat().st_size, "sha256": sha256_of(path)})

        # S4 replay seam mirrors run receipt admission. Failed measurements still round-trip.
        review_diff.admit(self.out, population=self.population)
        path = self.out / "arms/1/base.stderr"
        original = path.read_bytes()
        path.write_bytes(b"changed" + original[7:])
        with self.assertRaises((OSError, ValueError)):
            review_diff.admit(self.out, population=self.population)
        path.write_bytes(original)
        review_diff.admit(self.out, population=self.population)
        receipt["arms"]["base"], receipt["arms"]["head"] = receipt["arms"]["head"], receipt["arms"]["base"]
        totals = self.read_json("totals.json")
        totals["arms"] = receipt["arms"]
        path = self.out / "totals.json"
        path.write_bytes(json.dumps(totals).encode("utf-8"))
        receipt["artifacts"]["totals.json"] = {"bytes": path.stat().st_size, "sha256": sha256_of(path)}
        (self.out / "review-receipt.json").write_bytes(json.dumps(receipt).encode("utf-8"))
        with self.assertRaises((OSError, ValueError)):
            review_diff.admit(self.out, population=self.population)

    def test_only_records_success_failure_refusal_and_exclusion(self):
        self.make_population(ordinals=(4, 3, 1, 2))
        next(row for row in self.registration["diffs"] if row["ordinal"] == 3)["spec_sha256"] = "0" * 64
        self.write_population()
        self.make_arm("head", after_read=f"""
with open({str(self.log)!r}, encoding="utf-8") as prior:
    if any(item["role"] == "head" and item["root"] is not None
           for item in map(json.loads, prior)):
        output, code = b"partial", 7
""")
        self.review("--only", "1,2,3", expected=1)
        rows = self.read_rows()
        self.assertEqual([(row["ordinal"], row["status"]) for row in rows],
                         [(1, "ok"), (2, "failed"), (3, "refused"), (4, "excluded")])
        totals = self.read_json("totals.json")
        self.assertEqual({key: totals[key] for key in (
            "scope", "population", "selected", "ok", "failed_ordinals", "refused_ordinals",
            "excluded_ordinals", "sample_size")}, {
                "scope": "subset", "population": 4, "selected": 3, "ok": 1,
                "failed_ordinals": [2], "refused_ordinals": [3], "excluded_ordinals": [4],
                "sample_size": 1,
            })
        for row in rows[1:]:
            self.assertTrue(row["reason"])
            self.assert_null_metrics(row)
        for row in rows[2:]:
            for arm in row["arms"].values():
                self.assert_unlaunched(arm)
        self.assert_counts(totals)
        self.assertEqual([row["ordinal"] for row in self.read_rows("sample.jsonl")], [1])

    def test_gitlink_in_pinned_tree_refuses_the_ordinal_before_any_arm(self):
        """Known limit (PROVISIONAL, S1 rule open-n2-6): a submodule entry anywhere in the pinned tree
        refuses the ordinal. The shipped population has 20 such ordinals (apache/superset)."""
        self.make_population(ordinals=(1, 2))
        row_two = next(row for row in self.registration["diffs"] if row["ordinal"] == 2)
        # Control: ordinal 1 keeps the plain tree; ordinal 2 names a commit whose tree holds a gitlink.
        self.git("update-index", "--add", "--cacheinfo", f"160000,{self.commit},vendor/dep")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                 "-c", "commit.gpgsign=false", "commit", "-q", "--amend", "--no-edit")
        row_two["commit"] = self.git("rev-parse", "HEAD")
        row_two["tree"] = self.git("rev-parse", "HEAD^{tree}")
        self.write_population()
        self.review(expected=1)
        rows = {row["ordinal"]: row for row in self.read_rows()}
        self.assertEqual((rows[1]["status"], rows[2]["status"]), ("ok", "refused"))
        self.assertIn("gitlink", rows[2]["reason"])
        self.assert_null_metrics(rows[2])
        self.assertEqual({a["root"] is not None for a in self.attempts()}, {True})
        self.assertEqual(len(self.attempts()), 2)  # both arms of ordinal 1 only

    def test_sample_is_deterministic_for_a_seed(self):
        self.make_population(ordinals=(5, 3, 1, 4, 2))
        for run, seed in enumerate((11, 11, None)):
            self.out = self.work / f"draw-{run}"
            self.review("--sample", "2", *(() if seed is None else ("--seed", str(seed))))
            effective = 20261101 if seed is None else seed
            expected = sorted(random.Random(effective).sample(list(range(1, 6)), 2))
            self.assertEqual([row["ordinal"] for row in self.read_rows("sample.jsonl")], expected)
            totals = self.read_json("totals.json")
            self.assertEqual((totals["seed"], totals["sample_size"]), (effective, 2))

    def test_output_shapes_and_population_scope(self):
        self.make_population()
        self.pair([], [finding()])
        self.review()
        row, = self.read_rows()
        self.assertEqual(set(row), ROW_FIELDS)
        self.assertEqual((row["ordinal"], row["repo"], row["commit"], row["status"]),
                         (1, self.source.name, self.commit, "ok"))
        for arm in row["arms"].values():
            self.assertEqual(set(arm), ARM_FIELDS)
        anchor, = row["anchors"]
        self.assertEqual(set(anchor), {"anchor", "base_bag", "head_bag", "added", "removed",
                                       "payload_changes", "ambiguous", "pairs"})
        self.assert_counts(row, presence=1)
        totals = self.read_json("totals.json")
        self.assertEqual(set(totals), TOTAL_FIELDS)
        self.assertEqual((totals["schema"], totals["population_sha256"], totals["scope"]),
                         ("prism.measure.review/1", sha256_of(self.population), "population"))
        self.assertEqual((totals["population"], totals["selected"], totals["ok"]), (1, 1, 1))
        self.assertEqual((totals["failed_ordinals"], totals["refused_ordinals"],
                          totals["excluded_ordinals"]), ([], [], []))
        self.assert_counts(totals, presence=1)
        self.assertEqual(self.read_rows("sample.jsonl"), [{key: row[key] for key in SAMPLE_FIELDS}])

    def test_arm_and_shared_arg_parsing_do_not_use_a_shell(self):
        """open-n4-19: literal arm splitting and shared shlex args have distinct grammars."""
        self.make_population()
        left, right = self.binaries.values()
        self.review("--review-args", "--algorithm taint --taint-source 'std::exec'",
                    arms=[f"left={left}::--one=value=x::literal space::;echo no",
                          f"right={right}::--two::std::exec"])
        base, head = self.attempts()
        for item, extra in ((base, ["--one=value=x", "literal space", ";echo no"]),
                            (head, ["--two", "std", "exec"])):
            args = item["argv"]
            self.assertEqual(args[0:2], ["--repo", item["root"]])
            self.assertEqual(args[2], "--diff")
            self.assertEqual(args[4:], [
                "--algorithm", "taint", "--taint-source", "std::exec",
                *extra, "--format", "json", "--no-cache",
            ])

    def test_invalid_invocation_or_population_exits_two_before_enumeration(self):
        self.make_population()
        left, right = self.binaries.values()
        valid = [f"left={left}", f"right={right}"]
        invocations = [
            (["--only", "1,1"], valid), (["--only", "9"], valid), (["--sample", "-1"], valid),
            ([], valid[:1]), ([], [valid[0], f"left={right}"]),
            ([], [f"{valid[0]}::--repo::elsewhere", valid[1]]),
            ([], [f"{valid[0]}::--diff=bogus", valid[1]]),
            (["--review-args=--format text"], valid),
        ]
        for options, arms in invocations:
            with self.subTest(options=options, arms=arms):
                self.review(*options, arms=arms, expected=2)
                self.assertFalse((self.out / "per-diff.jsonl").exists())
        self.registration["diffs"].append(self.registration["diffs"][0].copy())
        self.write_population()
        self.review(expected=2)
        self.assertFalse((self.out / "review-receipt.json").exists())
        self.assertEqual(self.fake_log(), [])

    def test_child_exit_75_is_failed_and_guard_quiet_is_refused(self):
        self.make_population(ordinals=(1, 2, 3))
        self.make_arm("base", exit_code=75)
        self.review("--only", "1", expected=1)
        row = self.read_rows()[0]
        self.assertEqual(row["status"], "failed")
        self.assertEqual(row["arms"]["base"]["guard_status"],
                         {"kind": "child", "launched": True, "child_exit": 75})
        self.assert_streams(row, "base", stdout=envelope_bytes(EMPTY), stderr=b"base diagnostic\n")

        self.out = self.work / "quiet"
        self.log.unlink()
        self.make_guard({"--diff": (75, QUIET)})
        self.review("--only", "1,3", expected=75)
        rows = self.read_rows()
        self.assertEqual([row["status"] for row in rows], ["refused", "excluded", "refused"])
        self.assertEqual(rows[0]["arms"]["base"]["guard_status"], QUIET)
        self.assertEqual(self.attempts(), [])
        for row in rows:
            self.assert_null_metrics(row)
            for arm in row["arms"].values():
                self.assert_unlaunched(arm)
        totals = self.read_json("totals.json")
        self.assertEqual(totals["refused_ordinals"], [1, 3])
        self.assertEqual(totals["excluded_ordinals"], [2])
        self.assertEqual(totals["ok"], 0)

    def test_guard_quiet_after_failure_retains_both_dispositions(self):
        """open-n2-27: the host-quiet exit applies and earlier failures remain recorded."""
        self.make_population(ordinals=(1, 2, 3))
        self.make_arm("head", exit_code=7)
        normal = self.guard
        self.guard = self.work / "quiet-guard"
        quiet = self.make_guard({"--diff": (75, QUIET)})
        self.guard = normal
        self.guard_for = lambda argv: quiet if "--diff" in argv and len(self.attempts()) >= 2 else normal
        self.review(expected=75)
        rows = self.read_rows()
        self.assertEqual([row["status"] for row in rows], ["failed", "refused", "refused"])
        self.assertEqual(rows[1]["arms"]["base"]["guard_status"], QUIET)
        totals = self.read_json("totals.json")
        self.assertEqual((totals["failed_ordinals"], totals["refused_ordinals"]), ([1], [2, 3]))
        self.assertEqual([item["role"] for item in self.attempts()], ["base", "head"])

    def test_product_calls_share_one_controlled_environment(self):
        """open-n2-11: the S4 product launches use the §2 environment control."""
        self.make_population()
        self.env.update(PRISM_EDGES_BYTES="1", PRISM_TEST_SWITCH="tainted", MEASURE_TEST_AMBIENT="x")
        self.review()
        log = self.fake_log()
        self.assertEqual({(item["role"], item["root"] is None) for item in log}, {
            ("base", True), ("head", True), ("base", False), ("head", False),
        })
        for item in log:
            env = item["env"]
            self.assertEqual(env["LC_ALL"], "C")
            self.assertEqual(env["HOME"], self.env["HOME"])
            self.assertNotIn("MEASURE_TEST_AMBIENT", env)
            self.assertFalse(any(key.startswith("PRISM_") for key in env), env)
            self.assertEqual(env, log[0]["env"])

    def test_conflicting_single_and_multi_markers(self):
        """open-n1-27: contradictory envelope markers fail the arm."""
        self.make_population()
        self.make_arm("head", {**single(finding()), "algorithms_run": ["Taint"], "all_findings": []})
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["arms"]["head"]["status"]), ("failed", "failed"))
        self.assertIn("review-envelope-contradictory", row["reason"])
        self.assert_null_metrics(row)

    def test_algorithm_errors_in_a_zero_exit_envelope_fail_the_arm(self):
        """A multi-algorithm run can exit 0 and report per-algorithm errors; those earn no coverage."""
        self.make_population()
        multi = {"algorithms_run": ["taint"], "results": [], "all_findings": []}
        self.make_arm("base", multi)
        self.make_arm("head", {**multi, "errors": [{"algorithm": "taint", "error": "boom"}]})
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["arms"]["head"]["status"]), ("failed", "failed"))
        self.assertIn("review-algorithm-errors", row["reason"])
        self.assert_null_metrics(row)

    def test_empty_errors_array_is_a_clean_run(self):
        self.make_population()
        multi = {"algorithms_run": ["taint"], "results": [], "all_findings": [], "errors": []}
        self.make_arm("base", multi)
        self.make_arm("head", multi)
        self.review()
        row, = self.read_rows()
        self.assertEqual(row["status"], "ok")

    def test_description_and_message_change_together(self):
        """open-n1-29: a change to both text fields is text-only."""
        self.make_population()
        self.pair([finding(message="old")], [finding(description="new", message="new")])
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=1)
        anchor, = row["anchors"]
        pair, = anchor["pairs"]
        self.assertEqual((pair["fields"], pair["text_only"]), (["description", "message"], True))

    @unittest.skip("OPEN-ROW open-n2-11: re-scoped to S5 (review environment recording for gate custody)")
    def test_effective_review_environment_is_recorded(self):
        """open-n2-11: §5 gives no location in its exact receipt shapes for the effective environment."""
        self.make_population()
        self.env["PRISM_TEST_SWITCH"] = "tainted"

    def test_envelope_without_mode_marker(self):
        """open-n2-26: an unmarked envelope fails the arm."""
        self.make_population()
        self.make_arm("head", {"findings": [finding()], "all_findings": []})
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["arms"]["head"]["status"]), ("failed", "failed"))
        self.assertIn("review-envelope-unmarked", row["reason"])
        self.assert_null_metrics(row)

    def test_unlocated_finding(self):
        """open-n3-18: a finding without a location fails the ordinal."""
        self.make_population()
        self.make_arm("head", single({"category": "taint_sink"}))
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual(row["status"], "failed")
        self.assertIn("review-finding-location-missing", row["reason"])
        self.assert_null_metrics(row)

    def test_repeated_rule_ids_with_distinct_metadata(self):
        """open-n3-19: different metadata on the same ID gives distinct rule units."""
        self.make_population()
        self.pair([finding(rules=[{"id": "A", "cwe": "CWE-1"}])],
                  [finding(rules=[{"id": "A", "cwe": "CWE-1"}, {"id": "A", "cwe": "CWE-2"}])])
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=1)
        self.assertEqual(row["rule_transitions"], [
            {"from": ["A"], "to": ["A", "A"], "count": 1, "from_state": "list", "to_state": "list"},
        ])

    def test_only_explicitly_lists_the_complete_population(self):
        """open-n3-20: an exhaustive --only still has population scope."""
        self.make_population(ordinals=(2, 1))
        self.review("--only", "1,2")
        totals = self.read_json("totals.json")
        self.assertEqual((totals["selected"], totals["scope"]), (2, "population"))

    @unittest.skip("OPEN-ROW open-n4-19: re-scoped to S5 (structured arm argument array in gate steps)")
    def test_literal_double_colon_in_one_arm_only(self):
        """open-n4-19: the shared-argument escape route applies to both arms."""
        self.make_population()
        left, right = self.binaries.values()
        self.review(arms=[f"left={left}", f"right={right}::--taint-source::std::exec"])


FINDING_JSON = (b'{"algorithm":"Taint","findings":[{"file":"src/a.ts","line":3,"algorithm":"taint",'
                b'"category":"taint_sink","other":%s}]}\n')


class ReviewFoldOneTests(ReviewChecks):
    """Fold 1 of PR 984: payload spelling, marker contents, guard custody and replay."""

    def reseal(self, mutate):
        rows, totals = self.read_rows(), self.read_json("totals.json")
        record = self.read_json("review-receipt.json")
        mutate(rows, totals, record)
        totals["arms"] = record["arms"]
        for name, data in (("per-diff.jsonl", b"".join(json.dumps(row).encode() + b"\n" for row in rows)),
                           ("totals.json", json.dumps(totals).encode() + b"\n")):
            path = self.out / name
            path.write_bytes(data)
            record["artifacts"][name] = {"bytes": len(data), "sha256": sha256_of(path)}
        (self.out / "review-receipt.json").write_bytes(json.dumps(record).encode())

    def admit(self):
        from measure import review_diff

        return review_diff.admit(self.out, population=self.population)

    def test_integer_token_spelling_is_part_of_the_payload(self):
        self.make_population()
        self.make_arm("base", stdout=FINDING_JSON % b"-0")
        self.make_arm("head", stdout=FINDING_JSON % b"0")
        self.review()
        row, = self.read_rows()
        self.assert_counts(row, payload=1)
        self.admit()
        # Control: equal spellings cancel.
        self.out = self.work / "same"
        self.make_arm("base", stdout=FINDING_JSON % b"0")
        self.review()
        row, = self.read_rows()
        self.assert_counts(row)

    def test_line_is_still_a_positive_integer(self):
        from measure.review_findings import decode

        ok = b'{"algorithm":"Taint","findings":[{"file":"a.ts","line":%s}]}'
        self.assertEqual(len(decode(ok % b"3")), 1)
        for line in (b"0", b"-0", b"-1", b"1.0", b"1e0", b'"3"', b"true", b"null"):
            with self.subTest(line=line), self.assertRaises(ValueError):
                decode(ok % line)

    def test_marker_contents_are_validated(self):
        bad = (b'{"results":[42],"all_findings":[]}', b'{"algorithms_run":[false],"all_findings":[]}',
               b'{"algorithms_run":[],"all_findings":[]}', b'{"algorithms_run":[""],"all_findings":[]}',
               b'{"algorithm":"","findings":[]}', b'{"algorithm":7,"findings":[]}')
        self.make_population()
        for index, raw in enumerate(bad):
            with self.subTest(raw=raw):
                self.out = self.work / f"bad-{index}"
                self.make_arm("head", stdout=raw)
                self.review(expected=1)
                row, = self.read_rows()
                self.assertEqual(row["status"], "failed")
                self.assertIn("review-envelope-malformed", row["reason"])
                self.assert_null_metrics(row)
        self.out = self.work / "wellformed"
        self.make_arm("head", stdout=b'{"algorithms_run":["Taint"],"results":[{}],"all_findings":[]}')
        self.review()

    def silent_guard(self, *, run_child=True):
        guard = executable(self.work / "silent-guard", f"""
import subprocess
import sys
args = sys.argv[4:]
code = subprocess.call(args) if {run_child!r} else 1
sys.exit(128 - code if code < 0 else code)
""")
        normal = self.guard
        self.guard_for = lambda argv: guard if "--diff" in argv else normal

    def test_missing_guard_status_keeps_the_launched_streams(self):
        self.make_population()
        self.make_arm("base", exit_code=7)
        self.silent_guard()
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-guard-status-missing"))
        base = row["arms"]["base"]
        self.assertEqual((base["status"], base["guard_status"], base["exit"]), ("failed", None, None))
        self.assert_streams(row, "base", stdout=envelope_bytes(EMPTY), stderr=b"base diagnostic\n")
        self.assert_unlaunched(row["arms"]["head"])
        self.assert_null_metrics(row)
        self.admit()

    def test_missing_guard_status_without_a_launch_keeps_nothing(self):
        """Control: a guard that never started the child leaves no streams to keep."""
        self.make_population()
        self.silent_guard(run_child=False)
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-guard-failed"))
        self.assert_unlaunched(row["arms"]["base"])
        self.assertFalse((self.out / "arms").exists() and any((self.out / "arms").rglob("*.std*")))
        self.admit()

    def test_replay_rejects_structured_host_quiet_relabelled_as_failed(self):
        self.make_population()
        self.make_guard({"--diff": (75, QUIET)})
        self.review(expected=75)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["arms"]["base"]["guard_status"]), ("refused", QUIET))
        self.admit()

        def relabel(rows, totals, record):
            rows[0]["status"] = "failed"
            rows[0]["reason"] = "relabelled"
            rows[0]["arms"]["base"]["status"] = "failed"
            totals["failed_ordinals"], totals["refused_ordinals"] = [1], []

        self.reseal(relabel)
        with self.assertRaises(ValueError):
            self.admit()

    def test_known_limit_transient_writer_during_launch_is_not_detected(self):
        """PROVISIONAL (known-s4-2): the spec and worktree are checked at arm boundaries only."""
        edited = json.dumps({"files": [{"file_path": "src/a.ts", "modify_type": "Modified",
                                        "diff_lines": [1, 2]}]}).encode()
        original = json.dumps(STANDARD_SPEC).encode()
        self.make_population()
        cases = {
            "transient-spec": (
                f"os.chmod(spec_path, 0o600); spec_path.write_bytes({edited!r})",
                f"spec_path.write_bytes({original!r}); os.chmod(spec_path, 0o400)"),
            "mode-only": ("os.chmod(spec_path, 0o600)", ""),
            "transient-worktree": (
                "(root / 'src/a.ts').write_bytes(b'transient\\n')",
                f"(root / 'src/a.ts').write_bytes({AFTER.encode()!r})"),
        }
        for name, (before, after) in cases.items():
            with self.subTest(name):
                self.out = self.work / name
                self.make_arm("head", before_read=before, after_read=after)
                self.review()
                row, = self.read_rows()
                self.assertEqual(row["status"], "ok")
                self.assert_counts(row)
        # Control: the same writer without the restore is caught at the post-arm check.
        self.out = self.work / "control"
        self.make_arm("head", before_read=cases["transient-spec"][0])
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-input-changed"))

    def test_known_limit_replay_checks_internal_integrity_only(self):
        """PROVISIONAL (known-s4-3): a consistently resealed receipt is admitted."""
        self.make_population()
        self.review()
        self.admit()
        stderr = self.out / "arms/1/base.stderr"
        saved = stderr.read_bytes()
        stderr.write_bytes(b"x" + saved[1:])
        with self.assertRaises(ValueError):
            self.admit()
        stderr.write_bytes(saved)

        def unequal(rows, totals, record):
            rows[0]["arms"][record["arms"]["head"]["name"]]["input_after_sha256"] = "d" * 64

        self.reseal(unequal)
        with self.assertRaises(ValueError):
            self.admit()
        self.out = self.work / "forged"
        self.review()

        def digests(rows, totals, record):
            for arm in rows[0]["arms"].values():
                arm["input_before_sha256"] = arm["input_after_sha256"] = "f" * 64

        self.reseal(digests)
        self.admit()
        self.out = self.work / "forged-binary"
        self.review()

        def binary(rows, totals, record):
            head = record["arms"]["head"]
            head["binary"].update(path="/nonexistent/forged-head", sha256="e" * 64)
            for command in head["commands"]:
                command["argv"][0] = head["binary"]["path"]

        self.reseal(binary)
        self.admit()

    def reseal_accounting(self, mutate):
        """Rewrite rows and arm records, then recompute totals and sample so the whole
        receipt is internally consistent (the forger of known-s4-5 does the same)."""
        from measure import population, review_receipt
        from measure.review_findings import review_json

        record = self.read_json("review-receipt.json")
        rows, old = self.read_rows(), self.read_json("totals.json")
        mutate(rows, record)
        totals, sample = review_receipt._summarize(
            population.load(self.population), record["arms"], rows, old["sample_size"], old["seed"])
        for name, data in (("per-diff.jsonl", b"".join(review_json(row) + b"\n" for row in rows)),
                           ("totals.json", review_json(totals) + b"\n"),
                           ("sample.jsonl", b"".join(review_json(row) + b"\n" for row in sample))):
            path = self.out / name
            path.write_bytes(data)
            record["artifacts"][name] = {"bytes": len(data), "sha256": sha256_of(path)}
        (self.out / "review-receipt.json").write_bytes(json.dumps(record).encode())

    def quiet_first_ordinal(self, reason="host-quiet", status="refused", event=QUIET):
        def mutate(rows, record):
            row = rows[0]
            names = {role: record["arms"][role]["name"] for role in ("base", "head")}
            row.update(status=status, reason=reason, **dict.fromkeys(METRICS),
                       rule_transitions=[], anchors=[])
            before = row["arms"][names["base"]]["input_before_sha256"] if event else None
            empty = dict.fromkeys(ARM_FIELDS)
            row["arms"][names["base"]] = {**empty, "status": status, "guard_status": event,
                                          "input_before_sha256": before}
            row["arms"][names["head"]] = {**empty, "status": "unlaunched"}
            for role in ("base", "head"):
                arm = record["arms"][role]
                arm["commands"] = [c for c in arm["commands"] if c["ordinal"] != 1]
                for suffix in ("stdout", "stderr"):
                    record["artifacts"].pop(f"arms/1/{role}.{suffix}", None)
        return mutate

    def test_known_limit_replay_admits_contradictory_quiet_receipts(self):
        """PROVISIONAL (known-s4-5): a success after a quiet ordinal, a quiet ordinal with
        another reason and a quiet reason with no guard event are admitted; the live command refuses every ordinal after a quiet one."""
        self.make_population(ordinals=(1, 2))
        normal = self.guard
        self.guard = self.work / "quiet-guard"
        quiet = self.make_guard({"--diff": (75, QUIET)})
        self.guard = normal
        self.guard_for = lambda argv: quiet if "--diff" in argv else normal
        self.out = self.work / "live"
        self.review(expected=75)
        self.assertEqual([(row["status"], row["reason"]) for row in self.read_rows()],
                         [("refused", "host-quiet")] * 2)
        self.guard_for = lambda argv: normal
        for name, reason, event in (("success-after-quiet", "host-quiet", QUIET),
                                    ("wrong-reason", "review-guard-failed", QUIET),
                                    ("quiet-without-guard-event", "host-quiet", None)):
            with self.subTest(name):
                self.out = self.work / name
                self.review()
                self.admit()
                self.reseal_accounting(self.quiet_first_ordinal(reason, event=event))
                self.admit()
                self.assertEqual([row["status"] for row in self.read_rows()], ["refused", "ok"])
        # Control: the same quiet record relabelled failed is rejected.
        self.out = self.work / "relabelled"
        self.review()
        self.reseal_accounting(self.quiet_first_ordinal(status="failed"))
        with self.assertRaisesRegex(ValueError, "host-quiet review arm is not refused"):
            self.admit()

    def test_known_limit_executable_rewritten_during_launch(self):
        """PROVISIONAL (known-s4-2, executable clause): a head that rewrites its own executable
        during the launch fails the row, keeps its streams, and the receipt keeps the
        pre-launch hash; a rewrite restored before the post-arm check is not seen."""
        self.make_population()
        rewrite = "p = Path(sys.argv[0]); p.write_bytes(p.read_bytes() + b'\\n# rewritten\\n')"
        head = self.make_arm("head", after_read=rewrite)
        before = sha256_of(head)
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual(row["status"], "failed")
        self.assertTrue(row["reason"].startswith("review-executable-changed"), row["reason"])
        self.assertEqual(row["arms"]["head"]["status"], "failed")
        self.assert_streams(row, "head", stdout=envelope_bytes(EMPTY), stderr=b"head diagnostic\n")
        self.assert_null_metrics(row)
        self.assertNotEqual(sha256_of(head), before)
        self.assertEqual(self.read_json("review-receipt.json")["arms"]["head"]["binary"]["sha256"], before)
        self.admit()
        # Transient: restored before exit, the row is ok.
        self.out = self.work / "transient"
        restore = ("p = Path(sys.argv[0]); b = p.read_bytes(); p.write_bytes(b + b'x'); "
                   "(p.parent / 'transient-ran').touch(); p.write_bytes(b)")
        head = self.make_arm("head", after_read=restore)
        self.review()
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("ok", None))
        self.assertTrue((head.parent / "transient-ran").exists())
        self.assertEqual(sha256_of(head), self.read_json("review-receipt.json")["arms"]["head"]["binary"]["sha256"])

    def test_known_limit_missing_guard_status_custody_is_by_content(self):
        """PROVISIONAL (known-s4-5): with no guard status a launched child that wrote nothing
        keeps no streams, and a guard that only printed a diagnostic (no child) is recorded
        under `commands`, which §5 defines as launched commands."""
        self.make_population()
        self.make_arm("base", exit_code=7, stdout=b"", stderr=b"")
        self.silent_guard()
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-guard-failed"))
        self.assertEqual([item["role"] for item in self.attempts()], ["base"])
        self.assert_unlaunched(row["arms"]["base"])
        self.assertFalse((self.out / "arms").exists() and any((self.out / "arms").rglob("*.std*")))
        self.assertEqual(self.read_json("review-receipt.json")["arms"]["base"]["commands"], [])
        # No child at all, only a guard diagnostic.
        self.out = self.work / "guard-only"
        self.log.unlink()
        guard = executable(self.work / "diagnostic-guard", """
import sys
sys.stderr.write("guard diagnostic\\n")
sys.exit(1)
""")
        normal = self.guard
        self.guard_for = lambda argv: guard if "--diff" in argv else normal
        self.make_arm("base")
        self.review(expected=1)
        row, = self.read_rows()
        self.assertEqual((row["status"], row["reason"]), ("failed", "review-guard-status-missing"))
        self.assertEqual(self.attempts(), [])
        commands = self.read_json("review-receipt.json")["arms"]["base"]["commands"]
        self.assertEqual([item["ordinal"] for item in commands], [1])
        self.admit()


class ReadStatusRaceTests(unittest.TestCase):
    def test_status_written_before_exit_is_read_when_the_first_poll_reports_nothing(self):
        from measure import run

        process = subprocess.Popen([sys.executable, "-c", "pass"])
        process.wait()
        read_fd, write_fd = os.pipe()
        os.write(write_fd, b'{"kind":"child"}\n')
        os.close(write_fd)
        self.addCleanup(os.close, read_fd)
        real = run.select.select
        calls = []

        def stale_once(*args):
            calls.append(args)
            return ([], [], []) if len(calls) == 1 else real(*args)

        with patch.object(run.select, "select", stale_once):
            self.assertEqual(run.read_status(read_fd, process), b'{"kind":"child"}\n')
        self.assertGreaterEqual(len(calls), 2)
