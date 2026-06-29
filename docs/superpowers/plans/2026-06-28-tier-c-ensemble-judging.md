# Tier-C Ensemble Judging + Head-to-Head — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single-pass relevance judge with a 2-sonnet→opus-tiebreaker ensemble (YES/NO-first verdict + captured reason), and add an anonymized head-to-head spec-quality judge, then validate by rescoring the saved prometheus v1–v4 runs.

**Architecture:** A new `tier_c/ensemble.py` holds the verdict parser + ensemble runner over the existing `live_ask` seam. `judges_live.py` routes `LlmRelevanceJudge` through it and adds `SpecQualityJudge`. `partc.py`/`cli.py` add a per-cell `head_to_head` and capture per-cite ensemble votes. Coverage is deferred (no code).

**Tech Stack:** Python, pytest, `uv run` from `eval/`. Judges call `sonnet-4.6` / `opus-4.8` via `tier_c/llm.py:live_ask` (already MCP/tool-isolated). Tests inject a fake `ask`.

All commands run from `/Users/wesleyjinks/code/slicing/eval`. Never use `git add -A`; stage explicit paths. Never stage `tier_c/runs/`. Commit trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

## File Structure

- Create: `eval/tier_c/ensemble.py` — `EnsembleVerdict`, `parse_verdict`, `ensemble`.
- Modify: `eval/tier_c/llm.py` — add `JUDGE_TIEBREAKER = "opus-4.8"`.
- Modify: `eval/tier_c/judges_live.py` — `LlmRelevanceJudge` via ensemble + `relevance()`; `_RecordingRelevanceJudge` records votes; new `SpecQualityJudge`.
- Modify: `eval/tier_c/partc.py` — `PartCCell.head_to_head`; `run_partc_cell` calls `comps.head_to_head`; `render_partc` prints it.
- Modify: `eval/tier_c/cli.py` — `_LivePartCComps.head_to_head`; score() already records via `_RecordingRelevanceJudge`.
- Create tests: `eval/tests/test_tc_ensemble.py`; extend `eval/tests/test_tc_judges_live.py`, `eval/tests/test_tc_run_partc.py`.

---

## Task 1: Ensemble core (`tier_c/ensemble.py`)

**Files:**
- Create: `eval/tier_c/ensemble.py`
- Test: `eval/tests/test_tc_ensemble.py`

- [ ] **Step 1: Write the failing tests**

```python
# eval/tests/test_tc_ensemble.py
from tier_c.ensemble import EnsembleVerdict, parse_verdict, ensemble


def test_parse_verdict_yes_no_first():
    assert parse_verdict("YES, because line 42 is the buggy branch.", ("YES", "NO")) == \
        ("YES", "YES, because line 42 is the buggy branch.", False)
    assert parse_verdict("no - unrelated helper", ("YES", "NO"))[0] == "NO"


def test_parse_verdict_abtie():
    assert parse_verdict("TIE — both miss the root cause", ("A", "B", "TIE"))[0] == "TIE"
    assert parse_verdict("B is better", ("A", "B", "TIE"))[0] == "B"


def test_parse_verdict_unparsed_flagged():
    v, reason, unparsed = parse_verdict("I think probably yes", ("YES", "NO"))
    assert v == "" and unparsed is True and reason == "I think probably yes"


def test_ensemble_agree_no_opus():
    calls = []
    def ask(model, prompt):
        calls.append(model)
        return "YES, relevant"
    ev = ensemble(ask, "p", ("YES", "NO"), sonnet="sonnet-4.6", opus="opus-4.8", default="NO")
    assert ev.verdict == "YES" and ev.escalated is False
    assert calls == ["sonnet-4.6", "sonnet-4.6"]   # 2 sonnet, no opus
    assert len(ev.votes) == 2


def test_ensemble_disagree_escalates_to_opus():
    seq = iter(["YES because X", "NO because Y"])   # two sonnets split
    calls = []
    def ask(model, prompt):
        calls.append(model)
        return "NO, opus says unrelated" if model == "opus-4.8" else next(seq)
    ev = ensemble(ask, "p", ("YES", "NO"), sonnet="sonnet-4.6", opus="opus-4.8", default="NO")
    assert ev.escalated is True
    assert ev.verdict == "NO"                       # opus tiebreaker wins
    assert calls == ["sonnet-4.6", "sonnet-4.6", "opus-4.8"]
    assert len(ev.votes) == 3 and ev.votes[-1]["model"] == "opus-4.8"


def test_ensemble_unparsed_uses_default():
    def ask(model, prompt):
        return "hmm, hard to say"                   # unparsed -> default NO
    ev = ensemble(ask, "p", ("YES", "NO"), sonnet="sonnet-4.6", opus="opus-4.8", default="NO")
    assert ev.verdict == "NO" and ev.escalated is False
    assert all(v["unparsed"] for v in ev.votes)
```

