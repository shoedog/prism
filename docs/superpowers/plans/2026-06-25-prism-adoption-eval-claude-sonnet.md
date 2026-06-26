# Prism Adoption Eval (Claude + Sonnet, v1) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a deepeval-driven eval that measures whether the prism-code-navigation skill loads and the `mcp__prism__nav_*` tools get invoked when a Claude (Sonnet) agent is given navigation tasks, so the SKILL.md can be iterated to `pass^5 ≥ 80%` invocation reliability.

**Architecture:** A small Python package `eval/adoption/` (same `uv` style as `eval/tier_c`). A **runner** invokes `claude -p --model sonnet` in an isolated `CLAUDE_CONFIG_DIR` (only the prism skill + prism MCP), `k=5` trials/probe, and caches each parsed **trajectory** keyed by the SKILL.md content hash (so re-scoring is free; only a real skill edit re-spends). A **deepeval suite** loads cached trajectories into `LLMTestCase`s and scores them with a deterministic `SkillActivationMetric` (custom) + `ToolCorrectnessMetric`; an **aggregator** computes `pass^5` per probe → `benchmark.json`. The loop: run → read failures → edit `skills/prism-code-navigation/SKILL.md` → re-run.

**Tech Stack:** Python 3.9+, `uv`, `deepeval`, the `claude` CLI (`--output-format stream-json`), prism-mcp.

**Spec:** `docs/superpowers/specs/2026-06-25-prism-adoption-eval-design.md`.

---

## File Structure

| File | Responsibility |
|---|---|
| `eval/adoption/__init__.py` | package marker |
| `eval/adoption/model.py` | `Probe`, `Trajectory` dataclasses + helpers |
| `eval/adoption/trajectory.py` | parse `claude` stream-json JSONL → `Trajectory` |
| `eval/adoption/goldens.py` | load `probes.toml` → `list[Probe]` |
| `eval/adoption/goldens/probes.toml` | the ~12 micro-probes |
| `eval/adoption/env.py` | build the isolated `CLAUDE_CONFIG_DIR` + MCP config |
| `eval/adoption/runner.py` | live `claude -p` k-trial runner + hash-keyed trajectory cache |
| `eval/adoption/testcase.py` | `Trajectory` + `Probe` → deepeval `LLMTestCase` |
| `eval/adoption/metrics.py` | `SkillActivationMetric` (custom) + metric lists |
| `eval/adoption/aggregate.py` | `pass^k` pure functions + `benchmark.json` writer |
| `eval/adoption/tests/test_prism_adoption.py` | deepeval suite (`deepeval test run` target) |
| `eval/adoption/tests/unit/` | pytest unit tests for the pure modules |
| `eval/adoption/tests/fixtures/` | committed stream-json fixtures |
| `eval/adoption/results/` | gitignored: trajectory cache + benchmark.json |

Conventions: 1-indexed lines, BTreeMap-style deterministic ordering, mirror `eval/tier_c` import style. Run everything from `eval/` via `uv run`.

---

## Task 1: Verify `CLAUDE_CONFIG_DIR` skill isolation (load-bearing spike) + env builder

**Files:**
- Create: `eval/adoption/__init__.py`, `eval/adoption/env.py`
- Create: `eval/adoption/tests/unit/test_env.py`

