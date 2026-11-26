"""Round-1 review guards: byte binding, view isolation, ordering and conservation."""

from collections import Counter
import gzip
import json
import os
import sqlite3
from unittest import mock

from measure import diff_edges, diff_stats, edges as edge_module

from .comparator_fixtures import CLASSIFIER, TRACE_CLASSIFIER, ComparatorFixture, edge, lines


class AfterAdmission(ComparatorFixture):
    def replace_after_admission(self, module, run, name, data):
        real = module.admit_pair

        def admit(base, head, entries):
            runs = real(base, head, entries)
            (self.work / run / name).write_bytes(data)
            return runs
        return mock.patch.object(module, "admit_pair", admit)


class ByteBindingTests(AfterAdmission):
    def test_counter_bytes_are_the_admitted_bytes(self):
        allow = self.work / "allow.txt"
        allow.write_bytes(b"")
        for name, call in (("dfg-stats.json", None), ("call-stats.json", ({"x": 1}, {"x": 2}))):
            with self.subTest(name=name):
                if call is None:
                    self.arm("base", stats={"x": 1})
                    self.arm("head", stats={"x": 2})
                else:
                    self.arm("base", call=call[0])
                    self.arm("head", call=call[1])
                base = (self.work / "base" / name).read_bytes()
                head = (self.work / "head" / name).read_bytes()
                self.invoke("diff-stats", "--allow", allow, expected=1)
                with self.replace_after_admission(diff_stats, "head", name, head):
                    self.invoke("diff-stats", "--allow", allow, expected=1)
                with self.replace_after_admission(diff_stats, "head", name, base):
                    self.invoke("diff-stats", "--allow", allow, expected=2,
                                diagnostic="differs from the receipt")
                (self.work / "head" / name).write_bytes(head)

    def test_edge_bytes_are_the_admitted_bytes(self):
        self.arm("base", [edge()])
        self.arm("head", [edge(label="sameline")])
        name = "edges.jsonl.gz"
        base = (self.work / "base" / name).read_bytes()
        head = (self.work / "head" / name).read_bytes()
        with self.replace_after_admission(diff_edges, "head", name, head):
            self.compare()
        self.next_output()
        with self.replace_after_admission(diff_edges, "head", name, base):
            self.compare(expected=2, diagnostic="differs from the receipt")
        self.assertFalse((self.out / "certificate.json").exists())
        (self.work / "head" / name).write_bytes(head)
        self.next_output()
        self.compare()
        self.assertEqual(self.cert()["by_kind"]["label_moved"], 1)

    def test_trace_bytes_are_the_admitted_bytes(self):
        self.classifier.write_text(TRACE_CLASSIFIER)
        record = lines([{"tag": "t", "kind": "edge", "key": {"to": {"line": 99}}}])
        self.arm("base", [edge()])
        self.arm("head", [edge()], trace=record)
        name = "edges.stderr.gz"
        head = (self.work / "head" / name).read_bytes()
        with self.replace_after_admission(diff_edges, "head", name, head):
            self.compare()
        self.next_output()
        with self.replace_after_admission(diff_edges, "head", name, gzip.compress(b"", mtime=0)):
            self.compare(expected=2, diagnostic="differs from the receipt")

    def test_trailing_bytes_after_a_gzip_member_are_hashed(self):
        self.arm("base", [edge()])
        self.arm("head", [edge()])
        name = "edges.jsonl.gz"
        original = (self.work / "head" / name).read_bytes()
        with self.replace_after_admission(diff_edges, "head", name, original + b"\0\0\0"):
            self.compare(expected=2, diagnostic="differs from the receipt")
        (self.work / "head" / name).write_bytes(original)
        self.next_output()
        with self.replace_after_admission(diff_edges, "head", name, original):
            self.compare()


