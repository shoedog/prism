"""Measure and compare two guarded review arms over a pinned population."""

from __future__ import annotations

import os
from pathlib import Path
import shlex
import stat
import sys
from tempfile import TemporaryDirectory

from . import inputs, population, receipt, run
from .corpora import text
from .review_findings import compare, decode, METRICS
from .review_receipt import STATUS_MISSING, admit, argv_for, check_args, publish, ROLES

DEFAULT_SHARED = "--algorithm taint --resolution scoped"
DEFAULT_SAMPLE = 30
DEFAULT_SEED = 20261101
GUARD_LIMIT_GB = 8
QUIET = "host-quiet"
INPUT_DRIFT = "review-input-changed"


def register(commands):
    parser = commands.add_parser("review-diff", help="compare two review arms over a pinned diff population")
    for flag in ("--population", "--clones", "--out"):
        parser.add_argument(flag, type=Path, required=True)
    parser.add_argument("--arm", action="append", required=True, metavar="NAME=BIN[::ARG...]",
                        help="specify two arms in base/head order, with literal :: separators")
    parser.add_argument("--review-args", default=DEFAULT_SHARED, metavar="STRING",
                        help="shared arguments in POSIX shell quoting")
    parser.add_argument("--only", metavar="N,M,...", help="select existing population ordinals")
    parser.add_argument("--sample", type=int, default=DEFAULT_SAMPLE, metavar="N",
                        help="maximum successful rows to sample (default 30)")
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED, metavar="S",
                        help="sample seed (default 20261101)")


def _arms(specifications, shared_args):
    if len(specifications) != 2:
        raise ValueError("exactly two review arms are required")
    check_args(shared_args)
    arms = {}
    for role, value in zip(ROLES, specifications):
        name, separator, command = value.partition("=")
        executable, *args = command.split("::")
        if not separator or not text(name) or not executable:
            raise ValueError("expected a named arm NAME=BIN[::ARG...]")
        check_args(args)
        binary = Path(executable).resolve()
        arms[role] = {
            "name": name,
            "binary": {"path": str(binary), "sha256": run.executable_hash(binary), "version": None},
            "shared_args": shared_args, "arm_args": args, "commands": [],
        }
    if arms["base"]["name"] == arms["head"]["name"]:
        raise ValueError("review arm names must be distinct")
    return arms


def _selection(only, diffs):
    available = {row["ordinal"] for row in diffs}
    if only is None:
        return available
    parts = only.split(",")
    if not all(part.isascii() and part.isdecimal() for part in parts):
        raise ValueError("--only requires comma-separated ordinals")
    selected = {int(part) for part in parts}
    if len(selected) != len(parts) or not selected <= available:
        raise ValueError("--only requires distinct registered ordinals")
    return selected


def _row(pin, arms):
    return {
        **{key: pin[key] for key in ("ordinal", "repo", "commit")},
        "status": "excluded", "reason": "not-selected",
        "arms": {arm["name"]: {
            "status": "unlaunched", "exit": None, "guard_status": None,
            "output": None, "output_sha256": None, "stderr": None, "stderr_sha256": None,
            "input_before_sha256": None, "input_after_sha256": None,
        } for arm in arms.values()},
        **dict.fromkeys(METRICS), "rule_transitions": [], "anchors": [],
    }


