"""Offline, signed S1-shaped receipts for comparator tests."""

from contextlib import redirect_stderr, redirect_stdout
import gzip
import io
import json
from pathlib import Path
import tempfile
import unittest

from measure import receipt
from measure.cli import main
from measure.inputs import bind_tree

from .helpers import REPO, write_tree


CLASSIFIER = 'CLASSES = ["fixture"]\ndef classify(change, context): return "fixture"\n'
TRACE_CLASSIFIER = CLASSIFIER + """
import json
def parse_trace(line, arm):
    record = json.loads(line)
    return None if record is None else {"arm": arm, **record}
"""


def edge(line=6, sb=154, label=None, **attributes):
    def endpoint(line, start, access):
        return dict(file="a.ts", line=line, path=dict(base="x", fields=[]),
                    access=access, start_byte=start, end_byte=start + 1)
    return {"from": endpoint(5, 132, "def"), "to": endpoint(line, sb, "use"),
            "confidence": "exact" if label is None else "nameonly", "doubt": label,
            **attributes}


def lines(objects):
    return b"".join(json.dumps(item).encode() + b"\n" for item in objects)


def read_rows(path):
    return [json.loads(line) for line in path.read_bytes().split(b"\n") if line]


class ComparatorFixture(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix=".comparator-", dir=REPO)
        self.addCleanup(directory.cleanup)
        self.work = Path(directory.name)
        self.out = self.work / "diff"
        self.classifier = self.work / "classifier.py"
        self.classifier.write_text(CLASSIFIER, encoding="utf-8")
        self.manifest = self.work / "corpora.json"
        self.key = "Z"

    def arm(self, name, edges=(), *, transport=None, trace=None, stats=None, call=None,
            options=None, trace_env=None, source="export {};\n", version="9d1a7d06f653"):
        run = self.work / name
        snapshot = run / "snapshot"
        source_files = source if isinstance(source, dict) else {"a.ts": source}
        write_tree(snapshot, source_files)
        binding = bind_tree(snapshot)
        opts = dict(bytes=True, sites=False, cache="no-cache", trace_env=(
            {"PRISM_TEST_TRACE": "1"} if trace is not None else {}))
        opts.update(options or {})
        if trace_env is not None:
            opts["trace_env"] = trace_env
        trace = trace or b""
        binary = str(self.work / ("prism-" + name))
        stamp = dict(start="2026-11-25T00:00:00Z", end="2026-11-25T00:00:00Z")
        success = dict(exit=0, guard_status={"kind": "child", "launched": True, "child_exit": 0},
                       **stamp)
        write_tree(run, {"version.stdout": version + "\n", "version.stderr": ""})
        artifacts = {file: receipt.artifact(run / file) for file in ("version.stdout", "version.stderr")}
        version_cmd = dict(argv=[binary, "--version"], env={}, stderr="version.stderr",
                           stdout_sha256=artifacts["version.stdout"]["sha256"],
                           stdout_bytes=artifacts["version.stdout"]["bytes"], **success)
        commands = []
        for role, argv, env, out, err in receipt.analysis_commands(binary, snapshot, opts):
            if role == "edges":
                if transport is None:
                    if not isinstance(edges, bytes):
                        edges = lines(edges)
                    transport = gzip.compress(edges, mtime=0)
                write_tree(run, {out: transport, err: gzip.compress(trace, mtime=0)
                                if err.endswith(".gz") else trace})
            else:
                value = {"dfg-stats": stats, "call-stats": call}.get(role)
                if not isinstance(value, bytes):
                    value = json.dumps(value if value is not None else {})
                write_tree(run, {out: gzip.compress(b"", mtime=0) if role == "sites" else value, err: ""})
            artifacts[out], artifacts[err] = (receipt.artifact(run / file) for file in (out, err))
            commands.append(dict(id=role, argv=argv, env=env, output=out, stderr=err,
                                 output_sha256=artifacts[out]["sha256"], bytes=artifacts[out]["bytes"],
                                 **success))
        entry = dict(name="Fixture", kind="tree", url=None, commit=None, root=".", history=False,
                     local=["fixture"], manifest_sha256=None, cloud_asset=None,
                     tree_sha256=binding.sha256)
        if not self.manifest.exists():
            self.save(self.manifest, {"schema": "prism.measure.corpora/1", "corpora": {self.key: entry}})
        record = dict(schema=receipt.SCHEMA, kit_commit="1" * 40, status="ok", **stamp,
                      binary=dict(path=binary, sha256="2" * 64, version=version),
                      corpus=dict(key=self.key, root=str(snapshot), commit=None, pin_check="manifest",
                                  manifest_sha256=None, tree_sha256=binding.sha256,
                                  input_sha256=binding.sha256, end_input_sha256=binding.sha256,
                                  input_state_sha256=binding.state_sha256,
                                  end_input_state_sha256=binding.state_sha256),
                      prediction=None, gate_sha256=None, gate_step_id=None, options=opts,
                      version_command=version_cmd, commands=commands, artifacts=artifacts,
                      guard=dict(path="guard", limit_gb=8), host=dict(platform="test", cpus=1,
                                                                    env={"LC_ALL": "C"}))
        self.save(run / "receipt.json", record)
        return run

    @staticmethod
    def save(path, data):
        path.write_text(json.dumps(data), encoding="utf-8")

    def invoke(self, command, *args, expected=0, diagnostic=None):
        stdout, stderr = io.StringIO(), io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            try:
                code = main(["--manifest", str(self.manifest), command,
                             str(self.work / "base"), str(self.work / "head"), *map(str, args)])
            except SystemExit as error:
                code = error.code
        self.stderr = stderr.getvalue()
        self.assertEqual(code, expected, (stdout.getvalue(), stderr.getvalue()))
        if diagnostic:
            self.assertIn(diagnostic, stdout.getvalue() + stderr.getvalue())
        return stdout.getvalue()

    def compare(self, *args, **options):
        return self.invoke("diff-edges", "--classifier", self.classifier, "--out", self.out, *args, **options)

    def diff(self, base, head, *args, **options):
        self.arm("base", base)
        self.arm("head", head)
        return self.compare(*args, **options)

    def cert(self):
        return json.loads((self.out / "certificate.json").read_bytes())

    def changes(self):
        return read_rows(self.out / "changes.jsonl")

    def next_output(self):
        self.out = self.out.with_name(self.out.name + "-next")
