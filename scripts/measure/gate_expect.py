"""Preflight and evaluation of gate expectations and coverage."""

from __future__ import annotations

from pathlib import Path

from .diff_stats import parse_pattern
from .inputs import json_object
from .oracle_inputs import RESULTS, name
from .oracle_selection import TARGET_CONFIDENCES


def metric_path(text):
    path, wildcard = parse_pattern(text)
    if wildcard:
        raise ValueError("expect paths cannot contain wildcards")
    return path


def numeric(value):
    return type(value) in (int, float)


def validate_expect(step):
    expect = step.get("expect", {})
    if type(expect) is not dict:
        raise ValueError("expect must be an object")
    for text, bound in expect.items():
        metric_path(text)
        if numeric(bound):
            continue
        if (type(bound) is not dict or not bound or not bound.keys() <= {"min", "max"}
                or not all(map(numeric, bound.values()))
                or len(bound) == 2 and bound["min"] > bound["max"]):
            raise ValueError(f"invalid expectation at {text}")


def expectation_scopes(step):
    result = []
    for text in step.get("expect", {}):
        path = metric_path(text)
        if path[-1] not in (*RESULTS, "graded", "decided"):
            continue
        if len(path) == 4 and path[0] == "tuples":
            result.append(dict(relation=path[1], property=path[2]))
        if path[:2] == ("denominators", "populations"):
            if len(path) == 6 and path[3] == "kinds":
                result.append(dict(confidence=path[2], kind=path[4]))
            if len(path) == 5 and path[3] == "missing_kind":
                result.append(dict(confidence=path[2], kind=None))
    return [scope for index, scope in enumerate(result) if scope not in result[:index]]


def validate_claims(step):
    quality, absence = step.get("quality", []), step.get("absence", [])
    if not isinstance(quality, list) or not isinstance(absence, list):
        raise ValueError("quality and absence must be arrays")
    for item in quality:
        if not isinstance(item, dict) or item.keys() != {"scope", "max_wrong", "min_decided"}:
            raise ValueError("invalid oracle quality members")
        rate, minimum = item["max_wrong"], item["min_decided"]
        if (not isinstance(rate, list) or len(rate) != 2 or any(type(n) is not int for n in rate)
                or not 0 <= rate[0] <= rate[1] or rate[1] == 0
                or type(minimum) is not int or minimum <= 0):
            raise ValueError("invalid oracle quality bound")
    scopes = [item["scope"] for item in quality]
    for scope in [*scopes, *absence]:
        if (not isinstance(scope, dict) or not scope.keys() <= {"relation", "property", "confidence", "kind"}
                or not all(name(value) or key == "kind" and value is None for key, value in scope.items())):
            raise ValueError("invalid oracle scope")
    grading = [*scopes, *expectation_scopes(step)]
    global_verdict = any(metric_path(path) in {("denominators", key) for key in
                         ("graded", "decided", "correct", "wrong", "undecided", "inadmissible")}
                         for path in step.get("expect", {}))
    if global_verdict:
        grading.append({})
    for missing in absence:
        for graded in grading:
            if all(missing[key] == graded[key] for key in missing.keys() & graded.keys()):
                raise ValueError("overlapping oracle grading and absence scopes")
    if step.get("allow_empty") and not absence:
        raise ValueError("oracle allow_empty requires an explicit absence claim")


def validate_scope_selection(step, prepared):
    pairs = set()
    for case in prepared["cases"]:
        for line in (Path(case["root"]) / "expected.jsonl").read_bytes().split(b"\n"):
            if line.strip():
                row = json_object(line)
                pairs.add((row["relation"], row["property"]))
    calls = step["do"] == "oracle ts-calls"
    wire = TARGET_CONFIDENCES if calls else ("exact", "nameonly")
    confidences = step.get("target_confidence", ["exact"]) if calls else wire
    selected_relation, kinds = step.get("relation"), step.get("target_kind", [])
    if selected_relation and not any(r == selected_relation for r, _ in pairs):
        raise ValueError(f"unknown oracle relation {selected_relation!r}")
    for scope in [item["scope"] for item in step.get("quality", [])] + step.get("absence", []):
        relation, prop = scope.get("relation", selected_relation), scope.get("property")
        if selected_relation and relation != selected_relation:
            raise ValueError("oracle scope relation differs from selected relation")
        if (relation is not None or prop is not None) and not any(
                (relation is None or relation == r) and (prop is None or prop == p) for r, p in pairs):
            raise ValueError("oracle scope is outside the selected self-check")
        if "confidence" in scope and (scope["confidence"] not in wire or scope["confidence"] not in confidences):
            raise ValueError("oracle scope confidence is outside the selected wire population")
        if "kind" in scope and (not calls and scope["kind"] is not None or kinds and scope["kind"] not in kinds):
            raise ValueError("oracle scope kind is outside the selected population")