class ReviewExecution:
    def __init__(self, stage, guard, arms):
        self.stage = stage
        self.guard = guard
        self.arms = arms
        self.artifacts = {}
        self.version_failures = {}
        self.guard_sha = run.executable_hash(guard)
        self.helper_sha = run.quiet_helper_hash(guard)
        host = {name: os.environ[name] for name in receipt.HOST_ENV if name in os.environ}
        self.env = run.guard_env({**host, "LC_ALL": "C"}, {}, guard)

    def _guarded(self, argv, stdout, stderr):
        return run.guarded(self.guard, GUARD_LIMIT_GB, argv, self.env, stdout, stderr)

    def _recheck(self, binary):
        run.recheck_guard(self.guard, self.guard_sha, self.helper_sha)
        run.recheck_binary(Path(binary["path"]), binary["sha256"])

    def _version(self, role, binary, observed):
        if binary["version"] is not None:
            return None
        if role not in self.version_failures:
            stdout, stderr = (self.stage / f"version.{suffix}" for suffix in ("stdout", "stderr"))
            attempt = self._guarded([binary["path"], "--version"], stdout, stderr)
            if run.outcome(attempt) == 75:
                observed["guard_status"] = attempt["guard_status"]
                return "refused", QUIET
            try:
                self._recheck(binary)
                version = stdout.read_text(encoding="utf-8").strip()
                if run.outcome(attempt) or not version:
                    detail = stderr.read_text(encoding="utf-8", errors="replace").strip()
                    raise ValueError(detail or "version command failed or produced no version")
            except (OSError, ValueError) as error:
                self.version_failures[role] = f"review-version-failed: {error}"
            else:
                binary["version"] = version
                return None
        return "failed", self.version_failures[role]

    def _input(self, scratch, spec, pin, pinned, observed, when, state):
        try:
            binding = inputs.bind_tree(scratch)
            observed[f"input_{when}_sha256"] = binding.sha256
            if (binding.sha256 == pinned and (state is None or binding.state_sha256 == state)
                    and stat.S_ISREG(spec.lstat().st_mode)
                    and inputs.sha256_file(spec) == pin["spec_sha256"]):
                return binding.state_sha256
        except (OSError, ValueError):
            pass
        return None

    def _retain(self, role, ordinal, argv, observed, attempt, streams):
        status = attempt["guard_status"]
        observed["guard_status"] = status
        # Without a status the launch is indeterminate: streams the child wrote are
        # evidence of it and are kept, never deleted (the arm then fails).
        wrote = status is None and any(
            (self.stage / name).is_file() and (self.stage / name).stat().st_size
            for name in streams.values())
        if wrote or status and status["launched"]:
            self.arms[role]["commands"].append({"ordinal": ordinal, "argv": argv})
            observed["exit"] = status["child_exit"] if status else None
            for field, name in streams.items():
                self.artifacts[name] = receipt.artifact(self.stage / name)
                observed[field] = name
                observed[field + "_sha256"] = self.artifacts[name]["sha256"]
        else:
            for name in streams.values():
                (self.stage / name).unlink(missing_ok=True)

    def _measure_arm(self, role, pin, scratch, spec, pinned, observed, state):
        arm = self.arms[role]
        binary = arm["binary"]
        try:
            self._recheck(binary)
            version_error = self._version(role, binary, observed)
            if version_error:
                status, reason = version_error
                return status, reason, None, state
        except OSError as error:
            return "refused", f"review-executable-changed: {error}", None, state
        path = f"arms/{pin['ordinal']}/{role}"
        streams = {"output": path + ".stdout", "stderr": path + ".stderr"}
        stdout, stderr = (self.stage / name for name in streams.values())
        stdout.parent.mkdir(parents=True, exist_ok=True)
        argv = argv_for(arm, str(scratch), str(spec))
        state = self._input(scratch, spec, pin, pinned, observed, "before", state)
        if state is None:
            return "refused", INPUT_DRIFT, None, None
        attempt = self._guarded(argv, stdout, stderr)
        status = attempt["guard_status"]
        launched = status and status["launched"]
        after = self._input(scratch, spec, pin, pinned, observed, "after", state) if launched else None
        self._retain(role, pin["ordinal"], argv, observed, attempt, streams)
        result = run.outcome(attempt)
        if result == 75:
            return "refused", QUIET, None, state
        if observed["output"] is not None and not launched:
            return "failed", STATUS_MISSING, None, state
        if not launched:
            return "failed", "review-guard-failed", None, state
        if after is None:
            return "failed", INPUT_DRIFT, None, state
        try:
            self._recheck(binary)
        except OSError as error:
            return "failed", f"review-executable-changed: {error}", None, state
        if result:
            status = observed["guard_status"]["kind"]
            return "failed", f"review-{status}-failed", None, state
        try:
            findings = decode((self.stage / observed["output"]).read_bytes())
        except (OSError, ValueError, RecursionError) as error:
            return "failed", f"review-output-unusable: {error}", None, state
        return "ok", None, findings, state

    def measure(self, row, pin, source):
        with TemporaryDirectory(prefix=f"input-{row['ordinal']}-", dir=self.stage) as temp:
            scratch = Path(temp) / "repo"
            spec = Path(temp) / "spec.json"
            try:
                raw, pinned = population.materialize(source, scratch, pin)
                spec.write_bytes(raw)
                spec.chmod(0o400)
            except (OSError, ValueError) as error:
                row.update(status="refused", reason=str(error))
                return
            state = None
            findings = []
            for role, arm in self.arms.items():
                observed = row["arms"][arm["name"]]
                status, reason, findings_for_arm, state = self._measure_arm(
                    role, pin, scratch, spec, pinned, observed, state)
                observed["status"] = status
                if status != "ok":
                    row.update(status=status, reason=reason)
                    return
                findings.append(findings_for_arm)
            row.update(compare(*findings), status="ok", reason=None)


def review(args, *, gate_sha256=None, gate_step_id=None):
    receipt.gate_binding(gate_sha256, gate_step_id)
    registered = population.load(args.population)
    selected = _selection(args.only, registered.diffs)
    if args.sample < 0:
        raise ValueError("--sample must be nonnegative")
    arms = _arms(args.arm, shlex.split(args.review_args))
    if args.out.is_symlink():
        raise ValueError("review output directory may not be a symlink")
    clones, out = args.clones.resolve(), args.out.resolve()
    sources = {name: (clones / name).resolve() for name in registered.repos}
    if any(ancestor.exists() and ancestor.samefile(source)
           for source in sources.values() if source.is_dir()
           for ancestor in (out, *out.parents)):
        raise ValueError("review output directory is inside a source clone")
    with run.claim_output(out), TemporaryDirectory(prefix=".stage-", dir=out) as temp:
        execution = ReviewExecution(Path(temp), run.DEFAULT_GUARD, arms)
        rows = []
        quiet = False
        try:
            for pin in registered.diffs:
                row = _row(pin, arms)
                rows.append(row)
                if pin["ordinal"] in selected:
                    if quiet:
                        row.update(status="refused", reason=QUIET)
                    else:
                        execution.measure(row, pin, sources[pin["repo"]])
                        quiet = row["reason"] == QUIET
            publish(out, execution.stage, registered, arms, rows, args.sample, args.seed,
                    execution.artifacts, gate_sha256, gate_step_id)
        except (OSError, ValueError) as error:
            print(f"measure review-diff: {error}", file=sys.stderr)
            return 75 if quiet else 1
    return 75 if quiet else int(any(row["status"] in ("failed", "refused") for row in rows))
