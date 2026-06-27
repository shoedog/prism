# Tier-C Part C Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.
> Spec of record: `docs/superpowers/specs/2026-06-27-tier-c-part-c-prism-on-steered-design.md`.

**Goal:** Build the Part-C harness (steered prism-on vs re-scored recovered prism-off) and **prove it on one
live cell before spending on the full 16-arm run.**

**Architecture:** A new `run-partc` path in `eval/tier_c/` that (1) re-scores recovered base texts through a
fixed **issue+code-aware** relevance oracle, (2) runs **steered prism-on** arms behind a **real
`mcp__prism__*` invocation gate + dose meter** on **immutable per-arm checkouts** with a **fresh no-cache MCP**,
(3) scores on-vs-base precision/recall + the prism-blind rank judge with **leak-scanned blinding**, (4) reports
per-(stage×language) **directional pilot signal**. **Phase 1** builds the core and ends at a **single-cell
verify gate**; **Phase 2** (gated on verify, owner-triggered live) scales to 16 + adds the **GO-only
decomposition sentinel** + the full run.

**Tech stack:** Python (`eval/tier_c` package), `pytest`, `uv`; `claude -p --output-format stream-json` /
`codex exec --json`; `prism-mcp`. Existing tests use Fake judge/runner seams — keep that pattern (no live spend
in unit tests).

**Phases:**
- **Phase 1 — build + verify** (Tasks 1–11, ≤1 arm of live spend in the verify): core harness → single-cell gate.
- **Phase 2 — scale + sentinel + full run** (Tasks 12–15, owner-triggered live): gated on Phase-1 verify passing.

**Conventions:** all commands run from `eval/` via `uv run`. Run a single test with
`uv run pytest tests/<file>::<test> -q` (tier_c tests live under `eval/tier_c/tests/`). Commit per task with
explicit paths (never `git add -A`). Branch: `tier-c-part-c` off `nearer-samples-eval` (or main — confirm with
controller).

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `eval/tier_c/checkout.py` | pinned worktree | **+`read_window`** (code window for the oracle) |
| `eval/tier_c/parse.py` | model-output parsers | **+`parse_claude_stream_json`** (real `mcp__prism__*` calls + dose) |
| `eval/tier_c/model.py` | dataclasses | **+dose fields** on `ArmOutput`; new `Dose` |
| `eval/tier_c/judges_live.py` | LLM judges | `is_relevant(cite, issue_text, code)` + prompt |
| `eval/tier_c/investigator.py` | citation scoring | thread `issue_text`+`code`; Fake-judge signatures |
| `eval/tier_c/chain.py` | stage chain | pass `issue_text`/slice/upstream-spec into `score_citations` |
| `eval/tier_c/partc_baseline.py` | **NEW** | load + header-strip recovered base text |
| `eval/tier_c/partc_calib.py` + fixture | **NEW** | oracle calibration set + saturation guard |
| `eval/tier_c/arm_runner.py` | run one arm | stream-json claude; real invocation gate + dose; per-arm reset + fresh no-cache MCP |
| `eval/tier_c/prompts.py` | prompts | prism-on steer (+no-leak); capability sentinel steer |
| `eval/tier_c/leak.py` | **NEW** | `scan_leak(text)` → flag/redact `prism|nav_` |
| `eval/tier_c/run.py` | orchestrators | **+`run_partc`** (cell + all-16); sentinel pass |
| `eval/tier_c/report.py` | reporting | **+Part-C report** (on-vs-base, pilot-signal, dose flags, split) |
| `eval/tier_c/cli.py` | CLI | **+`tier-c run-partc`** subcommand |

---

# PHASE 1 — Build + single-cell verify

## Task 1: Code window for the oracle (`Checkout.read_window`)

**Files:** Modify `eval/tier_c/checkout.py`; Test `eval/tier_c/tests/test_checkout.py`

