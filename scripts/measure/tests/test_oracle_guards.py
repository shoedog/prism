"""Oracle selector and guard controls that can execute without a compiler."""

from __future__ import annotations

from collections import Counter
from contextlib import redirect_stderr
import copy
import gzip
import io
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from measure import oracle, oracle_selection, receipt
from measure.edges import canonical
from measure.oracle_inputs import OracleError, digest

from . import test_oracle
from .helpers import executable
from .test_matrix_rows import CASES, read_jsonl


class WireTests(unittest.TestCase):
    def fixture(self, key):
        return read_jsonl(CASES / f"oracle-{key:03d}" / "input.jsonl")

    def exercise(self, decoder, raw, message, mutate):
        self.assertEqual(decoder(raw), raw)
        bad = copy.deepcopy(raw)
        mutate(bad)
        with self.assertRaisesRegex(ValueError, message):
            decoder(bad)

    def test_edge_wire_refuses_invalid_consumed_fields(self):
        row, = self.fixture(1)
        variants = [
            ("missing field", lambda v: v.pop("to")),
            ("confidence", lambda v: v.update(confidence="name_only")),
            ("doubt", lambda v: v.update(doubt="noise")),
            ("kill_line", lambda v: v.update(kill_line=True)),
            ("relation", lambda v: v.update(relation=" ")),
            ("invalid file", lambda v: v["from"].update(file="../cases.ts")),
            ("expected integer", lambda v: v["from"].update(start_byte="105")),
            ("invalid access", lambda v: v["from"].update(access="read")),
            ("structured path", lambda v: v["from"]["path"].update(fields=[None])),
        ]
        for reason, mutate in variants:
            with self.subTest(reason=reason):
                self.exercise(oracle_selection.edge, row, reason, mutate)

    def test_call_wire_refuses_invalid_consumed_fields(self):
        row, = self.fixture(22)
        variants = [
            ("missing field", lambda v: v.pop("drop")),
            ("function_id", lambda v: v["caller"].update(start_line=0)),
            ("function_id", lambda v: v.update(exact_target={})),
            ("invalid call record", lambda v: v.update(drop=[])),
            ("confidence", lambda v: v["resolved_targets"][0].update(confidence="nameonly")),
            ("kind", lambda v: v["resolved_targets"][0].update(kind=False)),
            ("function_id", lambda v: v["resolved_targets"][0].pop("function_id")),
        ]
        for reason, mutate in variants:
            with self.subTest(reason=reason):
                self.exercise(oracle_selection.site, row, reason, mutate)

    def test_complete_context_cancels_before_call_projection(self):
        row, = self.fixture(22)
        a = (0, row, 0)
        different = dict(row, extra="must survive")
        b = (1, different, 0)
        for order in ([a, b], [b, a]):
            self.assertEqual(oracle_selection.head_surplus([a], order, 1), [b])
        c = (0, dict(row, extra="third"), 0)
        with self.assertRaisesRegex(ValueError, "ambiguous-occurrence"):
            oracle_selection.head_surplus([c], [a, b], 1)
        self.assertEqual(oracle_selection.head_surplus([c], [b, b], 1), [b])

    def test_drop_reason_loss_is_visible_at_the_retained_site(self):
        row, = self.fixture(30)
        original = [(0, row), (1, dict(row, drop="ArgFanout"))]
        after = [(0, row)]
        key = oracle_selection.site_key(row)
        no_targets = Counter()
        _, _, before_drops = oracle_selection.project(original)
        _, _, after_drops = oracle_selection.project(after)
        delta = oracle_selection.delta_for(key, original, after, no_targets, no_targets,
                                           before_drops, after_drops)
        self.assertEqual(delta["kind"], "multi-target")
        self.assertEqual(delta["base"]["drop_reasons"], {"UnknownName": 1, "ArgFanout": 1})
        self.assertEqual(delta["head"]["drop_reasons"], {"UnknownName": 1})
        self.assertIsNone(oracle_selection.delta_for(key, original, original, no_targets, no_targets,
                                                    before_drops, before_drops))

    def test_wire_data_does_not_round_unsafe_integers(self):
        self.assertEqual(oracle_selection.representable({"extra": [2**53 - 1]}),
                         {"extra": [2**53 - 1]})
        with self.assertRaisesRegex(ValueError, "exact Node range"):
            oracle_selection.representable({"extra": [2**53]})
        self.assertEqual(oracle_selection.representable({"extra": "é"}), {"extra": "é"})
        with self.assertRaises(UnicodeEncodeError):
            oracle_selection.representable({"extra": chr(0xD800)})