This gates the whole design (spec Risk #1). Build the isolated-env builder, unit-test its file layout, then **live-verify** that `claude -p` honors it: prism MCP connects, the prism skill is present, and the user's `superpowers` skills are NOT.

- [ ] **Step 1: Write the failing test for the env builder layout**

```python
# eval/adoption/tests/unit/test_env.py
import json, os
from adoption.env import build_isolated_config

def test_build_isolated_config_layout(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    (skill_src).mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    cfg = build_isolated_config(skill_src=str(skill_src), mcp_repo="/repo/x",
                                prism_mcp_bin="/bin/prism-mcp", root=str(tmp_path / "iso"))
    # the skill is present under the isolated config's skills dir
    assert os.path.isfile(os.path.join(cfg.config_dir, "skills", "prism-code-navigation", "SKILL.md"))
    # the MCP config points prism at the repo
    mcp = json.load(open(cfg.mcp_cfg))
    assert mcp["mcpServers"]["prism"]["args"] == ["--repo", "/repo/x"]
    # NO settings hooks leaked
    assert cfg.config_dir != os.path.expanduser("~/.claude")
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_env.py -v` → FAIL (`No module named env`).

- [ ] **Step 3: Implement `env.py`**

```python
# eval/adoption/env.py
"""Build an ISOLATED claude config home containing ONLY the prism skill + prism MCP.
This is the realistic-but-controlled deployment env (spec §Deployment): no SessionStart
hooks, no other skills (the prior run leaked superpowers:* — a confound)."""
from __future__ import annotations
import json, os, shutil, tempfile
from dataclasses import dataclass

@dataclass(frozen=True)
class IsolatedConfig:
    config_dir: str   # value for CLAUDE_CONFIG_DIR
    mcp_cfg: str      # path to the --mcp-config json

def build_isolated_config(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                          root: str | None = None) -> IsolatedConfig:
    base = root or tempfile.mkdtemp(prefix="tc-adopt-cfg-")
    cfg_dir = os.path.join(base, "config")
    skills_dir = os.path.join(cfg_dir, "skills")
    os.makedirs(skills_dir, exist_ok=True)
    dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(skill_src, dst)
    # minimal settings: explicitly NO hooks (empty), so nothing is injected.
    with open(os.path.join(cfg_dir, "settings.json"), "w") as f:
        json.dump({"hooks": {}}, f)
    mcp_cfg = os.path.join(base, "mcp.json")
    with open(mcp_cfg, "w") as f:
        json.dump({"mcpServers": {"prism": {"command": prism_mcp_bin,
                                            "args": ["--repo", mcp_repo]}}}, f)
    return IsolatedConfig(config_dir=cfg_dir, mcp_cfg=mcp_cfg)
```

- [ ] **Step 4: Run the unit test, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_env.py -v`.

- [ ] **Step 5: LIVE verification of isolation** (the load-bearing check). Build prism-mcp first, then run a trivial probe and inspect the init record.

```bash
cd /Users/wesleyjinks/code/slicing
cargo build --release --bin prism-mcp --features mcp
cd eval && uv run python -c "
from adoption.env import build_isolated_config
import subprocess, json, os
cfg = build_isolated_config(skill_src='../skills/prism-code-navigation', mcp_repo=os.path.abspath('tier_c'),
                            prism_mcp_bin=os.path.abspath('../target/release/prism-mcp'))
env = dict(os.environ); env['CLAUDE_CONFIG_DIR'] = cfg.config_dir
p = subprocess.run(['claude','-p','--output-format','stream-json','--verbose','--model','sonnet',
                    '--mcp-config', cfg.mcp_cfg, '--strict-mcp-config', 'Say only: ready.'],
                   capture_output=True, text=True, cwd='tier_c', env=env, timeout=180)
recs=[json.loads(l) for l in p.stdout.splitlines() if l.strip()]
init=[r for r in recs if r.get('type')=='system' and r.get('subtype')=='init'][0]
print('mcp_servers:', init.get('mcp_servers'))
print('prism tools present:', [t for t in init.get('tools',[]) if 'prism' in str(t)])
print('slash/skills leaked superpowers?:', any('superpower' in str(x).lower() for x in init.get('slash_commands',[])+init.get('tools',[])))
"
```
Expected: `mcp_servers: [{'name':'prism','status':'connected'}]`, prism tools present, **no superpowers leak**. If MCP not connected or superpowers leaks → STOP, switch to the documented fallback (a minimal `HOME` override) and note it in the spec before continuing.

- [ ] **Step 6: Commit** — `git add eval/adoption/__init__.py eval/adoption/env.py eval/adoption/tests/unit/test_env.py && git commit -m "feat(adoption): isolated CLAUDE_CONFIG_DIR env builder + live isolation check"`

---

## Task 2: Add the `deepeval` dependency

**Files:** Modify `eval/pyproject.toml`

- [ ] **Step 1: Add `deepeval` to eval deps**

Add `"deepeval"` to the `dependencies` array in `eval/pyproject.toml` (alongside the existing entries).

- [ ] **Step 2: Sync + verify** — `cd eval && uv sync && uv run python -c "import deepeval; from deepeval.test_case import LLMTestCase, ToolCall; from deepeval.metrics import BaseMetric, ToolCorrectnessMetric; print('deepeval ok')"`
Expected: `deepeval ok`.

- [ ] **Step 3: Verify the CLI** — `cd eval && uv run deepeval --help | head -3` → shows deepeval usage.

- [ ] **Step 4: Commit** — `git add eval/pyproject.toml eval/uv.lock && git commit -m "build(adoption): add deepeval dependency"`

---

## Task 3: `model.py` + `trajectory.py` — parse claude stream-json

**Files:**
- Create: `eval/adoption/model.py`, `eval/adoption/trajectory.py`
- Create: `eval/adoption/tests/unit/test_trajectory.py`
- Create fixtures: `eval/adoption/tests/fixtures/with_prism.jsonl`, `without_prism.jsonl`, `with_skill.jsonl`

- [ ] **Step 1: Create committed fixtures.** Copy the captured repro JSONL (proven real claude stream-json):
```bash
cp /tmp/r_E.jsonl eval/adoption/tests/fixtures/with_prism.jsonl   # has mcp__prism__nav_* calls
cp /tmp/r_C.jsonl eval/adoption/tests/fixtures/without_prism.jsonl # only Bash/Read
```
Then hand-author `with_skill.jsonl` (3 lines) containing one assistant `Skill` tool_use loading prism-nav:
```jsonl
{"type":"system","subtype":"init","mcp_servers":[{"name":"prism","status":"connected"}],"tools":["mcp__prism__nav_callers"]}
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"prism-code-navigation"}}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__prism__nav_callers","input":{"symbol":"foo"}},{"type":"text","text":"foo is called at a.py:1"}]}}
```

- [ ] **Step 2: Write the failing test**

```python
# eval/adoption/tests/unit/test_trajectory.py
from adoption.trajectory import parse_stream_json
F = "adoption/tests/fixtures"  # cwd = eval/ when running pytest

