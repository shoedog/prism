# Tier-C Phase-1b — Live drivers, report assembly, run orchestrator (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Close the Phase-1 known gaps that make `eval/tier_c/` actually RUNNABLE end-to-end: real model-call drivers (codex/claude), the `claim_count` extractor, random tie-break + detectability wiring, the per-(stage×language) **report assembly** with the GO/NO-GO gate, and the **run orchestrator** + `tier-c run` CLI.

**Architecture:** Extends the Phase-1 package (spec `docs/superpowers/specs/2026-06-23-tier-c-value-measurement-design.md`; Phase-1 plan `docs/superpowers/plans/2026-06-23-tier-c-phase1-harness.md`). Live model I/O sits behind the existing `ArmRunner` Protocol; output **parsers are pure functions** (TDD on captured/canned JSON), the subprocess spawn is the only un-unit-tested seam (a live-run integration step). LLM-judgment seams (relevance, condition-guesser) stay fakeable. Develop+review stages remain Phase 2.

**Tech Stack:** Python 3.12, `subprocess`, `json`, `dataclasses`, `pytest`, `uv`. Models: claude via `claude -p --output-format json`; codex via `codex exec --json`.

**Grounding — real `claude -p --output-format json` schema (captured 2026-06-23):**
```json
{"type":"result","subtype":"success","is_error":false,"num_turns":1,
 "result":"OK","total_cost_usd":0.0598,
 "usage":{"input_tokens":3,"output_tokens":4,"cache_read_input_tokens":16945,"cache_creation_input_tokens":9120}}
```
codex `--json` emits JSONL events to stdout (one JSON object per line; includes token-usage + tool-call + final agent-message events) — exact field names MUST be verified against a live `codex exec --json` capture before the first run (Task 2 Step 5).

---

### Task 1: Output parsers (pure, TDD on real/canned JSON)

**Files:** Create `eval/tier_c/parse.py`; Test `eval/tests/test_tc_parse.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_parse.py
import json
from tier_c.parse import parse_claude_json, parse_codex_jsonl

CLAUDE = json.dumps({"type":"result","is_error":False,"num_turns":3,"result":"spec sees src/a.py:1",
                     "total_cost_usd":0.05,"usage":{"input_tokens":10,"output_tokens":42}})

def test_parse_claude_extracts_text_tokens_cost_turns():
    r = parse_claude_json(CLAUDE)
    assert r.text == "spec sees src/a.py:1"
    assert r.output_tokens == 42 and r.input_tokens == 10
    assert abs(r.cost_usd - 0.05) < 1e-9
    assert r.tool_calls == 2  # num_turns - 1 (best-effort proxy; stream-json needed for exact)

def test_parse_claude_error_raises():
    import pytest
    with pytest.raises(ValueError, match="claude"):
        parse_claude_json(json.dumps({"type":"result","is_error":True,"result":None,"usage":{}}))

def test_parse_codex_jsonl_picks_last_message_and_sums_tokens():
    lines = "\n".join([
        json.dumps({"type":"item.completed","item":{"type":"reasoning"}}),
        json.dumps({"type":"item.completed","item":{"type":"command_execution"}}),  # a tool call
        json.dumps({"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":20}}),
        json.dumps({"type":"item.completed","item":{"type":"agent_message","text":"plan: src/b.go:9"}}),
    ])
    r = parse_codex_jsonl(lines)
    assert r.text == "plan: src/b.go:9"
    assert r.output_tokens == 20
    assert r.tool_calls == 1
```

- [ ] **Step 2: Run, confirm FAIL** — `cd eval && uv run pytest tests/test_tc_parse.py -q` → ModuleNotFound.

