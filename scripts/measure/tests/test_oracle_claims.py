"""Claim selection and admission controls: residual reconciliation, cancelled sites, caller
convention, denominators, wire classes and the mixed-language census."""

from __future__ import annotations

import copy
import json
import os
import re
import shutil
import tempfile
import unittest
from pathlib import Path

from measure import oracle_selection
from measure.corpora import load_manifest
from measure.edges import canonical

from . import test_oracle as fixtures
from .test_matrix_rows import CASES
from .test_oracle_guards import argparse_namespace

read_jsonl = fixtures.read_jsonl


def point(file, text, token, nth, access):
    starts = [m.start() for m in re.finditer(r"(?<![\w$])" + re.escape(token) + r"(?![\w$])", text)]
    start = starts[nth]
    return dict(file=file, start_byte=start, end_byte=start + len(token),
                line=text[:start].count("\n") + 1, access=access, path=dict(base=token, fields=[]))


def call_site(text, call, caller, line, target, target_line):
    start = text.index(call)
    return {"record_kind": "call_site", "callee_text": call, "caller": dict(file="cases.ts", name=caller,
            start_line=line), "origin": "Source", "drop": None, "exact_target": None,
            "resolved_targets": [dict(confidence="exact", kind="local_def", function_id=dict(
                file="cases.ts", name=target, start_line=target_line, end_line=target_line))],
            "source_span": dict(file="cases.ts", line=text[:start].count("\n") + 1,
                                start_byte=start, end_byte=start + len(call))}


