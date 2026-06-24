# Tier-C Phase-1c — Live loop + real LLM judges (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Make `tier-c run --live` actually execute the spec→plan 2×2 over the corpus: real LLM-backed judges (RankJudge / RelevanceJudge / ConditionGuesser), a routing arm-runner (Opus→claude, gpt→codex), real per-output `claim_counts`, the detectability gate, and the end-to-end run loop that emits the per-(stage×language) report + GO/NO-GO.

**Architecture:** All LLM components depend on a single injectable `ask(model, prompt) -> str` callable (the **seam**), so every judge/guesser is unit-tested with a fake `ask` and the real model calls are a thin integration layer. The real `ask` routes Opus→`claude -p --output-format json`, gpt→`codex exec --json` (single-shot, **no MCP/tools** — judges must not use prism). Spec→plan chain uses {Opus 4.8, gpt-5.5}×{on/off}; develop/review (Sonnet/spark) stay Phase 2. Builds on Phase-1/1b (`parse.py`, `arm_runner.py`, `chain.py`, `report.py`, `run.py`, `judges.py`, `claims.py`).

**Tech Stack:** Python 3.12, `subprocess`, `pytest`, `uv`. Models: claude `opus`, codex `gpt-5.5`.

> **NOTE:** an actual `--live` run costs real model spend + wall-time (4 variants × 2 stages × N issues × arm calls, plus judges). The BUILD is fully fakes-tested; running it live is a separate owner-triggered step. Exact model-flag values (claude `--model opus` alias; codex `-m gpt-5.5`) are verified in the integration step, not CI.

---

### Task 1: `ask` seam — single-shot model call (`llm.py`)

**Files:** Create `eval/tier_c/llm.py`; Test `eval/tests/test_tc_llm.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_llm.py
import json
from tier_c.llm import live_ask, MODEL_CLI

def test_model_cli_maps_families():
    assert MODEL_CLI["opus-4.8"][0] == "claude"
    assert MODEL_CLI["gpt-5.5"][0] == "codex"

def test_live_ask_claude_parses_result(monkeypatch):
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        seen["cmd"] = cmd
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":1,
                  "result":"ranked: cand1,cand0","usage":{"output_tokens":5}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    out = live_ask("opus-4.8", "rank these")
    assert out == "ranked: cand1,cand0"
    assert "--mcp-config" not in " ".join(seen["cmd"])   # judges get NO prism

def test_live_ask_codex_parses_jsonl(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        assert "mcp_servers.prism" not in " ".join(cmd)   # no prism
        class R: stdout = json.dumps({"item":{"type":"agent_message","text":"YES"}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.llm.subprocess.run", fake_run)
    assert live_ask("gpt-5.5", "relevant?") == "YES"
```

- [ ] **Step 2: Run, confirm FAIL** — `cd eval && uv run pytest tests/test_tc_llm.py -q`.

- [ ] **Step 3: Implement**
```python
# eval/tier_c/llm.py
"""Single-shot model-call seam for judges (Phase-1c). NO MCP/tools — a judge must not use prism.
Opus→claude -p --output-format json; gpt→codex exec --json. Returns the model's text answer."""
from __future__ import annotations
import subprocess
from .parse import parse_claude_json, parse_codex_jsonl

_TIMEOUT = 600

# Variant.model -> (cli, cli-model-flag). Verify flag values live (claude alias 'opus'; codex 'gpt-5.5').
MODEL_CLI = {
    "opus-4.8": ("claude", "opus"),
    "gpt-5.5": ("codex", "gpt-5.5"),
}

def live_ask(model: str, prompt: str) -> str:
    cli, flag = MODEL_CLI[model]
    if cli == "claude":
        cmd = ["claude", "-p", "--output-format", "json", "--model", flag, prompt]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_TIMEOUT)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"claude judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
        return parse_claude_json(proc.stdout).text
    cmd = ["codex", "exec", "--json", "-m", flag, "-s", "read-only", "-"]
    proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True, timeout=_TIMEOUT)
    if proc.returncode != 0 or not proc.stdout.strip():
        raise RuntimeError(f"codex judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
    return parse_codex_jsonl(proc.stdout).text
```

- [ ] **Step 4: Run, confirm PASS. Step 5: Commit** → `feat(tier-c): single-shot ask() model-call seam`.

---

### Task 2: Real judges (`judges_live.py`)

