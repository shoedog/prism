"""pass^k aggregation (spec §Success criteria). HEADLINE = nav invocation pass^5 rate
(fraction of nav probes where all k trials fired the right nav tool). Activation reported
alongside. Negatives pass when they DON'T over-reach (invocation False all k)."""
from __future__ import annotations
import collections, json, os

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

def prism_invoked(traj) -> bool:
    """Any-call invocation: did the trajectory fire ANY prism nav tool (open-ended-task metric)."""
    return bool(traj.prism_nav_calls())

def summarize_cells(cells: dict) -> dict:
    """cells: {cell_id: {sample_id: [(invoked: bool, skill_loaded: str|None), ... k trials]}}.
    Per cell: invocation_rate (over all sample*trial), pass5_rate (samples with all-k invoked),
    skill_attribution (count of which skill loaded across invoked runs)."""
    out = {}
    for cell, samples in cells.items():
        runs = [r for trials in samples.values() for r in trials]
        n = len(runs) or 1
        invoked = sum(1 for inv, _ in runs if inv)
        pass5 = sum(1 for trials in samples.values() if trials and all(inv for inv, _ in trials))
        attr = collections.Counter(sk for inv, sk in runs if inv and sk)
        out[cell] = {
            "invocation_rate": invoked / n,
            "pass5_rate": pass5 / (len(samples) or 1),
            "n_samples": len(samples), "n_runs": len(runs),
            "skill_attribution": dict(attr),
        }
    return out
