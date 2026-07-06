"""Citation-parity stage prompts (spec §3). BOTH arms (prism on/off) are required to
cite file/line/function for every substantive claim, so citation presence is not a tell."""
from __future__ import annotations

_PARITY = ("For every substantive claim about the code, you MUST cite the exact "
           "`file:line` (and `:function` where relevant). Unsupported claims count against you.")

_INLINE = ("Output the full spec/plan INLINE in your final response — do NOT write it to a file. "
           "Your response text IS the deliverable.")

_IMPACT_CONTRACT = (
    "You are analyzing a proposed SEMANTIC/behavioral change to a symbol in this "
    "codebase (not a syntactic signature change). Enumerate every site (file + "
    "enclosing symbol) that must be reviewed or changed because its behavior "
    "depends on the affected symbol — including sites reached only through "
    "interface/trait dispatch, callback registration, or other indirection "
    "where the symbol's NAME never literally appears in that file. Do not "
    "limit yourself to sites a plain text search would find.\n\n"
    "First, produce a fenced ```json code block with EXACTLY this shape:\n"
    "```json\n"
    '{"impact": [{"file": "path/to/file.ext", "symbol": "EnclosingFunctionOrMethod", '
    '"reason": "why this site depends on the change"}], '
    '"migration_order": ["path/to/file.ext:EnclosingFunctionOrMethod", "..."]}\n'
    "```\n"
    "Then, AFTER the JSON block, add free-text design discussion (risk notes, "
    "grouping by module, migration ordering rationale, anything else relevant)."
)

_STAGE = {
    "spec": "Write a short implementation SPEC for this issue, scoped to the stated slice.",
    "plan": "Write a step-by-step PLAN for this spec, scoped to the stated slice.",
    "impact": _IMPACT_CONTRACT,
}

# Steer directives injected per arm to control tool usage without leaking into
# the model's output text (each steer also instructs the model not to name the tools).
_STEER_PRISM_ON = (
    "NAVIGATION (do this FIRST, before any grep/Read): This repository has a structural "
    "code-navigation tool, prism, exposed as MCP tools you can call DIRECTLY by name — you do NOT "
    "need to search or discover them, just call them:\n"
    "  - mcp__prism__nav_repo_map  (module/dependency layout)\n"
    "  - mcp__prism__nav_callers / mcp__prism__nav_callees  (who calls a symbol / what it calls, "
    "resolved ACROSS files)\n"
    "  - mcp__prism__nav_nodes_at  (what is defined or called at a file:line)\n"
    "These tools register a second or two after the session starts. If your FIRST prism call returns "
    "'tool not found', 'No such tool', or an empty/no-matching-tools result, WAIT briefly and RETRY "
    "the SAME call once — do NOT fall back to grep/Read for navigation until a retried prism call has "
    "actually returned results.\n"
    "Always pass verbosity: \"detailed\" so each call returns code snippets and symbol names, not just "
    "line numbers — one detailed call tells you far more than a concise one.\n"
    "WHY prism, not grep/Read: grep only matches literal text in files you already guessed, and "
    "misses calls that cross files or go through imports/aliases; prism follows the actual call graph "
    "and import graph, so it finds ALL the callers, callees, and relevant modules — including the ones "
    "grep cannot. On a codebase you do not know, orienting with prism finds the right file and "
    "function faster and more completely than grepping blindly.\n"
    "So: START with mcp__prism__nav_repo_map for the layout, then use nav_callers/nav_callees "
    "(verbosity detailed) on the symbols the issue names to trace the relevant code, THEN read the "
    "specific sites prism points you to and ground every file:line citation in what you found. "
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
    parts = [_STAGE[stage], _PARITY, _INLINE, f"\nISSUE:\n{issue_text}", f"\nSCOPE (first slice only):\n{scoped_slice}"]
    if upstream:
        parts.append(f"\nUPSTREAM ARTIFACT:\n{upstream}")
    if steer == "prism_on":
        parts.append(f"\n{_STEER_PRISM_ON}")
    elif steer == "capability":
        parts.append(f"\n{_STEER_CAPABILITY}")
    return "\n".join(parts)