class LauncherTests(unittest.TestCase):
    def setUp(self):
        from tempfile import TemporaryDirectory
        directory = TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.stage = Path(directory.name).resolve()
        self.node = self.stage / "node"
        self.guard = self.stage / "guard"
        self.marker = self.stage / "node-started"

    def launch(self, code=None, status=None, payload="{}"):
        executable(self.node, f"""
import os
from pathlib import Path
assert "NODE_OPTIONS" not in os.environ and "NODE_PATH" not in os.environ
Path({str(self.marker)!r}).touch()
print({payload!r})
raise SystemExit({code or 0})
""")
        executable(self.guard, f"""
import json
import os
import subprocess
import sys
fd = int(sys.argv[2])
assert sys.argv[3] == "8"
status = {status!r}
if status is None:
    code = subprocess.call(sys.argv[4:])
    status = dict(kind="child", launched=True, child_exit=code)
else:
    code = {code!r}
if status != "missing":
    os.write(fd, json.dumps(status).encode() + b"\\n")
raise SystemExit(code)
""")
        host = dict(PATH=str(self.stage) + os.pathsep + os.environ["PATH"],
                    NODE_OPTIONS="--require=fake", NODE_PATH="/fake")
        with patch.dict(os.environ, host, clear=True), patch.object(oracle, "DEFAULT_GUARD", self.guard):
            return oracle.launch({"operation": "selfcheck"}, self.stage)

    def test_structured_quiet_and_child_75_are_distinct(self):
        with self.assertRaises(OracleError) as quiet:
            self.launch(75, dict(kind="host_quiet", launched=False, child_exit=None))
        self.assertEqual(quiet.exception.code, 75)
        self.assertFalse(self.marker.exists())
        with self.assertRaises(OracleError) as child:
            self.launch(75)
        self.assertEqual(child.exception.code, 1)
        self.assertTrue(self.marker.exists())
        self.assertEqual(self.launch(), {})

    def test_checker_environment_allowlist_keeps_node_preload_and_paths_out(self):
        self.assertEqual(self.launch(payload='{"started":true}'), {"started": True})
        self.assertTrue(self.marker.exists())

    def test_unauthenticated_guard_and_invalid_child_protocol_refuse(self):
        self.assertEqual(self.launch(), {})
        for code, status, payload in ((75, "missing", "{}"), (76, dict(kind="rss_unavailable",
                   launched=False, child_exit=None), "{}"), (0, None, "not-json")):
            with self.subTest(code=code, status=status), redirect_stderr(io.StringIO()):
                with self.assertRaises(OracleError) as error:
                    self.launch(code, status, payload)
                self.assertEqual(error.exception.code, 1)


class OracleAdmissionTests(unittest.TestCase):
    def setUp(self):
        harness = test_oracle.OracleTests()
        harness.setUp()
        self.addCleanup(harness.doCleanups)
        self.h = harness

    def test_nonselected_run_artifact_is_still_admitted(self):
        args = self.h.load_case("oracle-022")
        artifact = args[-1] / "dfg-stats.json"
        original = artifact.read_bytes()
        artifact.write_bytes(b"{}")
        log = self.h.fake_node()
        denied = self.h.oracle(args, typescript=Path("unused"))
        self.h.assert_exit(denied, 2)
        self.assertIn("unbound measurement artifact", denied.stderr)
        self.assertFalse(log.exists())
        artifact.write_bytes(original)
        denied = self.h.oracle(args, typescript=Path("missing"))
        self.h.assert_exit(denied, 3)
        self.assertIn("checker-pin", denied.stderr)

    def test_wrong_discriminator_refuses_but_ancillary_record_is_counted(self):
        args = self.h.load_case("oracle-030")
        run = args[-1]
        record, = read_jsonl(CASES / "oracle-030/input.jsonl")
        path = run / "sites.jsonl.gz"
        receipt_path = run / "receipt.json"
        original = json.loads(receipt_path.read_bytes())

        def set_rows(rows):
            path.write_bytes(gzip.compress(test_oracle.json_lines(rows).encode(), mtime=0))
            value = copy.deepcopy(original)
            artifact = receipt.artifact(path)
            value["artifacts"][path.name] = artifact
            for command in value["commands"]:
                if command["id"] == "sites":
                    command.update(output_sha256=artifact["sha256"], bytes=artifact["bytes"])
            receipt_path.write_bytes(canonical(value))

        set_rows([record, {"record_kind": "unknown"}])
        refused = self.h.oracle(args, typescript=Path("unused"))
        self.h.assert_exit(refused, 2)
        self.assertIn("record_kind", refused.stderr)
        set_rows([record, {"record_kind": "go_level3_b1_candidate", "opaque": {"anything": 1}}])
        request = argparse_namespace(args)
        from measure.corpora import load_manifest
        _, _, _, _, _, denominators = oracle_selection.call_population(request, load_manifest(self.h.manifest))
        self.assertEqual((denominators["census"]["other_records"], denominators["census"]["dropped"]), (1, 1))

    def test_question_closure_unlisted_data_digest_and_symlink_refuse_before_node(self):
        from measure.oracle_inputs import prepare_inputs
        question = self.h.question(dict(verdict="CORRECT", property="custom", why="fixture"))
        ts = self.h.typescript()
        checked = prepare_inputs(ts, question)
        datum = question.parent / "annotation.json"
        datum.write_bytes(b'{"answer":true}')
        declaration = question.parent / "question-data.json"
        declaration.write_bytes(canonical({datum.name: digest(datum.read_bytes())}))
        updated = prepare_inputs(ts, question)
        self.assertNotEqual(checked["key"], updated["key"])
        datum.write_bytes(b'{}')
        with self.assertRaisesRegex(OracleError, "unbound question data"):
            prepare_inputs(ts, question)
        datum.unlink()
        datum.symlink_to(question)
        with self.assertRaisesRegex(OracleError, "expected a regular file"):
            prepare_inputs(ts, question)


