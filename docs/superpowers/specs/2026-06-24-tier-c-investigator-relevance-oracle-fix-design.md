# Tier-C Investigator: Relevance-Oracle Code-Blindness Fix — Design (DRAFT)

> Status: **DRAFT for owner review.** Pairs with handoff `docs/superpowers/handoffs/2026-06-24-tier-c-value-measurement-handoff.md` (§"RUN COMPLETE") and memory `project_prism_measurement_maturity.md`.
> Discovered by: post-mortem of run `full-2026-06-24` (all 16 cells NO-GO with `precision≡0`).

## Problem

The Tier-C investment gate is driven **entirely by citation `precision`** (`report.py:96` `p(sm)=sm.precision`; threshold `_MATERIAL=0.1`). In `full-2026-06-24`, **`precision==0.00` and `recall==0.00` for every one of the 64 arm-runs** (directly verified on the 16 surviving excalidraw arms; the other 3 repos differ only by rare single-citation blips that never clear `_MATERIAL`). Result: every `prism_at_lsp_on` delta ≈ 0 → **every cell is a mechanical NO-GO that never actually weighed prism.** The run produced no signal about prism value.

### Root cause: the relevance oracle is blind to the code it scores

`investigator.score_citations` computes `precision = valid / cited`, where `is_valid = (not hallucination) AND relevant` (`investigator.py:20-21,51-53`). The `relevant` bit comes from `LlmRelevanceJudge.is_relevant` (`judges_live.py:33-37`):

```python
prompt = (f"Issue:\n{issue_text}\n\nIs the code at {cite.file}:{cite.line} "
          f"(symbol {cite.symbol}) actually relevant to fixing this issue? Answer with exactly YES or NO and nothing else.")
return self.ask(self.model, prompt).strip().upper().startswith("YES")
```

The judge receives the issue text and a bare `file:line` **reference**, but **never the code at that line**, and `cite.symbol` is usually `None`. Asked to judge relevance of a location it cannot see, the model conservatively returns non-`YES` → `relevant=False` for essentially every structurally-valid citation → `valid=0` → `precision=0`.

Confirmed in the artifacts: excalidraw `opus-4.8` spec cited 13 locations, all `file_ok/line_ok/symbol_ok=true`, **all `relevant=false`**. The only `relevant=true` verdicts in the whole run are **hallucinations** — for bad-structure cites the judge is skipped (`investigator.py:44` guards on `file_ok and line_ok and symbol_ok`) and `relevant` keeps its default `True`, so `is_valid` is still `False` via the `not hallucination` term. That inversion is why `hallucinations == relevant_count` per arm.

## Goal

Give the relevance oracle the **actual cited code** (the line plus a small context window) so it can make a real relevance judgment, restoring a non-degenerate `precision` signal — while keeping the oracle **prism-free** (neutral git/file primitives only, per spec §6a).

Non-goal: re-tuning recall's `claim_count` denominator, adding the rank-judge as a co-primary objective, or per-issue artifact namespacing — those are tracked separately (handoff NEXT STEPS 2–4).

## The Fix

Four small, seam-respecting changes. The enabling read (`co.read_line`) is already called two lines away in `verify_citation`, so this adds a context-window read and threads the snippet through the existing seam.

### 1. `checkout.py` — add a neutral context-window reader
```python
def read_window(self, rel: str, line: int, ctx: int = 3) -> str | None:
    """Cited line ± ctx lines, prism-free, with a '>' marker on the cited line.
    None if the file is missing or the line is out of range."""
    p = self.root / rel
    if not p.is_file():
        return None
    lines = p.read_text(errors="replace").splitlines()
    if not (1 <= line <= len(lines)):
        return None
    lo, hi = max(1, line - ctx), min(len(lines), line + ctx)
    return "\n".join(("> " if n == line else "  ") + lines[n - 1] for n in range(lo, hi + 1))
```

### 2. `interfaces.py` — extend the `RelevanceJudge` seam (breaking signature)
```python
class RelevanceJudge(Protocol):
    def is_relevant(self, cite: Citation, issue_text: str, code: str) -> bool: ...
```
`code` is the `read_window` snippet (`""` when unavailable — judge then falls back to the old reference-only behavior, which is acceptable since structure already failed in that path).

