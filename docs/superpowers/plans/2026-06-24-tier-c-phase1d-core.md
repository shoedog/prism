# Tier-C Phase-1d-core — LSP 2×2 matrix + run-store (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Make the first full `--live` run sound + reproducible: add the LSP 2×2 dimension (shim-deny control), per-command logging + tool-usage classification, the 5-contrast report, and a deterministic run-artifact store. (Replay engine = deferred Phase-1d-replay.)

**Architecture:** Per spec `docs/superpowers/specs/2026-06-24-tier-c-phase1d-lsp-matrix-and-runstore-design.md` (rev-2). `Variant` gains `lsp`; LSP-off enforced by a temp **deny-shim PATH dir** (model-agnostic, symmetric across claude/codex); arms log commands; the run-store persists everything for deterministic replay later. Builds on Phase-1/1b/1c.

**Tech Stack:** Python 3.12, `subprocess` (env/PATH control), `json`, `pytest`, `uv`.

---

### Task 1: `Variant.lsp` dimension

**Files:** Modify `eval/tier_c/model.py`; Test `eval/tests/test_tc_model.py` (append)

- [ ] **Step 1: Failing test**
```python
# append to eval/tests/test_tc_model.py
def test_variant_lsp_dimension_id():
    assert Variant("opus-4.8", True, True).id == "opus-4.8+prism+lsp"
    assert Variant("opus-4.8", False, True).id == "opus-4.8+lsp"
    assert Variant("opus-4.8", True).id == "opus-4.8+prism"   # lsp defaults False, back-compat
    assert Variant("gpt-5.5", False, False).family == "openai"
```
- [ ] **Step 2: Run, confirm FAIL** — `cd eval && uv run pytest tests/test_tc_model.py -q`.
- [ ] **Step 3: Implement** — in `model.py`, add `lsp: bool = False` to `Variant` (after `prism`) and update `id`:
```python
    @property
    def id(self) -> str:
        return f"{self.model}{'+prism' if self.prism else ''}{'+lsp' if self.lsp else ''}"
```
(`family` unchanged.)
- [ ] **Step 4: Run, confirm PASS** (+ `cd eval && uv run pytest tests/test_tc_*.py -q` — existing 2-arg Variant calls still work). **Step 5: Commit** → `feat(tier-c): Variant.lsp dimension`.

---

### Task 2: LSP deny-shim

**Files:** Create `eval/tier_c/lspshim.py`; Test `eval/tests/test_tc_lspshim.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_lspshim.py
import os, subprocess, json
from tier_c.lspshim import make_lsp_deny_shim, DENIED

def test_shim_dir_has_stub_for_each_denied(tmp_path):
    log = str(tmp_path / "shim.jsonl")
    d = make_lsp_deny_shim(log)
    for tool in DENIED:
        assert os.access(os.path.join(d, tool), os.X_OK)

def test_stub_logs_and_fails(tmp_path):
    log = str(tmp_path / "shim.jsonl")
    d = make_lsp_deny_shim(log)
    env = {**os.environ, "PATH": d + os.pathsep + os.environ["PATH"]}
    r = subprocess.run(["pyright", "foo.py"], capture_output=True, text=True, env=env)
    assert r.returncode != 0
    assert "disabled" in r.stderr.lower()
    rec = json.loads(open(log).read().splitlines()[0])
    assert rec["tool"] == "pyright"
```
- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
```python
# eval/tier_c/lspshim.py
"""LSP-off enforcement (spec §2.2): a temp dir of failing stub executables for dedicated
type-intelligence binaries + launchers, prepended to an arm's PATH so lsp=False arms cannot
use them. Symmetric across claude/codex. Each stub logs the attempt and exits non-zero.
Compilers (cargo/go/tsc-via-build) are intentionally NOT denied — see spec §2.2 compiler caveat."""
from __future__ import annotations
import os, stat, tempfile

DENIED = [
    "rust-analyzer", "gopls", "pyright", "pyright-langserver", "basedpyright", "pylsp",
    "ruff-lsp", "typescript-language-server", "tsserver", "tsc", "clangd", "mypy",
    "npx", "uvx", "mise",  # launchers that bypass bare-name shims (spec §2.2 codex new-4)
]

def make_lsp_deny_shim(log_path: str) -> str:
    """Create the deny-shim dir; return its path (prepend to PATH for lsp-off arms)."""
    d = tempfile.mkdtemp(prefix="tc-lspdeny-")
    for name in DENIED:
        p = os.path.join(d, name)
        with open(p, "w") as f:
            f.write(
                "#!/bin/sh\n"
                f'printf \'{{"tool":"%s","argv":"%s"}}\\n\' "$(basename \\"$0\\")" "$*" >> {log_path!r}\n'
                'echo "$(basename \\"$0\\") disabled (Tier-C lsp=off)" >&2\n'
                "exit 127\n"
            )
        os.chmod(p, os.stat(p).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return d
```
- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): LSP deny-shim (lsp-off enforcement)`.

---

### Task 3: Arm runners honor `lsp` via PATH env

**Files:** Modify `eval/tier_c/arm_runner.py`; Test `eval/tests/test_tc_lsp_runner.py`

- [ ] **Step 1: Failing test** (monkeypatch subprocess; assert env PATH)
```python
# eval/tests/test_tc_lsp_runner.py
import json
from tier_c.model import Variant
from tier_c.arm_runner import ClaudeRunner