class Claims(unittest.TestCase):
    def setUp(self):
        self.h = fixtures.OracleTests()
        self.addCleanup(self.h.doCleanups)
        self.h.setUp()

    def test_residual_requests_reconcile_with_both_bound_arms(self):
        h = self.h
        case = CASES / "oracle-001"
        h.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        row, = read_jsonl(case / "input.jsonl")
        base = h.make_run(h.work / "base", [row])
        head = h.make_run(h.work / "head", [row, row])
        classifier = h.work / "classifier.py"
        classifier.write_text("CLASSES = ['bound']\ndef classify(change, context): return 'bound'\n")
        diff = h.work / "diff"
        h.assert_exit(h.kit("diff-edges", base, head, "--classifier", classifier, "--out", diff), 0)
        entries = load_manifest(h.manifest)

        def select(directory, run):
            args = argparse_namespace(("ts-edges", "--run", run, "--changes", directory / "changes.jsonl",
                                       "--kind", "added", "--side", "head"))
            return oracle_selection.edge_population(args, entries, None)[3]

        # An honest residual (one more copy in the head) keeps its selection.
        self.assertEqual([c["arm"] for c in select(diff, head)], ["head"])
        # The same digest-consistent stream bound to two runs with identical bags is no residual.
        first, second = (h.make_run(h.work / name, [row, row]) for name in ("first", "second"))
        forged = h.work / "forged"
        shutil.copytree(diff, forged)
        certificate = json.loads((forged / "certificate.json").read_bytes())
        for arm, run in (("base", first), ("head", second)):
            certificate["inputs"][arm] = dict(run=os.path.relpath(run, forged),
                                              receipt_sha256=oracle_selection.admit(run, entries).receipt_sha256)
        (forged / "certificate.json").write_bytes(canonical(certificate) + b"\n")
        with self.assertRaisesRegex(ValueError, "selector: change stream does not reconcile"):
            select(forged, second)

    def bound_streams(self, tag, base_rows, head_rows):
        h = self.h
        base = h.make_run(h.work / f"{tag}-base", base_rows)
        head = h.make_run(h.work / f"{tag}-head", head_rows)
        classifier = h.work / "classifier.py"
        classifier.write_text("CLASSES = ['bound']\ndef classify(change, context): return 'bound'\n")
        diff = h.work / f"{tag}-diff"
        h.assert_exit(h.kit("diff-edges", base, head, "--classifier", classifier, "--out", diff), 0)
        return base, head, diff

    def restream(self, diff, changes, runs=None):
        """Replace the change stream by `changes`, keeping the certificate digest-consistent."""
        raw = b"".join(canonical(change) + b"\n" for change in changes)
        (diff / "changes.jsonl").write_bytes(raw)
        path = diff / "certificate.json"
        certificate = json.loads(path.read_bytes())
        certificate["changes_sha256"] = oracle_selection.digest(raw)
        for arm, run in (runs or {}).items():
            certificate["inputs"][arm] = dict(
                run=os.path.relpath(run, diff),
                receipt_sha256=oracle_selection.admit(run, load_manifest(self.h.manifest)).receipt_sha256)
        path.write_bytes(canonical(certificate) + b"\n")

    def select_added(self, diff, run, side="head"):
        args = argparse_namespace(("ts-edges", "--run", run, "--changes", diff / "changes.jsonl",
                                   "--kind", "added", "--side", side))
        return oracle_selection.edge_population(args, load_manifest(self.h.manifest), None)[3]

    def source_row(self):
        h = self.h
        case = CASES / "oracle-001"
        h.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        row, = read_jsonl(case / "input.jsonl")
        return row

    def test_identical_arms_with_an_added_and_a_removed_pair_do_not_reconcile(self):
        row = self.source_row()
        _, _, diff = self.bound_streams("honest", [row], [row, row])
        added, = read_jsonl(diff / "changes.jsonl")
        removed = dict(added, id=2, kind="removed", base_bag=added["head_bag"], head_bag=[])
        first, second = (self.h.make_run(self.h.work / name, [row]) for name in ("same-a", "same-b"))
        self.restream(diff, [added, removed], dict(base=first, head=second))
        with self.assertRaisesRegex(ValueError, "selector: change stream does not reconcile"):
            self.select_added(diff, second)
        # The honest stream bound to its own unequal arms keeps its single selection.
        base, head, honest = self.bound_streams("control", [row], [row, row])
        self.assertEqual([c["arm"] for c in self.select_added(honest, head)], ["head"])

    def test_a_moved_label_cannot_be_restreamed_as_a_removed_and_an_added_edge(self):
        row = self.source_row()
        soft = dict(row, confidence="nameonly", doubt="alias_unstable")
        _, head, diff = self.bound_streams("moved", [soft], [row])
        moved, = read_jsonl(diff / "changes.jsonl")
        self.assertEqual(moved["kind"], "label_moved")
        # The genuine movement is not an addition: the selection is empty, and it reconciles.
        self.assertEqual(self.select_added(diff, head), [])
        # Same net balance per attribute, a different grouping: S2 reports one label move.
        removed = dict(moved, id=1, kind="removed", head_bag=[])
        added = dict(moved, id=2, kind="added", base_bag=[])
        self.restream(diff, [removed, added])
        with self.assertRaisesRegex(ValueError, "selector: change stream does not reconcile"):
            self.select_added(diff, head)
        # Genuine additions and moves keep their selections in one stream.
        _, head2, honest = self.bound_streams("mixed", [soft], [row, row])
        kinds = sorted(c["kind"] for c in read_jsonl(honest / "changes.jsonl"))
        self.assertEqual(kinds, ["added", "label_moved"])
        self.assertEqual([c["arm"] for c in self.select_added(honest, head2)], ["head"])

    def test_a_stream_that_omits_a_real_residual_does_not_reconcile(self):
        row = self.source_row()
        near, far = (dict(row, to=dict(row["to"], line=row["to"]["line"] + step)) for step in (1, 2))
        _, head, diff = self.bound_streams("omit", [row], [row, near, far])
        changes = read_jsonl(diff / "changes.jsonl")
        self.assertEqual(len(changes), 2)
        self.assertEqual(len(self.select_added(diff, head)), 2)
        kept = [dict(changes[0], id=1)]
        self.restream(diff, kept)
        with self.assertRaisesRegex(ValueError, "selector: change stream does not reconcile"):
            self.select_added(diff, head)

    def test_change_key_decoding_and_opposite_arm_errors_keep_their_wire_class(self):
        row = self.source_row()
        base, head, diff = self.bound_streams("wire", [row], [row, row])
        change, = read_jsonl(diff / "changes.jsonl")
        self.assertEqual(len(self.select_added(diff, head)), 1)
        bad = copy.deepcopy(change)
        bad["key"]["to"]["fields"] = [None]
        self.restream(diff, [bad])
        with self.assertRaisesRegex(ValueError, "^changes-wire: expected structured path"):
            self.select_added(diff, head)
        # An unsafe file in the opposite arm keeps its own class, not an `edge-wire` wrapper.
        unsafe = copy.deepcopy(row)
        unsafe["from"]["file"] = "../escape.ts"
        other, head2, diff2 = self.bound_streams("unsafe", [unsafe], [row, row])
        with self.assertRaisesRegex(ValueError, "^wire-coordinate: invalid file"):
            self.select_added(diff2, head2)

    def test_head_only_snapshot_mismatch_is_inadmissible_for_the_selected_claim_only(self):
        h = self.h
        setup = CASES / "oracle-021"
        h.fresh_tree(files={"cases.ts": (setup / "repo/cases.ts").read_bytes()})
        first, second = read_jsonl(setup / "input.jsonl")
        second["resolved_targets"][0]["function_id"]["name"] = "sameB"
        base = h.make_run(h.work / "base", [first], sites=True)
        head = h.make_run(h.work / "head", [second], sites=True)
        h.synthetic_qualification(base, head)
        h.prepare()
        control = h.work / "control"
        h.assert_exit(h.oracle(("ts-calls", "--base", base, "--head", head), out=control), 0)
        self.assertEqual([v["verdict"] for v in read_jsonl(control / "verdicts.jsonl")], ["CORRECT"])
        root = Path(json.loads((head / "receipt.json").read_bytes())["corpus"]["root"])
        (root / "cases.ts").write_bytes((root / "cases.ts").read_bytes() + b"\n")
        result = h.oracle(("ts-calls", "--base", base, "--head", head))
        h.assert_exit(result, 0)
        verdict, = read_jsonl(h.out / "verdicts.jsonl")
        self.assertEqual((verdict["verdict"], verdict["why"]), ("INADMISSIBLE", "source-digest-mismatch"))

    def census_run(self, name, text, caller_line, forms, *, caller="m"):
        h = self.h
        row = call_site(text, "g(1)", caller, caller_line, "g", 1)
        run = h.make_run(h.work / name, [row], sites=True)
        h.synthetic_qualification(run, forms=forms)
        h.prepare()
        out = h.work / (name + "-out")
        h.assert_exit(h.oracle(("ts-calls", "--census", run), out=out), 0)
        verdict, = read_jsonl(out / "verdicts.jsonl")
        return verdict

    def test_caller_triple_contradicts_only_for_a_qualified_caller_form(self):
        h = self.h
        text = "function g(v: number) { return v; }\nclass K {\n  @dec\n  m() { return g(1); }\n}\n"
        h.fresh_tree(files={"cases.ts": text})
        # The decorated method names the decorator line. Without a qualified decorated-method
        # convention for this build that triple is no evidence against the claim.
        open_form = self.census_run("open", text, 3, ("named-function",))
        self.assertEqual((open_form["verdict"], open_form["why"]), ("CORRECT", "callee-implementation"))
        # With the form qualified the same triple contradicts, and the qualified line is the control.
        both = ("named-function", "decorated-method")
        contradicted = self.census_run("qualified", text, 3, both)
        self.assertEqual((contradicted["verdict"], contradicted["why"]),
                         ("INADMISSIBLE", "caller-identity-mismatch"))
        self.assertEqual(self.census_run("control", text, 4, both)["verdict"], "CORRECT")

    def test_unselected_site_of_an_unauthenticated_snapshot_refuses(self):
        h = self.h
        setup = CASES / "oracle-021"
        h.fresh_tree(files={"cases.ts": (setup / "repo/cases.ts").read_bytes()})
        first, second = read_jsonl(setup / "input.jsonl")
        base = h.make_run(h.work / "base", [first], sites=True)
        second["resolved_targets"][0]["function_id"]["name"] = "sameB"
        head = h.make_run(h.work / "head", [copy.deepcopy(first), second], sites=True)
        h.synthetic_qualification(base, head)
        h.prepare()
        control = h.work / "control"
        h.assert_exit(h.oracle(("ts-calls", "--base", base, "--head", head), out=control), 0)
        self.assertEqual([v["verdict"] for v in read_jsonl(control / "verdicts.jsonl")], ["CORRECT"])
        for run in (base, head):
            root = Path(json.loads((run / "receipt.json").read_bytes())["corpus"]["root"])
            (root / "cases.ts").write_bytes((root / "cases.ts").read_bytes() + b"\n")
        refused = h.oracle(("ts-calls", "--base", base, "--head", head))
        h.assert_exit(refused, 2)
        self.assertIn("cancelled-site-coordinate: unauthenticated", refused.stderr)
        self.assertFalse((h.out / "oracle-receipt.json").exists())

    def test_empty_name_only_population_still_reports_the_missing_kind_zero(self):
        h = self.h
        case = CASES / "oracle-022"
        h.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        row, = read_jsonl(case / "input.jsonl")
        entries = load_manifest(h.manifest)
        select = ("--question", h.work / "q.cjs", "--relation", "argfollow",
                  "--target-confidence", "name_only", "--target-kind", "argument_follow")

        def populations(source):
            run = h.make_run(h.work / "run", [source], sites=True)
            return oracle_selection.call_population(
                argparse_namespace(("ts-calls", "--census", run, *select)), entries)[5]["populations"]

        empty = populations(row)
        zeros = dict.fromkeys(empty["name_only"]["kinds"]["argument_follow"], 0)
        self.assertEqual(empty["name_only"]["missing_kind"], zeros)
        self.assertEqual(empty["exact"]["missing_kind"], zeros | {"census": 0})
        # Actual null-kind claims keep their counts.
        shutil.rmtree(h.work / "run")
        nullkind = copy.deepcopy(row)
        nullkind["resolved_targets"][0].update(confidence="name_only", kind=None)
        counted = populations(nullkind)["name_only"]["missing_kind"]
        self.assertEqual(counted["census"], 1)

    def test_wire_classes_name_structured_path_and_duplicate_member_refusals(self):
        row, = read_jsonl(CASES / "oracle-001/input.jsonl")
        self.assertEqual(oracle_selection.edge(row), row)
        bad = copy.deepcopy(row)
        bad["to"]["path"]["fields"] = [None]
        with self.assertRaisesRegex(ValueError, "^edge-wire: expected structured path"):
            oracle_selection.edge(bad)

        class Run:
            receipt = {"commands": [{"id": "edges"}]}

            def __init__(self, path):
                self.path = path

            def artifact(self, role, kind):
                return self.path

            def expected(self, role, kind):
                return None

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "edges.jsonl"
            path.write_text(json.dumps(row) + "\n")
            self.assertEqual(list(oracle_selection.read(Run(path), "edges")), [row])
            path.write_text('{"a":1,"a":2}\n')
            with self.assertRaisesRegex(ValueError, "^wire-json: duplicate JSON field"):
                list(oracle_selection.read(Run(path), "edges"))

    def test_name_only_confidence_only_successor_is_admitted(self):
        h = self.h

        def spell(rows):
            for row in rows:
                for target in row["resolved_targets"]:
                    target["confidence"] = "name_only"

        args = h.load_case("oracle-028", transform=spell)
        h.prepare()
        h.assert_exit(h.oracle(args), 0)
        delta, = read_jsonl(h.out / "deltas.jsonl")
        self.assertEqual(delta["kind"], "confidence-only")
        self.assertEqual(read_jsonl(h.out / "verdicts.jsonl"), [])

    def mixed_census(self, name, mutate=None):
        h = self.h
        ts, go = "let ox = 1;\nlet oy = ox;\n", "package main\nfunc f(a int) int { return a }\n"
        h.fresh_tree(files={"cases.ts": ts, "main.go": go})
        typescript = {"from": point("cases.ts", ts, "ox", 0, "def"), "to": point("cases.ts", ts, "ox", 1, "use"),
                      "confidence": "exact", "doubt": None}
        golang = {"from": point("main.go", go, "a", 0, "def"), "to": point("main.go", go, "a", 1, "use"),
                  "confidence": "exact", "doubt": None}
        crossing = {"from": point("cases.ts", ts, "ox", 0, "def"), "to": point("main.go", go, "a", 1, "use"),
                    "confidence": "exact", "doubt": None}
        if mutate:
            mutate(typescript)
        run = h.make_run(h.work / name, [typescript, golang, crossing])
        h.prepare()
        h.assert_exit(h.oracle(("ts-edges", "--all", "--run", run)), 0)
        return json.loads((h.out / "denominators.json").read_bytes())

    def test_census_excludes_each_non_typescript_edge_once(self):
        wanted = dict(selected=3, out_of_scope=2, ungraded=2, graded=1, decided=1, correct=1, inadmissible=0)
        census = self.mixed_census("control")
        self.assertEqual({key: census[key] for key in wanted}, wanted)
        # A bad in-program span on an in-scope edge is inadmissible, never an exclusion.
        shutil.rmtree(self.h.out)
        spans = self.mixed_census("spans", lambda row: row["to"].update(end_byte=10_000))
        self.assertEqual((spans["out_of_scope"], spans["inadmissible"], spans["correct"]), (2, 1, 0))


if __name__ == "__main__":
    unittest.main()