- [ ] **Step 2: Run to verify they fail**

Run: `uv run python -m pytest tests/test_tc_ensemble.py -q`
Expected: FAIL (`ModuleNotFoundError: tier_c.ensemble`).

- [ ] **Step 3: Implement `tier_c/ensemble.py`**

```python
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
    verdict: str                       # one of `choices` (upper-case), or `default` if all unparsed
    escalated: bool                    # were the two sonnet votes split (opus consulted)?
    votes: list = field(default_factory=list)   # [{model, verdict, reason, unparsed}, ...]


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
```

- [ ] **Step 4: Run to verify they pass**

Run: `uv run python -m pytest tests/test_tc_ensemble.py -q`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/ensemble.py eval/tests/test_tc_ensemble.py
git commit -m "feat(tier-c): ensemble verdict parser + 2-sonnet/opus-tiebreaker runner"
```

---

## Task 2: `JUDGE_TIEBREAKER` constant (`tier_c/llm.py`)

**Files:**
- Modify: `eval/tier_c/llm.py`
- Test: extend `eval/tests/test_tc_llm.py`

- [ ] **Step 1: Write the failing test** (append to `tests/test_tc_llm.py`)

```python
def test_judge_tiebreaker_is_opus():
    from tier_c.llm import JUDGE_TIEBREAKER, MODEL_CLI
    assert JUDGE_TIEBREAKER in MODEL_CLI
    assert MODEL_CLI[JUDGE_TIEBREAKER] == ("claude", "opus")
```

- [ ] **Step 2: Run to verify it fails**

Run: `uv run python -m pytest tests/test_tc_llm.py::test_judge_tiebreaker_is_opus -q`
Expected: FAIL (`ImportError: JUDGE_TIEBREAKER`).

- [ ] **Step 3: Implement** — add below `JUDGE_MODEL` in `tier_c/llm.py`:

```python
# Ensemble tiebreaker: consulted ONLY when the two sonnet judges disagree (see ensemble.py).
# Opus is authoritative-but-pricier, so it adjudicates contested calls, not every call.
JUDGE_TIEBREAKER = "opus-4.8"
```

- [ ] **Step 4: Run to verify it passes**

Run: `uv run python -m pytest tests/test_tc_llm.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/llm.py eval/tests/test_tc_llm.py
git commit -m "feat(tier-c): JUDGE_TIEBREAKER=opus-4.8 for ensemble escalation"
```

---

## Task 3: Relevance judge via ensemble (`tier_c/judges_live.py`)

**Files:**
- Modify: `eval/tier_c/judges_live.py`
- Test: extend `eval/tests/test_tc_judges_live.py`

- [ ] **Step 1: Write the failing tests** (append to `tests/test_tc_judges_live.py`)

```python
from tier_c.judges_live import LlmRelevanceJudge, _RecordingRelevanceJudge
from tier_c.model import Citation


def _cite():
    return Citation(file="model/labels/regexp.go", line=96, symbol=None)


def test_relevance_ensemble_yes_when_both_sonnet_yes(monkeypatch):
    j = LlmRelevanceJudge(lambda m, p: "YES because it installs trueMatcher")
    ev = j.relevance(_cite(), "issue", "code")
    assert ev.verdict == "YES" and ev.escalated is False and len(ev.votes) == 2
    assert j.is_relevant(_cite(), "issue", "code") is True


