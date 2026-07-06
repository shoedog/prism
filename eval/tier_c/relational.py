"""D2 — relational-fact accuracy (mechanical; the "prism grounds, not changes, the
answer" axis). Extraction is ONE cheap LLM call run IDENTICALLY and BLIND on both arms
(same prompt template + model, symmetric) — extraction cannot itself carry a
prism-detectable tell because it never sees which arm produced the text. Verification
is MECHANICAL, never prism:

  - calls(X,Y) / called_by(X,Y) -> a pluggable CallOracle. NullCallOracle (the shipped
    default) ALWAYS returns UNKNOWN: full LSP call-hierarchy wiring (bridging
    tier_a.oracles.LspOracle, which needs a live per-language server lifecycle plus
    symbol -> FunctionDef resolution glue) is a follow-up — see rubric-v2-report.md
    "D2 status" for the explicit escalation. Fail-open, never silently fabricate a
    verdict.
  - depends(modA, modB) -> a NEUTRAL per-language import-text parser (NOT prism
    module-deps): resolves modA to a real file via the checkout's exact/unique-basename
    resolver, parses its import statements, and checks whether modB appears among them.

CAVEAT (owner directive): an oracle that CANNOT confirm a claim reports UNKNOWN — never
CONTRADICTED. Only a positive disproof (a fully-resolved, fully-parsed import list that
provably omits modB, where modB itself resolves to a real, distinct file) may report
CONTRADICTED. Tier-A's oracle_miss category (prism right, LSP wrong) is exactly why: an
absent-oracle-signal must never read as "false" — fail-open toward UNKNOWN throughout,
and the UNKNOWN rate is always reported alongside precision.
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Protocol, runtime_checkable

_KINDS = ("calls", "called_by", "depends")

_EXTRACT_PROMPT = (
    "Read the following technical text about a code change. Extract ONLY relational "
    "claims the text explicitly asserts about code structure, one per line, using "
    "EXACTLY one of these formats (no other text on the line):\n"
    "calls: <caller> -> <callee>\n"
    "called_by: <callee> <- <caller>\n"
    "depends: <modA> -> <modB>\n"
    "If there are none, output exactly NONE.\n\nTEXT:\n"
)

_LINE_RE = re.compile(
    r"^\s*(calls|called_by|depends)\s*:\s*(.+?)\s*(->|<-)\s*(.+?)\s*$", re.MULTILINE
)


@dataclass(frozen=True)
class RelationalClaim:
    kind: str   # "calls" | "called_by" | "depends"
    a: str
    b: str


def parse_relational_claims(raw: str) -> list[RelationalClaim]:
    """Parse the extractor's fixed-format lines into RelationalClaim objects. Garbage /
    unparseable lines (including a bare NONE) are silently skipped."""
    out: list[RelationalClaim] = []
    for kind, left, _arrow, right in _LINE_RE.findall(raw or ""):
        out.append(RelationalClaim(kind=kind, a=left.strip(), b=right.strip()))
    return out


def extract_relational_claims(ask, model: str, text: str) -> list[RelationalClaim]:
    """ONE cheap extractor call. Callers MUST pass the SAME model/prompt template for
    both the off-arm and on-arm text (blind/symmetric by construction — this function
    itself has no notion of "which arm")."""
    if not text:
        return []
    raw = ask(model, _EXTRACT_PROMPT + text)
    return parse_relational_claims(raw)


@runtime_checkable
class CallOracle(Protocol):
    def confirm_calls(self, caller: str, callee: str) -> str: ...  # SUPPORTED|CONTRADICTED|UNKNOWN


class NullCallOracle:
    """Fail-open stub for calls()/called_by() verification (spec D2 escalation: full LSP
    call-hierarchy wiring deferred — see module docstring). ALWAYS returns UNKNOWN so
    this axis degrades gracefully (reported coverage) rather than fabricating a verdict.

    TODO(D2 follow-up): bridge tier_a.oracles.LspOracle.callers/callees — needs a live
    language-server lifecycle (rust-analyzer/gopls/pyright) plus workspace/symbol ->
    FunctionDef resolution for the claimed caller/callee names, which is nontrivial glue
    deferred per the spec's explicit "stub D2, don't block the rest" allowance.
    """

    def confirm_calls(self, caller: str, callee: str) -> str:
        return "UNKNOWN"


# --- depends() neutral import parsing -------------------------------------------------

# Mechanical, neutral (NOT prism) per-language import-list regexes. `python`'s group is
# handled specially below (two alternative capture groups: `from X import` / `import X`).
_IMPORT_PATTERNS: dict[str, re.Pattern] = {
    "go": re.compile(r'"([^"]+)"'),                       # import ( "..." ) or import "..."
    "rust": re.compile(r'\buse\s+([\w:{},\s]+?);'),
    "python": re.compile(r'^\s*(?:from\s+([\w.]+)\s+import|import\s+([\w.]+))', re.MULTILINE),
    "javascript": re.compile(r'''(?:from\s+|require\()['"]([^'"]+)['"]'''),
    "typescript": re.compile(r'''(?:from\s+|require\()['"]([^'"]+)['"]'''),
}


def extract_imports(language: str, source: str) -> list[str]:
    """Mechanical, neutral (NOT prism) import-list extraction. An empty return means
    "could not parse" (unknown language or no import block found) — callers MUST fail
    open to UNKNOWN on an empty list, never CONTRADICTED."""
    lang = (language or "").lower()
    pat = _IMPORT_PATTERNS.get(lang)
    if pat is None or not source:
        return []
    if lang == "python":
        return [a or b for (a, b) in pat.findall(source) if (a or b)]
    return [m.strip() for m in pat.findall(source) if m.strip()]


def _resolve_module_file(co, mod: str) -> str | None:
    """Best-effort neutral resolution of a claimed module/identifier to a real repo file
    via the checkout's exact/unique-basename resolver (Checkout.resolve_rel). Returns
    None (=> UNKNOWN upstream) rather than guess when ambiguous — deliberately
    conservative, matching the fail-open doctrine."""
    if hasattr(co, "resolve_rel"):
        return co.resolve_rel(mod)
    return None


def confirm_depends(co, mod_a: str, mod_b: str, language: str) -> str:
    """Mechanical depends(modA, modB) verification. SUPPORTED when modA's resolved
    file's parsed imports mention modB; CONTRADICTED only when modA's COMPLETE parsed
    import list omits modB AND modB itself resolves to a real, distinct file (positive
    disproof); UNKNOWN in every other case (fail-open)."""
    mod_a_file = _resolve_module_file(co, mod_a)
    if mod_a_file is None or not hasattr(co, "read_file"):
        return "UNKNOWN"
    source = co.read_file(mod_a_file)
    if not source:
        return "UNKNOWN"
    imports = extract_imports(language, source)
    if not imports:
        return "UNKNOWN"  # couldn't parse a complete import list -> can't positively disprove
    needle = mod_b.strip().strip("./").lower()
    if any(needle in imp.lower() or imp.lower() in needle for imp in imports):
        return "SUPPORTED"
    mod_b_file = _resolve_module_file(co, mod_b)
    if mod_b_file is not None and mod_b_file != mod_a_file:
        return "CONTRADICTED"
    return "UNKNOWN"


@dataclass(frozen=True)
class RelationalVerdict:
    claim: RelationalClaim
    status: str   # SUPPORTED | CONTRADICTED | UNKNOWN


@dataclass(frozen=True)
class RelationalReport:
    precision: float       # SUPPORTED / (SUPPORTED + CONTRADICTED); excludes UNKNOWN
    unknown_rate: float    # UNKNOWN / total
    contradicted: int
    total: int
    verdicts: list[RelationalVerdict]


def score_relational_claims(claims: list[RelationalClaim], *, call_oracle: "CallOracle",
                            co, language: str = "") -> RelationalReport:
    verdicts: list[RelationalVerdict] = []
    for claim in claims:
        if claim.kind == "depends":
            status = confirm_depends(co, claim.a, claim.b, language)
        else:  # calls | called_by
            caller, callee = (claim.b, claim.a) if claim.kind == "called_by" else (claim.a, claim.b)
            status = call_oracle.confirm_calls(caller, callee)
        verdicts.append(RelationalVerdict(claim=claim, status=status))
    total = len(verdicts)
    supported = sum(1 for v in verdicts if v.status == "SUPPORTED")
    contradicted = sum(1 for v in verdicts if v.status == "CONTRADICTED")
    unknown = sum(1 for v in verdicts if v.status == "UNKNOWN")
    denom = supported + contradicted
    precision = supported / denom if denom else 0.0
    unknown_rate = unknown / total if total else 0.0
    return RelationalReport(precision=precision, unknown_rate=unknown_rate,
                            contradicted=contradicted, total=total, verdicts=verdicts)
