"""Round-3 review guards: closed records, certificate storage diagnostic, integer limit wording."""

import copy
import json
import sqlite3
import sys
import unittest
from unittest import mock

from measure import edges as edge_module
from measure.corpora import REPO

from .comparator_fixtures import ComparatorFixture, edge, lines

README = REPO / "scripts/measure/README.md"

TOP = {"schema", "kit_commit", "start", "end", "status", "binary", "corpus", "prediction",
       "gate_sha256", "gate_step_id", "options", "version_command", "commands", "guard",
       "host", "artifacts"}
# FORMATS section 2: record -> (required keys, optional keys). Optional keys are the
# additive S1 fields "validated when present"; nothing else may appear.
RECORDS = {
    (): (TOP, {"kit_dirty"}),
    ("binary",): ({"path", "sha256", "version"}, set()),
    ("corpus",): ({"key", "commit", "root", "pin_check", "manifest_sha256", "tree_sha256",
                   "input_sha256", "end_input_sha256", "input_state_sha256",
                   "end_input_state_sha256"}, set()),
    ("options",): ({"bytes", "sites", "cache", "trace_env"}, set()),
    ("version_command",): ({"argv", "env", "exit", "guard_status", "start", "end",
                            "stdout_sha256", "stdout_bytes", "stderr"}, set()),
    ("version_command", "guard_status"): ({"kind", "launched", "child_exit"}, set()),
    ("guard",): ({"path", "limit_gb"}, {"sha256", "quiet_helper_sha256"}),
    ("host",): ({"platform", "cpus", "env"}, set()),
}
COMMAND = {"id", "argv", "env", "output", "output_sha256", "bytes", "stderr", "exit",
           "guard_status", "start", "end"}


def walk(record, path):
    for part in path:
        record = record[part]
    return record


class ClosedReceiptRecordTests(ComparatorFixture):
    """A receipt record with a key FORMATS does not define, or without one it does, is refused."""

    def setUp(self):
        super().setUp()
        self.arm("base", [edge()])
        self.head = self.arm("head", [edge()])
        self.pristine = json.loads((self.head / "receipt.json").read_text())

    def attempt(self, mutate, expected, diagnostic="run binding"):
        record = copy.deepcopy(self.pristine)
        mutate(record)
        self.save(self.head / "receipt.json", record)
        self.invoke("diff-stats", expected=expected,
                    diagnostic=diagnostic if expected else None)

    def records(self):
        found = dict(RECORDS)
        for index in range(len(self.pristine["commands"])):
            found[("commands", index)] = (COMMAND, set())
            found[("commands", index, "guard_status")] = ({"kind", "launched", "child_exit"}, set())
        for name in self.pristine["artifacts"]:
            found[("artifacts", name)] = ({"sha256", "bytes"}, set())
        return found

    def test_every_record_refuses_an_undeclared_key_and_a_missing_required_key(self):
        for path, (required, optional) in self.records().items():
            with self.subTest(record=path, mutation="extra key"):
                self.attempt(lambda record: walk(record, path).update(extra=True), 2)
            for key in sorted(required):
                with self.subTest(record=path, mutation="missing", key=key):
                    self.attempt(lambda record: walk(record, path).pop(key), 2)
        self.attempt(lambda record: None, 0)

    def test_declared_optional_keys_may_be_absent_or_present(self):
        for path, (required, optional) in self.records().items():
            for key in sorted(optional):
                with self.subTest(record=path, optional=key):
                    self.attempt(lambda record: walk(record, path).pop(key, None), 0)
        self.attempt(lambda record: record.update(kit_dirty=True), 0)
        self.attempt(lambda record: record["guard"].update(
            sha256="3" * 64, quiet_helper_sha256=None), 0)

    def test_the_version_artifact_record_is_exactly_digest_and_size(self):
        # The reviewed shape: a correct digest and size plus one more member.
        self.attempt(lambda record: record["artifacts"]["version.stdout"].update(extra=True), 2)
        self.attempt(lambda record: record["artifacts"]["version.stdout"].update(
            extra={"nested": [1]}), 2)
        self.attempt(lambda record: record["artifacts"]["version.stdout"].pop("bytes"), 2)
        self.attempt(lambda record: record["artifacts"]["version.stdout"].pop("sha256"), 2)
        # Control: the declared pair, re-serialized, is admitted.
        self.attempt(lambda record: record["artifacts"].update(
            {"version.stdout": dict(reversed(list(record["artifacts"]["version.stdout"].items())))}),
            0)

    def test_the_version_stdout_is_still_read_once(self):
        reads = []
        real = type(self.head).read_bytes

        def counting(path):
            if path.name == "version.stdout":
                reads.append(str(path))
            return real(path)

        with mock.patch.object(type(self.head), "read_bytes", counting):
            self.invoke("diff-stats")
        self.assertEqual(len(reads), 2)  # one per arm

    def test_a_gate_prediction_binding_is_exactly_path_and_digest(self):
        def bind(record):
            record["prediction"] = {"path": "p.json", "sha256": "4" * 64}

        self.attempt(bind, 0)
        for mutation in (lambda p: p.update(extra=1), lambda p: p.pop("path"),
                         lambda p: p.pop("sha256")):
            self.attempt(lambda record: (bind(record), mutation(record["prediction"])), 2)