def test_detects_prism_calls():
    t = parse_stream_json(open(f"{F}/with_prism.jsonl").read())
    assert t.prism_nav_calls()              # e.g. ['nav_nodes_at','nav_callers',...]
    assert "nav_callers" in t.prism_nav_calls()

def test_no_prism_calls_in_baseline():
    t = parse_stream_json(open(f"{F}/without_prism.jsonl").read())
    assert t.prism_nav_calls() == []

def test_detects_skill_load_and_args():
    t = parse_stream_json(open(f"{F}/with_skill.jsonl").read())
    assert t.loaded_prism_skill() is True
    assert ("nav_callers", {"symbol": "foo"}) in t.tool_calls
    assert t.final_text == "foo is called at a.py:1"
```

- [ ] **Step 3: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_trajectory.py -v` → FAIL.

- [ ] **Step 4: Implement `model.py`**

```python
# eval/adoption/model.py
from __future__ import annotations
from dataclasses import dataclass, field

@dataclass(frozen=True)
class Probe:
    id: str
    kind: str                  # "nav" | "negative"
    prompt: str
    repo: str                  # path (relative to eval/) of the small target repo
    expected_tools: list[str]  # bare nav names, e.g. ["nav_callers"]; [] for negatives
    expected_symbol: str | None = None

@dataclass(frozen=True)
class Trajectory:
    final_text: str
    skill_loads: list[str]               # skill names loaded via the Skill tool
    tool_calls: list[tuple[str, dict]]   # (bare_or_builtin_name, input)
    def prism_nav_calls(self) -> list[str]:
        return [n for n, _ in self.tool_calls if n.startswith("nav_")]
    def loaded_prism_skill(self) -> bool:
        return any("prism" in s.lower() for s in self.skill_loads)
```

- [ ] **Step 5: Implement `trajectory.py`**

```python
# eval/adoption/trajectory.py
"""Parse `claude -p --output-format stream-json` JSONL into a Trajectory. mcp__prism__nav_X
tool names are normalised to bare `nav_X` so probes match on the nav verb."""
from __future__ import annotations
import json
from .model import Trajectory

def _norm(name: str) -> str:
    # mcp__prism__nav_callers -> nav_callers ; leave builtins (Bash/Read/Skill) as-is
    return name.split("__")[-1] if name.startswith("mcp__prism__") else name

def parse_stream_json(out: str) -> Trajectory:
    final_text, skill_loads, calls = "", [], []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue
        if r.get("type") != "assistant":
            continue
        for c in r.get("message", {}).get("content", []) or []:
            if not isinstance(c, dict):
                continue
            if c.get("type") == "text" and c.get("text", "").strip():
                final_text = c["text"]
            elif c.get("type") == "tool_use":
                name = c.get("name", "")
                inp = c.get("input", {}) or {}
                if name == "Skill":
                    skill_loads.append(str(inp.get("skill", "")))
                else:
                    calls.append((_norm(name), inp))
    return Trajectory(final_text=final_text, skill_loads=skill_loads, tool_calls=calls)
```

- [ ] **Step 6: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_trajectory.py -v`.

- [ ] **Step 7: Commit** — `git add eval/adoption/model.py eval/adoption/trajectory.py eval/adoption/tests/unit/test_trajectory.py eval/adoption/tests/fixtures/ && git commit -m "feat(adoption): stream-json trajectory parser (prism calls + skill loads)"`

---

## Task 4: `goldens/probes.toml` + `goldens.py` loader

**Files:**
- Create: `eval/adoption/goldens/probes.toml`, `eval/adoption/goldens.py`
- Create: `eval/adoption/tests/unit/test_goldens.py`

- [ ] **Step 1: Author `probes.toml`** (12 probes — the spec table + breadth fillers; all target `tier_c`, a small Python pkg)

```toml
[[probe]]
id = "callers-count-claims"
kind = "nav"
prompt = "List every call site of `count_claims` as file:line. Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_callers"]
expected_symbol = "count_claims"

[[probe]]
id = "callees-run-stage"
kind = "nav"
prompt = "What functions does `run_stage` call? List them. Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_callees"]
expected_symbol = "run_stage"

[[probe]]
id = "impact-stage-prompt"
kind = "nav"
prompt = "What call sites break if I change the signature of `stage_prompt`? List them. Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_callers", "nav_ego_graph"]
expected_symbol = "stage_prompt"

