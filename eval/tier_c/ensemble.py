"""Ensemble judging over the live_ask seam.

A judge is asked to START its reply with a verdict token (YES/NO or A/B/TIE) so we
regex it reliably, then give a one-sentence reason (kept verbatim). Two sonnet
judges vote independently; if they agree we trust them, otherwise one opus call
breaks the tie. This collapses the single-pass judge nondeterminism that made the
Part-C precision metric noise-dominated.
"""
from __future__ import annotations
import re
from dataclasses import dataclass, field


@dataclass(frozen=True)
class EnsembleVerdict:
    verdict: str
    escalated: bool
    votes: list = field(default_factory=list)


def parse_verdict(text: str, choices: tuple[str, ...]) -> tuple[str, str, bool]:
    """Return (verdict, reason, unparsed). The model is told to START with the token.

    verdict is the matched choice upper-cased, or "" when the reply does not start
    with a valid token (unparsed=True). reason is the full stripped reply.
    """
    t = (text or "").strip()
    pat = r"^\s*(" + "|".join(re.escape(c) for c in choices) + r")\b"
    m = re.match(pat, t, re.IGNORECASE)
    if m:
        return m.group(1).upper(), t, False
    return "", t, True


def ensemble(ask, prompt: str, choices: tuple[str, ...], *,
             sonnet: str, opus: str, default: str) -> EnsembleVerdict:
    """2 sonnet votes; on disagreement, 1 opus tiebreaker. `default` is used for an
    unparsed reply (conservative)."""
    def one(model: str) -> dict:
        verdict, reason, unparsed = parse_verdict(ask(model, prompt), choices)
        if unparsed:
            verdict = default
        return {"model": model, "verdict": verdict, "reason": reason[:2000], "unparsed": unparsed}

    a = one(sonnet)
    b = one(sonnet)
    votes = [a, b]
    if a["verdict"] == b["verdict"]:
        return EnsembleVerdict(a["verdict"], False, votes)
    c = one(opus)
    votes.append(c)
    return EnsembleVerdict(c["verdict"], True, votes)
