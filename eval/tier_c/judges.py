"""Judge combination + bias instrumentation (spec §6b). Consensus cancels symmetric
family bias; family_bias() reports residual; detectable() gates the subjective channel
(codex new-1/new-7: if prism condition is detectable, the judge prism-delta is INVALID)."""
from __future__ import annotations
import random

def _points(rankings):
    ids = {c for r in rankings.values() for c in r}
    pts = {c: 0 for c in ids}
    for r in rankings.values():
        n = len(r)
        for pos, c in enumerate(r):
            pts[c] += (n - pos)
    return pts

def borda_consensus(rankings, seed=None):
    pts = _points(rankings)
    if seed is None:
        return sorted(pts, key=lambda c: (-pts[c], c))
    rng = random.Random(seed)
    groups = {}
    for c, p in pts.items():
        groups.setdefault(p, []).append(c)
    out = []
    for p in sorted(groups, reverse=True):
        g = groups[p][:]
        rng.shuffle(g)
        out.extend(g)
    return out

def has_tie(rankings) -> bool:
    pts = sorted(_points(rankings).values(), reverse=True)
    return len(pts) >= 2 and pts[0] == pts[1]

def _mean_rank(order: list[str], ids: set[str]) -> float:
    ranks = [i for i, c in enumerate(order) if c in ids]
    return sum(ranks) / len(ranks) if ranks else 0.0

def family_bias(rankings: dict[str, list[str]], family_of: dict[str, str]) -> float:
    """How much each family-judge favors its OWN family vs the other judge does.
    0 = no own-family inflation; larger = stronger 'judges-own-family' trend."""
    judge_fams = list(rankings.keys())
    if len(judge_fams) != 2:
        return 0.0
    jf_a, jf_b = judge_fams
    own_ids = {f: {c for c, fam in family_of.items() if fam == f} for f in judge_fams}
    # mean rank (lower=better) each judge gives family jf_a:
    a_to_a = _mean_rank(rankings[jf_a], own_ids[jf_a])
    b_to_a = _mean_rank(rankings[jf_b], own_ids[jf_a])
    a_to_b = _mean_rank(rankings[jf_a], own_ids[jf_b])
    b_to_b = _mean_rank(rankings[jf_b], own_ids[jf_b])
    # own-family advantage: other judge ranks my family WORSE (higher) than I do
    return max(0.0, ((b_to_a - a_to_a) + (a_to_b - b_to_b)) / 2.0)

def detectable(correct: int, n: int, threshold: float = 0.7) -> bool:
    return n > 0 and (correct / n) > threshold