### 3. `investigator.py` — fetch and pass the snippet; update the in-module fakes
```python
def verify_citation(co, cite, *, issue_text="", relevance=None) -> CitationVerdict:
    ...
    relevant = True
    if relevance is not None and file_ok and line_ok and symbol_ok:
        code = co.read_window(cite.file, cite.line) or ""
        relevant = relevance.is_relevant(cite, issue_text, code)
    return CitationVerdict(cite, file_ok, line_ok, symbol_ok, relevant)
```
`RelevanceAllTrue` / `RelevanceNone` gain the `code` parameter.

### 4. `judges_live.py` — put the code in the prompt
```python
def is_relevant(self, cite: Citation, issue_text: str, code: str) -> bool:
    prompt = (f"Issue:\n{issue_text}\n\nCited code at {cite.file}:{cite.line} "
              f"(the '>' line is the citation):\n```\n{code}\n```\n\n"
              f"Is THIS code relevant to fixing the issue? Answer with exactly YES or NO and nothing else.")
    return self.ask(self.model, prompt).strip().upper().startswith("YES")
```
The conservative `.startswith("YES")` rule is retained.

### 5. Test fakes (`eval/tests/test_tc_*.py`) — update any `RelevanceJudge` impl to the 3-arg signature.

## TDD plan (subagent-driven, fresh impl per step)

1. **`read_window`** — returns cited line + ±ctx, marks the cited line, `None` on missing file / out-of-range line; clamps at file edges. (`test_tc_checkout.py`)
2. **`verify_citation` passes code** — a spy `RelevanceJudge` records its `code` arg; assert it is non-empty and contains the cited source for a real fixture line, empty-string for a hallucinated line. (`test_tc_investigator.py`)
3. **`LlmRelevanceJudge` uses the code** — a fake `ask` that returns `YES` only when the prompt contains a sentinel source token; assert `is_relevant` is `True` with code present and `False` when `code=""`. (`test_tc_judges_live.py`)
4. **Regression pin: precision > 0** — investigator over a fixture whose cited line is the fix locus, with a stub relevance that returns `YES` for the locus and `NO` otherwise → `precision > 0` and tracks the relevant fraction. Pins against the `≡0` bug. (`test_tc_investigator.py`)
5. Full `eval/tests` green; `cargo`-side untouched.

## Methodology caution (gating)

Changing the oracle **changes every future `precision`/`recall` number** — pre- and post-fix runs are not comparable. Therefore:
- Land behind the standard spec → **codex `gpt-5.5` xhigh review** → subagent-TDD → review pipeline. Not a hotfix.
- After landing, **re-baseline cheaply**: re-run only the judge/investigator step on the **recovered** spec/plan text (opus arms in `~/.claude/projects/*-T-tc-co-*/`, gpt arms in `~/.codex/sessions/`) — no expensive model-arm re-run, no new `--live` spend on the arms.
- Then evaluate **judge calibration**: confirm the fixed oracle discriminates (not a new all-`YES` degeneracy). If precision saturates near 1.0 for all arms, tighten the prompt or widen `ctx`.

## Considered alternatives

- **Whole enclosing function instead of ±N lines** — richer context but needs structural parsing (tree-sitter). YAGNI; ±3 lines is enough for a relevance call. Revisit only if post-fix recall stays low.
- **Drop relevance; score precision = structural validity only** — simplest, but then citing *any* real line (irrelevant included) scores → defeats grounding. Reject.
- **Replace precision with the rank judge** — the rank judge worked and carried the only signal, but it is a *preference* (A-vs-B) signal, not an *absolute* grounding signal. Keep both: this spec fixes precision; adding rank as a **co-primary** objective is handoff NEXT-STEP 2 (separate change).

## Follow-up noted (not in this change)

Spec §6a calls the relevance seam "secondary, **audit-sampled**," but the wiring applies it to **every** citation as the precision denominator. Whether to sample (cheaper, noisier) vs. score-all (current) is a separate design question; this fix keeps score-all and only makes each call see the code.