- [ ] **Step 3: Implement**
```python
# eval/tier_c/parse.py
"""Model-output parsers (Phase-1b). Pure functions over the CLIs' JSON so they are
fully unit-tested; the subprocess spawn (arm_runner) is the only un-tested seam.
claude: `--output-format json` single object. codex: `--json` JSONL events."""
from __future__ import annotations
import json
from dataclasses import dataclass

@dataclass(frozen=True)
class ModelResult:
    text: str
    input_tokens: int
    output_tokens: int
    tool_calls: int
    cost_usd: float

def parse_claude_json(out: str) -> ModelResult:
    d = json.loads(out)
    if d.get("is_error") or not d.get("result"):
        raise ValueError(f"claude run failed: {d.get('subtype') or d.get('api_error_status')}")
    u = d.get("usage", {})
    return ModelResult(
        text=d["result"],
        input_tokens=int(u.get("input_tokens", 0)),
        output_tokens=int(u.get("output_tokens", 0)),
        tool_calls=max(0, int(d.get("num_turns", 1)) - 1),  # best-effort; exact needs stream-json
        cost_usd=float(d.get("total_cost_usd", 0.0)),
    )

# codex --json event item types that count as a tool call (verify against live output, Task 2 Step 5):
_CODEX_TOOL_ITEMS = {"command_execution", "mcp_tool_call", "file_change", "web_search"}

def parse_codex_jsonl(out: str) -> ModelResult:
    text, inp, outp, tools = "", 0, 0, 0
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        item = ev.get("item") or {}
        if item.get("type") == "agent_message" and item.get("text"):
            text = item["text"]              # last agent message wins
        if item.get("type") in _CODEX_TOOL_ITEMS:
            tools += 1
        u = ev.get("usage") or {}
        if u:
            inp = int(u.get("input_tokens", inp))
            outp = int(u.get("output_tokens", outp))
    if not text:
        raise ValueError("codex run produced no agent_message")
    return ModelResult(text=text, input_tokens=inp, output_tokens=outp, tool_calls=tools, cost_usd=0.0)
```

- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** `parse.py` + test → `feat(tier-c): codex/claude output parsers`.

---

### Task 2: Live runners (`ClaudeRunner`, `CodexRunner`)

**Files:** Modify `eval/tier_c/arm_runner.py`; Test `eval/tests/test_tc_live_runner.py`

- [ ] **Step 1: Failing test** (subprocess monkeypatched — no live call in CI)
```python
# eval/tests/test_tc_live_runner.py
import json
from tier_c.model import Variant
from tier_c.arm_runner import ClaudeRunner, CodexRunner

def test_claude_runner_builds_output(monkeypatch):
    captured = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        captured["cmd"] = cmd
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":2,
                  "result":"spec cites src/a.py:1","total_cost_usd":0.01,
                  "usage":{"input_tokens":5,"output_tokens":7}}); returncode = 0; stderr = ""
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    out = ClaudeRunner(mcp_cfg="/tmp/p.json").run(Variant("opus-4.8", True), "spec", "PROMPT", "/repo")
    assert out.text == "spec cites src/a.py:1"
    assert out.tokens == 7 and out.tool_calls == 1
    assert out.citations[0].file == "src/a.py"
    assert out.used_prism is True  # prism-ON variant + a tool call occurred
    assert "--mcp-config" in captured["cmd"]

def test_codex_runner_off_has_no_prism(monkeypatch):
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None):
        captured = cmd
        class R:
            stdout = "\n".join([
                json.dumps({"item":{"type":"command_execution"}}),
                json.dumps({"usage":{"input_tokens":3,"output_tokens":9}}),
                json.dumps({"item":{"type":"agent_message","text":"plan src/b.go:2"}})])
            returncode = 0; stderr = ""
        assert "mcp_servers.prism" not in " ".join(cmd)
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    out = CodexRunner().run(Variant("gpt-5.5", False), "plan", "PROMPT", "/repo")
    assert out.text == "plan src/b.go:2" and out.tokens == 9
    assert out.used_prism is False
```

- [ ] **Step 2: Run, confirm FAIL.**