[[probe]]
id = "nodes-at-chain-35"
kind = "nav"
prompt = "What symbol is defined at chain.py:35? Answer in 1 line."
repo = "tier_c"
expected_tools = ["nav_nodes_at"]

[[probe]]
id = "repo-map-top3"
kind = "nav"
prompt = "What are the 3 most-depended-on modules in this package? Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_repo_map", "nav_module_deps"]

[[probe]]
id = "module-deps-cli"
kind = "nav"
prompt = "What does cli.py depend on within this package? Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_module_deps"]

[[probe]]
id = "ego-run-chain"
kind = "nav"
prompt = "Show the local call graph around `run_spec_plan_chain`. Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_ego_graph"]
expected_symbol = "run_spec_plan_chain"

[[probe]]
id = "callees-score-citations"
kind = "nav"
prompt = "What does `score_citations` call? Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_callees"]
expected_symbol = "score_citations"

[[probe]]
id = "callers-parse-citations"
kind = "nav"
prompt = "Who uses `parse_citations`? List call sites. Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_callers"]
expected_symbol = "parse_citations"

[[probe]]
id = "compound-run-35"
kind = "nav"
prompt = "What is defined at run.py:35, and who calls it? Answer in <=3 lines."
repo = "tier_c"
expected_tools = ["nav_nodes_at", "nav_callers"]

[[probe]]
id = "neg-docstring-salt"
kind = "negative"
prompt = "Add a one-line docstring to the `_salt` function in chain.py. Show only the new line."
repo = "tier_c"
expected_tools = []

[[probe]]
id = "neg-readme-typo"
kind = "negative"
prompt = "Suggest a one-line wording fix for the first sentence of README.md. Answer in 1 line."
repo = "tier_c"
expected_tools = []
```

- [ ] **Step 2: Write the failing test**

```python
# eval/adoption/tests/unit/test_goldens.py
from adoption.goldens import load_probes

def test_loads_all_probes():
    ps = load_probes()
    assert len(ps) == 12
    assert {p.kind for p in ps} == {"nav", "negative"}
    by = {p.id: p for p in ps}
    assert by["callers-count-claims"].expected_tools == ["nav_callers"]
    assert by["neg-docstring-salt"].expected_tools == []
```

- [ ] **Step 3: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_goldens.py -v` → FAIL.

- [ ] **Step 4: Implement `goldens.py`**

```python
# eval/adoption/goldens.py
from __future__ import annotations
import os, tomllib
from .model import Probe

_PATH = os.path.join(os.path.dirname(__file__), "goldens", "probes.toml")

def load_probes(path: str = _PATH) -> list[Probe]:
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return [Probe(id=p["id"], kind=p["kind"], prompt=p["prompt"], repo=p["repo"],
                  expected_tools=list(p.get("expected_tools", [])),
                  expected_symbol=p.get("expected_symbol"))
            for p in data["probe"]]
```

- [ ] **Step 5: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_goldens.py -v`.

- [ ] **Step 6: Commit** — `git add eval/adoption/goldens.py eval/adoption/goldens/probes.toml eval/adoption/tests/unit/test_goldens.py && git commit -m "feat(adoption): 12 nav micro-probe goldens + loader"`

---

## Task 5: `testcase.py` — Trajectory + Probe → deepeval `LLMTestCase`

**Files:**
- Create: `eval/adoption/testcase.py`
- Create: `eval/adoption/tests/unit/test_testcase.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/adoption/tests/unit/test_testcase.py
from adoption.model import Probe, Trajectory
from adoption.testcase import build_test_case

def test_build_test_case_maps_tools_and_skill():
    probe = Probe(id="x", kind="nav", prompt="who calls foo", repo="tier_c",
                  expected_tools=["nav_callers"], expected_symbol="foo")
    traj = Trajectory(final_text="foo at a.py:1", skill_loads=["prism-code-navigation"],
                      tool_calls=[("nav_callers", {"symbol": "foo"})])
    tc = build_test_case(traj, probe)
    assert tc.input == "who calls foo"
    assert tc.actual_output == "foo at a.py:1"
    assert [t.name for t in tc.tools_called] == ["nav_callers"]
    assert [t.name for t in tc.expected_tools] == ["nav_callers"]
    assert tc.additional_metadata["prism_skill_loaded"] is True
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_testcase.py -v` → FAIL.

- [ ] **Step 3: Implement `testcase.py`**

```python
# eval/adoption/testcase.py
"""Map a parsed Trajectory + Probe into a deepeval LLMTestCase (no-tracing path).
tools_called = prism nav calls only (the signal); skill-load goes in additional_metadata
for the custom SkillActivationMetric."""
from __future__ import annotations
from deepeval.test_case import LLMTestCase, ToolCall
from .model import Probe, Trajectory

