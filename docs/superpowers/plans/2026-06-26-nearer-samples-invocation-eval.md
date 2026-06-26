# Nearer-Samples Invocation De-Risk Eval — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the 2×2 (prompt-realism × skill-competition) invocation de-risk eval that extends `eval/adoption/`, so we can measure whether prism invocation survives realistic conditions before part C.

**Architecture:** Reuse the adoption harness (runner, trajectory parsers, SKILL.md-hash cache, aggregate). Add: two **competing-env builders** (claude + codex — real skills minus the memory injection; load-bearing, verify-first), **5 realistic goldens**, an **any-prism-call** invocation metric + 2×2 aggregation, and a **2×2 driver** that runs each (cell, model) and writes the cell table. Cells 1/3 use the existing isolated env; Cells 2/4 use the competing env.

**Tech Stack:** Python 3.9+, `uv`, the `claude`/`codex` CLIs, prism-mcp. Run from `eval/`.

**Spec:** `docs/superpowers/specs/2026-06-26-nearer-samples-invocation-eval-design.md`.

---

## File Structure

| File | Responsibility |
|---|---|
| `eval/adoption/competing_env.py` | claude competing `CLAUDE_CONFIG_DIR` (real skills, no SessionStart hook) |
| `eval/adoption/codex_competing_env.py` | codex competing `CODEX_HOME` (real skills, no prism hint) |
| `eval/adoption/goldens/realistic_prompts.toml` | the 5 realistic spec/plan/analysis prompts |
| `eval/adoption/goldens.py` | + `load_realistic_probes()` |
| `eval/adoption/aggregate.py` | + `prism_invoked()`, `summarize_cells()` (2×2 + invocation_rate + which-skill) |
| `eval/adoption/runner.py` | `run_trial` accepts a pre-built `codex_home` (driver controls isolated-vs-competing) |
| `eval/adoption/twobytwo.py` | the 2×2 driver: run each (cell, model) → `results/twobytwo-<id>.json` |
| `eval/adoption/tests/unit/test_competing_env.py` | claude competing layout |
| `eval/adoption/tests/unit/test_codex_competing_env.py` | codex competing layout |
| `eval/adoption/tests/unit/test_realistic_goldens.py` | loader |
| `eval/adoption/tests/unit/test_invocation_metric.py` | `prism_invoked` + `summarize_cells` |

Conventions: top-level imports (`from adoption.X`), run from `eval/` via `uv`, mirror existing `eval/adoption` style. Branch `nearer-samples-eval` (already checked out).

---

## Task 1: Claude competing-env builder + LIVE VERIFY GATE (load-bearing)

**Files:**
- Create: `eval/adoption/competing_env.py`, `eval/adoption/tests/unit/test_competing_env.py`

The competing env carries the *real* skills (superpowers plugins + user skills) + the tuned `prism-code-navigation`, but with the **SessionStart hook stripped** (no memory injection). The exact recipe is uncertain — verify it live before relying on it.

- [ ] **Step 1: Write the failing layout test**

```python
# eval/adoption/tests/unit/test_competing_env.py
import json, os
from adoption.competing_env import build_competing_config

def test_competing_layout(tmp_path):
    # fake real ~/.claude with a competing skill + a SessionStart hook + creds
    real = tmp_path / "realclaude"; (real / "skills" / "prism-nav").mkdir(parents=True)
    (real / "skills" / "prism-nav" / "SKILL.md").write_text("---\nname: prism-nav\n---\nx")
    (real / "plugins").mkdir()
    (real / "settings.json").write_text(json.dumps({"hooks": {"SessionStart": [{"hooks": [{"type":"command","command":"echo hi"}]}]}, "permissions": {"allow": ["Read"]}}))
    (real / ".credentials.json").write_text('{"t":"x"}')
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir()
    (tuned / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\ny")

    cfg = build_competing_config(skill_src=str(tuned), mcp_repo="/repo", prism_mcp_bin="/bin/p",
                                 root=str(tmp_path/"iso"), real_home=str(real),
                                 credentials_src=str(real/".credentials.json"))
    sk = os.path.join(cfg.config_dir, "skills")
    assert os.path.exists(os.path.join(sk, "prism-nav", "SKILL.md"))            # real competitor present
    assert os.path.exists(os.path.join(sk, "prism-code-navigation", "SKILL.md"))# tuned skill present
    settings = json.load(open(os.path.join(cfg.config_dir, "settings.json")))
    assert settings.get("hooks", {}) == {}                                      # SessionStart STRIPPED
    assert "Read" in settings["permissions"]["allow"]                           # other settings kept
    assert json.load(open(os.path.join(cfg.config_dir, ".credentials.json")))["t"] == "x"
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_competing_env.py -v` → FAIL (no module).

