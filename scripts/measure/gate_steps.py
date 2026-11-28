"""Execute bound gate steps and seal their direct inputs and artifacts."""

from __future__ import annotations

from contextlib import ExitStack, redirect_stderr, redirect_stdout
from dataclasses import dataclass, field
import io
from pathlib import Path
import stat

from . import gate_classifier, gate_expect, gate_file, gate_review, inputs, receipt
from .edges import canonical
from .gate_decision import digest
from .gate_oracle import read_oracle
from .oracle_selection import certificate_arms

WRAPPER = "step-receipt.json"


def admitted_run(directory, decision):
    record, checksum = receipt.admit(directory, entries=decision.entries, with_digest=True)
    return directory, record, checksum


def measurement_options(step):
    return dict(bytes=not step.get("no_bytes", False), sites=step.get("sites", False), cache="no-cache",
                trace_env=receipt.trace_settings(step.get("trace_env", [])))


def check_run(decision, step, record, retrospective=False):
    ident, binary = step["id"], decision.binaries[step["binary"]]
    gate_file.require(record["binary"]["sha256"] == binary["sha256"], f"{ident}: reused or fresh binary digest differs")
    gate_file.require(receipt.matches_version(record["binary"]["version"], binary["expect_version"]),
                      f"{ident}: version pin mismatch")
    gate_file.require(record["corpus"]["key"] == step["corpus"] and record["options"] == measurement_options(step),
                      f"{ident}: corpus or run options differ")
    guard = record["guard"]
    gate_file.require(guard["limit_gb"] == step.get("limit_gb", 8)
                      and all((key not in guard and retrospective) or guard.get(key) == decision.files[role]
                              for key, role in (("sha256", f"{ident}/memguard"),
                                                ("quiet_helper_sha256", f"{ident}/quiet_helper"))),
                      f"{ident}: guard or guard options differ")
    old_prediction = (record.get("prediction") or {}).get("sha256")
    old_gate, old_step = record.get("gate_sha256"), record.get("gate_step_id")
    prediction = decision.projection["prediction_sha256"]
    reason = "prediction-digest-matched"
    if old_prediction is None:
        gate_file.require(retrospective, "prediction-digest-missing")
        reason = "prediction-digest-missing"
    else:
        gate_file.require(old_prediction == prediction, "prediction-digest-mismatch")
    gate_file.require(old_step == ident or (old_step is None and retrospective), f"{ident}: run step ID differs")
    gate_file.require(old_gate == decision.sha256 or retrospective, f"{ident}: reuse gate digest differs")
    return dict(step=ident, reason=reason, old_gate_sha256=old_gate, gate_sha256=decision.sha256,
                old_prediction_sha256=old_prediction, prediction_sha256=prediction,
                kit_commit=record["kit_commit"])


def load_reuses(args, decision):
    found = {}
    for step in decision.steps:
        if "reuse" not in step:
            continue
        ident, spec = step["id"], step["reuse"]
        source = (decision.path.parent / spec["path"]).resolve(strict=True)
        bound = admitted_run(source, decision)
        independent = spec.get("receipt_sha256")
        gate_file.require(independent is not None or args.retrospective, f"{ident}: reuse.receipt_sha256 required")
        gate_file.require(independent is None or independent == bound[2], f"{ident}: reuse receipt digest differs")
        reuse = check_run(decision, step, bound[1], args.retrospective)
        if reuse["old_gate_sha256"] != reuse["gate_sha256"]:
            reuse["gate_reason"] = "reuse-gate-digest-missing" if reuse["old_gate_sha256"] is None else "reuse-gate-digest-different"
        decision.limits.append(reuse)
        found[ident] = bound
    return found


def artifacts(directory):
    found = {}
    for path in sorted(directory.rglob("*")):
        name = path.relative_to(directory)
        if name.parts[0] in ("snapshot", WRAPPER):
            continue
        mode = path.lstat().st_mode
        if stat.S_ISREG(mode):
            found[name.as_posix()] = receipt.artifact(path)
        elif not stat.S_ISDIR(mode):
            raise ValueError(f"linked or special step artifact: {name}")
    return found