- [ ] **Step 3: Implement** (append to `arm_runner.py`; `import subprocess, time` are re-added — now used)
```python
# append to eval/tier_c/arm_runner.py  (add at top: import subprocess, time)
from .parse import parse_claude_json, parse_codex_jsonl
from .model import ArmOutput

_TIMEOUT = 1800  # 30 min per arm call

class ClaudeRunner:
    """ArmRunner via `claude -p --output-format json`. prism ON = --mcp-config."""
    def __init__(self, mcp_cfg: str):
        self.mcp_cfg = mcp_cfg
    def run(self, variant, stage, prompt, repo_root) -> ArmOutput:
        cmd = build_claude_cmd(variant, mcp_cfg=self.mcp_cfg) + [prompt]
        t0 = time.monotonic()
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=repo_root, timeout=_TIMEOUT)
        r = parse_claude_json(proc.stdout)
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=variant.prism and r.tool_calls > 0)

class CodexRunner:
    """ArmRunner via `codex exec --json` (prompt on stdin). prism ON = inline -c mcp_servers."""
    def run(self, variant, stage, prompt, repo_root) -> ArmOutput:
        cmd = build_codex_cmd(variant, repo=repo_root)
        cmd.insert(2, "--json")  # codex exec --json ...
        t0 = time.monotonic()
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                              cwd=repo_root, timeout=_TIMEOUT)
        r = parse_codex_jsonl(proc.stdout)
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=variant.prism and r.tool_calls > 0)
```

- [ ] **Step 4: Run, confirm PASS.**
- [ ] **Step 5 (integration, manual — NOT CI):** capture one real codex `--json` run and confirm `_CODEX_TOOL_ITEMS` + the token/agent-message field names in `parse.py` match; adjust if needed. Document the captured shape in a comment.
- [ ] **Step 6: Commit** → `feat(tier-c): live claude/codex arm runners`.

---

### Task 3: `claim_count` extractor

**Files:** Create `eval/tier_c/claims.py`; Test `eval/tests/test_tc_claims.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_claims.py
from tier_c.claims import count_claims

def test_counts_sentences_asserting_code_facts():
    text = ("The matcher lives in globset. It calls compile(). The sky is blue. "
            "We must update the Glob struct.")
    # 3 code-claim sentences (globset, compile(), Glob struct); 'sky is blue' excluded
    assert count_claims(text) == 3

def test_minimum_one_when_any_code_token():
    assert count_claims("uses Foo") >= 1
    assert count_claims("hello world") == 0
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
```python
# eval/tier_c/claims.py
"""claim_count = the recall denominator (spec §6a, codex new-2): how many substantive
code-claims an output makes, so under-citing (claims without citations) is penalized.
Heuristic proxy: sentences that reference a code entity (identifier-ish / path / call)."""
from __future__ import annotations
import re

_SENT = re.compile(r"[.!?\n]+")
# a code-ish token: snake/camel ident with a call or _, a path, or CamelCase type
_CODE = re.compile(r"\b[a-z_][a-z0-9_]*\(\)|\b[a-z_][a-z0-9_]*_[a-z0-9_]+\b|"
                   r"\b[A-Z][a-zA-Z0-9]*[a-z][A-Z][a-zA-Z0-9]*\b|[\w/.-]+\.[a-z]{1,4}\b")

def count_claims(text: str) -> int:
    n = 0
    for sent in _SENT.split(text):
        if sent.strip() and _CODE.search(sent):
            n += 1
    return n
```

- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): claim_count extractor (recall denominator)`.

---

### Task 4: Random tie-break + tied-cell flag

**Files:** Modify `eval/tier_c/judges.py`; Test `eval/tests/test_tc_judges.py` (append)