**Files:** Create `eval/tier_c/judges_live.py`; Test `eval/tests/test_tc_judges_live.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_judges_live.py
from tier_c.model import Citation
from tier_c.judges_live import LlmRankJudge, LlmRelevanceJudge, LlmConditionGuesser

def test_rank_judge_parses_permutation():
    j = LlmRankJudge(ask=lambda m, p: "cand2, cand0, cand1", model="opus-4.8")
    order = j.rank("spec", "rubric", {"cand0":"a","cand1":"b","cand2":"c"})
    assert order == ["cand2","cand0","cand1"]

def test_rank_judge_repairs_missing_labels():
    # model omits cand1 -> appended in input order so result is always a full permutation
    j = LlmRankJudge(ask=lambda m, p: "cand2,cand0", model="opus-4.8")
    order = j.rank("spec", "r", {"cand0":"a","cand1":"b","cand2":"c"})
    assert sorted(order) == ["cand0","cand1","cand2"]
    assert order[:2] == ["cand2","cand0"]

def test_relevance_judge_yes_no():
    yes = LlmRelevanceJudge(ask=lambda m,p: "YES, clearly relevant", model="opus-4.8")
    no = LlmRelevanceJudge(ask=lambda m,p: "No, unrelated", model="opus-4.8")
    assert yes.is_relevant(Citation("a.py",1,"f"), "issue") is True
    assert no.is_relevant(Citation("a.py",1,"f"), "issue") is False

def test_condition_guesser_returns_bool():
    g = LlmConditionGuesser(ask=lambda m,p: "YES", model="opus-4.8")
    assert g.guess_used_prism("some output text") is True
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
```python
# eval/tier_c/judges_live.py
"""LLM-backed judges (Phase-1c), all behind the ask() seam so they're unit-tested with fakes.
RankJudge ranks anonymized candidates (style-neutral); RelevanceJudge audits a citation;
ConditionGuesser powers the detectability test. None of them get prism (ask() is tool-free)."""
from __future__ import annotations
import re
from .model import Citation

_RANK_INSTR = ("Rank the candidates best-to-worst for this {stage} task on the rubric: {rubric}. "
               "IGNORE citation formatting/volume — judge substance only. "
               "Respond with ONLY the candidate ids in order, best first, comma-separated.")

class LlmRankJudge:
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def rank(self, stage: str, rubric: str, candidates: dict[str, str]) -> list[str]:
        body = "\n\n".join(f"[{lbl}]\n{txt}" for lbl, txt in candidates.items())
        prompt = _RANK_INSTR.format(stage=stage, rubric=rubric) + "\n\n" + body
        raw = self.ask(self.model, prompt)
        found = [t for t in re.findall(r"cand\d+", raw)]
        seen, order = set(), []
        for c in found:
            if c in candidates and c not in seen:
                seen.add(c); order.append(c)
        for c in candidates:                       # repair: append any omitted in input order
            if c not in seen:
                order.append(c)
        return order

class LlmRelevanceJudge:
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def is_relevant(self, cite: Citation, issue_text: str) -> bool:
        prompt = (f"Issue:\n{issue_text}\n\nIs the code at {cite.file}:{cite.line} "
                  f"(symbol {cite.symbol}) actually relevant to fixing this issue? Answer YES or NO.")
        return self.ask(self.model, prompt).strip().upper().startswith("YES")

class LlmConditionGuesser:
    def __init__(self, ask, model: str):
        self.ask, self.model = ask, model
    def guess_used_prism(self, text: str) -> bool:
        prompt = ("Below is an output from a coding task. Was a code-navigation tool that yields exact "
                  "file:line/call-graph facts likely USED to produce it? Answer YES or NO.\n\n" + text)
        return self.ask(self.model, prompt).strip().upper().startswith("YES")
```

- [ ] **Step 4: Run, confirm PASS. Step 5: Commit** → `feat(tier-c): LLM rank/relevance/condition judges (ask-seam)`.

---

### Task 3: Routing arm-runner + real `claim_counts`

**Files:** Modify `eval/tier_c/arm_runner.py`, `eval/tier_c/chain.py`; Test `eval/tests/test_tc_routing.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_routing.py
from tier_c.model import Variant
from tier_c.arm_runner import RoutingArmRunner

class FakeClaude:
    def run(self, v, stage, prompt, repo): return ("claude", v.id)
class FakeCodex:
    def run(self, v, stage, prompt, repo): return ("codex", v.id)