- [ ] **Step 1 — failing test.** Add:
```python
def test_read_window_returns_centered_context(tmp_path):
    # build a tiny git repo + worktree via the existing Checkout fixture/helpers
    co = _checkout_with_file(tmp_path, "a.py", "\n".join(f"L{i}" for i in range(1, 21)))
    with co as c:
        assert c.read_window("a.py", 10, ctx=2) == "L8\nL9\nL10\nL11\nL12"
        assert c.read_window("a.py", 1, ctx=2) == "L1\nL2\nL3"           # clamps at top
        assert c.read_window("missing.py", 5, ctx=2) is None             # missing file
```
- [ ] **Step 2 — run, expect FAIL** (`AttributeError: read_window`).
- [ ] **Step 3 — implement** next to `read_line` (1-indexed, clamp to file bounds, `None` if file absent):
```python
def read_window(self, rel: str, line: int, ctx: int = 3) -> str | None:
    p = self.root / rel
    if not p.is_file():
        return None
    lines = p.read_text(errors="replace").splitlines()
    lo, hi = max(0, line - 1 - ctx), min(len(lines), line + ctx)
    return "\n".join(lines[lo:hi]) if lo < hi else None
```
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit** `git add eval/tier_c/checkout.py eval/tier_c/tests/test_checkout.py && git commit -m "feat(tier-c): Checkout.read_window for oracle code context"`

## Task 2: Real `mcp__prism__*` parsing + dose (`parse_claude_stream_json`)

**Files:** Modify `eval/tier_c/parse.py`, `eval/tier_c/model.py`; Test `eval/tier_c/tests/test_parse.py`

- [ ] **Step 1 — failing test.** Feed a small `stream-json` fixture (assistant messages with `tool_use`
  entries named `mcp__prism__nav_callers`, a non-prism `Bash`, and one prism error result) and assert:
```python
def test_stream_json_counts_real_prism_calls_and_dose():
    r = parse_claude_stream_json(open("eval/tier_c/tests/fixtures/claude_stream_prism.jsonl").read())
    assert r.prism_calls == 2                       # only mcp__prism__* tool_use
    assert r.dose.distinct_tools == {"nav_callers", "nav_repo_map"}
    assert r.dose.errors == 1
    assert r.text.strip()                            # final assistant text captured
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement.** Add a `Dose` dataclass to `model.py` (`count:int, distinct_tools:set[str],
  errors:int`); add `parse_claude_stream_json(out)` porting the adoption eval's `parse_stream_json`
  (`eval/adoption/trajectory.py`): iterate `type=="assistant"` → `content[].type=="tool_use"`, normalise
  `mcp__prism__X`→`X`, count prism tool_uses, collect distinct nav tools, count tool_result entries flagged
  `is_error`. Return a `ModelResult` extended with `prism_calls:int` and `dose:Dose`.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit** (`parse.py`, `model.py`, the fixture, the test).

## Task 3: Relevance judge sees issue + code (`is_relevant(cite, issue_text, code)`)

**Files:** Modify `eval/tier_c/judges_live.py`, `eval/tier_c/investigator.py` (Fake judges); Test
`eval/tier_c/tests/test_judges_live.py`

- [ ] **Step 1 — failing test.** With a fake `ask` seam, assert the relevance prompt now contains BOTH the
  issue text and the cited code window, and that the signature is `is_relevant(cite, issue_text, code)`:
```python
def test_relevance_prompt_includes_issue_and_code():
    seen = {}
    j = LlmRelevanceJudge(ask=lambda p: seen.setdefault("p", p) or "YES")
    assert j.is_relevant(cite=_cite("a.py", 10, "f"), issue_text="ISSUE-XYZ", code="def f(): ...") is True
    assert "ISSUE-XYZ" in seen["p"] and "def f()" in seen["p"]
```
- [ ] **Step 2 — run, expect FAIL** (signature mismatch / code absent).
- [ ] **Step 3 — implement.** Change `LlmRelevanceJudge.is_relevant` to take `code` and put issue+code in the
  prompt (`judges_live.py:33`). Update the two Fake judges in `investigator.py:31,33` to the 3-arg signature.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit.**

## Task 4: Thread issue+code through citation scoring (`investigator.py`)

**Files:** Modify `eval/tier_c/investigator.py`; Test `eval/tier_c/tests/test_investigator.py`

- [ ] **Step 1 — failing test.** `verify_citation`/`score_citations` accept a `read_code(file,line)->str|None`
  callable + `issue_text`, and pass the code to `is_relevant`:
```python
def test_score_citations_threads_code_to_relevance():
    calls = []
    judge = _RecordingRelevance(calls)               # records (issue_text, code)
    rep = score_citations(co_fake, [_cite("a.py", 10, "f")], claim_count=1,
                          relevance=judge, issue_text="I", read_code=lambda f, l: "CODE@%s:%s" % (f, l))
    assert calls == [("I", "CODE@a.py:10")]
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement.** Add `read_code` param (default `lambda *_: None`) + `issue_text` (already
  present, default `""`) to `verify_citation`/`score_citations`; compute `code = read_code(cite.file,
  cite.line)` and call `relevance.is_relevant(cite, issue_text, code)` (`investigator.py:45,48`).
