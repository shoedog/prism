"""P/R + Wilson 95% (spec §2.10). Denominators per the §2.8 truth table —
the adjudication transforms are applied by adjudication.apply() before this."""
from __future__ import annotations

import math


def wilson(successes: int, n: int, z: float = 1.959964) -> tuple[float, float, float]:
    if n == 0:
        return (0.0, 0.0, 1.0)
    p = successes / n
    denom = 1 + z * z / n
    center = (p + z * z / (2 * n)) / denom
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / denom
    return (p, max(0.0, center - half), min(1.0, center + half))


def precision_recall(tp: int, fp: int, fn: int) -> dict:
    return {
        "precision": wilson(tp, tp + fp),
        "recall": wilson(tp, tp + fn),
        "tp": tp, "fp": fp, "fn": fn,
    }