- [ ] **Step 1: Failing test**
```python
# append to eval/tests/test_tc_judges.py
from tier_c.judges import borda_consensus, has_tie

def test_borda_seeded_random_tiebreak_is_deterministic():
    r = {"A": ["x","y"], "B": ["y","x"]}  # x,y tie
    o1 = borda_consensus(r, seed="issue1|spec")
    o2 = borda_consensus(r, seed="issue1|spec")
    assert o1 == o2                      # reproducible
    assert set(o1) == {"x","y"}

def test_has_tie_detects_top_tie():
    assert has_tie({"A": ["x","y"], "B": ["y","x"]})      # x,y both score equal
    assert not has_tie({"A": ["x","y"], "B": ["x","y"]})  # x strictly wins
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** — change `borda_consensus(rankings)` to `borda_consensus(rankings, seed=None)`: compute points as before, then if `seed` is given break ties by a seeded `random.Random(seed).shuffle` over the tied group (spec §5/§8 pre-registered random; not lexical), else keep the deterministic `-points, id` order. Add `has_tie(rankings) -> bool` = top-2 Borda points equal.
```python
# eval/tier_c/judges.py  (replace borda_consensus; add has_tie + import random)
import random

def _points(rankings):
    ids = {c for r in rankings.values() for c in r}
    pts = {c: 0 for c in ids}
    for r in rankings.values():
        n = len(r)
        for pos, c in enumerate(r):
            pts[c] += (n - pos)
    return pts

def borda_consensus(rankings, seed=None):
    pts = _points(rankings)
    if seed is None:
        return sorted(pts, key=lambda c: (-pts[c], c))
    rng = random.Random(seed)
    groups = {}
    for c, p in pts.items():
        groups.setdefault(p, []).append(c)
    out = []
    for p in sorted(groups, reverse=True):
        g = groups[p][:]
        rng.shuffle(g)
        out.extend(g)
    return out

def has_tie(rankings) -> bool:
    pts = sorted(_points(rankings).values(), reverse=True)
    return len(pts) >= 2 and pts[0] == pts[1]
```
(Keep `family_bias`, `detectable` unchanged. `chain.py` calls `borda_consensus` with no seed → still works; wire the seed in Task 7.)

- [ ] **Step 4: Run, confirm PASS** (full `tests/test_tc_judges.py` + `tests/test_tc_chain.py`). **Step 5: Commit** → `feat(tier-c): pre-registered random tie-break + has_tie`.

---

### Task 5: Detectability wiring

**Files:** Modify `eval/tier_c/judges.py`; Test `eval/tests/test_tc_judges.py` (append)

- [ ] **Step 1: Failing test**
```python
# append to eval/tests/test_tc_judges.py
from tier_c.judges import detectability_pvalue

def test_detectability_pvalue_low_when_guesser_is_accurate():
    # 10 issues, guesser got 9/10 prism-condition guesses right -> low p (detectable)
    p = detectability_pvalue(correct=9, n=10)
    assert p < 0.05
def test_detectability_pvalue_high_at_chance():
    assert detectability_pvalue(correct=5, n=10) > 0.2
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** — exact binomial tail (chance = 0.5) as the pre-registered permutation/exact test.
```python
# eval/tier_c/judges.py  (add; math import)
from math import comb

def detectability_pvalue(correct: int, n: int, chance: float = 0.5) -> float:
    """One-sided exact binomial P(X >= correct | p=chance). Low => prism condition is
    detectable above chance => the judge prism-delta is INVALID (spec §6b/new-7)."""
    if n <= 0:
        return 1.0
    return sum(comb(n, k) * chance**k * (1 - chance)**(n - k) for k in range(correct, n + 1))
```
(`detectable(...)` from Phase-1 stays; the gate uses `detectability_pvalue(...) < 0.05` → `detectable_judges=True`.)

- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): exact-binomial detectability p-value`.

---

### Task 6: Report assembly (per stage×language, deltas, GO/NO-GO)

**Files:** Modify `eval/tier_c/report.py`; Test `eval/tests/test_tc_report.py` (append)

- [ ] **Step 1: Failing test**
```python
# append to eval/tests/test_tc_report.py
from tier_c.report import StageMetrics, assemble_cell, Cell