def test_lsp_off_prepends_deny_shim_to_path(monkeypatch, tmp_path):
    seen = {}
    def fake_run(cmd, input=None, capture_output=None, text=None, cwd=None, timeout=None, env=None):
        seen["path"] = (env or {}).get("PATH", "")
        class R: stdout = json.dumps({"type":"result","is_error":False,"num_turns":1,
                  "result":"ok","usage":{"output_tokens":1}}); returncode=0; stderr=""
        return R()
    monkeypatch.setattr("tier_c.arm_runner.subprocess.run", fake_run)
    deny = str(tmp_path / "deny")
    r = ClaudeRunner(lsp_deny_dir=deny)
    r.run(Variant("opus-4.8", False, lsp=False), "spec", "p", "/repo")   # lsp OFF -> deny on PATH
    assert seen["path"].startswith(deny)
    r.run(Variant("opus-4.8", False, lsp=True), "spec", "p", "/repo")    # lsp ON -> no deny
    assert not seen["path"].startswith(deny)
```
- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** — `ClaudeRunner` and `CodexRunner` gain `lsp_deny_dir: str | None = None` in `__init__`. In each `run()`, build the env and pass it to `subprocess.run(..., env=env)`:
```python
        import os
        env = dict(os.environ)
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, ..., env=env)   # add env= to the existing call
```
(No `--disallowedTools` — the shim is the symmetric enforcement, spec §2.2 codex new-5.)
- [ ] **Step 4: Run, confirm PASS** (+ full `tests/test_tc_*.py`). **Step 5: Commit** → `feat(tier-c): arm runners enforce lsp-off via deny-shim PATH`.

---

### Task 4: Per-command logging + tool-usage classification

**Files:** Modify `eval/tier_c/parse.py`, `eval/tier_c/model.py`, `eval/tier_c/arm_runner.py`; Create `eval/tier_c/classify.py`; Test `eval/tests/test_tc_classify.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_classify.py
from tier_c.classify import classify_tools

def test_classify_flags_lsp_and_compiler():
    assert classify_tools(["grep -n Foo src", "cat a.py"]) == {"lsp_leak": False, "compiler_assisted": False}
    assert classify_tools(["pyright a.py"])["lsp_leak"] is True
    assert classify_tools(["cargo check"])["compiler_assisted"] is True
    assert classify_tools(["go vet ./..."])["compiler_assisted"] is True
