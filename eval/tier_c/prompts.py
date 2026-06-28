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
    "NAVIGATION (do this FIRST): This repository has a structural code-navigation tool, prism: "
    "nav_repo_map (module/dependency layout), nav_callers and nav_callees (who calls a symbol and "
    "what it calls, resolved ACROSS files), and nav_nodes_at (what is defined or called at a line). "
    "Use it to LOCATE the code this issue concerns before you read anything.\n"
    "WHY prism, not grep/Read: grep only matches literal text in files you already guessed, and "
    "misses calls that cross files or go through imports/aliases; prism follows the actual call graph "
    "and import graph, so it finds ALL the callers, callees, and relevant modules — including the ones "
    "grep cannot. On a codebase you do not know, orienting with prism finds the right file and "
    "function faster and more completely than grepping blindly.\n"
    "So: start with nav_repo_map for the layout, use nav_callers/nav_callees on the symbols the issue "
    "is about to trace the relevant code, THEN read the specific sites prism points you to and ground "
    "every file:line citation in what you found. "
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
