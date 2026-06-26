"""pass^k aggregation (spec §Success criteria). HEADLINE = nav invocation pass^5 rate
(fraction of nav probes where all k trials fired the right nav tool). Activation reported
alongside. Negatives pass when they DON'T over-reach (invocation False all k)."""
from __future__ import annotations
import json, os

def passes_k(trial_flags: list[bool]) -> bool:
    return bool(trial_flags) and all(trial_flags)

def summarize(per_probe: dict) -> dict:
    nav = {pid: r for pid, r in per_probe.items() if r["kind"] == "nav"}
    neg = {pid: r for pid, r in per_probe.items() if r["kind"] == "negative"}
    nav_inv = sum(passes_k(r["invocation"]) for r in nav.values())
    nav_act = sum(passes_k(r["activation"]) for r in nav.values())
    # negative passes^k = no prism invoked across all k (no over-reach)
    neg_ok = sum(passes_k([not x for x in r["invocation"]]) for r in neg.values())
    return {
        "nav_count": len(nav),
        "nav_invocation_pass5_rate": (nav_inv / len(nav)) if nav else 0.0,
        "nav_activation_pass5_rate": (nav_act / len(nav)) if nav else 0.0,
        "negative_count": len(neg),
        "negative_no_overreach_rate": (neg_ok / len(neg)) if neg else 0.0,
        "per_probe": per_probe,
    }

def write_benchmark(summary: dict, results_root: str, identifier: str) -> str:
    os.makedirs(results_root, exist_ok=True)
    path = os.path.join(results_root, f"benchmark-{identifier}.json")
    with open(path, "w") as f:
        json.dump(summary, f, indent=2)
    return path
