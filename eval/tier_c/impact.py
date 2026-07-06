"""Impact-stage output contract (design-of-record §4, spec P4): a deterministic
parser for the `impact` stage's fenced ```json block, plus the one-retry helper.
NEVER an LLM judge — this is mechanical extraction, adjacent to citations.py's
mechanical citation extraction.

A parse failure is a RETRY SIGNAL, not a score: `run_impact_with_retry` issues
ONE auto re-prompt; failure-after-retry raises the existing arm-failure path
(`ArmRunError`), never a silent zero score.
"""
from __future__ import annotations

import json
import re
from dataclasses import dataclass, field

_FENCE = re.compile(r"```json\s*(.*?)```", re.DOTALL)


@dataclass(frozen=True)
class ImpactSite:
    file: str
    symbol: str
    reason: str = ""


@dataclass(frozen=True)
class ImpactParseResult:
    ok: bool
    sites: list = field(default_factory=list)          # list[ImpactSite]
    migration_order: list = field(default_factory=list)
    error: str | None = None


def parse_impact_block(text: str) -> ImpactParseResult:
    """Extract the LAST fenced ```json block matching the impact contract
    `{"impact": [{"file","symbol",["reason"]}], "migration_order": [...]}`.

    The LAST block is used (not the first) so an earlier think-out-loud draft
    block doesn't shadow the model's final answer. Never raises; a malformed
    or absent block returns ok=False with a human-readable `error`.
    """
    matches = _FENCE.findall(text)
    if not matches:
        return ImpactParseResult(ok=False, error="no fenced ```json block found")

    raw = matches[-1]
    try:
        obj = json.loads(raw)
    except json.JSONDecodeError as e:
        return ImpactParseResult(ok=False, error=f"invalid JSON: {e}")

    if not isinstance(obj, dict) or "impact" not in obj:
        return ImpactParseResult(ok=False, error="JSON block missing 'impact' key")

    impact = obj.get("impact")
    if not isinstance(impact, list):
        return ImpactParseResult(ok=False, error="'impact' must be a list")

    sites = []
    for i, item in enumerate(impact):
        if not isinstance(item, dict) or "file" not in item or "symbol" not in item:
            return ImpactParseResult(ok=False, error=f"impact[{i}] missing file/symbol")
        sites.append(ImpactSite(file=item["file"], symbol=item["symbol"],
                                reason=item.get("reason", "")))

    migration_order = obj.get("migration_order", [])
    if not isinstance(migration_order, list):
        migration_order = []

    return ImpactParseResult(ok=True, sites=sites, migration_order=migration_order)


RETRY_PROMPT = (
    "Your previous response did not contain a valid impact JSON block. You MUST "
    "include a fenced ```json code block with EXACTLY this shape BEFORE any "
    "free-text discussion:\n"
    '```json\n{"impact": [{"file": "path/to/file.ext", "symbol": "Name", '
    '"reason": "..."}], "migration_order": ["path/to/file.ext:Name", ...]}\n```\n'
    "Re-emit your full answer with this block included."
)


def run_impact_with_retry(run_once, prompt: str):
    """Run the impact stage once; if the arm's text doesn't parse, re-prompt
    ONCE with `RETRY_PROMPT` appended. If the retry ALSO fails to parse, raise
    `ArmRunError` (arm failure — never a silent zero score).

    `run_once`: callable(prompt: str) -> ArmOutput-like (needs only `.text`;
    `.argv` is used for the failure record when present). Returns
    (arm_output, ImpactParseResult) on success.
    """
    from .arm_runner import ArmRunError

    out = run_once(prompt)
    parsed = parse_impact_block(out.text)
    if parsed.ok:
        return out, parsed

    retry_prompt = f"{prompt}\n\n{RETRY_PROMPT}"
    out2 = run_once(retry_prompt)
    parsed2 = parse_impact_block(out2.text)
    if parsed2.ok:
        return out2, parsed2

    raise ArmRunError(
        argv=getattr(out2, "argv", []),
        returncode=-1,
        stderr=f"impact JSON parse failed after 1 retry: {parsed2.error}",
        stdout=out2.text,
    )