- [ ] **Step 4 — run, expect PASS** (+ existing investigator tests still green — update their judge fakes to
  3-arg).
- [ ] **Step 5 — commit.**

## Task 5: Chain passes issue/slice/upstream-spec into scoring (`chain.py`)

**Files:** Modify `eval/tier_c/chain.py`; Test `eval/tier_c/tests/test_chain.py`

- [ ] **Step 1 — failing test.** `run_spec_plan_chain` passes `issue_text` and a `read_code` bound to the
  checkout into `score_citations`, and for the **plan** stage includes the upstream spec text in `issue_text`
  context. Assert via a recording relevance fake that the plan-stage relevance call sees both issue + spec.
- [ ] **Step 2 — run, expect FAIL** (today `chain.py:42` passes neither).
- [ ] **Step 3 — implement.** At the `score_citations` call (`chain.py:42`): pass `issue_text=co.text` (+ the
  upstream spec for the plan stage), and `read_code=lambda f, l: co_checkout.read_window(f, l)` (thread the
  `Checkout` into the chain — add a param if needed).
- [ ] **Step 4 — run, expect PASS** (+ existing chain tests green).
- [ ] **Step 5 — commit.**

## Task 6: Oracle calibration set + saturation guard

**Files:** Create `eval/tier_c/partc_calib.py`, `eval/tier_c/tests/fixtures/oracle_calib.toml`; Test
`eval/tier_c/tests/test_partc_calib.py`

- [ ] **Step 1 — failing test.** A fixture of ~6 citations labelled `relevant|irrelevant|hallucinated` (with
  issue text + code). `calibrate(relevance, read_code, cases)` returns accuracy + saturation flags; the run
  must FAIL on all-YES or all-NO:
```python
def test_calibration_flags_saturation_and_accuracy():
    res = calibrate(AllYesRelevance(), read_code=_code_of(cases), cases=cases)
    assert res.saturated_all_yes and not res.ok
    res2 = calibrate(GoldRelevance(), read_code=_code_of(cases), cases=cases)   # the labelled-correct fake
    assert res2.accuracy >= 0.8 and res2.ok
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement** `calibrate(...)`: run the judge over each case, compute accuracy vs labels, set
  `saturated_all_yes`/`saturated_all_no`, `ok = accuracy>=0.8 and not saturated`.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit.**

## Task 7: Baseline loader with header strip (`partc_baseline.py`)

**Files:** Create `eval/tier_c/partc_baseline.py`; Test `eval/tier_c/tests/test_partc_baseline.py`

- [ ] **Step 1 — failing test.** Recovered files begin with a leaking metadata block (`prism=False`,
  `prism_called`, `session:` …) ended by a `---` line; the loader returns ONLY the assistant text after the
  first `---`:
```python
def test_loader_strips_condition_header_through_first_separator():
    raw = "# x\n- prism=**False**\n- session: /p\n\n---\n\nSpec body here\n## Summary\n"
    assert load_base_text(raw) == "Spec body here\n## Summary"
    assert "prism" not in load_base_text(raw).lower()       # no condition leak
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement** `load_base_text(raw)` (split on first line equal to `---`, return the remainder
  stripped) and `load_base(model, repo, stage, root)` (read the recovered `<model>.md`, return `load_base_text`).
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit.**

## Task 8: Real invocation gate + dose on the arm (`arm_runner.py`)

**Files:** Modify `eval/tier_c/arm_runner.py`, `eval/tier_c/model.py`; Test
`eval/tier_c/tests/test_arm_runner.py`

- [ ] **Step 1 — failing test.** A claude arm parsed via `parse_claude_stream_json` sets
  `ArmOutput.prism_calls`/`.dose`/`.used_prism = prism_calls>0` (NOT the `num_turns` heuristic), and
  `low_dose = used_prism and prism_calls<=1`:
```python
def test_arm_output_uses_real_prism_calls_and_flags_low_dose(monkeypatch):
    out = _run_fake_claude_arm(stream_with(prism_calls=1))
    assert out.used_prism and out.prism_calls == 1 and out.low_dose
    out2 = _run_fake_claude_arm(stream_with(prism_calls=0))
    assert not out2.used_prism                          # true 0-call → not administered
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement.** Build the claude command with `--output-format stream-json` (mirror the adoption
  eval); parse via `parse_claude_stream_json`; set `used_prism = prism_calls>0`, add `prism_calls`, `dose`,
  `low_dose` to `ArmOutput`; codex path uses `parse_codex_jsonl` (already yields prism mcp calls). Replace all
  three `used_prism = variant.prism and tool_calls>0` sites (`arm_runner.py:85,109,120`).
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit.**

## Task 9: Per-arm SUT + prism-cache immutability (`arm_runner.py`)

**Files:** Modify `eval/tier_c/arm_runner.py` (or a small `partc_arm.py` wrapper); Test
`eval/tier_c/tests/test_arm_runner.py`

- [ ] **Step 1 — failing test.** A wrapper `run_arm_isolated(...)` (a) `git reset --hard && git clean -fd`s the
  checkout before AND after the arm (assert a file the fake arm mutated is reverted after), and (b) builds the
  prism MCP config with `--no-cache` + a per-arm cache dir and records `cache_mode` in the result:
```python
def test_arm_is_isolated_and_no_cache(tmp_git_checkout):
    res = run_arm_isolated(fake_runner_that_writes("x.py"), checkout=tmp_git_checkout, variant=prism_on)
    assert not (tmp_git_checkout.root / "x.py").exists()      # reverted after
    assert res.cache_mode == "no-cache"
    assert "--no-cache" in res.mcp_args
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement.** Wrap the arm call: reset/clean before+after (subprocess `git -C <root>`); when
  `variant.prism`, append `--no-cache` (or `--cache-dir <tmp>`) to the prism `--repo` args
  (`arm_runner.py:38,48`) and a fresh server per arm; surface `cache_mode`. Keep file-write deny as a
  belt-and-suspenders note in the runner docstring.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit.**

## Task 10: Steer prompts + leak scanner (`prompts.py`, `leak.py`)

**Files:** Modify `eval/tier_c/prompts.py`; Create `eval/tier_c/leak.py`; Tests `test_prompts.py`,
`test_leak.py`

- [ ] **Step 1 — failing tests.**
```python
def test_prism_on_steer_directs_nav_and_forbids_naming_tools():
    p = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="prism_on")
    assert "nav_callers" in p and "do not name the tools" in p.lower()

def test_capability_steer_names_task_not_prism():
    p = stage_prompt("spec", issue_text="i", scoped_slice="s", steer="capability")
    assert "who calls" in p.lower() and "prism" not in p.lower() and "nav_" not in p.lower()

def test_leak_scanner_flags_and_redacts():
    f = scan_leak("We used nav_callers and Prism to find foo().")
    assert f.leaked and "nav_callers" not in f.redacted and "prism" not in f.redacted.lower()
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement.** Add a `steer: str = ""` param to `stage_prompt` appending the prism-on directive
  (tool-level + "do not name the tools you used") or the capability directive (task-level, no prism/nav). Add
  `leak.py`: `scan_leak(text) -> {leaked: bool, redacted: str}` (regex `(?i)\b(prism|nav_[a-z_]+)\b` → `[tool]`).
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit.**

## Task 11: Single-cell Part-C runner + minimal report + CLI

**Files:** Modify `eval/tier_c/run.py`, `eval/tier_c/report.py`, `eval/tier_c/cli.py`; Test `test_run_partc.py`

- [ ] **Step 1 — failing test (fakes, no live spend).** `run_partc_cell(cell, comps)` loads the base text,
  runs ONE steered-prism-on arm (fake runner), gates on real prism calls, re-scores BOTH base and on-arm with
  the fixed oracle (fake), leak-scans the on-arm text, and returns a `PartCCell` with `precision_on`,
  `precision_base`, `bundle_delta`, `dose`, `leaked`:
```python
def test_run_partc_cell_scores_on_vs_base_with_fakes():
    cell = run_partc_cell(_cell("ruff","spec","opus-4.8"), _fake_comps(on_precision=0.8, base_precision=0.4))
    assert cell.bundle_delta == pytest.approx(0.4) and cell.dose.count >= 1 and not cell.leaked
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement** `run_partc_cell` in `run.py` (compose Tasks 1–10), a `PartCCell` + minimal
  `render_partc([cells])` in `report.py` (table: cell, precision base→on, Δ, rank, dose, leaked, pilot n=1),
  and a `tier-c run-partc --cell <repo>:<stage>:<model> [--live]` subcommand in `cli.py` (defaults to fakes;
  `--live` uses real runners + `live_ask` judges).