- [ ] **Step 3: Implement `competing_env.py`**

```python
# eval/adoption/competing_env.py
"""COMPETING claude env: the REAL skill set (superpowers plugins + user skills + the tuned
prism-code-navigation) with the SessionStart memory hook STRIPPED — tests the tuned skill under
realistic competition, no prism hint. Pairs with env.py's isolated builder."""
from __future__ import annotations
import atexit, json, os, shutil, tempfile
from .env import IsolatedConfig   # reuse the (config_dir, mcp_cfg) dataclass

def build_competing_config(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                           root: str | None = None, real_home: str = "~/.claude",
                           credentials_src: str = "~/.claude/.credentials.json") -> IsolatedConfig:
    base = root or tempfile.mkdtemp(prefix="tc-compete-cfg-")
    if root is None:
        atexit.register(shutil.rmtree, base, True)
    cfg_dir = os.path.join(base, "config"); os.makedirs(cfg_dir, exist_ok=True)
    real = os.path.expanduser(real_home)

    # plugins: symlink the real (big) dir so superpowers etc. load unchanged
    rp = os.path.join(real, "plugins")
    if os.path.exists(rp):
        os.symlink(rp, os.path.join(cfg_dir, "plugins"))

    # skills: COPY real skills (so we can add the tuned one alongside) + add the tuned skill
    skills_dir = os.path.join(cfg_dir, "skills"); os.makedirs(skills_dir, exist_ok=True)
    rs = os.path.join(real, "skills")
    if os.path.isdir(rs):
        for name in os.listdir(rs):
            src = os.path.join(rs, name)
            if os.path.isdir(src):
                shutil.copytree(src, os.path.join(skills_dir, name), symlinks=True)
    tuned_dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
    if os.path.exists(tuned_dst):
        shutil.rmtree(tuned_dst)
    shutil.copytree(skill_src, tuned_dst)

    # settings: copy real, but STRIP hooks (no SessionStart memory injection)
    settings = {}
    rsj = os.path.join(real, "settings.json")
    if os.path.exists(rsj):
        settings = json.load(open(rsj))
    settings["hooks"] = {}                       # the key control: no memory injection
    with open(os.path.join(cfg_dir, "settings.json"), "w") as f:
        json.dump(settings, f)

    # creds (secret; temp dir only, chmod 0600)
    cred = os.path.expanduser(credentials_src)
    if os.path.exists(cred):
        dst = os.path.join(cfg_dir, ".credentials.json")
        shutil.copy2(cred, dst); os.chmod(dst, 0o600)

    # prism MCP via --mcp-config (same as isolated)
    mcp_cfg = os.path.join(base, "mcp.json")
    with open(mcp_cfg, "w") as f:
        json.dump({"mcpServers": {"prism": {"command": prism_mcp_bin,
                                            "args": ["--repo", mcp_repo]}}}, f)
    return IsolatedConfig(config_dir=cfg_dir, mcp_cfg=mcp_cfg)
```

- [ ] **Step 4: Run the layout test, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_competing_env.py -v`.

- [ ] **Step 5: LIVE VERIFY GATE** (the load-bearing check — real skills present, NO memory injection, prism works). Build prism-mcp first.

```bash
cd /Users/wesleyjinks/code/slicing && cargo build --release --bin prism-mcp --features mcp
cd eval && uv run python -c "
from adoption.competing_env import build_competing_config
import subprocess, json, os
R='/Users/wesleyjinks/code/slicing'
cfg = build_competing_config(skill_src=R+'/skills/prism-code-navigation', mcp_repo=os.path.abspath('tier_c'),
                             prism_mcp_bin=R+'/target/release/prism-mcp')