def build_test_case(traj: Trajectory, probe: Probe) -> LLMTestCase:
    tools_called = [ToolCall(name=n) for n in traj.prism_nav_calls()]
    expected = [ToolCall(name=n) for n in probe.expected_tools]
    return LLMTestCase(
        input=probe.prompt,
        actual_output=traj.final_text or "(no answer)",
        tools_called=tools_called,
        expected_tools=expected,
        additional_metadata={
            "prism_skill_loaded": traj.loaded_prism_skill(),
            "probe_id": probe.id, "kind": probe.kind,
        },
    )
```

- [ ] **Step 4: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_testcase.py -v`.

- [ ] **Step 5: Commit** — `git add eval/adoption/testcase.py eval/adoption/tests/unit/test_testcase.py && git commit -m "feat(adoption): Trajectory+Probe -> deepeval LLMTestCase"`

---

## Task 6: `metrics.py` — custom `SkillActivationMetric` + metric lists

**Files:**
- Create: `eval/adoption/metrics.py`
- Create: `eval/adoption/tests/unit/test_metrics.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/adoption/tests/unit/test_metrics.py
from deepeval.test_case import LLMTestCase, ToolCall
from adoption.metrics import SkillActivationMetric

def _tc(loaded):
    return LLMTestCase(input="i", actual_output="o", tools_called=[ToolCall(name="nav_callers")],
                       expected_tools=[ToolCall(name="nav_callers")],
                       additional_metadata={"prism_skill_loaded": loaded})

def test_skill_activation_passes_when_loaded():
    m = SkillActivationMetric()
    assert m.measure(_tc(True)) == 1.0 and m.is_successful()

def test_skill_activation_fails_when_not_loaded():
    m = SkillActivationMetric()
    assert m.measure(_tc(False)) == 0.0 and not m.is_successful()
    assert "not load" in m.reason.lower()
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_metrics.py -v` → FAIL.

- [ ] **Step 3: Implement `metrics.py`**

```python
# eval/adoption/metrics.py
"""Deterministic metrics carry the pass^5 signal (no LLM cost). SkillActivation reads the
skill-load flag stashed in additional_metadata; ToolCorrectness (deepeval) compares the
nav tools fired vs expected. ArgumentCorrectness/TaskCompletion (LLM-judge) are quality-only
and added later, not part of the v1 gate."""
from __future__ import annotations
from deepeval.metrics import BaseMetric, ToolCorrectnessMetric
from deepeval.test_case import LLMTestCase

class SkillActivationMetric(BaseMetric):
    """1.0 iff the prism-nav skill loaded in this trajectory (deterministic)."""
    def __init__(self, threshold: float = 1.0):
        self.threshold = threshold
        self.score = 0.0
        self.reason = ""
        self.success = False
    def measure(self, test_case: LLMTestCase) -> float:
        loaded = bool((test_case.additional_metadata or {}).get("prism_skill_loaded"))
        self.score = 1.0 if loaded else 0.0
        self.reason = "prism-nav skill loaded" if loaded else "prism-nav skill did not load"
        self.success = self.score >= self.threshold
        return self.score
    async def a_measure(self, test_case: LLMTestCase, *args, **kwargs) -> float:
        return self.measure(test_case)
    def is_successful(self) -> bool:
        return self.success
    @property
    def __name__(self):
        return "SkillActivation"

# The two deterministic gate metrics (no model credentials needed).
GATE_METRICS = [SkillActivationMetric(), ToolCorrectnessMetric()]
```

- [ ] **Step 4: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_metrics.py -v`.

- [ ] **Step 5: Commit** — `git add eval/adoption/metrics.py eval/adoption/tests/unit/test_metrics.py && git commit -m "feat(adoption): SkillActivation custom metric + deterministic gate metrics"`

---

## Task 7: `runner.py` — k-trial live runner + SKILL.md-hash trajectory cache

**Files:**
- Create: `eval/adoption/runner.py`
- Create: `eval/adoption/tests/unit/test_runner.py`

The live subprocess seam is exercised only in the smoke (Task 10). Unit-test the **cache key** + **command construction** (no spawn), matching `eval/tier_c`'s build-cmd test style.

- [ ] **Step 1: Write the failing test (pure parts)**

```python
# eval/adoption/tests/unit/test_runner.py
from adoption.runner import build_claude_cmd, cache_key

def test_build_claude_cmd_sonnet_with_mcp():
    cmd = build_claude_cmd(prompt="hi", mcp_cfg="/m.json", model="sonnet")
    assert cmd[:6] == ["claude","-p","--output-format","stream-json","--verbose"][:5] + ["--model"]
    assert "--mcp-config" in cmd and "/m.json" in cmd and "--strict-mcp-config" in cmd
    assert cmd[-1] == "hi"