def test_assemble_cell_computes_prism_deltas_and_gate():
    # one stage, one language; precision ON vs OFF per model
    per_id = {
        "opus-4.8+prism": StageMetrics(precision=0.9, recall=0.8, planted=0.7, used_prism=True, tokens=100),
        "opus-4.8":       StageMetrics(precision=0.6, recall=0.5, planted=0.4, used_prism=False, tokens=90),
    }
    cell = assemble_cell(stage="spec", language="python", per_id=per_id,
                         models=["opus-4.8"], analyze_failure_rate=0.0, detectable=False)
    assert isinstance(cell, Cell)
    assert abs(cell.prism_precision_delta["opus-4.8"] - 0.3) < 1e-9
    assert cell.gate.decision == "GO"          # material lift, low failure
    assert cell.itt_used_prism_rate == 0.5     # 1 of 2 variants actually used prism
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** — `StageMetrics` (per-variant), `Cell` (a stage×language result), `assemble_cell` (per-model prism deltas via `prism_delta`, ITT/per-protocol from `used_prism`, family-bias band passthrough, `gate_decision` from the max objective delta). Build on the existing `prism_delta`/`gate_decision`.
```python
# eval/tier_c/report.py  (append)
from dataclasses import dataclass, field

@dataclass(frozen=True)
class StageMetrics:
    precision: float
    recall: float
    planted: float
    used_prism: bool
    tokens: int

@dataclass(frozen=True)
class Cell:
    stage: str
    language: str
    prism_precision_delta: dict        # model -> ON-OFF
    prism_recall_delta: dict
    prism_planted_delta: dict
    itt_used_prism_rate: float
    gate: Gate

def assemble_cell(*, stage, language, per_id, models, analyze_failure_rate, detectable,
                  family_bias_band: float = 0.0) -> Cell:
    def dlt(attr):
        return {m: getattr(per_id.get(f"{m}+prism"), attr, 0.0) - getattr(per_id.get(m), attr, 0.0)
                for m in models if (f"{m}+prism" in per_id and m in per_id)}
    pd, rd, ld = dlt("precision"), dlt("recall"), dlt("planted")
    used = [v.used_prism for v in per_id.values()]
    itt = sum(used) / len(used) if used else 0.0
    best = max([max(pd.values(), default=0.0), max(rd.values(), default=0.0), max(ld.values(), default=0.0)])
    gate = gate_decision(precision_delta=max(pd.values(), default=0.0),
                         recall_delta=max(rd.values(), default=0.0),
                         planted_delta=max(ld.values(), default=0.0),
                         analyze_failure_rate=analyze_failure_rate, cost_ok=True,
                         detectable_judges=detectable)
    return Cell(stage, language, pd, rd, ld, itt, gate)
```

- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): per stage×language report cell + deltas + gate`.

---

### Task 7: Run orchestrator + `tier-c run`

**Files:** Modify `eval/tier_c/cli.py`; Create `eval/tier_c/run.py`; Test `eval/tests/test_tc_run.py`

- [ ] **Step 1: Failing test** (fakes for runner/judges/relevance; no live calls)
```python
# eval/tests/test_tc_run.py
from tier_c.model import Issue, Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.investigator import RelevanceAllTrue
from tier_c.run import run_issue

class FakeCo:
    root = "."
    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line): return "x" if rel == "a.py" else None
class FakeRank:
    def rank(self, stage, rubric, candidates): return sorted(candidates, key=lambda k: -len(candidates[k]))

def test_run_issue_returns_chain_with_two_stages():
    issue = Issue("k","python","r","sha","u","bug text","slice 1")
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec a.py:1 with detail", "gpt-5.5": "x"})
    res = run_issue(issue, variants=variants, runner=runner, co=FakeCo(),
                    judges={"anthropic": FakeRank(), "openai": FakeRank()},
                    relevance=RelevanceAllTrue(), plants=[])
    assert [s.stage for s in res.stages] == ["spec", "plan"]
    assert res.provenance.spec_best in {v.id for v in variants}