def initial_step(spec):
    return dict(id=spec["id"], do=spec["do"], status="not_run", claim_scope="none",
                child_exit=None, guard_status=None, metrics={}, expect=spec.get("expect", {}),
                failures=[], artifacts={})


@dataclass
class Execution:
    args: object
    decision: object
    runs: dict
    wrappers: dict = field(default_factory=dict)
    node_versions: set = field(default_factory=set)

    def verify_run(self, ident):
        directory, _, checksum = self.runs[ident]
        gate_file.require(admitted_run(directory, self.decision)[2] == checksum,
                          f"{ident}: admitted run receipt changed")

    def verify_wrapper(self, ident):
        path = self.args.out / ident / WRAPPER
        checksum, known = self.wrappers[ident]
        gate_file.require(stat.S_ISREG(path.lstat().st_mode) and inputs.sha256_file(path) == checksum
                          and artifacts(path.parent) == known, f"{ident}: step wrapper or artifacts changed")
        if ident in self.runs:
            self.verify_run(ident)
        return dict(path=f"{ident}/{WRAPPER}", sha256=checksum)

    def verify_all(self):
        for ident in self.wrappers:
            self.verify_wrapper(ident)

    def step(self, spec, row):
        ident, kind = spec["id"], spec["do"]
        directory = self.args.out / ident
        files = self.decision.step_files(spec)
        predecessors, context = {}, {}
        result = "pass"
        try:
            self.decision.recheck()
            for ref in gate_file.references(spec):
                predecessors[ref] = self.verify_wrapper(ref)
            if "reuse" in spec:
                self.verify_run(ident)
                _, record, checksum = self.runs[ident]
                files[f"{ident}/reuse"] = checksum
                raw = (self.runs[ident][0] / "receipt.json").read_bytes()
                gate_file.require(digest(raw) == checksum, f"{ident}: reused receipt changed")
                directory.mkdir()
                (directory / "receipt.json").write_bytes(raw)
                row.update(child_exit=0, metrics={"commands": len(record["commands"])})
            else:
                if kind == "review-diff":
                    gate_review.configure(self.decision, spec, context)
                result = self.fresh(spec, row, directory, context)
            if ident in self.runs:
                files[f"{ident}/corpus_input"] = self.runs[ident][1]["corpus"]["input_sha256"]
            if "environment" in context:
                # The wrapper retains its six top-level fields and two input maps.
                files[f"{ident}/product_environment"] = context["environment"]
            if result == "pass" or result == "fail" and row["metrics"]:
                failures, minima, waived = gate_expect.evaluate(spec, row["metrics"], context.get("coverage", {}))
                row["failures"].extend(failures)
                context.update(minima=minima, waived=waived)
                if failures:
                    result = "fail"
            elif spec.get("allow_empty") and row["metrics"]:
                context["minima"] = gate_expect.minima(kind, row["metrics"], context.get("coverage", {}))
        except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
            result = "refused"
            row["failures"].append(str(error))
        try:
            self.decision.recheck()
            for ref in predecessors:
                self.verify_wrapper(ref)
        except (OSError, ValueError) as error:
            result = "refused"
            row["failures"].append(str(error))
        row["status"] = "refused" if result == "host_quiet" else result
        if directory.exists() and directory.is_symlink():
            row["status"] = result = "refused"
            row["failures"].append("linked step directory")
            return result, context
        directory.mkdir(exist_ok=True)
        try:
            row["artifacts"] = artifacts(directory)
        except (OSError, ValueError) as error:
            row["status"] = result = "refused"
            row["failures"].append(str(error))
        wrapper = dict(gate_sha256=self.decision.sha256, gate_step_id=ident,
                       prediction_sha256=self.decision.projection["prediction_sha256"],
                       status=row["status"], inputs=dict(files=files, steps=predecessors), artifacts=row["artifacts"])
        raw = canonical(wrapper) + b"\n"
        with (directory / WRAPPER).open("xb") as target:
            target.write(raw)
        self.wrappers[ident] = digest(raw), row["artifacts"]
        return result, context

    def fresh(self, spec, row, directory, context):
        from .cli import main

        decision, ident, kind = self.decision, spec["id"], spec["do"]
        argv = child_argv(self.args, decision, spec, directory, self.runs)
        stdout, stderr = io.StringIO(), io.StringIO()
        with ExitStack() as stack:
            if kind == "diff-edges":
                pair = [self.runs[spec[arm]] for arm in ("base", "head")]
                module = stack.enter_context(gate_classifier.snapshot(decision, spec, pair, context, self.args.out))
                argv.extend(("--classifier", str(module)))
            if kind != "diff-edges" or not spec.get("list"):
                stack.enter_context(redirect_stdout(stdout))
            stack.enter_context(redirect_stderr(stderr))
            try:
                code = main(argv, gate_sha256=decision.sha256, gate_step_id=ident,
                            prediction_sha256=decision.projection["prediction_sha256"], gate_context=context)
            except SystemExit as error:
                code = error.code
        attempt = context.get("attempt", {})
        if code != 75 and code and "child_75" in context:
            attempt = context["child_75"]
        guard = attempt.get("guard_status")
        raw_child = guard["child_exit"] if guard else None
        row["child_exit"] = code
        row["guard_status"] = guard
        if raw_child and (kind == "run" and code == 4 or code != 0 and raw_child == 75):
            row["child_exit"] = raw_child
        detail = stderr.getvalue().strip()
        if code:
            row["failures"].append(detail or f"{kind} exited {code}")
        result = child_result(kind, code, guard)
        row["metrics"] = self.read_metrics(spec, directory, context, code, stdout.getvalue(), detail)
        return result

    def read_metrics(self, step, directory, context, code, stdout, stderr):
        kind, ident = step["do"], step["id"]
        if kind.startswith("corpora-"):
            rows = [line.split("\t", 2) for line in stdout.rstrip("\n").split("\n")[1:]]
            directory.mkdir(exist_ok=True)
            (directory / "corpora.tsv").write_bytes(stdout.encode("utf-8"))
            return (dict(listed=len(rows), keys=[row[0] for row in rows]) if kind == "corpora-list"
                    else dict(verified=sum(row[1] == "ok" for row in rows),
                              dispositions=[dict(key=key, status=status, reason=reason)
                                            for key, status, reason in rows]))
        if kind == "run":
            path = directory / "receipt.json"
            if code == 0:
                bound = admitted_run(directory, self.decision)
                check_run(self.decision, step, bound[1])
                self.runs[ident] = bound
                return {"commands": len(bound[1]["commands"])}
            if path.is_file():
                partial = read_object(path)
                return {"commands": sum(command["exit"] == 0 for command in partial["commands"]),
                        "corpus_input_sha256": partial["corpus"]["input_sha256"]}
        if kind == "diff-stats" and "stats" in context:
            directory.mkdir(exist_ok=True)
            (directory / "stats.stdout").write_bytes(stdout.encode("utf-8"))
            if stderr:
                (directory / "stats.stderr").write_bytes((stderr + "\n").encode("utf-8"))
            tables = context["stats"]
            context["coverage"] = {kind: table["coverage"] for kind, table in tables.items()}
            metrics = {kind: dict(base_leaves=table["base"], head_leaves=table["head"],
                                  deltas=len(table["changed"]), forbidden=len(table["forbidden"]))
                       for kind, table in tables.items()}
            return {**{key: sum(table[key] for table in metrics.values())
                       for key in ("base_leaves", "head_leaves", "deltas", "forbidden")}, "tables": metrics}
        if kind == "diff-edges" and code in (0, 1):
            return self.read_certificate(step, directory)
        if kind.startswith("oracle ") and code == 0:
            metrics, runtime = read_oracle(self.decision, step, directory, self.runs, self.args.out)
            self.node_versions.add(runtime["node"])
            return metrics
        if kind == "review-diff" and (directory / "review-receipt.json").is_file():
            return gate_review.actual_rows(self.decision, step, directory, context, code)
        if code == 0:
            raise ValueError(f"{ident}: successful child omitted required outputs")
        return {}

    def read_certificate(self, step, directory):
        from argparse import Namespace

        decision, ident = self.decision, step["id"]
        changes = directory / "changes.jsonl"
        cert, arms, _ = certificate_arms(Namespace(changes=changes), decision.entries, decision.sha256)
        gate_file.require(cert["gate_step_id"] == ident
                          and cert["classifier_sha256"] == decision.files[f"{ident}/classifier"]
                          and cert["changes_sha256"] == inputs.sha256_file(changes)
                          and (directory / "classes.tsv").is_file()
                          and all(arms[arm].receipt_sha256 == self.runs[step[arm]][2]
                                  for arm in ("base", "head")), "edge certificate binding differs")
        return cert