def test_cache_key_changes_with_skill_hash():
    a = cache_key(skill_bytes=b"v1", probe_id="p", trial=0, model="sonnet")
    b = cache_key(skill_bytes=b"v2", probe_id="p", trial=0, model="sonnet")
    assert a != b
    assert a == cache_key(skill_bytes=b"v1", probe_id="p", trial=0, model="sonnet")
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_runner.py -v` → FAIL.

- [ ] **Step 3: Implement `runner.py`**

```python
# eval/adoption/runner.py
"""Invoke `claude -p --model sonnet` in the isolated env, k trials per probe, and cache
each parsed Trajectory keyed by (SKILL.md bytes, probe, trial, model) so re-scoring during
the iteration loop is free and only a real skill edit re-spends (spec §Cost controls)."""
from __future__ import annotations
import hashlib, json, os, subprocess
from dataclasses import asdict
from .model import Probe, Trajectory
from .trajectory import parse_stream_json
from .env import build_isolated_config, IsolatedConfig

_TIMEOUT = 600

def build_claude_cmd(*, prompt: str, mcp_cfg: str, model: str = "sonnet") -> list[str]:
    return ["claude", "-p", "--output-format", "stream-json", "--verbose",
            "--model", model, "--mcp-config", mcp_cfg, "--strict-mcp-config", prompt]

def cache_key(*, skill_bytes: bytes, probe_id: str, trial: int, model: str) -> str:
    h = hashlib.sha256()
    h.update(skill_bytes); h.update(f"|{probe_id}|{trial}|{model}".encode())
    return h.hexdigest()[:24]

def _cache_dir(results_root: str) -> str:
    d = os.path.join(results_root, "cache"); os.makedirs(d, exist_ok=True); return d

def run_trial(probe: Probe, trial: int, *, cfg: IsolatedConfig, eval_root: str,
              results_root: str, skill_bytes: bytes, model: str = "sonnet") -> Trajectory:
    key = cache_key(skill_bytes=skill_bytes, probe_id=probe.id, trial=trial, model=model)
    cpath = os.path.join(_cache_dir(results_root), key + ".json")
    if os.path.exists(cpath):
        d = json.load(open(cpath))
        return Trajectory(final_text=d["final_text"], skill_loads=d["skill_loads"],
                          tool_calls=[tuple(x) for x in d["tool_calls"]])
    env = dict(os.environ); env["CLAUDE_CONFIG_DIR"] = cfg.config_dir
    cmd = build_claude_cmd(prompt=probe.prompt, mcp_cfg=cfg.mcp_cfg, model=model)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_TIMEOUT,
                          cwd=os.path.join(eval_root, probe.repo), env=env)
    traj = parse_stream_json(proc.stdout)
    with open(cpath, "w") as f:
        json.dump({"final_text": traj.final_text, "skill_loads": traj.skill_loads,
                   "tool_calls": [list(t) for t in traj.tool_calls]}, f)
    return traj
```

- [ ] **Step 4: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_runner.py -v`.

- [ ] **Step 5: Commit** — `git add eval/adoption/runner.py eval/adoption/tests/unit/test_runner.py && git commit -m "feat(adoption): k-trial claude runner + skill-hash trajectory cache"`

---

## Task 8: `aggregate.py` — `pass^k` + benchmark writer

**Files:**
- Create: `eval/adoption/aggregate.py`
- Create: `eval/adoption/tests/unit/test_aggregate.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/adoption/tests/unit/test_aggregate.py
from adoption.aggregate import passes_k, summarize

def test_passes_k_requires_all():
    assert passes_k([True, True, True, True, True]) is True
    assert passes_k([True, True, False, True, True]) is False

def test_summarize_pass5_rate():
    # 2 nav probes: one all-5-pass, one not; 1 negative all-pass
    per_probe = {
        "a": {"kind": "nav", "invocation": [True]*5,  "activation": [True]*5},
        "b": {"kind": "nav", "invocation": [True]*4+[False], "activation": [True]*5},
        "n": {"kind": "negative", "invocation": [True]*5, "activation": [False]*5},
    }
    s = summarize(per_probe)
    assert s["nav_invocation_pass5_rate"] == 0.5   # 1 of 2 nav probes pass^5
    assert s["nav_count"] == 2
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_aggregate.py -v` → FAIL.

- [ ] **Step 3: Implement `aggregate.py`**