- [ ] **Step 4 — run, expect PASS** (fakes).
- [ ] **Step 5 — commit.**

## ✅ VERIFY GATE (Task V) — one live cell, then STOP

**This is the de-risk before the full run. Owner-triggered (real spend, ~1 arm).**

- [ ] **Step 1 — run one live cell:** `cd eval && uv run tier-c run-partc --cell ruff:spec:opus-4.8 --live`
  (ruff = prism-strong Rust so prism clearly engages; gpt-5.5 is the cheaper alt).
- [ ] **Step 2 — assert ALL of:**
  1. **prism administered:** the on-arm `dose.count > 0` real `mcp__prism__nav_*` calls (gate passed, not the
     old heuristic). If 0 → the steer/MCP wiring is broken — fix before anything else.
  2. **oracle non-degenerate AND not saturated:** `precision_on > 0` and the calibration set passes
     (`partc_calib` ok, no all-YES/all-NO). This proves defect #2 is actually fixed.
  3. **baseline re-scored:** `precision_base` computed from the header-stripped recovered text (no `prism`
     token in the loaded base).
  4. **immutable SUT:** the checkout is byte-identical before/after (`git status` clean); prism `cache_mode`
     logged.
  5. **blinding clean:** `leaked == False` (no `prism|nav_` survived into the judged on-arm text).
  6. **report renders** the single `PartCCell`.
- [ ] **Step 3 — record** the cell's numbers + the 6 assertions in the PR / handoff. **Do NOT proceed to
  Phase 2 until every assertion holds.** If any fails, it is a Phase-1 bug — fix and re-verify (cache makes the
  fake-side cheap; only the live cell re-spends).

---

## Phase-2 prerequisites (from the Phase-1 final opus review — fix in the gap AFTER the Verify Gate)

The final whole-branch review (READY FOR VERIFY GATE) found two integration bugs that do NOT touch the
documented verify cell (`ruff:spec:opus-4.8`, claude, spec) but **corrupt half the Phase-2 matrix** — fix both
before any gpt-5.5 or plan cell runs (verify each against the code first):

- **[IMPORTANT] Codex dose double-count** — `eval/tier_c/parse.py` `parse_codex_jsonl` iterates events without
  gating on `ev.get("type") == "item.completed"`, so each codex `mcp_tool_call` (emitted under both
  `item.started` AND `item.completed`) is counted twice → `prism_calls`/`dose`/`tool_calls`/`commands` all 2× for
  every gpt-5.5 cell, making `low_dose` (`prism_calls<=1`) unreachable. The reference `adoption/trajectory.py`
  gates correctly. **Fix:** add `if ev.get("type") != "item.completed": continue` after the JSON parse; add a
  regression fixture emitting each item under both event types and assert count==1.
- **[IMPORTANT] Plan-stage cells lose the upstream spec** — `eval/tier_c/cli.py` `_LivePartCComps.run_on_arm`
  calls `stage_prompt(stage, …, steer="prism_on")` with no `upstream=`, and `score()` passes only
  `issue_text=self._issue.text`. For a `plan` cell the on-arm is asked to plan "for this spec" without the spec,
  and its citations are judged without spec context — mismatched vs the recovered baseline (built WITH a chained
  spec). `chain.py`'s threading is dead for the Part-C path (cells are scored independently). **Fix:** for plan
  cells, load the recovered spec body via `partc_baseline.load_base(model, repo, "spec", root)`, pass it as
  `stage_prompt(…, upstream=<spec>)` AND fold it into `score()`'s `issue_text` (mirror `chain.py`'s
  `plan_issue_text`). Add a plan-cell test asserting both.

Minor (cosmetic / Phase-2-wiring, fold opportunistically): 3 dead imports (`arm_runner.py:10` `field`,
`arm_runner.py:14` `parse_claude_json`, `cli.py:126` `shutil`); `scan_leak().redacted` is computed but only
`.leaked` is consumed (wire `.redacted` into the judged text when the rank judge lands in Task 12/13); Part-C
does not yet measure recall or the rank judge (Task 12+; the spec calls them co-primary) — wire a real
`claim_count` (not `max(len,1)`) and the rank-judge consensus in Phase 2.

