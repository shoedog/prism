"""Round-2 review guards: single-read admission, immutable views, wording, tokens, diagnostics."""

import hashlib
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest
from unittest import mock

from measure import classifier, edges as edge_module, inputs
from measure.corpora import REPO

from .comparator_fixtures import CLASSIFIER, TRACE_CLASSIFIER, ComparatorFixture, edge, lines

PLAN = REPO / "docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit"


def sha(data):
    return hashlib.sha256(data).hexdigest()


class VersionBytesTests(ComparatorFixture):
    """Receipt admission verifies and decodes the same version.stdout bytes."""

    def bind(self, run, data):
        """Make the receipt bind `data` as version.stdout."""
        (run / "version.stdout").write_bytes(data)
        path = run / "receipt.json"
        record = json.loads(path.read_text())
        digest = {"sha256": sha(data), "bytes": len(data)}
        record["artifacts"]["version.stdout"] = digest
        record["version_command"]["stdout_sha256"] = digest["sha256"]
        record["version_command"]["stdout_bytes"] = digest["bytes"]
        path.write_text(json.dumps(record))

    def swapping(self, run, replacement):
        """Replace version.stdout just before every read after the first."""
        target = str(run / "version.stdout")
        real = Path.open
        state = {"reads": 0}

        def opening(path, mode="r", *args, **kwargs):
            if str(path) == target and "r" in mode:
                state["reads"] += 1
                if state["reads"] > 1:
                    with open(target, "wb") as stream:
                        stream.write(replacement)
            return real(path, mode, *args, **kwargs)
        return mock.patch.object(Path, "open", opening), state

    def test_a_replacement_between_reads_cannot_launder_the_version(self):
        self.arm("base", [edge()])
        head = self.arm("head", [edge()], version="fake")
        self.bind(head, b"real\n")
        self.invoke("diff-stats", expected=2, diagnostic="differs from the recorded version")
        patch, state = self.swapping(head, b"fake\n")
        with patch:
            self.invoke("diff-stats", expected=2, diagnostic="differs from the recorded version")
        self.assertGreaterEqual(state["reads"], 1)
        # Control: the same replacement mechanism with identical bytes is admitted.
        self.bind(head, b"fake\n")
        patch, state = self.swapping(head, b"fake\n")
        with patch:
            self.invoke("diff-stats")
        self.invoke("diff-stats")


class ContextViewTests(unittest.TestCase):
    TABLES = {"dfg": {"changed": [{"key": "x", "delta": 1}]},
              "call": {"changed": [{"key": "y", "delta": 2}]}}

    def context(self):
        store = mock.Mock()
        return classifier.Context([None, None], "bytes", store, self.TABLES)

    def test_every_published_view_refuses_reassignment(self):
        context = self.context()
        for name in ("counter_deltas", "call_counter_deltas", "stats_deltas", "key_mode",
                     "unchanged_keys", "all_records", "corpus_root", "base_run", "head_run",
                     "base_receipt", "head_receipt", "records", "_store", "_runs", "invented"):
            with self.subTest(name=name), self.assertRaises(AttributeError):
                setattr(context, name, {})
        with self.assertRaises(AttributeError):
            del context.counter_deltas
        bound = classifier.Bound(context)
        for name in ("counter_deltas", "stats_deltas", "records", "key_mode", "invented"):
            with self.subTest(bound=name), self.assertRaises(AttributeError):
                setattr(bound, name, ())
        # Control: reading is unchanged.
        self.assertEqual(dict(context.counter_deltas), {"x": 1})
        self.assertEqual(dict(context.call_counter_deltas), {"y": 2})
        self.assertEqual(len(context.stats_deltas), 2)
        self.assertEqual((context.key_mode, bound.key_mode), ("bytes", "bytes"))
        self.assertEqual(dict(bound.counter_deltas), {"x": 1})


class ReassignmentThroughTheCliTests(ComparatorFixture):
    def run_case(self, assignment, validate, *, stats=2, call=2):
        self.arm("base", [edge()], stats={"x": 1}, call={"y": 1})
        self.arm("head", [edge(label="sameline")], stats={"x": stats}, call={"y": call})
        self.classifier.write_text(
            CLASSIFIER + "\ndef classify(change, context):\n    try:\n        " + assignment
            + "\n    except AttributeError:\n        pass\n    return 'fixture'\n"
            + "\ndef validate(bound):\n    " + validate + "\n")
        self.next_output()

    def test_a_classifier_cannot_hide_a_delta_by_reassigning_the_view(self):
        cases = (
            ("context.counter_deltas = {}", "return ['x delta'] if bound.counter_deltas.get('x', 0) else []"),
            ("context.call_counter_deltas = {}",
             "return ['y delta'] if bound.call_counter_deltas.get('y', 0) else []"),
            ("context.stats_deltas = ()",
             "return ['row delta'] if any(r['delta'] for r in bound.stats_deltas) else []"),
        )
        for assignment, validate in cases:
            with self.subTest(assignment=assignment):
                self.run_case(assignment, validate)
                self.compare(expected=1)
                self.assertTrue(self.cert()["validation"], assignment)
        # Control: the same validators with a read-only classifier and no delta pass.
        for assignment, validate in cases:
            with self.subTest(control=assignment):
                self.run_case("pass", validate, stats=1, call=1)
                self.compare()
                self.assertEqual(self.cert()["validation"], [])


