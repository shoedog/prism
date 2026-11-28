"""Effective arm identity and evidence required for gate claim credit."""

from __future__ import annotations

from .gate_expect import expectation_scopes, scope_counts

NEUTRAL_REVIEW_ARGS = {"--review-min-severity": True, "--cache-dir": True, "--no-cache": False}


def without_review_options(args, options):
    result = []
    skip_value = False
    for arg in args:
        if skip_value:
            if arg.startswith("-"):
                raise ValueError("output-neutral review option requires a value")
            skip_value = False
            continue
        option, equals, _ = arg.partition("=")
        if option in options:
            if not options[option] and equals:
                raise ValueError(f"{option} takes no value")
            skip_value = options[option] and not equals
        else:
            result.append(arg)
    if skip_value:
        raise ValueError("output-neutral review option requires a value")
    return result


def paired_runs(step):
    if "base" in step and "head" in step:
        return step["base"], step["head"]
    return ()


def require_difference(step, configurations):
    if configurations and configurations[0] == configurations[1] and not step.get("identity_control"):
        raise ValueError(f"{step['id']}: identical arms require identity_control:true")


def check_identities(decision):
    for step in decision.steps:
        if step["do"] == "review-diff":
            configs = [(decision.binaries[arm["binary"]]["sha256"],
                        without_review_options(arm["args"], NEUTRAL_REVIEW_ARGS)) for arm in step["arms"]]
        else:
            refs = paired_runs(step)
            if step["do"] == "oracle ts-edges" and "changes" in step:
                if decision.by_id[step["changes"]].get("identity_control"):
                    continue
                refs = paired_runs(decision.by_id[step["changes"]])
            arms = [decision.by_id[ref] for ref in refs]
            if arms:
                base, head = arms
                if any(base.get(key, False) != head.get(key, False) for key in ("no_bytes", "sites")):
                    raise ValueError(f"{step['id']}: run pairing has different analysis options")
                pins = [decision.entries[arm["corpus"]]["tree_sha256"] for arm in arms]
                if all(pins) and pins[0] != pins[1]:
                    raise ValueError(f"{step['id']}: run pairing has different pinned corpus digests")
            configs = [(decision.binaries[arm["binary"]]["sha256"],) for arm in arms]
        require_difference(step, configs)


def positive_wrong_expect(step, metrics):
    bound = step.get("expect", {}).get("denominators.wrong")
    return bound is not None and metrics["denominators"]["wrong"] > 0


def oracle_credit(step, metrics):
    if positive_wrong_expect(step, metrics):
        return "control"
    rows = metrics["denominators"]["scopes"]
    if any(p < q and scope_counts(rows, item["scope"])["CORRECT"] > 0
           for item in step.get("quality", []) for p, q in [item["max_wrong"]]):
        return "product"
    if any(scope_counts(rows, scope)["CORRECT"] > 0 for scope in expectation_scopes(step)):
        return "product"
    global_claims = {"denominators." + field for field in ("correct", "wrong", "graded", "decided")}
    if global_claims.intersection(step.get("expect", {})) and metrics["denominators"]["correct"] > 0:
        return "product"
    return "control" if step.get("absence") or step.get("quality") else "execution"


def step_credit(step, metrics, waived, predecessors):
    if (waived or step.get("identity_control")
            or any(previous["claim_scope"] == "control" for previous in predecessors)):
        return "control"
    kind = step["do"]
    if kind.startswith("oracle "):
        return oracle_credit(step, metrics)
    if kind == "review-diff" and not metrics["eligible_ordinals"]:
        return "control"
    if kind == "diff-stats" and "allow" in step or kind in (
            "diff-edges", "diff-stats", "review-diff") and step.get("expect"):
        return "product"
    return "execution"