def observed_at(metrics, path):
    current = metrics
    for part in path:
        if not isinstance(current, dict) or part not in current:
            return None
        current = current[part]
    return current


def threshold_failures(step, metrics):
    failures = []
    for text, bound in step.get("expect", {}).items():
        actual = observed_at(metrics, metric_path(text))
        if numeric(actual):
            if numeric(bound) and actual == bound:
                continue
            if isinstance(bound, dict) and bound.get("min", actual) <= actual <= bound.get("max", actual):
                continue
        failures.append(dict(reason="threshold-failed", path=text, observed=actual, expected=bound))
    return failures


def minima(kind, metrics, stats_coverage):
    simple = {"corpora-list": "listed", "corpora-verify": "verified", "run": "commands",
              "review-diff": "ok"}
    if kind in simple:
        field = simple[kind]
        return {field: metrics[field]}
    if kind == "diff-edges":
        return {field: metrics[field] for field in ("base_edges", "head_edges")}
    if kind == "diff-stats":
        return {f"tables.{table}.{arm}_leaves": count
                for table, arms in stats_coverage.items() for arm, count in arms.items()}
    return {f"denominators.{key}": metrics["denominators"][key] for key in ("graded", "decided")}


def matching(row, scope):
    return all(row[key] == value for key, value in scope.items())


def scope_counts(rows, scope):
    totals = dict.fromkeys(RESULTS, 0)
    for row in rows:
        if matching(row, scope):
            for label in RESULTS:
                totals[label] += row[label]
    return totals


def oracle_checks(step, metrics):
    den = metrics["denominators"]
    rows = den["scopes"]
    failures = []
    if den["inadmissible"]:
        failures.append(dict(reason="inadmissible-verdicts", observed=den["inadmissible"], expected=0))
    rate_scopes = [item["scope"] for item in step.get("quality", [])]
    if "denominators.wrong" not in step.get("expect", {}):
        wrong = sum(row["WRONG"] for row in rows if not any(matching(row, scope) for scope in rate_scopes))
        if wrong:
            failures.append(dict(reason="wrong-outside-quality", observed=wrong, expected=0))
    checks = []

    def record(scope, claim, rate=None, minimum=1):
        counts = scope_counts(rows, scope)
        graded = sum(counts.values())
        wrong = counts["WRONG"]
        decided = counts["CORRECT"] + wrong
        passed = graded == 0 if claim == "absence" else decided >= minimum
        if rate is not None:
            p, q = rate
            passed = passed and q * wrong <= p * decided
        check = dict(scope=scope, claim=claim, graded=graded, decided=decided, wrong=wrong, passed=passed)
        checks.append(check)
        if not passed:
            failures.append(dict(reason=f"{claim}-coverage-failed", **check))

    for scope in expectation_scopes(step):
        record(scope, "quality")
    for item in step.get("quality", []):
        record(item["scope"], "quality", item["max_wrong"], item["min_decided"])
    for scope in step.get("absence", []):
        record(scope, "absence")
    metrics["coverage_checks"] = checks
    return failures


def evaluate(step, metrics, stats_coverage):
    failures = threshold_failures(step, metrics)
    required = minima(step["do"], metrics, stats_coverage)
    if step["do"].startswith("oracle "):
        failures.extend(oracle_checks(step, metrics))
    empty = {key: count for key, count in required.items() if count <= 0}
    waiver = step.get("allow_empty", False)
    if step["do"].startswith("oracle "):
        waiver = waiver and not step.get("quality") and not expectation_scopes(step)
    if not waiver:
        failures.extend(dict(reason="coverage-failed", path=key, observed=count, expected={"min": 1})
                        for key, count in empty.items())
    return failures, required, bool(empty and waiver)