```

- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** `run.py` — `run_issue` wires `prompts.stage_prompt` as `prompt_fn`, computes `claim_counts` via `claims.count_claims` on each output is circular, so instead pass a fixed per-stage `claim_counts` derived AFTER outputs exist: have `run_spec_plan_chain` accept a `claim_count_fn`; simplest for Phase-1b — compute `claim_counts` as `{v.id: 1}` placeholder and leave the real per-output claim counting as a documented follow-up wired through `count_claims(output.text)` inside `run_stage`. Keep `run_issue` returning the `ChainResult`. Then `cli.py` gains a `run` subcommand that loads issues, and (live path) constructs `ClaudeRunner`/`CodexRunner` — but the live multi-model loop is gated behind a `--live` flag and is the first real execution (not unit-tested).
```python
# eval/tier_c/run.py
"""Run orchestrator (spec §5). run_issue drives one open issue through the spec->plan
chain with whatever ArmRunner/judges/investigator are supplied (fakes in tests, live
runners in a real run). The corpus + live runners are wired by cli.py run --live."""
from __future__ import annotations
from .chain import run_spec_plan_chain, ChainResult
from .prompts import stage_prompt

def run_issue(issue, *, variants, runner, co, judges, relevance, plants,
              claim_counts=None) -> ChainResult:
    cc = claim_counts or {v.id: 1 for v in variants}
    return run_spec_plan_chain(
        issue_text=issue.text, scoped_slice=issue.scoped_slice, variants=variants,
        runner=runner, co=co, claim_counts=cc, plants=plants, judges=judges,
        relevance=relevance, prompt_fn=stage_prompt)
```

- [ ] **Step 4: Run, confirm PASS.**
- [ ] **Step 5:** add a `run` subcommand to `cli.py` (`tier-c run --issues <f> [--live]`): without `--live`, prints "live run requires --live + corpus + API"; the `--live` wiring (construct runners, loop issues with `checkout.Checkout`, assemble report) is the first execution — leave a clear TODO comment citing this plan. Run the FULL suite `cd eval && uv run pytest tests/test_tc_*.py -q` + confirm tier_a unaffected. **Step 6: Commit** → `feat(tier-c): run orchestrator + tier-c run subcommand`.

---

## Self-Review
- **Spec coverage:** live driver (§3,§10) → Tasks 1-2; recall denominator (§6a) → Task 3; tie-break (§5/§8) → Task 4; detectability (§6b) → Task 5; report+gate (§7,§12) → Task 6; orchestrator (§5) → Task 7.
- **Placeholder scan:** none; the two genuinely-deferred seams (real `--live` loop, per-output `claim_counts` wiring) are explicitly flagged as the first-execution work, not silent gaps.
- **Type consistency:** `ModelResult`, `ArmOutput`, `ClaudeRunner`/`CodexRunner` (satisfy `ArmRunner`), `StageMetrics`/`Cell`, `borda_consensus(rankings, seed=)`, `run_issue`→`ChainResult` consistent across tasks.

## Remaining after Phase-1b (still before a trustworthy live verdict)
- Real `RelevanceJudge` (LLM, audit-sampled, blind) + `ConditionGuesser` for the detectability test (currently fakes/pure-math seams).
- Per-output `claim_counts` wired through `run_stage` (Task 7 uses a `{id:1}` placeholder).
- Sanitation **semantic** survival check + re-run loop (spec §5).
- Cold leakage probe (spec §2) using the live runner.
- The `--live` execution loop itself + **human-in-loop issue selection** → `eval/tier_c/issues/issues.toml`.
- Develop + Review stages + per-repo build sandboxes = **Phase 2**.
