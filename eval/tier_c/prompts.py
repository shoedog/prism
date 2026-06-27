"""Citation-parity stage prompts (spec §3). BOTH arms (prism on/off) are required to
cite file/line/function for every substantive claim, so citation presence is not a tell."""
from __future__ import annotations

_PARITY = ("For every substantive claim about the code, you MUST cite the exact "
           "`file:line` (and `:function` where relevant). Unsupported claims count against you.")

_STAGE = {
    "spec": "Write a short implementation SPEC for this issue, scoped to the stated slice.",
    "plan": "Write a step-by-step PLAN for this spec, scoped to the stated slice.",
}

# Steer directives injected per arm to control tool usage without leaking into
# the model's output text (each steer also instructs the model not to name the tools).
_STEER_PRISM_ON = (
    "TOOL INSTRUCTIONS: Use the navigation tools — nav_callers, nav_callees, "
    "nav_repo_map, nav_nodes_at — to ground every file:line reference before you "
    "write it. Trace callers and callees for every symbol you touch. "
    "Do not name the tools in your spec or plan; write as if you read the code directly."
)

_STEER_CAPABILITY = (
    "GROUNDING INSTRUCTIONS: Ground every file:line by tracing the code structure — "
    "who calls the symbols you touch, what they call, what a change would touch — "
    "then cite exact file:line. "
    "Do not name the tools in your spec or plan; write as if you read the code directly."
)


def stage_prompt(
    stage: str,
    *,
    issue_text: str,
    scoped_slice: str,
    upstream: str = "",
    steer: str = "",
) -> str:
    parts = [_STAGE[stage], _PARITY, f"\nISSUE:\n{issue_text}", f"\nSCOPE (first slice only):\n{scoped_slice}"]
    if upstream:
        parts.append(f"\nUPSTREAM ARTIFACT:\n{upstream}")
    if steer == "prism_on":
        parts.append(f"\n{_STEER_PRISM_ON}")
    elif steer == "capability":
        parts.append(f"\n{_STEER_CAPABILITY}")
    return "\n".join(parts)