```python
# eval/adoption/aggregate.py
"""pass^k aggregation (spec §Success criteria). HEADLINE = nav invocation pass^5 rate
(fraction of nav probes where all k trials fired the right nav tool). Activation reported
alongside. Negatives pass when they DON'T over-reach (invocation False all k)."""
from __future__ import annotations
import json, os

def passes_k(trial_flags: list[bool]) -> bool:
    return bool(trial_flags) and all(trial_flags)

def summarize(per_probe: dict) -> dict:
    nav = {pid: r for pid, r in per_probe.items() if r["kind"] == "nav"}
    neg = {pid: r for pid, r in per_probe.items() if r["kind"] == "negative"}
    nav_inv = sum(passes_k(r["invocation"]) for r in nav.values())
    nav_act = sum(passes_k(r["activation"]) for r in nav.values())
    # negative passes^k = no prism invoked across all k (no over-reach)
    neg_ok = sum(passes_k([not x for x in r["invocation"]]) for r in neg.values())
    return {
        "nav_count": len(nav),
        "nav_invocation_pass5_rate": (nav_inv / len(nav)) if nav else 0.0,
        "nav_activation_pass5_rate": (nav_act / len(nav)) if nav else 0.0,
        "negative_count": len(neg),
        "negative_no_overreach_rate": (neg_ok / len(neg)) if neg else 0.0,
        "per_probe": per_probe,
    }

def write_benchmark(summary: dict, results_root: str, identifier: str) -> str:
    os.makedirs(results_root, exist_ok=True)
    path = os.path.join(results_root, f"benchmark-{identifier}.json")
    with open(path, "w") as f:
        json.dump(summary, f, indent=2)
    return path
```

- [ ] **Step 4: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_aggregate.py -v`.

- [ ] **Step 5: Commit** — `git add eval/adoption/aggregate.py eval/adoption/tests/unit/test_aggregate.py && git commit -m "feat(adoption): pass^k aggregation + benchmark writer"`

---

## Task 9: The deepeval suite + gitignore

**Files:**
- Create: `eval/adoption/tests/test_prism_adoption.py`
- Create: `eval/adoption/tests/__init__.py`, `eval/adoption/tests/unit/__init__.py`
- Modify: `.gitignore`

The suite is the `deepeval test run` target. It loads probes, runs `k=5` trials/probe **through the cache** (Task 7), scores each trial with the deterministic gate metrics, asserts per-trial via `assert_test`, and writes `benchmark.json` with the `pass^5` rates.

- [ ] **Step 1: gitignore the results dir** — add to `.gitignore`:
```
eval/adoption/results/
```

- [ ] **Step 2: Write the suite**

```python
# eval/adoption/tests/test_prism_adoption.py
"""deepeval suite: `cd eval && uv run deepeval test run adoption/tests/test_prism_adoption.py
  --identifier prism-adoption-round-N -n 5 -i -s`.
Generation is cached by SKILL.md hash (runner.py) so re-scoring is free; editing
skills/prism-code-navigation/SKILL.md invalidates the cache and re-spends."""
from __future__ import annotations
import os, pytest
from deepeval import assert_test
from adoption.goldens import load_probes
from adoption.env import build_isolated_config
from adoption.runner import run_trial
from adoption.testcase import build_test_case
from adoption.metrics import GATE_METRICS
from adoption.aggregate import summarize, write_benchmark

K = 5
EVAL_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))  # eval/
REPO_ROOT = os.path.dirname(EVAL_ROOT)
SKILL_SRC = os.path.join(REPO_ROOT, "skills", "prism-code-navigation")
RESULTS = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "results")
SKILL_BYTES = open(os.path.join(SKILL_SRC, "SKILL.md"), "rb").read()
PRISM_BIN = os.path.join(REPO_ROOT, "target", "release", "prism-mcp")

_PROBES = load_probes()
_RESULTS: dict = {}

def _cfg_for(repo_rel: str):
    return build_isolated_config(skill_src=SKILL_SRC,
                                 mcp_repo=os.path.join(EVAL_ROOT, repo_rel),
                                 prism_mcp_bin=PRISM_BIN)

@pytest.mark.parametrize("probe", _PROBES, ids=[p.id for p in _PROBES])
def test_probe(probe):
    cfg = _cfg_for(probe.repo)
    inv, act = [], []
    for trial in range(K):
        traj = run_trial(probe, trial, cfg=cfg, eval_root=EVAL_ROOT, results_root=RESULTS,
                         skill_bytes=SKILL_BYTES, model="sonnet")
        tc = build_test_case(traj, probe)
        fired = set(traj.prism_nav_calls())
        inv.append(bool(fired & set(probe.expected_tools)) if probe.kind == "nav" else bool(fired))
        act.append(traj.loaded_prism_skill())
        # score with deepeval gate metrics (deterministic); nav probes only
        if probe.kind == "nav":
            assert_test(test_case=tc, metrics=GATE_METRICS, run_async=False)
    _RESULTS[probe.id] = {"kind": probe.kind, "invocation": inv, "activation": act}

def teardown_module(_):
    if _RESULTS:
        s = summarize(_RESULTS)
        path = write_benchmark(s, RESULTS, os.environ.get("ADOPT_ID", "latest"))
        print(f"\nnav invocation pass^{K} = {s['nav_invocation_pass5_rate']:.0%} "
              f"(activation {s['nav_activation_pass5_rate']:.0%})  -> {path}")