class EdgeWireTests(ComparatorFixture):
    """Edge lines keep their closed required members and still ignore additional ones."""

    def line(self, mutate):
        wire = json.loads(lines([edge()]))
        mutate(wire)
        return json.dumps(wire).encode() + b"\n"

    def test_required_members_are_required_and_additional_members_are_ignored(self):
        def drop(*path):
            def go(wire):
                for part in path[:-1]:
                    wire = wire[part]
                wire.pop(path[-1])
            return go

        for path in (("from",), ("to",), ("confidence",), ("doubt",), ("from", "file"),
                     ("from", "line"), ("from", "path"), ("from", "access"),
                     ("to", "path", "base"), ("to", "path", "fields")):
            with self.subTest(missing=path):
                bad = self.line(drop(*path))
                self.diff(bad, bad, expected=2, diagnostic="edge input")
                self.next_output()

        def extras(wire):
            wire.update(owner="f", more={"a": [1]})
            for side in ("from", "to"):
                wire[side].update(owner="f", more=1)
                wire[side]["path"].update(more=1)

        # Control: additional members on the line, endpoints and paths are ignored.
        extra = self.line(extras)
        self.diff(extra, extra)
        self.assertEqual(self.cert()["base_edges"], 1)
        self.next_output()
        self.diff(extra, lines([edge()]))
        self.assertEqual(self.cert()["changed_keys"], 0)


class CertificateStorageTests(ComparatorFixture):
    def test_a_storage_error_in_the_certificate_accounting_is_output_or_storage(self):
        self.arm("base", [edge()])
        self.arm("head", [edge(label="sameline")])
        self.compare()
        self.next_output()

        class Failing:
            def __init__(self, database):
                self._database = database

            def execute(self, sql, *args):
                if "coalesce(sum(count),0) FROM changes" in sql:
                    raise sqlite3.OperationalError("disk I/O error")
                return self._database.execute(sql, *args)

            def __getattr__(self, name):
                return getattr(self._database, name)

        real = edge_module.make_database
        with mock.patch.object(edge_module, "make_database", lambda path: Failing(real(path))):
            self.compare(expected=2, diagnostic="output or storage")
        self.assertIn("output or storage", self.stderr)
        self.assertIn("disk I/O error", self.stderr)
        self.assertFalse((self.out / "certificate.json").exists())
        # Control: the unpatched query succeeds on the same inputs.
        self.next_output()
        self.compare()
        self.assertEqual(self.cert()["result"], "pass")


class ProjectedIntegerWordingTests(unittest.TestCase):
    def accept_list(self):
        text = README.read_text()
        start = text.index("- **Transport and edges.**")
        return text[start:text.index("- **Counters.**")]

    def test_the_accept_list_states_the_projected_integer_digit_limit(self):
        section = " ".join(self.accept_list().split())
        self.assertIn("4,300", section)
        self.assertIn("projected integer", section)
        self.assertIn("`edge input`", section)
        # The caveat names the interpreters without the conversion limit.
        self.assertIn("3.9", section)
        self.assertIn("no limit", section)

    def test_the_limit_is_described_as_the_interpreter_s_not_the_kit_s(self):
        section = " ".join(self.accept_list().split())
        self.assertIn("integer string conversion limit", section)


if __name__ == "__main__":
    unittest.main()
