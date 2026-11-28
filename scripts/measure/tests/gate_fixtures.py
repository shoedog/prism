"""Hermetic fixtures for the §6 CLI and an independent controller decision pin."""

from __future__ import annotations

from contextlib import ExitStack, redirect_stderr, redirect_stdout
from copy import deepcopy
from hashlib import sha256
import importlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch

from .comparator_fixtures import CLASSIFIER, edge
from .helpers import DEFAULT_VERSION, Workbench, executable, sha256_of, write_tree

PREDICTION = "Predict zero unattributed changes.\n"
QUIET = {"kind": "host_quiet", "launched": False, "child_exit": None}
REPORT_SECTIONS = (
    "Receipt", "Prediction", "Population and denominators", "Classes and certificate",
    "Verdicts", "Controls", "Limits", "Result",
)
MANIFEST_FIELDS = (
    "kind", "url", "commit", "root", "history", "manifest_sha256", "tree_sha256", "cloud_asset",
)


def require_gate(module="gate"):
    importlib.import_module(f"measure.{module}")


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def digest(data):
    return sha256(data).hexdigest()


def save_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value) + b"\n")


def read_json(path):
    return json.loads(path.read_bytes())


def jsonl(rows):
    return b"".join(canonical(row) + b"\n" for row in rows)


def artifact(path):
    raw = path.read_bytes()
    return {"sha256": digest(raw), "bytes": len(raw)}


def section(report, name):
    return report.split(f"## {name}\n", 1)[1].split("\n## ", 1)[0]