class ViewIsolationTests(ComparatorFixture):
    VALIDATE = ("\ndef validate(bound):\n"
                "    return ['x delta'] if bound.counter_deltas.get('x', 0) else []\n")

    def run_pair(self, classifier, stats=2):
        self.arm("base", [edge()], stats={"x": 1})
        self.arm("head", [edge(label="sameline")], stats={"x": stats})
        self.classifier.write_text(classifier)
        self.next_output()

    def test_classifier_cannot_consume_a_counter_delta(self):
        self.run_pair(CLASSIFIER + self.VALIDATE)
        self.compare(expected=1)
        self.assertEqual(self.cert()["validation"], ["x delta"])
        self.run_pair(CLASSIFIER + "\ndef classify(change, context):\n"
                      "    context.counter_deltas.pop('x', 0)\n    return 'fixture'\n" + self.VALIDATE)
        self.compare(expected=1)
        self.assertEqual(self.cert()["validation"], ["x delta"])
        self.run_pair(CLASSIFIER + self.VALIDATE, stats=1)
        self.compare()

    def test_classifier_cannot_rewrite_the_stats_rows(self):
        validate = ("\ndef validate(bound):\n"
                    "    return ['row delta'] if any(r['delta'] for r in bound.stats_deltas) else []\n")
        self.run_pair(CLASSIFIER + "\ndef classify(change, context):\n"
                      "    context.stats_deltas[0]['delta'] = 0\n    return 'fixture'\n" + validate)
        self.compare(expected=1)
        self.assertEqual(self.cert()["validation"], ["row delta"])
        self.run_pair(CLASSIFIER + "\ndef classify(change, context):\n"
                      "    context.stats_deltas.clear()\n    return 'fixture'\n" + validate)
        self.compare(expected=1)
        self.assertEqual(self.cert()["validation"], ["row delta"])
        self.run_pair(CLASSIFIER + validate, stats=1)
        self.compare()

    def test_readers_of_the_views_keep_their_behaviour(self):
        self.run_pair(CLASSIFIER + "\ndef classify(change, context):\n"
                      "    return 'fixture' if dict(context.counter_deltas) == {'x': 1} and "
                      "list(context.call_counter_deltas) == [] and "
                      "len(context.stats_deltas) == 1 else None\n")
        self.compare()


class MoveOrderTests(ComparatorFixture):
    def transitions(self, base_labels, head_labels):
        self.diff([edge()] * len(base_labels), [edge(label=name) for name in head_labels])
        return [(row["id"], row["count"], [m["doubt"] for m in row["head_bag"]],
                 [m["doubt"] for m in row["base_bag"]]) for row in self.changes()]

    def test_moves_sort_by_both_serialised_bags(self):
        rows = self.transitions("aaa", ["bbb", "ccc", "ccc"])
        self.assertEqual(rows, [(1, 2, ["ccc", "ccc"], [None, None]),
                                (2, 1, ["bbb"], [None])])

    def test_equal_sized_moves_keep_their_order(self):
        rows = self.transitions("aaa", ["bbb", "bbb", "ccc"])
        self.assertEqual(rows, [(1, 2, ["bbb", "bbb"], [None, None]),
                                (2, 1, ["ccc"], [None])])


class IgnoredNumberTests(ComparatorFixture):
    def test_finite_numbers_in_ignored_members_are_not_binary64(self):
        good = lines([edge()])
        for token in (b"1e309", b"-1e999999999999999999999999", b"[1e400,{\"k\":2.5e700}]"):
            with self.subTest(token=token):
                bad = good[:-2] + b',"extra":' + token + b"}\n"
                self.diff(bad, bad)
                self.assertEqual(self.cert()["base_edges"], 1)
                self.next_output()

    def test_projected_fields_and_real_nonfinite_tokens_still_refuse(self):
        good = lines([edge()])
        for field, token in (("line", b"1e309"), ("start_byte", b"1.5"), ("end_byte", b"1e309")):
            with self.subTest(field=field):
                original = {"line": b"6", "start_byte": b"154", "end_byte": b"155"}[field]
                bad = good.replace(b'"%s": %s' % (field.encode(), original),
                                   b'"%s": %s' % (field.encode(), token))
                self.assertNotEqual(bad, good)
                self.diff(bad, bad, expected=2, diagnostic="edge input")
                self.diff(good, good)
                self.next_output()
        for token in (b"NaN", b"Infinity", b"-Infinity"):
            bad = good[:-2] + b',"extra":' + token + b"}\n"
            self.diff(bad, bad, expected=2, diagnostic="edge input")
            self.diff(good, good)
            self.next_output()


class RefusalClassTests(ComparatorFixture):
    def test_huge_exponent_counter_token_is_a_counter_input_refusal(self):
        self.arm("base", stats={"a": 1})
        for token in (b"1e999999999999999999999999", b"-1E-999999999999999999999999",
                      b"1.5e999999999999999999999999"):
            with self.subTest(token=token):
                self.arm("head", stats=b'{"a":' + token + b"}")
                self.invoke("diff-stats", expected=2, diagnostic="counter input")
        self.arm("head", stats={"a": 2})
        self.invoke("diff-stats", diagnostic="a: 1 -> 2")

    def test_storage_failure_in_trace_ingestion_is_not_trace_input(self):
        self.classifier.write_text(TRACE_CLASSIFIER)
        record = lines([{"tag": "t", "kind": "edge", "key": {"to": {"line": 99}}}])
        self.arm("base", [edge()])
        self.arm("head", [edge()], trace=record)
        self.compare()
        self.next_output()

        class Failing:
            def __init__(self, database):
                self._database = database

            def execute(self, sql, *args):
                if sql.lstrip().startswith("INSERT INTO records"):
                    raise sqlite3.OperationalError("database or disk is full")
                return self._database.execute(sql, *args)

            def __getattr__(self, name):
                return getattr(self._database, name)

        real = edge_module.make_database
        with mock.patch.object(edge_module, "make_database", lambda path: Failing(real(path))):
            self.compare(expected=2, diagnostic="output or storage")
        self.assertIn("output or storage", self.stderr)
        self.assertNotIn("trace input", self.stderr)