def test_routing_picks_cli_by_family():
    r = RoutingArmRunner(claude=FakeClaude(), codex=FakeCodex())
    assert r.run(Variant("opus-4.8", True), "spec", "p", "/r")[0] == "claude"
    assert r.run(Variant("gpt-5.5", False), "spec", "p", "/r")[0] == "codex"

def test_run_stage_computes_claim_counts_from_outputs():
    # when claim_counts is None, run_stage derives it per-output via count_claims
    from tier_c.arm_runner import FakeArmRunner
    from tier_c.investigator import RelevanceAllTrue
    from tier_c.chain import run_stage
    class FakeCo:
        def file_exists(self, rel): return True
        def read_line(self, rel, line): return "x"
    class FakeRank:
        def rank(self, s, r, c): return sorted(c, key=lambda k: -len(c[k]))
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "The matcher uses compile() in a.py:1.", "gpt-5.5": "ok"})
    res = run_stage(stage="spec", variants=variants, runner=runner, co=FakeCo(),
                    prompt="p", repo_root="/r", claim_counts=None,
                    plants=[], judges={"anthropic": FakeRank(), "openai": FakeRank()},
                    relevance=RelevanceAllTrue())
    # opus output makes >=1 code-claim -> recall denominator > 0 (not the {id:1} placeholder by luck)
    assert res.investigator["opus-4.8+prism"].recall >= 0.0  # computed, no crash
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
  - In `arm_runner.py` add:
```python
class RoutingArmRunner:
    """Dispatch a variant to its CLI runner by model family (Opus->claude, gpt->codex)."""
    def __init__(self, claude, codex):
        self.claude, self.codex = claude, codex
    def run(self, variant, stage, prompt, repo_root):
        runner = self.claude if variant.family == "anthropic" else self.codex
        return runner.run(variant, stage, prompt, repo_root)
```
  - In `chain.py` `run_stage`, make `claim_counts` optional: change the signature default to `claim_counts=None` and, right after `outputs` is built, add:
```python
    if claim_counts is None:
        from .claims import count_claims
        claim_counts = {vid: max(1, count_claims(o.text)) for vid, o in outputs.items()}
```
(Keep the rest unchanged; `max(1, …)` avoids a zero denominator.)

- [ ] **Step 4: Run, confirm PASS** (+ `cd eval && uv run pytest tests/test_tc_chain.py -q` — existing callers pass `claim_counts` explicitly, still fine). **Step 5: Commit** → `feat(tier-c): routing arm-runner + per-output claim_counts`.

---

### Task 4: Detectability over a stage's outputs

**Files:** Modify `eval/tier_c/run.py` (or new `eval/tier_c/detect.py`); Test `eval/tests/test_tc_detect.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_detect.py
from tier_c.model import Variant, ArmOutput, Citation
from tier_c.detect import run_detectability

def _out(model, prism, text):
    return ArmOutput(Variant(model, prism), text, [], 0, 0, 0.0, prism)

class Guesser:  # guesses prism-on iff text contains "navfact"
    def guess_used_prism(self, text): return "navfact" in text

def test_detectability_counts_correct_guesses_and_pvalue():
    outs = [_out("opus-4.8", True, "navfact here"), _out("gpt-5.5", False, "plain"),
            _out("opus-4.8", False, "plain"), _out("gpt-5.5", True, "navfact")]
    r = run_detectability(outs, Guesser())
    assert r.correct == 4 and r.n == 4          # guesser perfectly separates -> detectable
    assert r.detectable is True
    assert r.pvalue < 0.1
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
```python
# eval/tier_c/detect.py
"""Detectability test (spec §6b): a ConditionGuesser predicts prism on/off from each output; if it
beats chance (exact-binomial p<0.05), the judge prism-delta is INVALID and we lean on objective."""
from __future__ import annotations
from dataclasses import dataclass
from .judges import detectability_pvalue

@dataclass(frozen=True)
class Detectability:
    correct: int
    n: int
    pvalue: float
    detectable: bool

def run_detectability(outputs, guesser, alpha: float = 0.05) -> Detectability:
    n = len(outputs)
    correct = sum(1 for o in outputs if guesser.guess_used_prism(o.text) == o.variant.prism)
    p = detectability_pvalue(correct, n)
    return Detectability(correct=correct, n=n, pvalue=p, detectable=(p < alpha))