env=dict(os.environ); env['CLAUDE_CONFIG_DIR']=cfg.config_dir
p=subprocess.run(['claude','-p','--output-format','stream-json','--verbose','--model','sonnet',
                  '--mcp-config',cfg.mcp_cfg,'--strict-mcp-config',
                  'List the skills available to you by name. Then call mcp__prism__nav_repo_map and report its first line.'],
                 capture_output=True,text=True,cwd='tier_c',env=env,timeout=200)
recs=[json.loads(l) for l in p.stdout.splitlines() if l.strip()]
calls=[c.get('name') for r in recs if r.get('type')=='assistant' for c in (r.get('message',{}).get('content') or []) if isinstance(c,dict) and c.get('type')=='tool_use']
res=[r for r in recs if r.get('type')=='result']
blob=json.dumps(recs).lower()
print('prism fired ok:', any(str(c).startswith('mcp__prism') for c in calls), '| is_error:', res[0].get('is_error') if res else None)
print('competing skills visible (superpowers mentioned):', 'superpower' in blob or 'writing-plans' in blob)
print('MEMORY-INJECTION leaked? (SessionStart hook_success attachment):', 'sessionstart' in blob and 'hook_success' in blob)
print('result head:', (res[0].get('result') if res else p.stderr[:200])[:160])
"
```
Expected: `prism fired ok: True | is_error: False`, **competing skills visible: True**, **MEMORY-INJECTION leaked: False**. If memory injection still fires (SessionStart hook not stripped) or skills aren't visible or prism fails → STOP, report BLOCKER with the evidence; the controller adjusts the recipe (e.g. also clear `settings.local.json` hooks, or a different skill-carry mechanism).

- [ ] **Step 6: Commit** — `git add eval/adoption/competing_env.py eval/adoption/tests/unit/test_competing_env.py && git commit -m "feat(nearer-samples): claude competing-env builder (real skills, no memory injection) + live gate"` (append the `Co-Authored-By` trailer).

---

## Task 2: Codex competing-env builder + LIVE VERIFY GATE (load-bearing)

**Files:**
- Create: `eval/adoption/codex_competing_env.py`, `eval/adoption/tests/unit/test_codex_competing_env.py`

The codex competing env carries the real `~/.codex` config + skills (which auto-load `knowledge-ref`'s `prism-nav`/`lsp-nav`) + prism MCP, minus the prism-naming instruction. Fuzzier than claude — verify-first.

- [ ] **Step 1: Write the failing layout test**

```python
# eval/adoption/tests/unit/test_codex_competing_env.py
import os
from adoption.codex_competing_env import build_competing_codex_home

def test_codex_competing_layout(tmp_path):
    realcodex = tmp_path / "realcodex"; (realcodex).mkdir()
    (realcodex / "auth.json").write_text('{"t":"x"}')
    (realcodex / "config.toml").write_text("[mcp_servers.node_repl]\ncommand='x'\n")
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir(); (tuned/"SKILL.md").write_text("y")
    home = build_competing_codex_home(skill_src=str(tuned), mcp_repo="/repo",
                                      prism_mcp_bin="/bin/p", root=str(tmp_path/"home"),
                                      real_home=str(realcodex))
    assert os.path.isfile(os.path.join(home, "auth.json"))                       # auth carried
    cfg = open(os.path.join(home, "config.toml")).read()
    assert "[mcp_servers.prism]" in cfg                                          # prism added
    assert os.path.isfile(os.path.join(home, "skills", "prism-code-navigation", "SKILL.md"))
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_codex_competing_env.py -v` → FAIL.

- [ ] **Step 3: Implement `codex_competing_env.py`**

```python
# eval/adoption/codex_competing_env.py
"""COMPETING codex CODEX_HOME: the real ~/.codex config + skills (auto-loads knowledge-ref's
prism-nav/lsp-nav) + the tuned skill + prism MCP, minus the prism-naming instruction. Pairs with
codex_env.py's isolated builder. Verify-first: the 'no prism hint' condition is recipe-uncertain."""
from __future__ import annotations
import atexit, os, shutil, tempfile