class ConservationTests(ComparatorFixture):
    def failed(self, patch):
        with patch:
            self.diff([edge()] * 2, [edge(label="sameline")] * 2, expected=1,
                      diagnostic="conservation failure")
        self.assertEqual(self.cert()["result"], "fail")
        self.assertIn("conservation failure in edge partition", self.cert()["failures"])
        self.next_output()
        self.diff([edge()] * 2, [edge(label="sameline")] * 2)

    def test_residual_that_drops_an_occurrence_fails_the_certificate(self):
        real = edge_module.residual_rows

        def lossy(base, head):
            common, rows = real(base, head)
            kind, left, right = rows[0]
            left = Counter(left)
            left[next(iter(left))] -= 1
            return common, [(kind, +left, right), *rows[1:]]
        self.failed(mock.patch.object(edge_module, "residual_rows", lossy))

    def test_published_bags_are_rebuilt_from_stored_rows(self):
        real = edge_module.bag_bytes

        def short(counts):
            members = [m for m in sorted(counts) for _ in range(counts[m])]
            return b"[" + b",".join(members[:-1] if len(members) > 1 else members) + b"]"
        self.assertEqual(real(Counter({b"a": 2})), b"[a,a]")
        self.failed(mock.patch.object(edge_module, "bag_bytes", short))

    def test_arm_table_totals_must_equal_decoded_totals(self):
        real = edge_module.ingest

        def miscount(*args):
            mode, discarded, totals = real(*args)
            return mode, discarded, [totals[0] + 1, totals[1]]
        self.failed(mock.patch.object(edge_module, "ingest", miscount))


class SourceViewTests(ComparatorFixture):
    def classifier_with(self, expression):
        self.classifier.write_text(CLASSIFIER + "\ndef classify(change, context):\n"
                                   f"    return 'fixture' if {expression} else None\n")

    def drift(self, arm):
        (self.work / arm / "snapshot/a.ts").write_text("changed")

    def test_run_directories_verify_only_their_own_arm(self):
        for arm, expression in (("base", "context.base_run.is_dir()"),
                                ("head", "context.head_run.is_dir()")):
            other = "head" if arm == "base" else "base"
            with self.subTest(arm=arm):
                def fresh():
                    self.arm("base", [edge()])
                    self.arm("head")
                fresh()
                self.classifier_with(expression)
                self.compare()
                self.next_output()
                self.drift(other)
                self.compare()
                self.next_output()
                fresh()
                self.drift(arm)
                self.compare(expected=2, diagnostic="source binding: " + arm)
                self.next_output()

    def test_a_swallowed_source_error_is_still_raised_after_validation(self):
        snapshot = self.work / "head" / "snapshot/a.ts"
        self.arm("base", [edge()])
        self.arm("head")
        body = f"""
from pathlib import Path
def classify(change, context):
    path = Path({str(snapshot)!r})
    DRIFT
    try:
        context.corpus_root
    except Exception:
        pass
    path.write_text("export {{}};\\n")
    return "fixture"
"""
        self.classifier.write_text(CLASSIFIER + body.replace("DRIFT", "path.write_text('changed')"))
        self.compare(expected=2, diagnostic="source binding: head")
        self.next_output()
        self.classifier.write_text(CLASSIFIER + body.replace("DRIFT", "pass"))
        self.compare()

    def test_relative_snapshot_path_is_refused_even_when_it_resolves(self):
        self.arm("base", [edge()])
        head = self.arm("head")
        self.classifier_with("context.corpus_root.is_dir()")
        self.compare()
        self.next_output()
        text = (head / "receipt.json").read_text()
        (head / "receipt.json").write_text(text.replace(str(head / "snapshot"), "snapshot"))
        previous = os.getcwd()
        os.chdir(head)
        try:
            self.compare(expected=2, diagnostic="expected an absolute snapshot path")
        finally:
            os.chdir(previous)