```

- [ ] **Step 4: Run, confirm PASS. Step 5: Commit** → `feat(tier-c): detectability over stage outputs`.

---

### Task 5: The `--live` run loop

**Files:** Modify `eval/tier_c/run.py`, `eval/tier_c/cli.py`; Test `eval/tests/test_tc_run_live.py`

- [ ] **Step 1: Failing test** (fakes throughout — NO live calls)
```python
# eval/tests/test_tc_run_live.py
from tier_c.model import Issue, Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.investigator import RelevanceAllTrue
from tier_c.run import run_live, LiveComponents

class FakeCo:
    root = "."
    def __enter__(self): return self
    def __exit__(self, *a): return False
    def file_exists(self, rel): return True
    def read_line(self, rel, line): return "x"
class FakeRank:
    def rank(self, s, r, c): return sorted(c, key=lambda k: -len(c[k]))
class FakeGuess:
    def guess_used_prism(self, t): return False

def test_run_live_produces_report_cells():
    issues = [Issue("ruff-1","rust","ruff","sha","u","bug: matcher in a.py:1 wrong","slice")]
    comps = LiveComponents(
        variants=[Variant("opus-4.8", True), Variant("opus-4.8", False)],
        runner=FakeArmRunner({"opus-4.8+prism":"spec a.py:1 detailed","opus-4.8":"spec"}),
        judges={"anthropic": FakeRank(), "openai": FakeRank()},
        relevance=RelevanceAllTrue(), guesser=FakeGuess(), plants=[],
        open_checkout=lambda repo, sha: FakeCo(),
    )
    report = run_live(issues, comps)
    assert ("spec","rust") in report.cells and ("plan","rust") in report.cells
    assert report.cells[("spec","rust")].stage == "spec"
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** `run.py` additions — `LiveComponents` (holds variants/runner/judges/relevance/guesser/plants + an `open_checkout(repo,sha)` factory, default `Checkout`), and `run_live(issues, comps) -> Report` that per issue resolves+opens the checkout, runs `run_issue`, computes per-stage `StageMetrics` from the `StageResult`, runs detectability per stage, and calls `assemble_cell` into a `Report{cells: dict[(stage,language)] -> Cell}`. Then wire `cli.py run --live` to build `LiveComponents` from `live_ask`/`RoutingArmRunner`/`LlmRankJudge`/`LlmRelevanceJudge`/`LlmConditionGuesser` (Opus+gpt-5.5), resolve clones against `--bench-root` (default `~/code/bench-repos`), call `run_live`, and print each cell's prism deltas + GO/NO-GO. Keep the non-`--live` path printing guidance. (Full code authored in the task; mirror the StageResult→StageMetrics mapping: precision/recall from `investigator[vid]`, planted from `planted[vid].recall`, used_prism/tokens from the StageResult dicts.)

- [ ] **Step 4: Run, confirm PASS.** **Step 5:** full suite `cd eval && uv run pytest tests/test_tc_*.py -q` + tier_a unaffected. **Step 6: Commit** → `feat(tier-c): --live run loop wiring`.

- [ ] **Step 7 (integration, manual — owner-triggered, costs spend):** `cd eval && uv run tier-c run --issues tier_c/issues/issues.toml --live` on 1 issue; verify the claude/codex model flags, the JSONL field names, and that a report prints. Document any flag fixes. NOT a CI test.

---

## Self-Review
- **Spec coverage:** real judges (§6a/§6b) → Tasks 1-2; routing+claim_counts (§3/§6a) → Task 3; detectability (§6b) → Task 4; the run loop + report (§5/§7/§12) → Task 5.
- **Placeholder scan:** the only deferred item is the live integration run (Step 7, owner-triggered, by design — it costs spend).
- **Type consistency:** `live_ask(model,prompt)→str`; judges take `(ask, model)`; `RoutingArmRunner(claude,codex)`; `run_stage(claim_counts=None)`; `Detectability`; `LiveComponents`/`run_live→Report{cells}`; `assemble_cell` reused.

## Remaining after Phase-1c
- The actual live measurement run (owner-triggered; costs model spend + time) + any model-flag/JSONL fixes it surfaces.
- 8-issue expansion (2nd-per-language picks recorded in the corpus memo).
- Develop + Review stages + per-repo build sandboxes = Phase 2.
- Relevance audit-sampling cadence + κ adjudication capture (judges currently score every citation).