```
- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
  - `parse.py` `parse_codex_jsonl`: collect command strings from `command_execution` items into `ModelResult.commands` (add `commands: list[str] = field(default_factory=list)` to `ModelResult`). claude's `parse_claude_json` sets `commands=[]` (full claude per-command = follow-up; the shim-log + tool_calls cover lsp-off detection).
  - `model.py` `ArmOutput`: add `commands: list[str] = field(default_factory=list)`, `lsp_leak: bool = False`, `compiler_assisted: bool = False`.
  - `classify.py`:
```python
# eval/tier_c/classify.py
"""Classify an arm's recorded commands (spec §3): did it reach a dedicated LSP/type-checker
(lsp_leak — should be impossible for lsp-off arms via the shim, flags bypass) or a compiler
type-check (compiler_assisted — the 'no dedicated LSP' caveat, reported per-protocol)."""
from __future__ import annotations
import re
from .lspshim import DENIED

_LSP = re.compile(r"\b(" + "|".join(re.escape(t) for t in DENIED if t not in {"npx","uvx","mise"}) + r")\b")
_COMPILER = re.compile(r"\b(cargo\s+(check|clippy|build)|go\s+(vet|build)|rustc|tsc)\b")

def classify_tools(commands: list[str]) -> dict:
    joined = "\n".join(commands)
    return {"lsp_leak": bool(_LSP.search(joined)),
            "compiler_assisted": bool(_COMPILER.search(joined))}
```
  - `arm_runner.py`: in each runner's `run()`, after parsing, set `commands=r.commands` and `flags = classify_tools(r.commands)` → pass `lsp_leak`/`compiler_assisted` into the `ArmOutput`.
- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): per-command logging + lsp_leak/compiler_assisted classification`.

---

### Task 5: Report 2×2 — 5 contrasts

**Files:** Modify `eval/tier_c/report.py`; Test `eval/tests/test_tc_report.py` (append)

- [ ] **Step 1: Failing test**
```python
# append to eval/tests/test_tc_report.py
from tier_c.report import StageMetrics, assemble_cell_2x2

def _sm(p): return StageMetrics(precision=p, recall=p, planted=0.0, used_prism=False, tokens=10)

def test_2x2_five_contrasts():
    per_id = {
        "opus-4.8": _sm(0.4), "opus-4.8+lsp": _sm(0.6),
        "opus-4.8+prism": _sm(0.7), "opus-4.8+prism+lsp": _sm(0.9),
    }
    c = assemble_cell_2x2(stage="spec", language="python", per_id=per_id, models=["opus-4.8"],
                          analyze_failure_rate=0.0, detectable=False)
    assert abs(c.prism_at_lsp_off["opus-4.8"] - 0.3) < 1e-9    # 0.7-0.4
    assert abs(c.prism_at_lsp_on["opus-4.8"]  - 0.3) < 1e-9    # 0.9-0.6  (primary gate)
    assert abs(c.lsp_at_prism_off["opus-4.8"] - 0.2) < 1e-9    # 0.6-0.4
    assert abs(c.interaction["opus-4.8"]      - 0.0) < 1e-9    # (0.9-0.6)-(0.7-0.4)
```
- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** — add `Cell2x2` (frozen dataclass: stage, language, the 5 delta dicts, itt/per-protocol, gate) + `assemble_cell_2x2` computing, per model `m`, from precision (extend to recall too — keep one metric set per the test, precision shown): `prism_at_lsp_off={m: P(m+prism)-P(m)}`, `prism_at_lsp_on={m: P(m+prism+lsp)-P(m+lsp)}`, `lsp_at_prism_off={m: P(m+lsp)-P(m)}`, `lsp_at_prism_on={m: P(m+prism+lsp)-P(m+prism)}`, `interaction={m: prism_at_lsp_on-prism_at_lsp_off}`; gate via `gate_decision` on the **prism_at_lsp_on** max delta. Reuse `prism_delta`-style lookups with the `+lsp` keys. (Keeps the Phase-1c `assemble_cell` for back-compat; the 2×2 is the new path.) NOTE in a docstring: contrasts are computed from per_id which, for >1 issue/language, is the `_avg` of issues — paired-per-issue contrasts are a follow-up for the 8-issue corpus (spec §2.3); exact for the 1-issue-per-language minimal corpus.
- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): 2x2 report cell (5 contrasts, prism@LSP-on gate)`.

---

### Task 6: Run-artifact store

**Files:** Create `eval/tier_c/store.py`; Modify `.gitignore`; Test `eval/tests/test_tc_store.py`

- [ ] **Step 1: Failing test**
```python
# eval/tests/test_tc_store.py
import json, os, pytest
from tier_c.store import RunStore

