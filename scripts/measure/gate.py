"""Command-line entry and publication for a frozen measurement gate."""

from __future__ import annotations

from pathlib import Path
import platform
import sys
from tempfile import TemporaryDirectory

from . import gate_credit, gate_decision, gate_file, gate_steps, receipt, report
from .corpora import REPO
from .edges import canonical
from .inputs import git
from .run import claim_output

EXIT = {"pass": 0, "fail": 1, "retrospective": 1, "refused": 2, "host_quiet": 75}


def register(commands):
    parser = commands.add_parser("gate", help="execute a frozen measurement gate and write its report")
    parser.add_argument("file", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--binary", action="append", default=[], metavar="NAME=PATH")
    parser.add_argument("--corpora-root", action="append", type=Path, default=[], metavar="DIR")
    parser.add_argument("--decision-pin", metavar="SHA256")
    parser.add_argument("--retrospective", action="store_true")


def completed_credit(summary, result):
    if result != "pass":
        if result == "retrospective":
            for row in summary["steps"]:
                row["claim_scope"] = "none"
        return
    products = [row["id"] for row in summary["steps"] if row["claim_scope"] == "product"]
    control = any(row["claim_scope"] == "control" for row in summary["steps"])
    summary.update(product_steps=products, claim_scope="product" if products else "control" if control else "execution")


def execute(args, summary):
    decision = None
    diagnostics, waivers, wrappers = [], {}, {}
    try:
        path, raw, gate = gate_file.load(args.file)
        summary.update(lane=gate["lane"], steps=[
            gate_steps.initial_step(spec) for spec in gate["steps"]
            if isinstance(spec, dict) and all(isinstance(spec.get(k), str) for k in ("id", "do"))
        ])
        gate_file.validate_graph(gate)
        decision = gate_decision.freeze(args, path, raw, gate)
        summary.update(gate_sha256=decision.sha256, prediction_sha256=decision.projection["prediction_sha256"])
        gate_decision.check_pin(args, decision)
        reuses = gate_steps.load_reuses(args, decision)
        gate_credit.check_identities(decision)
        execution = gate_steps.Execution(args, decision, reuses)
        result = "pass"
        previous = {}
        for spec, row in zip(decision.steps, summary["steps"]):
            result, context = execution.step(spec, row)
            if spec.get("allow_empty"):
                waivers[spec["id"]] = dict(observed=context.get("minima", {}), consumed=context.get("waived", False))
            if result != "pass":
                break
            if not args.retrospective:
                row["claim_scope"] = gate_credit.step_credit(
                    spec, row["metrics"], context["waived"],
                    [previous[ref] for ref in gate_file.references(spec)])
            previous[spec["id"]] = row
        wrappers = {ident: entry[0] for ident, entry in execution.wrappers.items()}
        summary["node_versions"] = sorted(execution.node_versions)
        execution.verify_all()
        decision.recheck()
        if args.retrospective and result in ("pass", "fail"):
            result = "retrospective"
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
        diagnostics.append(str(error))
        result = "refused"
    completed_credit(summary, result)
    summary.update(result=result, exit=EXIT[result])
    return decision, diagnostics, waivers, wrappers


def run_gate(args):
    args.out = args.out.absolute()
    try:
        with claim_output(args.out):
            summary = dict(schema="prism.measure.gate-result/1", lane=None, gate_sha256=None,
                           decision_pin=args.decision_pin, prediction_sha256=None,
                           retrospective=args.retrospective, claim_scope="none", product_steps=[],
                           kit_commit=git(REPO, "rev-parse", "HEAD").decode("ascii").strip(),
                           python_version=platform.python_version(), node_versions=[], steps=[])
            with receipt.legacy_gate_receipts(args.retrospective):
                decision, diagnostics, waivers, wrappers = execute(args, summary)
            with TemporaryDirectory(prefix=".publish-", dir=args.out) as temporary:
                stage = Path(temporary)
                if decision is not None:
                    (stage / "decision.json").write_bytes(decision.encoded)
                (stage / "REPORT.md").write_bytes(report.render(summary, decision, diagnostics, waivers, wrappers))
                (stage / "gate.json").write_bytes(canonical(summary) + b"\n")
                if decision is not None:
                    # If reporting raced a persistent input change, publish a refusal using the
                    # original decision and prediction bytes, with all attempted evidence retained.
                    try:
                        decision.recheck()
                    except (OSError, ValueError) as error:
                        if str(error) not in diagnostics:
                            diagnostics.append(str(error))
                        summary.update(result="refused", exit=2, claim_scope="none", product_steps=[])
                        (stage / "REPORT.md").write_bytes(report.render(summary, decision, diagnostics, waivers, wrappers))
                        (stage / "gate.json").write_bytes(canonical(summary) + b"\n")
                    (stage / "decision.json").replace(args.out / "decision.json")
                for name in ("REPORT.md", "gate.json"):
                    (stage / name).replace(args.out / name)
    except (OSError, ValueError, RuntimeError) as error:
        print(f"measure gate: {error}", file=sys.stderr)
        return 2
    for message in diagnostics:
        print(f"measure gate: {message}", file=sys.stderr)
    print(f"{summary['result']} ({summary['exit']})")
    return summary["exit"]