def test_relevance_ensemble_escalates(monkeypatch):
    seq = iter(["YES x", "NO y"])
    def ask(m, p):
        return "NO opus" if m == "opus-4.8" else next(seq)
    j = LlmRelevanceJudge(ask)
    ev = j.relevance(_cite(), "issue", "code")
    assert ev.escalated is True and ev.verdict == "NO"


def test_recording_relevance_captures_votes():
    records = []
    inner = LlmRelevanceJudge(lambda m, p: "YES it is the root cause")
    rec = _RecordingRelevanceJudge(inner, records)
    assert rec.is_relevant(_cite(), "issue", "code") is True
    r = records[0]
    assert r["file"] == "model/labels/regexp.go" and r["line"] == 96
    assert r["relevant"] is True and r["escalated"] is False
    assert len(r["votes"]) == 2 and "reason" in r["votes"][0]
```

- [ ] **Step 2: Run to verify they fail**

Run: `uv run python -m pytest tests/test_tc_judges_live.py -q -k "ensemble or recording_relevance_captures"`
Expected: FAIL (`relevance` attr / votes key missing).

- [ ] **Step 3: Implement** — replace the `LlmRelevanceJudge` and `_RecordingRelevanceJudge` classes in `judges_live.py` with:

```python
class LlmRelevanceJudge:
    """Per-citation relevance via the 2-sonnet/opus ensemble (ensemble.py)."""
    def __init__(self, ask, model: str | None = None, *, opus: str | None = None):
        from .llm import JUDGE_MODEL, JUDGE_TIEBREAKER
        self.ask = ask
        self.sonnet = model or JUDGE_MODEL
        self.opus = opus or JUDGE_TIEBREAKER

    def relevance(self, cite: Citation, issue_text: str, code: str = ""):
        from .ensemble import ensemble
        code_section = f"\n\nCode at {cite.file}:{cite.line}:\n{code}" if code else ""
        prompt = (f"Issue:\n{issue_text}{code_section}\n\nIs the code at {cite.file}:{cite.line} "
                  f"(symbol {cite.symbol}) actually relevant to fixing this issue? "
                  f"Start your reply with YES or NO, then one sentence why.")
        return ensemble(self.ask, prompt, ("YES", "NO"),
                        sonnet=self.sonnet, opus=self.opus, default="NO")

    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        return self.relevance(cite, issue_text, code).verdict == "YES"


class _RecordingRelevanceJudge:
    """Wraps LlmRelevanceJudge; appends one record per cite to *records*:
    {file, line, symbol, verdict, escalated, votes, relevant}."""
    def __init__(self, inner: "LlmRelevanceJudge", records: list):
        self.inner, self.records = inner, records

    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        ev = self.inner.relevance(cite, issue_text, code)
        self.records.append({
            "file": cite.file, "line": cite.line, "symbol": cite.symbol,
            "verdict": ev.verdict, "escalated": ev.escalated, "votes": ev.votes,
            "relevant": ev.verdict == "YES",
        })
        return ev.verdict == "YES"
```

- [ ] **Step 4: Run to verify they pass**

Run: `uv run python -m pytest tests/test_tc_judges_live.py -q`
Expected: PASS (existing + new). If an existing test asserted the old `{prompt,response}` record schema, update it to the new `{verdict,escalated,votes}` schema.

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/judges_live.py eval/tests/test_tc_judges_live.py
git commit -m "feat(tier-c): relevance judge via ensemble; recorder captures votes+reasons"
```

---

## Task 4: Head-to-head `SpecQualityJudge` (`tier_c/judges_live.py`)

**Files:**
- Modify: `eval/tier_c/judges_live.py`
- Test: extend `eval/tests/test_tc_judges_live.py`

- [ ] **Step 1: Write the failing tests**