def test_store_writes_manifest_and_stage_and_rejects_collision(tmp_path):
    root = str(tmp_path / "runs")
    s = RunStore(root, run_id="r1", manifest={"models": ["opus-4.8"], "prism_sha": "abc"})
    s.write_manifest()
    assert json.load(open(os.path.join(root, "r1", "manifest.json")))["prism_sha"] == "abc"
    s.write_stage_artifact("spec", "prompt", {"text": "P", "upstream": ""})
    assert json.load(open(os.path.join(root, "r1", "stages", "spec", "prompt.json")))["text"] == "P"
    s.write_stage_artifact("spec", "seeds", {"shuffle": "spec|x", "tiebreak": "spec|x"})
    with pytest.raises(FileExistsError):
        RunStore(root, run_id="r1", manifest={}).ensure_new()
```
- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement** — `RunStore(root, run_id, manifest)`:
```python
# eval/tier_c/store.py
"""Run-artifact store (spec §4): persists a run under runs/<run-id>/ for deterministic replay +
audit. JSON, diff-able. gitignored. Replay (Phase-1d-replay) consumes this."""
from __future__ import annotations
import json, os

class RunStore:
    def __init__(self, root: str, run_id: str, manifest: dict):
        self.dir = os.path.join(root, run_id)
        self.manifest = manifest
    def ensure_new(self, force: bool = False):
        if os.path.exists(self.dir) and not force:
            raise FileExistsError(f"run-id dir exists: {self.dir} (use --force-new)")
        os.makedirs(os.path.join(self.dir, "stages"), exist_ok=True)
    def _write(self, rel: str, obj):
        p = os.path.join(self.dir, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as f:
            json.dump(obj, f, indent=1, default=str)
    def write_manifest(self):
        os.makedirs(self.dir, exist_ok=True)
        self._write("manifest.json", self.manifest)
    def write_stage_artifact(self, stage: str, name: str, obj):
        self._write(os.path.join("stages", stage, f"{name}.json"), obj)
```
(`ensure_new` is the collision guard, spec §7. `default=str` lets dataclasses serialize via a to-dict the caller passes — Task 7 converts dataclasses to dicts before writing.)
  - `.gitignore`: append `eval/tier_c/runs/`.
- [ ] **Step 4: Run, confirm PASS.** **Step 5: Commit** → `feat(tier-c): run-artifact store + gitignore runs/`.

---

### Task 7: Wire `run_live` — 8 variants, shim, persistence, 2×2 report

**Files:** Modify `eval/tier_c/run.py`, `eval/tier_c/chain.py`, `eval/tier_c/cli.py`; Test `eval/tests/test_tc_run_live.py` (extend)

- [ ] **Step 1: Failing test** (fakes; assert 8-variant flow + store written + 2×2 cell)
```python
# append to eval/tests/test_tc_run_live.py
def test_run_live_8_variants_writes_store_and_2x2(tmp_path):
    from tier_c.model import Issue, Variant
    from tier_c.arm_runner import FakeArmRunner
    from tier_c.investigator import RelevanceAllTrue
    from tier_c.run import run_live, LiveComponents
    class FakeCo:
        root="."
        def __enter__(self): return self
        def __exit__(self,*a): return False
        def file_exists(self,r): return True
        def read_line(self,r,l): return "x"
    class FakeRank:
        def rank(self,s,r,c): return sorted(c, key=lambda k: -len(c[k]))
    class FakeGuess:
        def guess_used_prism(self,t): return False
    variants = [Variant(m,p,l) for m in ("opus-4.8","gpt-5.5") for p in (False,True) for l in (False,True)]
    runner = FakeArmRunner({v.id: f"spec a.py:1 {v.id}" for v in variants})
    comps = LiveComponents(variants=variants, runner=runner,
        judges={"anthropic": FakeRank(), "openai": FakeRank()}, relevance=RelevanceAllTrue(),
        guesser=FakeGuess(), plants=[], open_checkout=lambda repo,sha: FakeCo(),
        run_store_root=str(tmp_path/"runs"), run_id="t1")
    issues=[Issue("k","python","pydantic","sha","u","bug a.py:1","slice")]
    report = run_live(issues, comps)
    assert ("spec","python") in report.cells
    import os; assert os.path.exists(str(tmp_path/"runs"/"t1"/"manifest.json"))
```
- [ ] **Step 2: Run, confirm FAIL.**
- [ ] **Step 3: Implement**
  - `chain.py` `run_stage`: capture the seeds it uses — record the shuffle seed string, the `label_to_vid` map, and the tie-break seed into the returned `StageResult` (add fields `shuffle_seed: str`, `label_map: dict`). (Pass a tie-break seed derived from `f"{stage}|tiebreak"` into `borda_consensus(..., seed=...)` so it's deterministic + recordable.)
  - `run.py` `LiveComponents`: add `run_store_root: str | None`, `run_id: str | None`. `run_live`: create one `lspshim.make_lsp_deny_shim(<shimlog under run dir>)`; the cli wires `RoutingArmRunner(ClaudeRunner(lsp_deny_dir=shim), CodexRunner(lsp_deny_dir=shim))`. Per issue/stage: persist `prompt`, `seeds`, each variant's `ArmOutput` (incl. commands/flags), `judges`, `investigator`, `best`; at end persist `detectability`, `report`, write `manifest`. Assemble cells via `assemble_cell_2x2`. (Convert dataclasses → dicts via `dataclasses.asdict` before `store.write_*`.)
  - `cli.py` `run --live`: build the 8 variants; require `--run-id` (collision-guard via `RunStore.ensure_new`); build the manifest (models, prism SHA via `prism --version` or the resolved bin, harness git SHA, corpus snapshot+hashes, CLI versions, env PATH, shim DENIED list); pass `run_store_root` (default `eval/tier_c/runs`) + `--bench-root`. Print the 5 contrasts per cell + GO/NO-GO + detectability + the run dir path.
- [ ] **Step 4: Run, confirm PASS.** **Step 5:** full `cd eval && uv run pytest tests/test_tc_*.py -q` + tier_a unaffected. **Step 6: Commit** → `feat(tier-c): run_live 8-variant 2x2 + run-store persistence`.
- [ ] **Step 7 (integration, manual — owner-triggered):** a 1-issue `--live --run-id smoke-1d` confirms: 8 arms run, lsp-off arms show the shim engaged (shim-log non-empty / lsp_leak=False), the run dir is written + reloadable, and the 2×2 cell prints. Verify before the full run.

---

## Self-Review
- **Spec coverage:** §2.1 Variant.lsp → T1; §2.2 shim + symmetry → T2,T3; §3 logging/classify → T4; §2.3 5 contrasts → T5; §4 run-store → T6; wiring + seeds-persist + manifest → T7. Replay (§5) = deferred.
- **Placeholders:** none; deferred items (claude full per-command, paired-per-issue at N>1, replay engine) are explicitly flagged.
- **Type consistency:** `Variant(model,prism,lsp)`; `make_lsp_deny_shim(log)->dir` + `DENIED`; runners `lsp_deny_dir`; `ModelResult.commands`/`ArmOutput.{commands,lsp_leak,compiler_assisted}`; `classify_tools`; `Cell2x2`/`assemble_cell_2x2`; `RunStore`; `LiveComponents.{run_store_root,run_id}`; `StageResult.{shuffle_seed,label_map}` consistent.

## Deferred (Phase-1d-replay + later)
- The replay engine (`tier-c replay`, frozen-control, re-score-all) — spec §5.
- claude full per-command capture (stream-json); paired-per-issue contrasts for the 8-issue corpus; the cost/analyze-failure gate arms.
