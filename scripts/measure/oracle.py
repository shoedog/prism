"""The guarded TypeScript oracle's public measurement and self-check entry points."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import sys
from tempfile import NamedTemporaryFile, TemporaryDirectory

from .edges import KINDS, canonical
from .inputs import json_object, sha256_file
from .oracle_inputs import KIT, OracleError, cache_path, cached, name, prepare_inputs, source
from .oracle_selection import call_population, edge_population
from .receipt import HOST_ENV, gate_binding
from .run import DEFAULT_GUARD, claim_output, guard_env, guarded


def register(commands):
    actions = commands.add_parser("oracle").add_subparsers(dest="operation", required=True)
    edge = actions.add_parser("ts-edges")
    call = actions.add_parser("ts-calls")
    for parser in (edge, call):
        parser.add_argument("--out", required=True, type=Path)
        parser.add_argument("--typescript", type=Path)
        parser.add_argument("--question", type=Path)
        parser.add_argument("--relation")
    mode = edge.add_mutually_exclusive_group(required=True)
    mode.add_argument("--all", action="store_true")
    mode.add_argument("--changes", type=Path)
    edge.add_argument("--run", required=True, type=Path)
    edge.add_argument("--side", choices=("base", "head"))
    edge.add_argument("--kind", choices=KINDS, action="append", default=[])
    edge.add_argument("--class", dest="classes", action="append", default=[])
    mode = call.add_mutually_exclusive_group(required=True)
    mode.add_argument("--census", type=Path)
    mode.add_argument("--base", type=Path)
    call.add_argument("--head", type=Path)
    call.add_argument("--target-confidence", choices=("exact", "name_only"), action="append")
    call.add_argument("--target-kind", action="append", default=[])
    call.add_argument("--aggregate-only", action="store_true")


def report(error, code):
    print(f"measure oracle: {error}", file=sys.stderr)
    return code


def launch(request, stage):
    host = {key: os.environ[key] for key in HOST_ENV if key in os.environ}
    host["LC_ALL"] = "C"
    job = stage / "job.json"
    with job.open("w", encoding="utf-8") as stream:
        json.dump(request, stream, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
    result = stage / "result.json"
    log = stage / "checker.stderr"
    attempt = guarded(DEFAULT_GUARD, 8,
                      ["node", str(KIT / "ts_oracle.cjs"), str(job), str(stage)],
                      guard_env(host, {}, DEFAULT_GUARD), result, log)
    status = attempt["guard_status"]
    if status and status["kind"] == "host_quiet":
        raise OracleError(75, "host-quiet: guarded checker was not launched")
    if not status or status["kind"] != "child" or attempt["exit"] != 0:
        detail = log.read_text(encoding="utf-8", errors="replace").strip()
        raise OracleError(1, f"checker-execution: {detail or attempt['exit']}")
    try:
        response = json_object(result.read_bytes())
    except (OSError, ValueError) as error:
        raise OracleError(1, "checker-execution: missing or invalid result") from error
    if response.get("refusal"):
        raise OracleError(2, response["refusal"])
    return response


def recheck(inputs, typescript, question, **kwargs):
    try:
        current = prepare_inputs(typescript, question, **kwargs)
    except OracleError as error:
        raise OracleError(1, f"checker-or-question-drift: {error}") from error
    if current != inputs:
        raise OracleError(1, "checker-or-question-drift: input bytes changed during analysis")


def selfcheck(argv):
    parser = argparse.ArgumentParser(prog="node oracle/selfcheck.cjs")
    parser.add_argument("--typescript", type=Path)
    parser.add_argument("--question", type=Path)
    args = parser.parse_args(argv)
    temporary = None
    try:
        inputs = prepare_inputs(args.typescript, args.question)
        with TemporaryDirectory(prefix="measure-selfcheck-") as directory:
            counts = launch(dict(inputs, operation="selfcheck"), Path(directory))
        recheck(inputs, args.typescript, args.question)
        record = dict(schema="prism.measure.selfcheck/1", key=inputs["key"], status="ok",
                      components=inputs["components"], cases=counts["cases"],
                      expected=counts["expected"], agreed=counts["expected"], failures=[],
                      runtime=dict(node=counts["node"], python=platform.python_version()))
        path = cache_path(inputs["key"])
        path.parent.mkdir(parents=True, exist_ok=True)
        with NamedTemporaryFile(dir=path.parent, prefix=".receipt-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(canonical(record) + b"\n")
        temporary.replace(path)
        print(canonical(record).decode("utf-8"))
        return 0
    except OracleError as error:
        return report(error, error.code if not str(error).startswith("selfcheck-binding:") else 1)
    except (OSError, ValueError, KeyError) as error:
        return report(error, 1)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def run_oracle(args, entries, *, gate_sha256=None, gate_step_id=None):
    try:
        gate_binding(gate_sha256, gate_step_id)
        if args.question and args.out.resolve().is_relative_to(args.question.parent.resolve()):
            raise ValueError("usage: output must be outside the question directory")
        if args.relation is not None and not name(args.relation):
            raise ValueError("selector: invalid relation")
        if args.question is None and args.relation not in (
            None, "argument_binding", "lexical_binding", "call_target", "relation-not-graded"
        ):
            raise ValueError("selector: unknown built-in relation")
        if args.operation == "ts-edges":
            runs, selection, bindings, claims = edge_population(args, entries, gate_sha256)
            places, deltas = {}, None
            mode = "edge-census" if args.all else "edge-changes"
            denominators = dict(mode=mode, sites=None, base=None, head=None, census=None,
                                selected=len(claims), out_of_scope=0, populations={})
        else:
            runs, selection, claims, places, deltas, denominators = call_population(args, entries)
            bindings = {}
        with claim_output(args.out), TemporaryDirectory(prefix=".stage-", dir=args.out) as directory:
            stage = Path(directory)
            sources, states = {}, {}
            for arm, run in runs.items():
                sources[arm], states[arm] = source(run)
            if not all(item["matches"] for item in sources.values()):
                for item in sources.values():
                    item["matches"] = False
            inputs = prepare_inputs(args.typescript, args.question)
            prior = cached(inputs)
            if platform.python_version() != prior["runtime"]["python"]:
                raise OracleError(3, "selfcheck-runtime: use the self-check Python runtime")
            request = dict(inputs, operation="grade", sources=sources, selection=selection,
                           relation=args.relation, claims=claims, places=places,
                           denominators=denominators, prior=prior["expected"], node=prior["runtime"]["node"])
            response = launch(request, stage)
            del claims, request
            for arm, run in runs.items():
                if source(run, recheck=True)[1] != states[arm]:
                    raise OracleError(1, "source-drift: bound source changed during analysis")
            recheck(inputs, args.typescript, args.question)
            if (response.get("node") != prior["runtime"]["node"]
                    or platform.python_version() != prior["runtime"]["python"]):
                raise OracleError(1, "selfcheck-runtime: run self-check under the grading runtimes")
            qualification = inputs["qualification_sha256"]
            bindings.update(runs=[dict(arm=arm, receipt_sha256=run.receipt_sha256)
                                  for arm, run in runs.items()],
                            qualification=(dict(path="oracle/qualifications/bundle.json",
                                                sha256=qualification) if qualification else None),
                            runtime=prior["runtime"])
            files = ["tuples.json", "denominators.json"]
            if not selection.get("aggregate_only"):
                files.append("verdicts.jsonl")
                if deltas is not None:
                    with (stage / "deltas.jsonl").open("wb") as stream:
                        for delta in deltas:
                            stream.write(canonical(delta) + b"\n")
                    files.append("deltas.jsonl")
            artifacts = {path: sha256_file(stage / path) for path in files}
            output = dict(schema="prism.measure.oracle/1", inputs=bindings,
                          selfcheck_key=inputs["key"], mode=denominators["mode"],
                          relation=args.relation, selection=selection, qualification_sha256=qualification,
                          gate_sha256=gate_sha256, gate_step_id=gate_step_id, artifacts=artifacts)
            (stage / "oracle-receipt.json").write_bytes(canonical(output) + b"\n")
            for file in (*files, "oracle-receipt.json"):
                (stage / file).replace(args.out / file)
        return 0
    except OracleError as error:
        return report(error, error.code)
    except (OSError, ValueError, TypeError, KeyError, RecursionError) as error:
        return report(error, 2)


def build_qualification(runs, labels, output_dir):
    from .oracle_qualification import build
    return build(runs, labels, output_dir)


if __name__ == "__main__":
    raise SystemExit(selfcheck(sys.argv[1:]))