```python
from tier_c.judges_live import SpecQualityJudge


def test_spec_quality_deterministic_assignment():
    """Same (off,on) inputs -> same A/B assignment (rescore reproducibility)."""
    j = SpecQualityJudge(lambda m, p: "A is better")
    r1 = j.compare("issue", "OFF spec body", "ON spec body")
    r2 = j.compare("issue", "OFF spec body", "ON spec body")
    assert r1["swap"] == r2["swap"]


def test_spec_quality_winner_maps_back_to_arm():
    # Force a known swap by choosing texts, then check A maps to the right arm.
    calls = {}
    def ask(m, p):
        calls["prompt"] = p
        return "A wins, clearer root cause"
    j = SpecQualityJudge(ask)
    r = j.compare("issue", "OFF", "ON")
    a_is = "on" if r["swap"] else "off"
    assert r["winner"] == a_is


def test_spec_quality_tie():
    j = SpecQualityJudge(lambda m, p: "TIE, both miss it")
    r = j.compare("issue", "OFF", "ON")
    assert r["winner"] == "tie" and r["escalated"] is False


def test_spec_quality_escalates_on_split():
    seq = iter(["A better", "B better"])
    def ask(m, p):
        return "TIE final" if m == "opus-4.8" else next(seq)
    r = SpecQualityJudge(ask).compare("issue", "OFF", "ON")
    assert r["escalated"] is True and r["winner"] == "tie"
```

- [ ] **Step 2: Run to verify they fail**

Run: `uv run python -m pytest tests/test_tc_judges_live.py -q -k spec_quality`
Expected: FAIL (`ImportError: SpecQualityJudge`).

- [ ] **Step 3: Implement** — append to `judges_live.py`:

```python
class SpecQualityJudge:
    """Anonymized head-to-head: which spec better identifies root cause + fix.

    A/B assignment is derived deterministically from sha256(off+on) so position
    bias cannot favor an arm AND rescores reproduce the same assignment (no RNG).
    Runs through the same 2-sonnet/opus ensemble; winner maps back to off/on/tie.
    """
    def __init__(self, ask, model: str | None = None, *, opus: str | None = None):
        from .llm import JUDGE_MODEL, JUDGE_TIEBREAKER
        self.ask = ask
        self.sonnet = model or JUDGE_MODEL
        self.opus = opus or JUDGE_TIEBREAKER

    def compare(self, issue_text: str, off_spec: str, on_spec: str) -> dict:
        import hashlib
        from .ensemble import ensemble
        swap = int(hashlib.sha256((off_spec + on_spec).encode("utf-8")).hexdigest(), 16) % 2 == 1
        spec_a, spec_b = (on_spec, off_spec) if swap else (off_spec, on_spec)
        prompt = (f"Issue:\n{issue_text}\n\nTwo implementation specs were written for this issue.\n\n"
                  f"=== SPEC A ===\n{spec_a}\n\n=== SPEC B ===\n{spec_b}\n\n"
                  f"Which spec better identifies the root cause and a correct, complete fix? "
                  f"Ignore writing style, length, and formatting — judge substance only. "
                  f"Start your reply with A, B, or TIE, then one sentence why.")
        ev = ensemble(self.ask, prompt, ("A", "B", "TIE"),
                      sonnet=self.sonnet, opus=self.opus, default="TIE")
        if ev.verdict == "TIE":
            winner = "tie"
        else:
            a_is = "on" if swap else "off"
            b_is = "off" if swap else "on"
            winner = a_is if ev.verdict == "A" else b_is
        return {"winner": winner, "escalated": ev.escalated, "votes": ev.votes, "swap": swap}
```

- [ ] **Step 4: Run to verify they pass**