def build_competing_codex_home(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                               root: str | None = None, real_home: str = "~/.codex") -> str:
    base = root or tempfile.mkdtemp(prefix="tc-codex-compete-")
    if root is None:
        atexit.register(shutil.rmtree, base, True)
    os.makedirs(base, exist_ok=True)
    real = os.path.expanduser(real_home)

    # carry the real codex home (auth + skills) so the user's real skills compete; symlink the
    # bulky bits, copy the small ones we must extend.
    ra = os.path.join(real, "auth.json")
    if os.path.exists(ra):
        d = os.path.join(base, "auth.json"); shutil.copy2(ra, d); os.chmod(d, 0o600)
    # skills: copy real skills then add the tuned one
    skills_dir = os.path.join(base, "skills"); os.makedirs(skills_dir, exist_ok=True)
    rs = os.path.join(real, "skills")
    if os.path.isdir(rs):
        for name in os.listdir(rs):
            src = os.path.join(rs, name)
            if os.path.isdir(src):
                shutil.copytree(src, os.path.join(skills_dir, name), symlinks=True,
                                dirs_exist_ok=True)
    dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(skill_src, dst)

    # config.toml: copy the real one (keeps user's mcp_servers = competition) + append prism MCP.
    # Deliberately do NOT carry any [projects.*] / instruction keys that name prism (the memory hint).
    lines = []
    rc = os.path.join(real, "config.toml")
    if os.path.exists(rc):
        for ln in open(rc):
            # drop project-instruction blocks that may inject prism hints; keep mcp_servers etc.
            if ln.strip().startswith("[projects."):
                break  # everything after the projects table is instruction config — exclude it
            lines.append(ln)
    lines += ["\n[mcp_servers.prism]\n", f'command = "{prism_mcp_bin}"\n',
              f'args = ["--repo", "{mcp_repo}"]\n']
    with open(os.path.join(base, "config.toml"), "w") as f:
        f.writelines(lines)
    return base
```

- [ ] **Step 4: Run the layout test, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_codex_competing_env.py -v`.

- [ ] **Step 5: LIVE VERIFY GATE** — confirm real skills present (knowledge-ref `prism-nav` reachable), prism MCP fires, and no instruction directly names the prism nav tools.