def argparse_namespace(args):
    import argparse
    parser = argparse.ArgumentParser()
    oracle.register(parser.add_subparsers())
    return parser.parse_args(["oracle", *map(str, args), "--out", "ignored"])


class OraclePopulationTests(unittest.TestCase):
    def setUp(self):
        self.h = test_oracle.OracleTests()
        self.addCleanup(self.h.doCleanups)
        self.h.setUp()

    def test_argfollow_kind_only_gain_keeps_both_population_bags(self):
        from measure.corpora import load_manifest
        case = CASES / "oracle-022"
        self.h.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        row, = read_jsonl(case / "input.jsonl")
        target = row["resolved_targets"][0]
        target["confidence"] = "name_only"
        base = self.h.make_run(self.h.work / "base", [row], sites=True)
        target["kind"] = "argument_follow"
        head = self.h.make_run(self.h.work / "head", [row], sites=True)
        args = ("ts-calls", "--base", base, "--head", head)
        entries = load_manifest(self.h.manifest)
        default = oracle_selection.call_population(argparse_namespace(args), entries)
        self.assertEqual(default[2], [])
        select = (*args, "--question", self.h.work / "question.cjs", "--relation", "argfollow",
                  "--target-confidence", "name_only", "--target-kind", "argument_follow")
        _, _, claims, _, deltas, denominators = oracle_selection.call_population(argparse_namespace(select),
                                                                                 entries)
        request, = claims
        delta, = deltas
        self.assertEqual((request["row"], request["index"], request["arm"]), (row, 0, "head"))
        self.assertEqual(delta["kind"], "multi-target")
        self.assertEqual(denominators["selected"], 2)
        kinds = denominators["populations"]["name_only"]["kinds"]
        self.assertEqual(kinds["local_def"]["lost"], 1)
        self.assertEqual(kinds["argument_follow"]["eligible"], 1)

    def test_residual_edge_subset_refuses_unequal_original_rows(self):
        from measure.corpora import load_manifest
        case = CASES / "oracle-001"
        self.h.fresh_tree(files={"cases.ts": (case / "repo/cases.ts").read_bytes()})
        row, = read_jsonl(case / "input.jsonl")
        original = self.h.make_run(self.h.work / "base", [row])
        unequal = self.h.make_run(self.h.work / "unequal", [row, dict(row, relation="other")])
        equal = self.h.make_run(self.h.work / "equal", [row, row])
        manifest = load_manifest(self.h.manifest)
        classifier = self.h.work / "classifier.py"
        classifier.write_text("CLASSES = ['bound']\ndef classify(change, context): return 'bound'\n")
        for control, run in ((False, unequal), (True, equal)):
            diff = self.h.work / ("diff-" + run.name)
            self.h.assert_exit(self.h.kit("diff-edges", original, run, "--classifier", classifier, "--out", diff), 0)
            args = argparse_namespace(("ts-edges", "--run", run, "--changes", diff / "changes.jsonl"))
            if control:
                claims = oracle_selection.edge_population(args, manifest, None)[3]
                self.assertEqual([(c["arm"], c["row"]) for c in claims], [("head", row)])
            else:
                with self.assertRaisesRegex(ValueError, "ambiguous-occurrence"):
                    oracle_selection.edge_population(args, manifest, None)

    def test_source_drift_discards_every_staged_artifact(self):
        import platform
        args = self.h.load_case("oracle-001")
        record = json.loads((args[-1] / "receipt.json").read_bytes())
        file = Path(record["corpus"]["root"]) / "cases.ts"
        fixture = {"operation": "grade", "key": "fixture", "qualification_sha256": None}
        cached = {"runtime": {"node": "fixture", "python": platform.python_version()}, "expected": 1}
        program = """
import sys
stage = Path(sys.argv[3])
for name in ("tuples.json", "denominators.json", "verdicts.jsonl"):
    (stage / name).write_text("{}\\n")
print('{"node":"fixture"}')
"""
        with patch.object(oracle, "DEFAULT_GUARD", self.h.guard), patch.object(
            oracle, "prepare_inputs", return_value=fixture
        ), patch.object(oracle, "cached", return_value=cached):
            self.h.fake_node(program)
            control = self.h.oracle(args, typescript=Path("unused"), out=self.h.work / "stable")
            self.h.assert_exit(control, 0)
            self.h.fake_node(f"p = Path({str(file)!r})\np.write_bytes(p.read_bytes() + b' ')\n" + program)
            refused = self.h.oracle(args, typescript=Path("unused"))
            self.h.assert_exit(refused, 1)
            self.assertIn("source-drift", refused.stderr)
            self.assertEqual(list(self.h.out.iterdir()), [])
