"""Citation-parity stage prompts (spec §3). BOTH arms (prism on/off) are required to
cite file/line/function for every substantive claim, so citation presence is not a tell."""
from __future__ import annotations

_PARITY = ("For every substantive claim about the code, you MUST cite the exact "
           "`file:line` (and `:function` where relevant). Unsupported claims count against you.")

_STAGE = {
    "spec": "Write a short implementation SPEC for this issue, scoped to the stated slice.",
    "plan": "Write a step-by-step PLAN for this spec, scoped to the stated slice.",
}

def stage_prompt(stage: str, *, issue_text: str, scoped_slice: str, upstream: str = "") -> str:
    parts = [_STAGE[stage], _PARITY, f"\nISSUE:\n{issue_text}", f"\nSCOPE (first slice only):\n{scoped_slice}"]
    if upstream:
        parts.append(f"\nUPSTREAM ARTIFACT:\n{upstream}")
    return "\n".join(parts)