Run: `uv run python -m pytest tests/test_tc_judges_live.py -q -k spec_quality`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/judges_live.py eval/tests/test_tc_judges_live.py
git commit -m "feat(tier-c): anonymized head-to-head SpecQualityJudge via ensemble"
```

---

## Task 5: `PartCCell.head_to_head` + render (`tier_c/partc.py`)

**Files:**
- Modify: `eval/tier_c/partc.py`
- Test: extend `eval/tests/test_tc_run_partc.py`

- [ ] **Step 1: Write the failing test** (append to `tests/test_tc_run_partc.py`)

```python
def test_run_partc_cell_records_head_to_head():
    """run_partc_cell calls comps.head_to_head(off,on,cell) and stores it on the cell."""
    from tier_c.partc import run_partc_cell, PartCCell
    from tests.test_tc_run_partc import _make_comps  # reuse existing fake-comps helper if present

    # Minimal inline fake comps with a head_to_head method:
    from tier_c.model import ArmOutput, Variant, Dose, Citation
    from tier_c.investigator import InvestigatorReport

    def _arm(prism, cites):
        return ArmOutput(variant=Variant("opus-4.8", prism), text="spec", citations=cites,
                         tokens=10, tool_calls=0, wall_s=1.0, used_prism=prism, prism_calls=1 if prism else 0,
                         dose=Dose(1 if prism else 0, frozenset(), 0), low_dose=prism, commands=[],
                         in_tokens=5, cost_usd=0.0, raw_stdout="", argv=[], returncode=0, stderr="", cwd="")

    class C:
        def run_off_arm(self, cell): return _arm(False, [Citation("a.go", 1, None)])
        def run_on_arm(self, cell): return _arm(True, [Citation("a.go", 1, None)])
        def score(self, cites, *, cell, arm):
            return InvestigatorReport(precision=1.0, recall=1.0, hallucinations=0, verdicts=[])
        def head_to_head(self, off, on, cell): return {"winner": "on", "escalated": False, "votes": []}

    cell = run_partc_cell(("prometheus", "spec", "opus-4.8"), C())
    assert cell.head_to_head == {"winner": "on", "escalated": False, "votes": []}
```

- [ ] **Step 2: Run to verify it fails**

Run: `uv run python -m pytest tests/test_tc_run_partc.py::test_run_partc_cell_records_head_to_head -q`
Expected: FAIL (`PartCCell` has no `head_to_head`).

- [ ] **Step 3: Implement**

(a) Add field to `PartCCell` dataclass in `partc.py` (with default for back-compat):
```python
    head_to_head: dict = field(default_factory=dict)
```
(ensure `from dataclasses import field` is imported).

(b) In `run_partc_cell`, after the leak scan / before building the cell, add:
```python
    # Head-to-head spec quality (optional on the comps protocol; back-compat for fakes).
    head_to_head = (comps.head_to_head(off_out, on_out, cell)
                    if hasattr(comps, "head_to_head") else {})
```
and pass `head_to_head=head_to_head` into the `PartCCell(...)` constructor.

(c) In `render_partc`, after the precision/dose line for each cell, add:
```python
        h2h = getattr(c, "head_to_head", {}) or {}
        if h2h:
            esc = " (opus tiebreak)" if h2h.get("escalated") else ""
            lines.append(f"  head-to-head spec quality: {h2h.get('winner','?')}{esc}")
```

- [ ] **Step 4: Run to verify it passes**

Run: `uv run python -m pytest tests/test_tc_run_partc.py -q`
Expected: PASS (new + existing).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/partc.py eval/tests/test_tc_run_partc.py
git commit -m "feat(tier-c): PartCCell.head_to_head + render; run_partc_cell calls comps.head_to_head"
```

---

## Task 6: Wire `_LivePartCComps.head_to_head` (`tier_c/cli.py`)

**Files:**
- Modify: `eval/tier_c/cli.py`
- Test: extend `eval/tests/test_tc_rescore.py`

- [ ] **Step 1: Write the failing test** (append to `tests/test_tc_rescore.py`)