class DocumentWordingTests(unittest.TestCase):
    def test_the_seven_pin_rollout_is_reference_only_not_an_adopter_requirement(self):
        plan = (PLAN / "PLAN.md").read_text()
        decisions = (PLAN / "FORMATS-DECISIONS.md").read_text()
        formats = (PLAN / "FORMATS.md").read_text()
        m4 = next(line for line in plan.splitlines() if "The owner named lane 10 for S2" in line)
        self.assertNotIn("the comparison plan is S0 against baseline", m4)
        self.assertIn("reference only, not an adopter requirement", m4)
        self.assertIn("terminal and inventory requirements", m4)
        self.assertNotIn("needs the [exact S0 producer and rollout text]", decisions)
        section = next(line for line in decisions.splitlines()
                       if line.startswith("**Lane 10 amendment"))
        self.assertIn("reference only, not an adopter requirement", section)
        self.assertIn("terminal counts and complete inventories", section)
        erratum = next(line for line in formats.splitlines() if "**Status (2026-11-25 erratum)" in line)
        self.assertIn("seven-corpus rollout is not an adopter requirement", erratum)
        self.assertIn("terminal and inventory requirements", erratum)
        validate = next(line for line in formats.splitlines() if line.startswith("Call optional `validate"))
        self.assertNotIn("Lane 10 vetoes", validate)
        self.assertIn("An adopter of the inventory profile vetoes", validate)


class IntegerTokenTests(ComparatorFixture):
    DIGITS = b"9" * 4301

    def test_the_decoder_defers_ignored_integers_and_keeps_projected_ones(self):
        good = lines([edge()])
        bad = good[:-2] + b',"extra":' + self.DIGITS + b',"more":[' + self.DIGITS + b"]}\n"
        path = self.work / "edges.jsonl"
        path.write_bytes(bad)
        wire = next(inputs.jsonl_objects(path, numbers=inputs.NumberToken))
        self.assertIsInstance(wire["extra"], inputs.NumberToken)
        self.assertEqual(wire["extra"].text, self.DIGITS.decode())
        # Control: the default decoder still produces real numbers.
        small = self.work / "small.jsonl"
        small.write_bytes(good)
        self.assertIs(type(next(inputs.jsonl_objects(small))["to"]["line"]), int)

    def test_a_huge_ignored_integer_does_not_refuse_the_edge_and_a_projected_one_does(self):
        good = lines([edge()])
        old = getattr(sys, "get_int_max_str_digits", None)
        if old is not None:
            sys.set_int_max_str_digits(4300)
            self.addCleanup(sys.set_int_max_str_digits, old())
        ignored = good[:-2] + b',"extra":' + self.DIGITS + b"}\n"
        self.diff(ignored, ignored)
        self.assertEqual(self.cert()["base_edges"], 1)
        self.next_output()
        if old is not None:
            for field, original in (("line", b"6"), ("start_byte", b"154"), ("kill_line", None)):
                with self.subTest(field=field):
                    if original is None:
                        bad = good[:-2] + b',"kill_line":' + self.DIGITS + b"}\n"
                    else:
                        bad = good.replace(b'"%s": %s' % (field.encode(), original),
                                           b'"%s": %s' % (field.encode(), self.DIGITS))
                    self.assertNotEqual(bad, good)
                    self.diff(bad, bad, expected=2, diagnostic="edge input")
                    self.diff(good, good)
                    self.next_output()
        # Controls: ordinary projected integers and nonfinite tokens behave as before.
        kill = good[:-2] + b',"kill_line":7}\n'
        self.diff(kill, kill)
        self.next_output()
        for token in (b"NaN", b"Infinity"):
            bad = good[:-2] + b',"extra":' + token + b"}\n"
            self.diff(bad, bad, expected=2, diagnostic="edge input")
            self.next_output()


class RefusalDiagnosticTests(ComparatorFixture):
    def test_a_trace_callback_that_raises_an_output_error_is_trace_input(self):
        record = lines([{"tag": "t", "kind": "edge", "key": {"to": {"line": 99}}}])
        raising = TRACE_CLASSIFIER.replace(
            "    record = json.loads(line)\n",
            "    from measure.run import OutputError\n    raise OutputError('invalid sidecar')\n")
        self.arm("base", [edge()])
        self.arm("head", [edge()], trace=record)
        self.classifier.write_text(raising)
        self.compare(expected=2, diagnostic="trace input: head line 1")
        self.assertIn("invalid sidecar", self.stderr)
        # Control: the same trace parses with the ordinary callback.
        self.classifier.write_text(TRACE_CLASSIFIER)
        self.next_output()
        self.compare()

    def test_a_storage_error_in_the_conservation_query_is_output_or_storage(self):
        self.arm("base", [edge()])
        self.arm("head", [edge(label="sameline")])
        self.compare()
        self.next_output()

        class Failing:
            def __init__(self, database):
                self._database = database

            def execute(self, sql, *args):
                if "SELECT coalesce(sum(n)" in sql:
                    raise sqlite3.OperationalError("disk I/O error")
                return self._database.execute(sql, *args)

            def __getattr__(self, name):
                return getattr(self._database, name)

        real = edge_module.make_database
        with mock.patch.object(edge_module, "make_database", lambda path: Failing(real(path))):
            self.compare(expected=2, diagnostic="output or storage")
        self.assertIn("output or storage", self.stderr)
        self.assertNotIn("trace input", self.stderr)
        # Control: the unpatched query succeeds on the same inputs.
        self.next_output()
        self.compare()


if __name__ == "__main__":
    unittest.main()