class GateFixture:
    def make_corpus(self, **kwargs):
        self.env.update(HOME="/nonexistent", GIT_CONFIG_GLOBAL=os.devnull)
        super().make_corpus(**kwargs)

    def setUp(self):
        super().setUp()
        old_guard = self.guard
        guard_dir = self.work / "guard-bin"
        guard_dir.mkdir()
        self.guard = old_guard.rename(guard_dir / "guard")
        self.quiet_helper = self.work / "hostquiet/host_quiet.py"
        write_tree(self.work, {"hostquiet/host_quiet.py": "# fixture helper\n"})
        self.prediction = self.work / "prediction.md"
        self.prediction.write_bytes(PREDICTION.encode("utf-8"))
        self.gate_path = self.work / "gate.json"
        self.classifier = self.work / "classifier.py"
        self.classifier.write_text(CLASSIFIER, encoding="utf-8")
        self.env.update(PRISM_MEASURE_SELFCHECK_DIR=str(self.work / "selfcheck"),
                        PYTHONDONTWRITEBYTECODE="1")
        self.guard_events = self.work / "guard.jsonl"
        self.make_guard()

    def make_guard(self, responses=None, *, hook=""):
        return executable(self.guard, f"""
import json
import os
from pathlib import Path
import subprocess
import sys
assert sys.argv[1] == "--status-fd"
fd = int(sys.argv[2])
assert int(sys.argv[3]) > 0
argv = sys.argv[4:]
args = argv[1:]
command = ("node" if argv[0] == "node" else "version" if args == ["--version"]
           else "review" if "--diff" in args else "edges" if "--edges" in args
           else "sites" if "--dump-sites" in args else args[2])
with open({str(self.work / "guard.jsonl")!r}, "a", encoding="utf-8") as stream:
    print(json.dumps(dict(command=command, argv=argv)), file=stream)
exec({hook!r})
responses = {responses or {}!r}
if command in responses:
    code, status = responses[command]
else:
    code = subprocess.call(argv)
    code = 128 - code if code < 0 else code
    status = dict(kind="child", launched=True, child_exit=code)
if status is not None:
    os.write(fd, (json.dumps(status) + chr(10)).encode())
sys.exit(code)
""")

    def local_command(self, args, *, command=None):
        from measure import cli, oracle, run

        stdout, stderr = io.StringIO(), io.StringIO()
        with ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, self.env, clear=True))
            stack.enter_context(patch.object(run, "DEFAULT_GUARD", self.guard))
            stack.enter_context(patch.object(oracle, "DEFAULT_GUARD", self.guard))
            # S5 may import the guard constant by value. The consumer and the
            # existing launchers must observe the same executable and helper.
            for name, module in tuple(sys.modules.items()):
                if (name == "measure.gate" or name.startswith("measure.gate_")) and hasattr(
                        module, "DEFAULT_GUARD"):
                    stack.enter_context(patch.object(module, "DEFAULT_GUARD", self.guard))
            stack.enter_context(redirect_stdout(stdout))
            stack.enter_context(redirect_stderr(stderr))
            try:
                code = (command or cli.main)(list(map(str, args)))
            except SystemExit as error:
                code = error.code
        return subprocess.CompletedProcess(args, code, stdout.getvalue(), stderr.getvalue())

    def plan(self, steps, **binaries):
        return dict(schema="prism.measure.gate/1", lane="test-lane", prediction=self.prediction.name,
                    binaries={name: {"path": str(path), "expect_version": DEFAULT_VERSION}
                              for name, path in binaries.items()}, steps=steps)

    def projection(self, plan, *, overrides=None):
        """Expected §6 projection. The CLI gets only the resulting independent pin."""
        from measure import oracle_inputs

        steps, roles, used = [], {}, set()
        for step in plan["steps"]:
            ident, kind = step["id"], step["do"]
            steps.append({k: v for k, v in step.items()
                          if k not in {"reuse", "clones", "memguard", "typescript"}})
            if kind.startswith("corpora-") or kind == "run":
                manifest = read_json(self.manifest)
                manifest["corpora"] = {key: {field: entry[field] for field in MANIFEST_FIELDS}
                                       for key, entry in manifest["corpora"].items()}
                roles[f"{ident}/manifest"] = digest(canonical(manifest))
            if kind in {"run", "review-diff", "oracle ts-edges", "oracle ts-calls"}:
                guard = self.work / step.get("memguard", self.guard)
                helper = guard.parent.parent / "hostquiet/host_quiet.py"
                roles[f"{ident}/memguard"] = sha256_of(guard)
                roles[f"{ident}/quiet_helper"] = sha256_of(helper)
            for field in ("classifier", "allow", "population"):
                if field in step:
                    roles[f"{ident}/{field}"] = sha256_of(self.work / step[field])
            for group, entries in step.get("classifier_files", {}).items():
                for name, entry in entries.items():
                    roles[f"{ident}/classifier_files/{group}/{name}"] = sha256_of(self.work / entry["path"])
            if kind.startswith("oracle "):
                typescript = self.work / step["typescript"]
                question = self.work / step["question"] if "question" in step else None
                with patch.dict(os.environ, self.env, clear=True):
                    components = oracle_inputs.prepare_inputs(typescript, question)["components"]
                roles[f"{ident}/question"] = components["question_sha256"]
                roles[f"{ident}/typescript"] = components["checker_tree_sha256"]
            if kind == "run":
                used.add(step["binary"])
            if kind == "review-diff":
                used.update(arm["binary"] for arm in step["arms"])
        binaries = {
            name: {"sha256": sha256_of(self.work / (overrides or {}).get(name, entry["path"])),
                   "expect_version": entry["expect_version"]}
            for name, entry in plan["binaries"].items() if name in used
        }
        return dict(schema=plan["schema"], lane=plan["lane"],
                    prediction_sha256=sha256_of(self.work / plan["prediction"]),
                    binaries=binaries, steps=steps, files=roles)

    def pin(self, plan, **options):
        return digest(canonical(self.projection(plan, **options)))

    def gate(self, plan, *, expected=0, pin=None, retrospective=False, overrides=None, roots=None,
             arguments=()):
        save_json(self.gate_path, plan)
        argv = ["--manifest", self.manifest, "gate", self.gate_path, "--out", self.out]
        for root in roots if roots is not None else [self.work]:
            argv.extend(("--corpora-root", root))
        for name, path in (overrides or {}).items():
            argv.extend(("--binary", f"{name}={path}"))
        if pin is not None:
            argv.extend(("--decision-pin", pin))
        if retrospective:
            argv.append("--retrospective")
        result = self.local_command([*argv, *arguments])
        self.assert_exit(result, expected)
        self.diagnostic = result.stdout + result.stderr
        return result

    def result(self):
        return read_json(self.out / "gate.json")

    def step(self, ident):
        return next(step for step in self.result()["steps"] if step["id"] == ident)

    def report_bytes(self):
        return (self.out / "REPORT.md").read_bytes()

    def report(self):
        return self.report_bytes().decode("utf-8")

    def record(self, ident):
        return read_json(self.out / ident / "step-receipt.json")

    def next_output(self, name):
        self.out = self.work / name
        self.log.unlink(missing_ok=True)
        self.guard_events.unlink(missing_ok=True)

    def assert_no_launch(self):
        self.assertEqual(self.fake_log(), [])
        self.assertFalse(self.guard_events.exists())
        self.assertFalse(list(self.out.glob("*/step-receipt.json")))


