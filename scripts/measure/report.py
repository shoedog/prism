"""Render the eight sections of a gate report from measured evidence."""

from __future__ import annotations

import re

from .edges import canonical
from .gate_expect import scope_counts

SECTIONS = ("Receipt", "Prediction", "Population and denominators", "Classes and certificate",
            "Verdicts", "Controls", "Limits", "Result")


def item(value):
    return canonical(value).decode("utf-8")


def notes(summary, decision, diagnostics, waivers, wrappers):
    areas = {name: [] for name in SECTIONS}
    registry, prediction, populations, certificates, verdicts, controls, limits, result = areas.values()
    for key in ("lane", "kit_commit", "python_version", "node_versions", "gate_sha256", "decision_pin"):
        registry.append(f"- `{key}` = {item(summary[key])}")
    prediction.append(f"SHA-256 {item(summary['prediction_sha256'])}")
    if decision is not None:
        registry.append(f"- Binaries {item(decision.binaries)}")
        registry.append(f"- Filed inputs {item(decision.files)}")
        for spec in decision.projection["steps"]:
            registry.append(f"- {spec['id']} declared step {item(spec)}")
        text = decision.prediction.decode("utf-8")
        ticks = max((len(match[0]) for match in re.finditer(r"`+", text)), default=0)
        fence = "`" * max(3, ticks + 1)
        prediction.extend(("", fence + "\n" + text + ("" if text.endswith("\n") else "\n") + fence))
        for entry in decision.limits:
            limits.append(f"- Reuse {item(entry)}")
            if entry["kit_commit"] != summary["kit_commit"]:
                limits.append(f"- {entry['step']} reused a run from kit commit {entry['kit_commit']}.")
    limits.extend((
        "- The kit cannot determine when the controller first saw an outcome or prevent reruns under the same decision pin.",
        "- Kit commit, Python and observed Node versions are recorded and unbound by the decision digest.",
        "- Guard configuration read indirectly and changes restored within a step are outside the input contract.",
    ))
    limits.extend(f"- Refused: {item(message)}" for message in diagnostics)
    result.extend((f"- Result **{summary['result']}** with exit {summary['exit']}.",
                   f"- Claim scope `{summary['claim_scope']}`. Product steps {item(summary['product_steps'])}."))
    controls.append(f"- Gate scope `{summary['claim_scope']}`, exit {summary['exit']}, "
                    f"product steps {item(summary['product_steps'])}.")
    for step in summary["steps"]:
        ident, kind, status, metrics = (step[key] for key in ("id", "do", "status", "metrics"))
        if ident in wrappers:
            registry.append(f"- {ident}/step-receipt.json SHA-256 {wrappers[ident]}")
            registry.append(f"- {ident} artifacts {item(step['artifacts'])}")
        if status == "not_run":
            limits.append(f"- {ident} was not run. Measurements unavailable.")
        if status in ("refused", "fail"):
            result.append(f"- {ident} {status} with child exit {item(step['child_exit'])}.")
            for failure in step["failures"]:
                result.append(f"  - {item(failure)}")
            limits.append(f"- {ident} {status}. {item(step['failures'])}")
        if ident in waivers:
            limits.append(f"- {ident} declared allow_empty. {item(waivers[ident])}")
        spec = decision.by_id[ident] if decision is not None else {}
        if spec.get("key") == "line":
            limits.append(f"- {ident} line projection. Reason {item(spec.get('line_reason'))}. "
                          f"Loss {item(metrics.get('line_projection_loss'))}.")
        if (step["claim_scope"] == "control" or spec.get("identity_control")
                or spec.get("absence") or spec.get("allow_empty")):
            controls.append(f"- {ident} {status.replace('_', ' ')}, scope {step['claim_scope']}. "
                            f"Expected {item(step['expect'])}. Observed {item(control_metrics(kind, metrics))}.")
        if kind == "review-diff" and "population" in metrics:
            review_population(ident, metrics, populations)
        elif kind.startswith("oracle ") and "denominators" in metrics:
            oracle_sections(ident, metrics, populations, verdicts, controls)
        elif metrics and kind != "diff-edges":
            populations.append(f"- {ident} {item(metrics)}")
        if kind == "diff-edges" and metrics:
            fields = ("key_mode", "base_edges", "head_edges", "changed_keys", "occurrences", "by_class",
                      "by_kind", "unattributed", "records", "unjoined", "validation", "changes_sha256")
            certificates.append(f"- {ident} {item({key: metrics[key] for key in fields})}")
    return areas


def control_metrics(kind, metrics):
    if kind == "review-diff":
        return {key: metrics[key] for key in ("selected", "ok", "distinct_diffs") if key in metrics}
    if kind.startswith("oracle "):
        return {"coverage_checks": metrics.get("coverage_checks", [])}
    return metrics


def review_population(ident, metrics, lines):
    lines.append(f"- {ident} measured {metrics['ok']} of {metrics['population']} ordinals "
                 f"({metrics['selected']} selected), covering {metrics['distinct_diffs']} distinct diffs. "
                 f"Ordinals with eligible lines {item(metrics['eligible_ordinals'])}.")
    for row in metrics["dispositions"]:
        lines.append(f"  - Ordinal {row['ordinal']} {row['status']}. {item(row['reason'])}")


def oracle_sections(ident, metrics, populations, verdicts, controls):
    denominators = metrics["denominators"]
    overall = {key: value for key, value in denominators.items() if key not in ("scopes", "populations")}
    populations.append(f"- {ident} {item(overall)}")
    for confidence, counts in denominators["populations"].items():
        verdicts.append(f"- {ident} target population {item(confidence)} {item(counts)}")
    for relation, properties in metrics["tuples"].items():
        for prop, counts in properties.items():
            verdicts.append(f"- {ident} relation {item(relation)}, property {item(prop)}. {item(counts)}")
    for check in metrics.get("coverage_checks", []):
        counts = scope_counts(denominators["scopes"], check["scope"])
        lines = controls if check["claim"] == "absence" else verdicts
        lines.append(f"- {ident} {item(check)}. Verdict counts {item(counts)}.")


def render(summary, decision, diagnostics, waivers, wrappers):
    areas = notes(summary, decision, diagnostics, waivers, wrappers)
    return ("# Measurement gate\n\n" + "\n\n".join(
        f"## {section}\n\n" + "\n".join(lines or ["No observations."])
        for section, lines in areas.items()) + "\n").encode("utf-8")