```

Note: `assert_test` will fail the per-trial case when a nav probe doesn't fire the right tool — that's the visible per-trial signal the loop reads. The `teardown_module` writes the `pass^5` benchmark regardless (run with `-i` so one failing trial doesn't abort the suite).

- [ ] **Step 3: Verify it imports + collects (no spend)** — `cd eval && uv run pytest adoption/tests/test_prism_adoption.py --collect-only -q`
Expected: 12 items collected (one per probe), no import errors.

- [ ] **Step 4: Commit** — `git add eval/adoption/tests/test_prism_adoption.py eval/adoption/tests/__init__.py eval/adoption/tests/unit/__init__.py .gitignore && git commit -m "feat(adoption): deepeval suite (k=5 cached trials, pass^5 benchmark)"`

---

## Task 10: Live smoke + Round-1 baseline + iteration protocol

**Files:**
- Create: `eval/adoption/README.md` (the iteration protocol)

- [ ] **Step 1: 1-probe live smoke** (confirm isolated env + MCP + parse + cache end-to-end, minimal spend):
```bash
cd eval && uv run python -c "
import os
from adoption.goldens import load_probes
from adoption.env import build_isolated_config
from adoption.runner import run_trial
R=os.path.dirname(os.getcwd()); SS=os.path.join(R,'skills','prism-code-navigation')
cfg=build_isolated_config(skill_src=SS, mcp_repo=os.path.abspath('tier_c'),
                          prism_mcp_bin=os.path.join(R,'target','release','prism-mcp'))
p=[x for x in load_probes() if x.id=='callers-count-claims'][0]
t=run_trial(p,0,cfg=cfg,eval_root=os.getcwd(),results_root='adoption/results',
            skill_bytes=open(os.path.join(SS,'SKILL.md'),'rb').read())
print('skill_loaded=',t.loaded_prism_skill(),' prism_calls=',t.prism_nav_calls())
"
```
Expected: the call completes and prints `skill_loaded` + `prism_calls`. (Baseline behavior may be `skill_loaded=False prism_calls=[]` — that is the expected starting point the loop improves; the smoke only proves the pipeline runs.)

- [ ] **Step 2: Round-1 full baseline** — `cd eval && ADOPT_ID=round-1 uv run deepeval test run adoption/tests/test_prism_adoption.py --identifier prism-adoption-round-1 -n 5 -i -s`
Read: `adoption/results/benchmark-round-1.json` → record the baseline `nav_invocation_pass5_rate` + `nav_activation_pass5_rate`.

- [ ] **Step 3: Write `README.md` (the iteration protocol)** — document, for the executor: run command, where the benchmark lands, the loop (read failing probes + their trajectories under `results/cache/` → edit `skills/prism-code-navigation/SKILL.md` only → re-run, cache auto-invalidates → confirm `nav_invocation_pass5_rate` rose without negatives regressing), the stop condition (`≥ 0.80` on Sonnet), and the closing Opus confirmation run (`--model` swap is a follow-up flagged in the spec). Do NOT edit metrics/thresholds/goldens.

- [ ] **Step 4: Commit** — `git add eval/adoption/README.md && git commit -m "docs(adoption): iteration protocol + round-1 baseline recorded"`

- [ ] **Step 5: Run the full unit suite** — `cd eval && uv run pytest adoption/tests/unit/ -v` → all green.

---

## The iteration loop (post-build — this is the point)

Once Task 10 lands the baseline, run the deepeval loop (deepeval as ground truth), default 5 rounds:
1. `ADOPT_ID=round-N uv run deepeval test run adoption/tests/test_prism_adoption.py -id prism-adoption-round-N -n 5 -i -s`.
2. Read per-probe `assert_test` failures + the cached trajectories of failing probes (`results/cache/`).
3. Identify the smallest `skills/prism-code-navigation/SKILL.md` change (trigger wording, when-to-use table, explicit "ToolSearch 'prism nav' then call mcp__prism__nav_*" hint) that would fix the lowest probes.
4. Edit SKILL.md only (cache auto-invalidates via the hash → re-spends only the changed-skill trials).
5. Re-run; confirm `nav_invocation_pass5_rate` improved without negatives regressing.
6. Summarize what failed / changed / moved. Stop at `≥ 0.80` (Sonnet) or plateau, then one Opus confirmation run.

---

## Self-Review notes (done by plan author)

- **Spec coverage:** isolated env (T1), deepeval dep (T2), trajectory parse (T3), goldens incl. negatives (T4), LLMTestCase no-tracing (T5), SkillActivation+ToolCorrectness (T6), k-trial runner+cache (T7), pass^5 (T8), deepeval suite (T9), smoke+baseline+loop (T10). Quality metrics (ArgumentCorrectness/TaskCompletion) deferred per spec (reported-not-gated) — add in the loop if needed.
- **Out of scope (later specs):** codex side, Tier-C wiring, Opus confirmation automation.
- **Type consistency:** `Probe`/`Trajectory` (model.py) used identically across T3–T9; `prism_nav_calls()`/`loaded_prism_skill()` stable; `build_test_case`, `run_trial`, `summarize` signatures match call sites.
