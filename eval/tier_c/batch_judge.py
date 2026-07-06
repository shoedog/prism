"""Batch judging over the live_ask seam (perf fix): classify ALL of an arm's items in
ONE model call instead of one ensemble call PER item.

Root problem this replaces: D1 validity (validity.py) and relevance (judges_live.py)
each judged PER CITATION via ensemble.py's 2-sonnet+opus-tiebreak vote. An arm with N
citations cost N x (2-3) cold `claude -p` subprocess calls (~140-220 calls/cell,
>58 min/cell measured on one Part-C cell). A single strong model can classify ALL of
an arm's citations in ONE shot, so classify_batch trades the ensemble for ONE
single-model call per dimension per arm. The head-to-head (SpecQualityJudge, genuinely
subjective + cheap — 2 comparisons/cell) keeps the ensemble unchanged; see
rubric-v2-report.md "Batch judging (perf fix)".
"""
from __future__ import annotations

import re

from .ensemble import parse_verdict


def _build_prompt(intro: str, items: list[str], choices: tuple[str, ...]) -> str:
    body = "\n\n".join(f"--- Item #{i} ---\n{block}" for i, block in enumerate(items, start=1))
    instr = (
        "\n\nReply with EXACTLY one line per item, each line starting with "
        f"#<item number> then one of {'/'.join(choices)}, then a short reason. "
        f'Example: "#1 {choices[0]} because ...".'
    )
    return f"{intro}\n\n{body}{instr}"


def classify_batch(ask, model: str, intro: str, items: list[str],
                   choices: tuple[str, ...], *, default: str) -> list[str]:
    """Classify every item in `items` with ONE `ask(model, prompt)` call.

    Returns a list of verdicts aligned 1:1 with `items` (each in `choices`, upper-cased).
    `items == []` returns `[]` and makes NO `ask` call (anti-fanout: an arm with zero
    batchable items must not spend a call at all).

    Parsing: for each item index n (1-based), look for a line starting with `#<n>`
    (tolerant of leading zeros and `:`/`-`/whitespace between the index and the verdict
    token) followed by one of `choices` (case-insensitive). A missing line for an index,
    or a token on that line that doesn't match any choice, defaults conservatively to
    `default`.

    Singleton fallback: when there is exactly ONE item and the reply contains no
    numbered `#1 ...` line at all (the model ignored the batching instruction and just
    answered with a bare verdict token), the WHOLE reply is parsed the same way
    ensemble.parse_verdict does (first token at the start of the reply). This is
    unambiguous with only one item, and keeps every pre-existing single-primitive-era
    fake (`ask=lambda m, p: "SUPPORTED"`) green without rewriting it for batching.
    """
    if not items:
        return []
    prompt = _build_prompt(intro, items, choices)
    reply = ask(model, prompt) or ""

    choice_pat = "|".join(re.escape(c) for c in choices)
    line_pat = re.compile(r"^\s*#0*(\d+)[\s:\-]*\s*(" + choice_pat + r")\b", re.IGNORECASE)

    found: dict[int, str] = {}
    for line in reply.splitlines():
        m = line_pat.match(line)
        if m:
            idx = int(m.group(1))
            if idx not in found:
                found[idx] = m.group(2).upper()

    if not found and len(items) == 1:
        verdict, _reason, unparsed = parse_verdict(reply, choices)
        if not unparsed:
            return [verdict]

    return [found.get(i, default) for i in range(1, len(items) + 1)]