class GateWorkbench(GateFixture, Workbench):
    def make_fake(self):
        self.product("base")

    def product(self, name, *, edges=None, dfg=None, call=None, sites=(), version=DEFAULT_VERSION,
                actions=None, trace_rows=None):
        path = self.work / f"prism-{name}"
        if name == "base":
            self.fake = path
        outputs = {
            "edges": jsonl([edge()] if edges is None else edges),
            "dfg-stats": canonical({"count": 1} if dfg is None else dfg) + b"\n",
            "call-stats": canonical({"count": 1} if call is None else call) + b"\n",
            "sites": jsonl(sites),
            "version": (version + "\n").encode("utf-8"),
        }
        return executable(path, f"""
import json
import os
from pathlib import Path
import sys
args = sys.argv[1:]
command = ("version" if args == ["--version"] else "edges" if "--edges" in args else
           "sites" if "--dump-sites" in args else args[2])
with open({str(self.log)!r}, "a", encoding="utf-8") as stream:
    print(json.dumps(dict(arm={name!r}, argv=args, env=dict(os.environ), command=command)), file=stream)
sys.stdout.buffer.write({outputs!r}[command])
if command == "edges":
    sys.stderr.buffer.write({jsonl(trace_rows or ())!r})
exec({actions or {}!r}.get(command, ""))
""")

    def run_step(self, ident="base", *, binary=None, **options):
        return dict(id=ident, do="run", binary=binary or ident, corpus=self.key,
                    memguard=str(self.guard), **options)

    def pair(self, *, base=None, head=None, step=None, head_options=None):
        left = self.run_step()
        right = self.run_step("head", **(head_options or {}))
        step = self.edge_step() if step is None else step
        return self.plan([left, right, step], base=base or self.fake, head=head or self.product("head"))

    def edge_step(self, **options):
        step = dict(id="compare", do="diff-edges", base="base", head="head",
                    classifier=self.classifier.name, expect={"unattributed": 0})
        return dict(step, **options)

    def prior_run(self, plan, ident, directory, *, gate_pin=None, prediction=None, expected=0):
        """Use the existing public command, including its frozen-binding keyword arguments."""
        from measure import cli

        step = next(step for step in plan["steps"] if step["id"] == ident)
        binary = plan["binaries"][step["binary"]]
        args = ["--manifest", self.manifest, "run", "--binary", self.work / binary["path"],
                "--corpus", step["corpus"], "--corpora-root", self.work, "--out", directory,
                "--memguard", step.get("memguard", self.guard),
                "--expect-version", binary["expect_version"]]
        if prediction is not None:
            args.extend(("--prediction", prediction))
        for option in ("sites", "no_bytes"):
            if step.get(option):
                args.append("--" + option.replace("_", "-"))
        for setting in step.get("trace_env", []):
            args.extend(("--trace-env", setting))
        if "limit_gb" in step:
            args.extend(("--limit-gb", step["limit_gb"]))

        def child(argv):
            return cli.main(argv, gate_sha256=gate_pin, gate_step_id=ident if gate_pin else None,
                            prediction_sha256=sha256_of(prediction) if gate_pin else None)

        result = self.local_command(args, command=child)
        self.assert_exit(result, expected)
        return directory

    def reuse(self, plan, ident, directory, *, receipt_sha256=True):
        plan = deepcopy(plan)
        step = next(step for step in plan["steps"] if step["id"] == ident)
        step["reuse"] = {"path": directory.relative_to(self.work).as_posix()}
        if receipt_sha256:
            step["reuse"]["receipt_sha256"] = sha256_of(directory / "receipt.json")
        return plan