def read_object(path):
    return inputs.json_object(gate_file.regular(path))


def child_result(kind, code, guard):
    if code == 0:
        return "pass"
    if code == 2 or code == 3 and (kind == "run" or kind.startswith("oracle ")):
        return "refused"
    if code == 75 and guard and guard["kind"] == "host_quiet":
        return "host_quiet"
    return "fail"


def child_argv(args, decision, step, directory, runs):
    kind, ident = step["do"], step["id"]
    argv = ["--manifest", str(decision.manifest)]

    def pair(flag, value):
        argv.append(f"--{flag.replace('_', '-')}={value}")

    def options(*fields):
        for field in fields:
            if field not in step:
                continue
            value = step[field]
            if type(value) is bool:
                if value:
                    argv.append("--" + field.replace("_", "-"))
            elif isinstance(value, list):
                for member in value:
                    pair(field, member)
            else:
                pair(field, value)

    def input_paths(*fields):
        for field in fields:
            role = f"{ident}/{field}"
            if role in decision.paths:
                pair(field, decision.paths[role])

    def roots():
        for root in args.corpora_root:
            pair("corpora-root", root)

    if kind.startswith("corpora-"):
        argv.extend(("corpora", kind.split("-", 1)[1]))
        if kind == "corpora-verify":
            roots()
            argv.extend(step.get("keys", []))
    elif kind == "run":
        argv.append(kind)
        pair("out", directory)
        pair("binary", decision.binary_paths[step["binary"]])
        pair("expect-version", decision.binaries[step["binary"]]["expect_version"])
        pair("prediction", decision.prediction_path)
        input_paths("memguard")
        options("corpus", "trace_env", "no_bytes", "sites", "limit_gb")
        roots()
    elif kind in ("diff-edges", "diff-stats"):
        argv.extend([kind, *(str(runs[step[arm]][0]) for arm in ("base", "head"))])
        if kind == "diff-edges":
            pair("out", directory)
            options("key", "line_reason", "list")
        else:
            input_paths("allow")
            options("which")
    elif kind.startswith("oracle "):
        argv.extend(kind.split())
        pair("out", directory)
        input_paths("typescript", "question")
        for field in ("run", "base", "head", "census"):
            if field in step:
                pair(field, runs[step[field]][0])
        if "changes" in step:
            pair("changes", args.out / step["changes"] / "changes.jsonl")
        options("all", "relation", "side", "kind", "class", "target_confidence", "target_kind", "aggregate_only")
    else:
        argv.append(kind)
        pair("out", directory)
        pair("clones", decision.path.parent / step["clones"])
        input_paths("population")
        if "only" in step:
            pair("only", ",".join(map(str, step["only"])))
        options("seed", "sample")
    return argv