```bash
cd eval && uv run python -c "
from adoption.codex_competing_env import build_competing_codex_home
import subprocess, json, os
R='/Users/wesleyjinks/code/slicing'
home=build_competing_codex_home(skill_src=R+'/skills/prism-code-navigation', mcp_repo=os.path.abspath('tier_c'),
                                prism_mcp_bin=R+'/target/release/prism-mcp')
env=dict(os.environ); env['CODEX_HOME']=home
p=subprocess.run(['codex','exec','--json','-m','gpt-5.5','-C',os.path.abspath('tier_c'),'-s','read-only',
                  'List the skills available to you, then call the prism nav_repo_map tool and report its first line.'],
                 capture_output=True,text=True,cwd=os.path.abspath('tier_c'),env=env,timeout=400)
recs=[json.loads(l) for l in p.stdout.splitlines() if l.strip()]
prism=[(it.get('server'),it.get('tool')) for r in recs for it in [r.get('item') or {}] if it.get('type')=='mcp_tool_call' and it.get('server')=='prism']
blob=json.dumps(recs).lower()
print('prism fired:', prism)
print('competing skills visible (prism-nav or lsp-nav or superpower mentioned):', any(s in blob for s in ['prism-nav','lsp-nav','superpower']))
print('CODEX_HOME', home)
"
```
Expected: `prism fired: [('prism','nav_repo_map')]`, competing skills visible: True. If the real skills don't appear (CODEX_HOME doesn't carry them) or a prism-naming instruction still loads → STOP, report BLOCKER; the controller adjusts (the codex "memory hint" recipe is the fuzziest part of the spec — Risk #1).

- [ ] **Step 6: Commit** — `git add eval/adoption/codex_competing_env.py eval/adoption/tests/unit/test_codex_competing_env.py && git commit -m "feat(nearer-samples): codex competing-env builder (real skills, no prism hint) + live gate"` (+ trailer).

---

## Task 3: Realistic goldens + loader

**Files:**
- Create: `eval/adoption/goldens/realistic_prompts.toml`
- Modify: `eval/adoption/goldens.py`
- Create: `eval/adoption/tests/unit/test_realistic_goldens.py`

- [ ] **Step 1: Author `realistic_prompts.toml`** (the 5 from the spec)

```toml
[[probe]]
id = "spec-runstage-tiebreak"
kind = "realistic"
prompt = "Write a short implementation spec for changing run_stage's tie-break to use a different seed. Cite the exact file:line for every claim."
repo = "tier_c"

[[probe]]
id = "analysis-count-claims-blast"
kind = "realistic"
prompt = "Analyze the blast radius of changing count_claims's signature — list every site that would need to update, as file:line."
repo = "tier_c"

[[probe]]
id = "plan-split-chain"
kind = "realistic"
prompt = "Plan the refactor that splits chain.py into a stage-orchestration module and a chaining module. Cite the symbols/sites involved."
repo = "tier_c"

[[probe]]
id = "spec-sanitation-gate"
kind = "realistic"
prompt = "Write a short spec for fixing the sanitation gate in run_spec_plan_chain. Ground every claim in file:line."
repo = "tier_c"

[[probe]]
id = "analysis-dry-run-flag"
kind = "realistic"
prompt = "Which functions would a new --dry-run flag for the tier-c CLI touch? List them as file:line."
repo = "tier_c"
```

- [ ] **Step 2: Write the failing test**

```python
# eval/adoption/tests/unit/test_realistic_goldens.py
from adoption.goldens import load_realistic_probes

def test_loads_realistic():
    ps = load_realistic_probes()
    assert len(ps) == 5
    assert all(p.kind == "realistic" for p in ps)
    assert all(p.expected_tools == [] for p in ps)   # open-ended: no single expected tool
    assert {p.id for p in ps} >= {"spec-runstage-tiebreak", "analysis-dry-run-flag"}
```

- [ ] **Step 3: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_realistic_goldens.py -v` → FAIL.

- [ ] **Step 4: Add `load_realistic_probes` to `goldens.py`**

```python
# append to eval/adoption/goldens.py
_REALISTIC = os.path.join(os.path.dirname(__file__), "goldens", "realistic_prompts.toml")

def load_realistic_probes(path: str = _REALISTIC) -> list[Probe]:
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return [Probe(id=p["id"], kind=p["kind"], prompt=p["prompt"], repo=p["repo"],
                  expected_tools=[], expected_symbol=None)
            for p in data["probe"]]
```

- [ ] **Step 5: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_realistic_goldens.py -v`.

- [ ] **Step 6: Commit** — `git add eval/adoption/goldens.py eval/adoption/goldens/realistic_prompts.toml eval/adoption/tests/unit/test_realistic_goldens.py && git commit -m "feat(nearer-samples): 5 realistic spec/plan/analysis goldens + loader"` (+ trailer).

---

## Task 4: Invocation metric + 2×2 aggregation

**Files:**
- Modify: `eval/adoption/aggregate.py`
- Create: `eval/adoption/tests/unit/test_invocation_metric.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/adoption/tests/unit/test_invocation_metric.py
from adoption.model import Trajectory
from adoption.aggregate import prism_invoked, summarize_cells

def test_prism_invoked():
    assert prism_invoked(Trajectory("ans", ["prism-code-navigation"], [("nav_callers", {})])) is True
    assert prism_invoked(Trajectory("ans", [], [("Bash", {})])) is False

def test_summarize_cells_rate_and_attribution():
    # cell -> probe -> list of (invoked, skill_loaded_name_or_None) per trial
    cells = {
      "cell4": {
        "s1": [(True, "prism-code-navigation"), (True, "prism-nav"), (False, None), (True, "prism-code-navigation"), (True, "prism-code-navigation")],
        "s2": [(False, None)] * 5,
      }
    }
    out = summarize_cells(cells)
    c = out["cell4"]
    assert c["invocation_rate"] == 4/10          # 4 of 10 sample×trial runs invoked
    assert c["pass5_rate"] == 0.0                # neither sample hit 5/5
    assert c["skill_attribution"]["prism-code-navigation"] == 3
    assert c["skill_attribution"]["prism-nav"] == 1
```

- [ ] **Step 2: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_invocation_metric.py -v` → FAIL.

- [ ] **Step 3: Add to `aggregate.py`**

```python
# append to eval/adoption/aggregate.py
import collections

def prism_invoked(traj) -> bool:
    """Any-call invocation: did the trajectory fire ANY prism nav tool (open-ended-task metric)."""
    return bool(traj.prism_nav_calls())

def summarize_cells(cells: dict) -> dict:
    """cells: {cell_id: {sample_id: [(invoked: bool, skill_loaded: str|None), ... k trials]}}.
    Per cell: invocation_rate (over all sample*trial), pass5_rate (samples with all-k invoked),
    skill_attribution (count of which skill loaded across invoked runs)."""
    out = {}
    for cell, samples in cells.items():
        runs = [r for trials in samples.values() for r in trials]
        n = len(runs) or 1
        invoked = sum(1 for inv, _ in runs)
        pass5 = sum(1 for trials in samples.values() if trials and all(inv for inv, _ in trials))
        attr = collections.Counter(sk for inv, sk in runs if inv and sk)
        out[cell] = {
            "invocation_rate": invoked / n,
            "pass5_rate": pass5 / (len(samples) or 1),
            "n_samples": len(samples), "n_runs": len(runs),
            "skill_attribution": dict(attr),
        }
    return out
```

- [ ] **Step 4: Run tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/test_invocation_metric.py -v`.

- [ ] **Step 5: Commit** — `git add eval/adoption/aggregate.py eval/adoption/tests/unit/test_invocation_metric.py && git commit -m "feat(nearer-samples): any-prism-call metric + 2x2 cell aggregation (rate/pass5/attribution)"` (+ trailer).

---

## Task 5: `run_trial` env selection + the 2×2 driver

**Files:**
- Modify: `eval/adoption/runner.py:71-101` (`run_trial`, `_run_codex_trial`)
- Create: `eval/adoption/twobytwo.py`
- Create: `eval/adoption/tests/unit/test_twobytwo_skillname.py`

`run_trial` currently builds the codex home internally (always isolated). The driver must control isolated-vs-competing per cell, so add an optional pre-built `codex_home`; for claude the driver already controls the env via `cfg`.

- [ ] **Step 1: Add `codex_home` passthrough to `run_trial`** (`runner.py`)

In `_run_codex_trial`, accept an optional pre-built home:
```python
def _run_codex_trial(probe, *, eval_root, skill_src, prism_mcp_bin, model, codex_home=None):
    from .codex_env import build_isolated_codex_home
    repo_path = os.path.join(eval_root, probe.repo)
    home = codex_home or build_isolated_codex_home(skill_src=skill_src, mcp_repo=repo_path,
                                                   prism_mcp_bin=prism_mcp_bin)
    env = dict(os.environ); env["CODEX_HOME"] = home
    cmd = build_codex_cmd(prompt=probe.prompt, model=model)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_CODEX_TIMEOUT,
                          cwd=repo_path, env=env)
    return parse_codex_stream(proc.stdout)
```
And thread `codex_home=None` through `run_trial`'s signature + its codex branch call. (Claude branch unchanged — the driver passes the right `cfg`.)

- [ ] **Step 2: Write a small skill-name helper test** (the driver needs the loaded-skill name for attribution)

```python
# eval/adoption/tests/unit/test_twobytwo_skillname.py
from adoption.model import Trajectory
from adoption.twobytwo import loaded_skill_name

def test_loaded_skill_name():
    assert loaded_skill_name(Trajectory("a", ["prism-code-navigation"], [])) == "prism-code-navigation"
    assert loaded_skill_name(Trajectory("a", ["/x/prism-nav/SKILL.md"], [])) == "prism-nav"
    assert loaded_skill_name(Trajectory("a", [], [])) is None
```

- [ ] **Step 3: Run it, expect failure** — `cd eval && uv run pytest adoption/tests/unit/test_twobytwo_skillname.py -v` → FAIL.

- [ ] **Step 4: Implement `twobytwo.py`**

```python
# eval/adoption/twobytwo.py
"""2x2 driver (prompt-realism x skill-competition). Builds each cell's env, runs its samples x k
trials via run_trial (cached), scores with prism_invoked, writes results/twobytwo-<id>.json.
Cells: 1=micro+isolated (reuse cache), 2=micro+competing, 3=realistic+isolated, 4=realistic+competing."""
from __future__ import annotations
import json, os
from .goldens import load_probes, load_realistic_probes
from .env import build_isolated_config
from .competing_env import build_competing_config
from .codex_env import build_isolated_codex_home
from .codex_competing_env import build_competing_codex_home
from .runner import run_trial
from .aggregate import prism_invoked, summarize_cells

K = 5
def _skill_basename(s: str) -> str:
    """Extract the skill dir name from a skill_load string: '.../<name>/SKILL.md' or '<name>'."""
    s = s.strip()
    head = s.split("/SKILL.md")[0] if "/SKILL.md" in s else s
    return head.rstrip("/").split("/")[-1].lower()

def loaded_skill_name(traj) -> str | None:
    """The prism-related skill the model loaded (for attribution); else the first loaded skill."""
    names = [_skill_basename(s) for s in traj.skill_loads]
    for n in names:
        if "prism" in n:
            return n
    return names[0] if names else None

def run_cell(cell_id, probes, *, env_kind, model, eval_root, results_root, skill_src,
             prism_mcp_bin, repo="tier_c"):
    """env_kind in {'isolated','competing'}. Returns {sample_id: [(invoked, skill_name)] * K}."""
    skill_bytes = open(os.path.join(skill_src, "SKILL.md"), "rb").read()
    repo_path = os.path.join(eval_root, repo)
    is_codex = model.startswith("gpt")
    cfg = codex_home = None
    if is_codex:
        codex_home = (build_competing_codex_home if env_kind == "competing" else build_isolated_codex_home)(
            skill_src=skill_src, mcp_repo=repo_path, prism_mcp_bin=prism_mcp_bin)
    else:
        cfg = (build_competing_config if env_kind == "competing" else build_isolated_config)(
            skill_src=skill_src, mcp_repo=repo_path, prism_mcp_bin=prism_mcp_bin)
    out = {}
    for p in probes:
        trials = []
        for t in range(K):
            traj = run_trial(p, t, cfg=cfg, eval_root=eval_root, results_root=results_root,
                             skill_bytes=skill_bytes + env_kind.encode(),  # env in the cache key
                             model=model, skill_src=skill_src, prism_mcp_bin=prism_mcp_bin,
                             codex_home=codex_home)
            trials.append((prism_invoked(traj), loaded_skill_name(traj)))
        out[p.id] = trials
    return out

def run_2x2(*, model, eval_root, results_root, skill_src, prism_mcp_bin, cells=("1","2","3","4"),
            identifier="latest"):
    micro, real = load_probes(), load_realistic_probes()
    spec = {"1": (micro, "isolated"), "2": (micro, "competing"),
            "3": (real, "isolated"), "4": (real, "competing")}
    data = {}
    for c in cells:
        probes, env_kind = spec[c]
        data[f"cell{c}"] = run_cell(c, probes, env_kind=env_kind, model=model, eval_root=eval_root,
                                    results_root=results_root, skill_src=skill_src,
                                    prism_mcp_bin=prism_mcp_bin)
    summary = summarize_cells(data)
    os.makedirs(results_root, exist_ok=True)
    path = os.path.join(results_root, f"twobytwo-{identifier}.json")
    json.dump({"model": model, "cells": summary, "raw": data}, open(path, "w"), indent=2)
    return summary, path
```

Note: `skill_bytes + env_kind.encode()` folds the env into the cache key so isolated/competing don't collide. The 12-probe micro Cell 1 still re-runs here (different env-key than the adoption-eval cache) unless you point `results_root` at the same cache and accept the env-suffix; for the reference you may instead read Cell 1's existing any-call rate from the adoption benchmark — the driver supports running `cells=("2","3","4")` and supplying Cell 1 from the prior run.

- [ ] **Step 5: Run unit tests, expect PASS** — `cd eval && uv run pytest adoption/tests/unit/ -q` → all green (existing + new).

- [ ] **Step 6: Commit** — `git add eval/adoption/runner.py eval/adoption/twobytwo.py eval/adoption/tests/unit/test_twobytwo_skillname.py && git commit -m "feat(nearer-samples): 2x2 driver + run_trial codex_home passthrough (env per cell)"` (+ trailer).

---

## Task 6: Cheap Sonnet Cell-3 smoke (NO full 2×2 — spend boundary)

**Files:** none (verification only)

- [ ] **Step 1: 1-probe Cell-3 smoke** (realistic prompt, isolated env, Sonnet — confirms the realistic-prompt path runs e2e + the metric works; the Task-1/2 gates already proved the competing envs):

```bash
cd eval && uv run python -c "
import os
from adoption.goldens import load_realistic_probes
from adoption.twobytwo import run_cell
R=os.path.dirname(os.getcwd()); SS=os.path.join(R,'skills','prism-code-navigation'); BIN=os.path.join(R,'target','release','prism-mcp')
one=[p for p in load_realistic_probes() if p.id=='analysis-count-claims-blast']
res=run_cell('3', one, env_kind='isolated', model='sonnet', eval_root=os.getcwd(),
             results_root='adoption/results', skill_src=SS, prism_mcp_bin=BIN)
print('cell-3 smoke (analysis-count-claims-blast, sonnet, isolated):', res)
"
```
Expected: a list of 5 `(invoked, skill_name)` tuples; at least some `(True, 'prism-code-navigation')` (a realistic analysis task should trigger prism on Sonnet in the isolated env). The smoke only proves the path runs; the real rates come from the full 2×2.

- [ ] **Step 2: Full unit suite + commit a short `README` note** — `cd eval && uv run pytest adoption/tests/unit/ -q` green. Append a "2×2 nearer-samples" section to `eval/adoption/README.md` documenting the driver call (`run_2x2(model=..., cells=...)`), the cells, the metric, and the cheap→expensive model order. `git add eval/adoption/README.md && git commit -m "docs(nearer-samples): 2x2 driver usage + cell/metric notes"` (+ trailer).

---

## The 2×2 run (post-build — spend-gated, owner-triggered)

After Task 6, the run is: **Sonnet full 2×2** (`run_2x2(model="sonnet", cells=("1","2","3","4"))`) → read the cell table (invocation_rate per cell + attribution); if Cell 4 holds, **codex-spark Cell 4** then **Opus + gpt-5.5 Cell 4** (`cells=("4",)`), skipping the expensive tier if Cell 4 collapsed cheaply. Read: does Cell 4 ≈ Cell 1 (proceed to part C), or do Cells 2/3 localize a drop.

---

## Self-Review notes (plan author)

- **Spec coverage:** competing envs claude+codex with verify gates (T1,T2), 5 realistic goldens (T3), any-call metric + 2×2 aggregation + attribution (T4), env-per-cell + driver (T5), smoke (T6); cheap→expensive model order in the run section. The "minus memory injection" is the load-bearing gate in T1/T2 (verify-first, BLOCKER-on-fail) per spec Risk #1.
- **Type consistency:** `Probe`/`Trajectory` reused unchanged; `build_competing_config` returns the same `IsolatedConfig` the claude `run_trial` already consumes; `build_competing_codex_home` returns a path like `build_isolated_codex_home`; `run_trial` gains `codex_home=None` (back-compatible). `summarize_cells` input shape matches what `run_cell` emits (`{cell:{sample:[(bool,str|None)]}}`).
- **Out of scope:** value measurement (part C); a firmer Cell-4 pass bar (diagnostic read for now).