# PHASE 2 — Scale + sentinel + full run (gated on Verify, owner-triggered live)

## Task 12: All-16 orchestration + pilot-signal report

**Files:** Modify `eval/tier_c/run.py`, `eval/tier_c/report.py`; Test `test_run_partc.py`

- [ ] **Step 1 — failing test (fakes).** `run_partc(cells, comps)` loops the 16 cells, aggregates per
  `(stage, language)`, and `render_partc` labels each as **directional pilot signal** with `n=1` + exact
  counts (precision base/on, Δ, rank, dose, low_dose flag), and shows the **administration rate (72% /
  ~100%)** alongside with `bundle-lift × 0.72` as a caveated floor (NOT the headline).
- [ ] **Step 2–5 — TDD + commit** as above.

## Task 13: GO-only decomposition sentinel

**Files:** Modify `eval/tier_c/run.py`, `eval/tier_c/report.py`; Test `test_run_partc.py`

- [ ] **Step 1 — failing test (fakes).** `run_sentinels(partc_cells, comps)` selects cells with
  `bundle_delta > 0`, runs ONE **capability-steered prism-OFF** arm each (Task 10 `steer="capability"`),
  re-scores, and reports the split: `prism_share = on − cap_off`, `steer_share = cap_off − base`; classifies
  each GO cell `prism-confirmed | steer-only | mixed`:
```python
def test_sentinel_only_runs_on_positive_lift_and_splits():
    res = run_sentinels([cell(delta=+0.4), cell(delta=-0.1)], _fake_comps(cap_off_precision=0.45))
    assert len(res) == 1                                   # only the +0.4 cell
    assert res[0].verdict in {"prism-confirmed","steer-only","mixed"}
```
- [ ] **Step 2–5 — TDD + commit.**

## Task 14: Wire `--live` full path + calibration gate in CLI

**Files:** Modify `eval/tier_c/cli.py`, `eval/tier_c/run.py`; Test `test_cli.py`

- [ ] **Step 1 — failing test.** `tier-c run-partc --all --live` (a) runs the calibration set first and ABORTS
  if `not ok` (saturated/low-accuracy oracle), (b) runs 16 on-arms, (c) runs GO-only sentinels, (d) writes the
  report. Test with fakes that an all-YES oracle aborts before any arm runs.
- [ ] **Step 2–5 — TDD + commit.**

## Task 15: Full live run (owner-triggered — NOT executed by the implementer)

- [ ] Document the command in the PR: `cd eval && uv run tier-c run-partc --all --live --run-id partc-<date>`.
- [ ] Pre-flight: prism-mcp release built (`target/release/prism-mcp`), recovered base dir present, the 4
  issue checkouts resolvable, per-arm budget bumped to 1800s.
- [ ] After the run: paste the per-(stage×language) pilot table + sentinel verdicts + administration-rate line
  into the PR; update memory.

---

## Self-Review (controller, before dispatch)
- **Spec coverage:** oracle issue+code (T3–5), calibration/saturation (T6, T14), invocation gate+dose (T2,T8),
  low-dose flag (T8), baseline header-strip (T7), SUT+cache immutability (T9), steer + no-leak + scanner (T10),
  Part-C runner (T11–12), GO-only sentinel (T13), pilot-signal report (T12), single-cell verify gate (V),
  dedicated runner not the 8-variant loop (T11/14). All spec sections map to a task.
- **No placeholders:** each task has a concrete failing test + implementation sketch. Integration tasks (T5,
  T9, T11) name the exact call sites.
- **Type consistency:** `Dose` (T2) is consumed by `ArmOutput` (T8) and `render_partc` (T12); `load_base_text`
  (T7) feeds `run_partc_cell` (T11); `is_relevant(cite, issue_text, code)` is uniform across T3/T4/T6.

## Execution Handoff
Plan saved. Two options:
1. **Subagent-Driven (recommended)** — fresh subagent per task, two-stage review between tasks. Stop at the
   **Verify Gate** for owner-triggered live spend; do NOT auto-run Phase 2.
2. **Inline Execution** — executing-plans with checkpoints.