```python
def test_live_comps_head_to_head_uses_spec_quality_judge():
    """_LivePartCComps.head_to_head compares the two specs via SpecQualityJudge over self._ask."""
    from tier_c.cli import _LivePartCComps
    from tier_c.model import ArmOutput, Variant, Dose

    def _arm(text):
        return ArmOutput(variant=Variant("opus-4.8", False), text=text, citations=[], tokens=0,
                         tool_calls=0, wall_s=0.0, used_prism=False, prism_calls=0, dose=Dose(0, frozenset(), 0),
                         low_dose=False, commands=[], in_tokens=0, cost_usd=0.0, raw_stdout="",
                         argv=[], returncode=0, stderr="", cwd="")

    comps = _LivePartCComps(co=_FakeCo(), issue=_FakeIssue(), model="opus-4.8", base_root="",
                            ask=lambda m, p: "A clearer root cause")
    r = comps.head_to_head(_arm("OFF spec"), _arm("ON spec"), ("prometheus", "spec", "opus-4.8"))
    assert r["winner"] in ("off", "on", "tie") and "votes" in r
```

- [ ] **Step 2: Run to verify it fails**

Run: `uv run python -m pytest tests/test_tc_rescore.py::test_live_comps_head_to_head_uses_spec_quality_judge -q`
Expected: FAIL (`_LivePartCComps` has no `head_to_head`).

- [ ] **Step 3: Implement** — add to `_LivePartCComps` in `cli.py`:

```python
    def head_to_head(self, off_out, on_out, cell: tuple) -> dict:
        """Anonymized head-to-head spec-quality comparison (off vs on) via the ensemble."""
        from .judges_live import SpecQualityJudge
        judge = SpecQualityJudge(self._ask)
        return judge.compare(self._issue.text, off_out.text or "", on_out.text or "")
```

- [ ] **Step 4: Run to verify it passes**

Run: `uv run python -m pytest tests/test_tc_rescore.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/cli.py eval/tests/test_tc_rescore.py
git commit -m "feat(tier-c): _LivePartCComps.head_to_head via SpecQualityJudge"
```

---

## Task 7: Full-suite green + rescore validation

**Files:** none (verification only).

- [ ] **Step 1: Run the deterministic suite**

Run: `uv run python -m pytest tests/ -q`
Expected: all pass (the live `adoption/tests/test_prism_adoption.py` probes are NOT in `tests/` and are excluded here). Fix any fallout from the relevance-record schema change.

- [ ] **Step 2: Rescore the saved prometheus runs (no arm re-runs)**

Run each:
```bash
uv run tier-c rescore --run-dir tier_c/runs/partc/isorow-2026-06-28-prometheus-v2 --out-run-id v2-ensemble
uv run tier-c rescore --run-dir tier_c/runs/partc/isorow-2026-06-28-prometheus-v3 --out-run-id v3-ensemble
uv run tier-c rescore --run-dir tier_c/runs/partc/isorow-2026-06-28-prometheus-v4 --out-run-id v4-ensemble
```
Expected: each renders precision (off/on) + the head-to-head winner. **Key check:** the identical off-arm citations (v2/v3/v4 share the reused baseline) should now score the SAME precision across runs (the 0.364↔0.545 swing collapses), and contested cites should show `escalated: true` in the judge jsonl.

- [ ] **Step 3: Report residual variance**

Summarize off-arm precision across v2/v3/v4-ensemble (should be ~constant now), the on-arm precision, the head-to-head winners + reasons, and how often opus was consulted. Conclude whether a ±0.1 effect is now readable. Do NOT commit anything under `tier_c/runs/`.

---

## Self-Review

- **Spec coverage:** ensemble (T1), reasons + YES/NO-first parse (T1), relevance via ensemble (T3), head-to-head anonymized + deterministic (T4), PartCCell/render (T5), live wiring (T6), rescore validation (T7). Coverage metric intentionally absent (deferred). ✓
- **Type consistency:** `LlmRelevanceJudge(ask, model=None, *, opus=None)` keeps the existing `LlmRelevanceJudge(self._ask, JUDGE_MODEL)` call working (model→sonnet). `_RecordingRelevanceJudge` record schema changes `{prompt,response}`→`{verdict,escalated,votes}`; T3 step 4 says update any test asserting the old schema. `comps.head_to_head` added behind `hasattr` so existing fake comps don't break. ✓
- **No placeholders:** every step has real code/commands. ✓
- **Rescore reproducibility:** head-to-head A/B from sha256 parity (no RNG), so rescores are deterministic. ✓
