"""R2 — Q3 LLM disambiguator (resolver-fix-spec.md). Invoked ONLY when the layered
resolver (checkout.py::Checkout.resolve_cite, R1) still has >=2 same-basename
candidates after the deterministic exact/unique-basename/line-range/symbol/token
layers. Cheap (haiku) — NOT called per-citation in the common case; R1's layered
resolve is specifically designed so this is rare (the noqa.rs artifact alone is fully
fixed by the deterministic line-range layer).

Behind the harness's existing ask() seam (llm.live_ask / the judges' `ask` callable) —
never a new subprocess/model-call mechanism.
"""
from __future__ import annotations

from dataclasses import dataclass

from .ensemble import parse_verdict

# Owner-chosen model: cheap, haiku. Matches the MODEL_CLI internal-alias convention
# ("opus-4.8", "sonnet-4.6", ...) used throughout llm.py/judges_live.py.
Q3_MODEL = "haiku-4.5"

_LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"


@dataclass(frozen=True)
class DisambiguationResult:
    index: int | None   # chosen candidate index, or None on NONE / abstain / unparse
    raw: str


def _label(i: int) -> str:
    return _LETTERS[i] if i < len(_LETTERS) else f"#{i}"


def build_prompt(claim_text: str, candidates: list[str]) -> str:
    """Prompt: the claim sentence + each candidate's code window, labeled
    Candidate A/B/... (spec R2)."""
    lines = [
        "A written claim cites a file:line location, but the bare filename it used "
        "matches multiple files in this repository. Given the claim sentence and the "
        "code at each candidate location, say which candidate the citation refers to.",
        "",
        f"Claim sentence:\n{claim_text or '(no claim sentence available)'}",
        "",
    ]
    for i, window in enumerate(candidates):
        lines.append(f"Candidate {_label(i)}:\n{window}\n")
    choices = ", ".join(_label(i) for i in range(len(candidates)))
    lines.append(
        f"Start your reply with EXACTLY one token — one of {choices}, or NONE if you "
        "genuinely cannot tell — then one sentence why."
    )
    return "\n".join(lines)


def disambiguate(ask, claim_text: str, candidates: list[str], *,
                 model: str = Q3_MODEL) -> DisambiguationResult:
    """Ask a cheap model to pick which candidate a claim refers to.

    Returns the chosen index, or None on a NONE reply or an unparseable reply —
    abstain is the conservative default (the caller treats it as AMBIGUOUS, never
    guessing a fabricated pin — the load-bearing principle: never invent a resolution
    that isn't backed by the model actually picking one candidate).
    """
    if not candidates:
        return DisambiguationResult(index=None, raw="")
    if len(candidates) == 1:
        return DisambiguationResult(index=0, raw="")
    labels = [_label(i) for i in range(len(candidates))]
    prompt = build_prompt(claim_text, candidates)
    raw = ask(model, prompt) or ""
    verdict, reason, unparsed = parse_verdict(raw, tuple(labels) + ("NONE",))
    if unparsed or verdict == "NONE":
        return DisambiguationResult(index=None, raw=reason)
    return DisambiguationResult(index=labels.index(verdict), raw=reason)


def make_disambiguator(ask, model: str | None = None):
    """Build a `(claim_text, windows) -> int | None` callable for
    Checkout.resolve_cite's `disambiguate=` seam, wrapping the ask() judge seam.

    Any exception from `ask` (network/subprocess failure) is swallowed and treated as
    abstain — a judge-call failure must never crash scoring; conservative AMBIGUOUS is
    the safe fallback, matching the module's abstain-on-unparse contract.
    """
    def _fn(claim_text: str, windows: list[str]) -> int | None:
        try:
            result = disambiguate(ask, claim_text, windows, model=model or Q3_MODEL)
        except Exception:
            return None
        return result.index
    return _fn
