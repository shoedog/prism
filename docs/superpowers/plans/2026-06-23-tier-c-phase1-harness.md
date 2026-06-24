# Tier-C Value Measurement — Phase 1 (spec+plan stages) Implementation Plan

> **STATUS: BUILT 2026-06-23** — all 12 tasks implemented via subagent-driven TDD on branch
> `tier-c-value-measurement` (per-task commits; per-task spec+quality review; opus final review = **SHIP**).
> `eval/tier_c/` is a complete, importable, **35-test-green** package (full eval suite 178 green, tier_a
> unaffected). Mid-build reviews caught + fixed: checkout temp-dir leak, a vacuous test assertion, and three
> integration gaps — **judge blinding** (judges now see opaque `cand*` labels, never the `+prism` id), the
> **sanitation gate** (explicit `raise`, not a strippable `assert`), and **planted-error injection** (`inject()`
> wired into the chain so the probe isn't inert). See "Phase-1 known gaps" at the bottom before the next phase.
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Tier-C harness for the **spec** and **plan** stages — run the 4 variants, score them with the independent **investigator** (objective citation precision/recall + hallucination) and the **planted-error** diagnostic, run the **dual blind judges** (consensus + family-bias), chain spec→plan with **reset-to-cleaned-best**, and emit a per-stage report with the **GO/NO-GO** gate.

**Architecture:** A new Python package `eval/tier_c/` mirroring `eval/tier_a/` (frozen dataclasses in `model.py`, `Protocol` seams in `interfaces.py`, `subprocess` drivers, pytest in `eval/tests/`). The objective oracle (investigator + planted-error) uses **neutral repo primitives, never prism**. Model-driving (arm runner, judges) sits behind `Protocol` seams so all orchestration/scoring logic is TDD'd with fakes; the two concrete model drivers (`codex exec`, `claude -p`) are thin and unit-tested on the command they build. Develop+review stages and per-repo build sandboxes are **Phase 2** (separate plan).

**Tech Stack:** Python 3.12, `dataclasses`, `typing.Protocol`, `subprocess`, `tomllib`, `pytest`, `tree-sitter` (via the prism repo's grammars are NOT used here — the investigator uses `git`/`grep`/line lookups against a pinned checkout to stay prism-independent). `uv` for running.

---

## File Structure

- `eval/tier_c/__init__.py` — package marker.
- `eval/tier_c/model.py` — frozen dataclasses: `Issue`, `Variant`, `Citation`, `ArmOutput`, `CitationVerdict`, `InvestigatorReport`, `PlantedError`, `PlantedReport`, `JudgeRanking`, `StageResult`, `Provenance`.
- `eval/tier_c/interfaces.py` — `Protocol`s: `ArmRunner`, `RelevanceJudge`, `RankJudge`.
- `eval/tier_c/corpus.py` — load + validate the open-issue registry (Goldilocks rubric, pinned SHA).
- `eval/tier_c/citations.py` — parse `file:line` / `file:line:symbol` citations out of an arm's text.
- `eval/tier_c/checkout.py` — pinned `git worktree` checkout of a corpus repo at a SHA (read-only).
- `eval/tier_c/investigator.py` — verify citations against the checkout via neutral primitives; precision/recall/hallucination; `RelevanceJudge` seam for the secondary relevance call.
- `eval/tier_c/planted.py` — inject planted-error refs into a frame; score catch-rate; sanitation zero-survival gate.
- `eval/tier_c/prompts.py` — citation-parity stage prompt templates (spec, plan).
- `eval/tier_c/arm_runner.py` — `ArmRunner` Protocol + `FakeArmRunner`; command-builders for `codex` and `claude`.
- `eval/tier_c/judges.py` — Borda consensus, family-bias inflation, detectability test (pure functions over `JudgeRanking`s).
- `eval/tier_c/chain.py` — run a stage (4 variants → investigator + planted + judges → consensus → sanitation → cleaned best); chain spec→plan with provenance.
- `eval/tier_c/report.py` — per-stage×language deltas, family-bias band, ITT/per-protocol, GO/NO-GO gate.
- `eval/tier_c/cli.py` — `tier-c` entry.
- `eval/tests/test_tc_*.py` — one test module per `tier_c` module.
- `eval/pyproject.toml` — add `tier-c = "tier_c.cli:main"` script.

---

### Task 1: Package scaffold + core dataclasses

**Files:**
- Create: `eval/tier_c/__init__.py`, `eval/tier_c/model.py`
- Modify: `eval/pyproject.toml` (add script)
- Test: `eval/tests/test_tc_model.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_model.py
from tier_c.model import Issue, Variant, Citation, ArmOutput

def test_variant_id_is_stable_and_family_derived():
    v = Variant(model="opus-4.8", prism=True)
    assert v.id == "opus-4.8+prism"
    assert v.family == "anthropic"
    assert Variant(model="gpt-5.5", prism=False).family == "openai"

def test_armoutput_carries_text_and_citations():
    out = ArmOutput(
        variant=Variant("gpt-5.5", False),
        text="see src/a.py:10",
        citations=[Citation("src/a.py", 10, None)],
        tokens=123, tool_calls=4, wall_s=1.5, used_prism=False,
    )
    assert out.citations[0].file == "src/a.py"
    assert out.tokens == 123
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_model.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/__init__.py
```
```python
# eval/tier_c/model.py
"""Tier-C schemas (spec 2026-06-23 rev-3). Frozen dataclasses; files repo-relative POSIX, lines 1-based."""
from __future__ import annotations
from dataclasses import dataclass, field

_ANTHROPIC = {"opus-4.8", "sonnet-4.6"}
_OPENAI = {"gpt-5.5", "gpt-5.3-spark"}

@dataclass(frozen=True, order=True)
class Variant:
    model: str
    prism: bool
    @property
    def id(self) -> str:
        return f"{self.model}{'+prism' if self.prism else ''}"
    @property
    def family(self) -> str:
        if self.model in _ANTHROPIC: return "anthropic"
        if self.model in _OPENAI: return "openai"
        return "unknown"

@dataclass(frozen=True)
class Citation:
    file: str
    line: int | None
    symbol: str | None

@dataclass(frozen=True)
class Issue:
    key: str            # e.g. "ripgrep-12345"
    language: str       # rust|go|python|js|ts
    repo: str           # local path under bench-repos
    sha: str            # pinned commit (issue still OPEN here)
    url: str
    text: str           # issue body snapshot
    scoped_slice: str   # the first-slice scope statement

@dataclass(frozen=True)
class ArmOutput:
    variant: Variant
    text: str
    citations: list[Citation]
    tokens: int
    tool_calls: int
    wall_s: float
    used_prism: bool
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_model.py -q`
Expected: PASS (2 passed).

- [ ] **Step 5: Add the CLI script entry (no behavior yet)**

In `eval/pyproject.toml`, under `[project.scripts]`, add below the `tier-a` line:
```toml
tier-c = "tier_c.cli:main"
```

- [ ] **Step 6: Commit**

```bash
git add eval/tier_c/__init__.py eval/tier_c/model.py eval/tests/test_tc_model.py eval/pyproject.toml
git commit -m "feat(tier-c): package scaffold + core dataclasses"
```

---

### Task 2: Issue corpus loader + Goldilocks validation

**Files:**
- Create: `eval/tier_c/corpus.py`, `eval/tier_c/issues/README.md`
- Test: `eval/tests/test_tc_corpus.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_corpus.py
import pytest
from tier_c.corpus import load_issues, CorpusError

VALID = b'''
[[issue]]
key = "ripgrep-1"
language = "rust"
repo = "ripgrep"
sha = "abc123abc123"
url = "https://github.com/BurntSushi/ripgrep/issues/1"
text = "globset ** matches hidden dirs"
scoped_slice = "fix the matcher in globset/src/glob.rs only"
files_touched_hint = 3
'''

def test_load_valid_issue(tmp_path):
    p = tmp_path / "issues.toml"; p.write_bytes(VALID)
    issues = load_issues(p)
    assert issues[0].key == "ripgrep-1"
    assert issues[0].language == "rust"

def test_rejects_one_liner(tmp_path):
    bad = VALID.replace(b"files_touched_hint = 3", b"files_touched_hint = 1")
    p = tmp_path / "i.toml"; p.write_bytes(bad)
    with pytest.raises(CorpusError, match="multi-file"):
        load_issues(p)

def test_rejects_missing_scoped_slice(tmp_path):
    bad = VALID.replace(b'scoped_slice = "fix the matcher in globset/src/glob.rs only"\n', b"")
    p = tmp_path / "i.toml"; p.write_bytes(bad)
    with pytest.raises(CorpusError, match="scoped_slice"):
        load_issues(p)
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_corpus.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.corpus'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/corpus.py
"""Open-issue registry (spec §4). Enforces the Goldilocks rubric at load time so a
bad corpus can't silently weaken the study. Selection is frozen before any run."""
from __future__ import annotations
import tomllib
from pathlib import Path
from .model import Issue

_LANGS = {"rust", "go", "python", "js", "ts"}

class CorpusError(Exception): ...

def load_issues(path: str | Path) -> list[Issue]:
    raw = tomllib.loads(Path(path).read_text())
    out: list[Issue] = []
    for d in raw.get("issue", []):
        key = d.get("key", "<no key>")
        for req in ("key", "language", "repo", "sha", "url", "text", "scoped_slice"):
            if not d.get(req):
                raise CorpusError(f"{key}: missing required field {req!r}")
        if d["language"] not in _LANGS:
            raise CorpusError(f"{key}: language {d['language']!r} not in {_LANGS}")
        if int(d.get("files_touched_hint", 0)) < 2:
            raise CorpusError(f"{key}: must be multi-file (files_touched_hint >= 2), "
                              "not a one-liner (spec §4 Goldilocks)")
        out.append(Issue(key=d["key"], language=d["language"], repo=d["repo"],
                         sha=d["sha"], url=d["url"], text=d["text"],
                         scoped_slice=d["scoped_slice"]))
    if not out:
        raise CorpusError("no [[issue]] entries found")
    return out
```
```markdown
<!-- eval/tier_c/issues/README.md -->
# Tier-C open-issue registry
`issues.toml` holds the frozen selection (spec §4). Each `[[issue]]` MUST satisfy the Goldilocks rubric:
multi-file (`files_touched_hint >= 2`), needs spec+plan (not one-shot), tractable (scope to first slice via
`scoped_slice`), pinned `sha` where the issue is still OPEN, buildable repo. Do not edit after a run starts.
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_corpus.py -q`
Expected: PASS (3 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/corpus.py eval/tier_c/issues/README.md eval/tests/test_tc_corpus.py
git commit -m "feat(tier-c): issue corpus loader + Goldilocks validation"
```

---

### Task 3: Citation parsing

**Files:**
- Create: `eval/tier_c/citations.py`
- Test: `eval/tests/test_tc_citations.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_citations.py
from tier_c.citations import parse_citations

def test_parses_file_line_and_file_line_symbol():
    text = "The bug is in `src/glob.rs:42` in fn `compile` (src/glob.rs:42:compile)."
    cites = parse_citations(text)
    assert ("src/glob.rs", 42, None) in [(c.file, c.line, c.symbol) for c in cites]
    assert ("src/glob.rs", 42, "compile") in [(c.file, c.line, c.symbol) for c in cites]

def test_ignores_non_code_colons():
    assert parse_citations("see http://x.com:80/page") == []

def test_dedupes():
    cites = parse_citations("src/a.py:1 and again src/a.py:1")
    assert len(cites) == 1
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_citations.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.citations'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/citations.py
"""Extract code citations (file:line[:symbol]) from arm text (spec §3 citation parity,
§6a investigator). Conservative: a citation is a path with a code-ish extension + line."""
from __future__ import annotations
import re
from .model import Citation

_EXT = r"(?:rs|go|py|js|jsx|ts|tsx|c|cc|cpp|h|hpp|java|lua)"
# path/seg.ext : line [ : symbol ]   — path has no spaces, optional ./
_PAT = re.compile(
    rf"(?<![\w/.])((?:[\w./-]+/)?[\w.-]+\.{_EXT}):(\d+)(?::([A-Za-z_]\w*))?"
)

def parse_citations(text: str) -> list[Citation]:
    seen: set[tuple[str, int, str | None]] = set()
    out: list[Citation] = []
    for m in _PAT.finditer(text):
        file, line, sym = m.group(1), int(m.group(2)), m.group(3)
        key = (file, line, sym)
        if key not in seen:
            seen.add(key)
            out.append(Citation(file=file, line=line, symbol=sym))
    return out
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_citations.py -q`
Expected: PASS (3 passed). (URL case passes because `http://x.com:80` has no code extension before the colon.)

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/citations.py eval/tests/test_tc_citations.py
git commit -m "feat(tier-c): citation parsing from arm text"
```

---

### Task 4: Pinned checkout (read-only worktree)

**Files:**
- Create: `eval/tier_c/checkout.py`
- Test: `eval/tests/test_tc_checkout.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_checkout.py
import subprocess
from pathlib import Path
from tier_c.checkout import Checkout

def _init_repo(p: Path) -> str:
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    (p / "a.py").write_text("def foo():\n    return 1\n")
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                          capture_output=True, text=True, check=True).stdout.strip()

def test_checkout_reads_file_at_sha(tmp_path):
    sha = _init_repo(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.read_line("a.py", 1) == "def foo():"
        assert co.file_exists("a.py")
        assert not co.file_exists("missing.py")
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_checkout.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.checkout'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/checkout.py
"""Read-only pinned checkout via `git worktree` (spec §4 pinning). The investigator
verifies citations against THIS, using neutral git/file primitives — never prism."""
from __future__ import annotations
import subprocess, tempfile, shutil
from pathlib import Path

class Checkout:
    def __init__(self, repo: str, sha: str):
        self.repo, self.sha = repo, sha
        self._dir: Path | None = None
    def __enter__(self) -> "Checkout":
        self._dir = Path(tempfile.mkdtemp(prefix="tc-co-"))
        subprocess.run(["git", "worktree", "add", "--detach", "-q", str(self._dir), self.sha],
                       cwd=self.repo, check=True)
        return self
    def __exit__(self, *exc) -> None:
        if self._dir:
            subprocess.run(["git", "worktree", "remove", "--force", str(self._dir)],
                           cwd=self.repo, check=False)
            shutil.rmtree(self._dir, ignore_errors=True)
    @property
    def root(self) -> Path:
        assert self._dir is not None
        return self._dir
    def file_exists(self, rel: str) -> bool:
        return (self.root / rel).is_file()
    def read_line(self, rel: str, line: int) -> str | None:
        p = self.root / rel
        if not p.is_file(): return None
        lines = p.read_text(errors="replace").splitlines()
        return lines[line - 1] if 1 <= line <= len(lines) else None
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_checkout.py -q`
Expected: PASS (1 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/checkout.py eval/tests/test_tc_checkout.py
git commit -m "feat(tier-c): pinned read-only checkout"
```

---

### Task 5: Investigator — mechanical citation verification + precision/recall

**Files:**
- Create: `eval/tier_c/interfaces.py`, `eval/tier_c/investigator.py`
- Test: `eval/tests/test_tc_investigator.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_investigator.py
from tier_c.model import Citation
from tier_c.investigator import verify_citation, score_citations, RelevanceAllTrue, RelevanceNone

class FakeCo:
    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line):
        return "def foo():" if (rel == "a.py" and line == 1) else (None if rel == "a.py" else None)

def test_verify_existing_symbol_line():
    v = verify_citation(FakeCo(), Citation("a.py", 1, "foo"))
    assert v.file_ok and v.line_ok and v.symbol_ok

def test_verify_nonexistent_file_is_hallucination():
    v = verify_citation(FakeCo(), Citation("ghost.py", 1, "x"))
    assert not v.file_ok and v.is_hallucination

def test_symbol_not_on_line_fails_symbol():
    v = verify_citation(FakeCo(), Citation("a.py", 1, "bar"))
    assert v.file_ok and v.line_ok and not v.symbol_ok

def test_precision_recall_penalize_undercite():
    # 1 valid citation, but 3 substantive claims -> recall 1/3 (under-citing penalized)
    cites = [Citation("a.py", 1, "foo")]
    rep = score_citations(FakeCo(), cites, claim_count=3, relevance=RelevanceAllTrue())
    assert rep.precision == 1.0
    assert abs(rep.recall - 1/3) < 1e-9
    assert rep.hallucinations == 0

def test_precision_counts_irrelevant_against():
    cites = [Citation("a.py", 1, "foo")]
    rep = score_citations(FakeCo(), cites, claim_count=1, relevance=RelevanceNone())
    assert rep.precision == 0.0  # exists but judged irrelevant
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_investigator.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.investigator'`.

- [ ] **Step 3: Write the Protocol seam + implementation**

```python
# eval/tier_c/interfaces.py
"""Model-driving + relevance seams (spec §6). Orchestration/scoring depend ONLY on
these, so fakes drive all tests and the live drivers swap in behind them."""
from __future__ import annotations
from typing import Protocol, runtime_checkable
from .model import Citation, Variant, ArmOutput

@runtime_checkable
class RelevanceJudge(Protocol):
    """Secondary, audit-sampled relevance call (spec §6a) — NEVER prism-backed."""
    def is_relevant(self, cite: Citation, issue_text: str) -> bool: ...

@runtime_checkable
class ArmRunner(Protocol):
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput: ...

@runtime_checkable
class RankJudge(Protocol):
    """Returns a full ranking (best-first) of anonymized candidate ids."""
    def rank(self, stage: str, rubric: str, candidates: dict[str, str]) -> list[str]: ...
```
```python
# eval/tier_c/investigator.py
"""Independent citation oracle (spec §6a, codex new-2/new-3): mechanical existence via
neutral file/git primitives — NEVER prism — plus a SECONDARY relevance seam. Scores
citation PRECISION (real & relevant) and RECALL/claim-coverage (under-citing penalized)."""
from __future__ import annotations
from dataclasses import dataclass
from .model import Citation
from .interfaces import RelevanceJudge

@dataclass(frozen=True)
class CitationVerdict:
    cite: Citation
    file_ok: bool
    line_ok: bool
    symbol_ok: bool
    relevant: bool
    @property
    def is_hallucination(self) -> bool:
        return not (self.file_ok and self.line_ok and self.symbol_ok)
    @property
    def is_valid(self) -> bool:
        return not self.is_hallucination and self.relevant

@dataclass(frozen=True)
class InvestigatorReport:
    precision: float        # valid / cited
    recall: float           # valid / claim_count  (claim-coverage; under-cite -> low)
    hallucinations: int
    verdicts: list[CitationVerdict]

class RelevanceAllTrue:
    def is_relevant(self, cite, issue_text): return True
class RelevanceNone:
    def is_relevant(self, cite, issue_text): return False

def verify_citation(co, cite: Citation, *, issue_text: str = "",
                    relevance: RelevanceJudge | None = None) -> CitationVerdict:
    file_ok = co.file_exists(cite.file)
    line_ok = file_ok and (cite.line is None or co.read_line(cite.file, cite.line) is not None)
    symbol_ok = True
    if cite.symbol is not None:
        ln = co.read_line(cite.file, cite.line) if (file_ok and cite.line) else None
        symbol_ok = bool(ln and cite.symbol in ln)
    relevant = True
    if relevance is not None and file_ok and line_ok and symbol_ok:
        relevant = relevance.is_relevant(cite, issue_text)
    return CitationVerdict(cite, file_ok, line_ok, symbol_ok, relevant)

def score_citations(co, cites: list[Citation], *, claim_count: int,
                    relevance: RelevanceJudge, issue_text: str = "") -> InvestigatorReport:
    verdicts = [verify_citation(co, c, issue_text=issue_text, relevance=relevance) for c in cites]
    valid = sum(v.is_valid for v in verdicts)
    halluc = sum(v.is_hallucination for v in verdicts)
    precision = valid / len(verdicts) if verdicts else 0.0
    recall = valid / claim_count if claim_count > 0 else 0.0
    return InvestigatorReport(precision, recall, halluc, verdicts)
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_investigator.py -q`
Expected: PASS (5 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/interfaces.py eval/tier_c/investigator.py eval/tests/test_tc_investigator.py
git commit -m "feat(tier-c): investigator (neutral citation oracle, precision/recall)"
```

---

### Task 6: Planted-error injector + catch scorer + sanitation gate

**Files:**
- Create: `eval/tier_c/planted.py`
- Test: `eval/tests/test_tc_planted.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_planted.py
from tier_c.planted import PlantedError, inject, score_catch, sanitation_ok

PLANTS = [
    PlantedError(kind="file", token="src/ghost_module.rs"),
    PlantedError(kind="function", token="frobnicate_nonexistent"),
]

def test_inject_appends_planted_refs():
    frame, plants = inject("Original spec text.", PLANTS)
    assert "src/ghost_module.rs" in frame and "frobnicate_nonexistent" in frame
    assert plants == PLANTS

def test_score_catch_counts_flagged_plants():
    out_text = "Note: src/ghost_module.rs does not exist; ignoring it."
    rep = score_catch(out_text, PLANTS)
    assert rep.caught == 1 and rep.total == 2
    assert abs(rep.recall - 0.5) < 1e-9

def test_sanitation_rejects_surviving_plant():
    # carried frame still references a planted token -> not clean
    assert not sanitation_ok("the plan uses frobnicate_nonexistent()", PLANTS)
    assert sanitation_ok("the plan uses real_function()", PLANTS)
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_planted.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.planted'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/planted.py
"""Planted-error sensitivity probe (spec §6a). DIAGNOSTIC metric (codex new-4): catch-rate
is reported alongside real task correctness, not as standalone value. Sanitation gate
(codex new-5) guarantees zero planted residue before a frame is carried forward."""
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class PlantedError:
    kind: str    # file|function|variable|claim
    token: str   # the invalid reference text

@dataclass(frozen=True)
class PlantedReport:
    caught: int
    total: int
    @property
    def recall(self) -> float:
        return self.caught / self.total if self.total else 0.0

_FLAG_WORDS = ("does not exist", "doesn't exist", "no such", "invalid", "nonexistent",
               "not found", "incorrect", "wrong", "ignore", "remove", "typo")

def inject(frame: str, plants: list[PlantedError]) -> tuple[str, list[PlantedError]]:
    salt = "\n\n[references to verify] " + ", ".join(p.token for p in plants)
    return frame + salt, list(plants)

def score_catch(out_text: str, plants: list[PlantedError]) -> PlantedReport:
    low = out_text.lower()
    caught = 0
    for p in plants:
        i = low.find(p.token.lower())
        if i == -1:
            continue
        window = low[max(0, i - 80): i + len(p.token) + 80]
        if any(w in window for w in _FLAG_WORDS):
            caught += 1
    return PlantedReport(caught=caught, total=len(plants))

def sanitation_ok(carried_frame: str, plants: list[PlantedError]) -> bool:
    low = carried_frame.lower()
    return not any(p.token.lower() in low for p in plants)
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_planted.py -q`
Expected: PASS (3 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/planted.py eval/tests/test_tc_planted.py
git commit -m "feat(tier-c): planted-error probe + sanitation gate"
```

---

### Task 7: Judge consensus, family-bias, detectability (pure logic)

**Files:**
- Create: `eval/tier_c/judges.py`
- Test: `eval/tests/test_tc_judges.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_judges.py
from tier_c.judges import borda_consensus, family_bias, detectable

def test_borda_consensus_combines_two_rankings():
    a = ["x", "y", "z", "w"]   # judge A best-first
    b = ["y", "x", "w", "z"]   # judge B
    order = borda_consensus({"A": a, "B": b})
    assert order[0] in ("x", "y")   # x and y tie at top; deterministic tie-break by id
    assert order == sorted(order, key=lambda c: order.index(c))  # stable list

def test_family_bias_detects_own_family_inflation():
    # anthropic judge ranks anthropic ids high; openai judge ranks openai high
    fam = {"a1": "anthropic", "a2": "anthropic", "o1": "openai", "o2": "openai"}
    rankings = {"anthropic": ["a1", "a2", "o1", "o2"], "openai": ["o1", "o2", "a1", "a2"]}
    bias = family_bias(rankings, fam)
    assert bias > 0  # each judge favors own family

def test_no_bias_when_judges_agree():
    fam = {"a1": "anthropic", "o1": "openai"}
    rankings = {"anthropic": ["a1", "o1"], "openai": ["a1", "o1"]}
    assert family_bias(rankings, fam) == 0.0

def test_detectable_true_above_chance():
    # classifier guessed condition correctly 9/10 -> detectable
    assert detectable(correct=9, n=10, threshold=0.7)
    assert not detectable(correct=5, n=10, threshold=0.7)
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_judges.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.judges'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/judges.py
"""Judge combination + bias instrumentation (spec §6b). Consensus cancels symmetric
family bias; family_bias() reports residual; detectable() gates the subjective channel
(codex new-1/new-7: if prism condition is detectable, the judge prism-delta is INVALID)."""
from __future__ import annotations

def borda_consensus(rankings: dict[str, list[str]]) -> list[str]:
    ids = {c for r in rankings.values() for c in r}
    points: dict[str, int] = {c: 0 for c in ids}
    for r in rankings.values():
        n = len(r)
        for pos, c in enumerate(r):
            points[c] += (n - pos)  # best-first => most points
    return sorted(ids, key=lambda c: (-points[c], c))  # deterministic tie-break by id

def _mean_rank(order: list[str], ids: set[str]) -> float:
    ranks = [i for i, c in enumerate(order) if c in ids]
    return sum(ranks) / len(ranks) if ranks else 0.0

def family_bias(rankings: dict[str, list[str]], family_of: dict[str, str]) -> float:
    """How much each family-judge favors its OWN family vs the other judge does.
    0 = no own-family inflation; larger = stronger 'judges-own-family' trend."""
    judge_fams = list(rankings.keys())
    if len(judge_fams) != 2:
        return 0.0
    jf_a, jf_b = judge_fams
    own_ids = {f: {c for c, fam in family_of.items() if fam == f} for f in judge_fams}
    # mean rank (lower=better) each judge gives family jf_a:
    a_to_a = _mean_rank(rankings[jf_a], own_ids[jf_a])
    b_to_a = _mean_rank(rankings[jf_b], own_ids[jf_a])
    a_to_b = _mean_rank(rankings[jf_a], own_ids[jf_b])
    b_to_b = _mean_rank(rankings[jf_b], own_ids[jf_b])
    # own-family advantage: other judge ranks my family WORSE (higher) than I do
    return max(0.0, ((b_to_a - a_to_a) + (a_to_b - b_to_b)) / 2.0)

def detectable(correct: int, n: int, threshold: float = 0.7) -> bool:
    return n > 0 and (correct / n) > threshold
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_judges.py -q`
Expected: PASS (4 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/judges.py eval/tests/test_tc_judges.py
git commit -m "feat(tier-c): borda consensus + family-bias + detectability"
```

---

### Task 8: Arm runner — Protocol, fake, and command builders

**Files:**
- Create: `eval/tier_c/prompts.py`, `eval/tier_c/arm_runner.py`
- Test: `eval/tests/test_tc_arm_runner.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_arm_runner.py
from tier_c.model import Variant
from tier_c.prompts import stage_prompt
from tier_c.arm_runner import build_codex_cmd, build_claude_cmd, FakeArmRunner

def test_stage_prompt_requires_citations():
    p = stage_prompt("spec", issue_text="bug X", scoped_slice="slice 1")
    assert "cite" in p.lower() and "file" in p.lower()  # citation parity

def test_codex_cmd_on_off_toggles_mcp():
    # codex configures MCP via inline `-c mcp_servers.prism...` (not a --mcp-config file)
    on = build_codex_cmd(Variant("gpt-5.5", True), repo="/r")
    off = build_codex_cmd(Variant("gpt-5.5", False), repo="/r")
    assert "mcp_servers.prism" in " ".join(on)
    assert "mcp_servers.prism" not in " ".join(off)
    assert "gpt-5.5" in " ".join(on)

def test_claude_cmd_on_off_toggles_mcp():
    on = build_claude_cmd(Variant("opus-4.8", True), mcp_cfg="/tmp/p.json")
    off = build_claude_cmd(Variant("opus-4.8", False), mcp_cfg="/tmp/p.json")
    assert "/tmp/p.json" in " ".join(on)
    assert "--mcp-config" not in " ".join(off)

def test_fake_runner_is_deterministic():
    r = FakeArmRunner({"gpt-5.5+prism": "spec cites src/a.py:1"})
    out = r.run(Variant("gpt-5.5", True), "spec", "prompt", "/r")
    assert out.text == "spec cites src/a.py:1"
    assert out.citations[0].file == "src/a.py"
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_arm_runner.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.prompts'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/prompts.py
"""Citation-parity stage prompts (spec §3). BOTH arms (prism on/off) are required to
cite file/line/function for every substantive claim, so citation presence is not a tell."""
from __future__ import annotations

_PARITY = ("For every substantive claim about the code, you MUST cite the exact "
           "`file:line` (and `:function` where relevant). Unsupported claims count against you.")

_STAGE = {
    "spec": "Write a short implementation SPEC for this issue, scoped to the stated slice.",
    "plan": "Write a step-by-step PLAN for this spec, scoped to the stated slice.",
}

def stage_prompt(stage: str, *, issue_text: str, scoped_slice: str, upstream: str = "") -> str:
    parts = [_STAGE[stage], _PARITY, f"\nISSUE:\n{issue_text}", f"\nSCOPE (first slice only):\n{scoped_slice}"]
    if upstream:
        parts.append(f"\nUPSTREAM ARTIFACT:\n{upstream}")
    return "\n".join(parts)
```
```python
# eval/tier_c/arm_runner.py
"""Concrete model drivers + a fake (spec §3). Command builders are unit-tested on the
ARGV they assemble (the live subprocess call is exercised only in an integration run).
prism ON = MCP config passed; OFF = omitted. Mirrors tier_a/sut.py's subprocess style."""
from __future__ import annotations
import json, subprocess, time
from .model import Variant, ArmOutput
from .citations import parse_citations

def build_codex_cmd(variant: Variant, *, repo: str) -> list[str]:
    # codex MCP is inline `-c mcp_servers.prism...`; OFF omits it. `-` reads prompt from stdin.
    cmd = ["codex", "exec", "-m", variant.model, "-C", repo, "-s", "workspace-write", "-"]
    if variant.prism:
        cmd[6:6] = ["-c", "mcp_servers.prism.command=prism-mcp",
                    "-c", f'mcp_servers.prism.args=["--repo","{repo}"]']
    return cmd

def build_claude_cmd(variant: Variant, *, mcp_cfg: str) -> list[str]:
    cmd = ["claude", "-p", "--output-format", "json", "--model", variant.model]
    if variant.prism:
        cmd += ["--mcp-config", mcp_cfg, "--strict-mcp-config"]
    return cmd

class FakeArmRunner:
    """Deterministic runner keyed by variant.id -> canned text (spec §6 fakes-drive-tests)."""
    def __init__(self, by_id: dict[str, str]):
        self._by_id = by_id
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        text = self._by_id.get(variant.id, "")
        return ArmOutput(variant=variant, text=text, citations=parse_citations(text),
                         tokens=len(text.split()), tool_calls=0, wall_s=0.0,
                         used_prism="prism" in text.lower() if variant.prism else False)
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_arm_runner.py -q`
Expected: PASS (4 passed).

- [ ] **Step 5: Integration check (manual, not in CI) — verify the live codex command runs**

Run: `cd eval && uv run python -c "from tier_c.arm_runner import build_codex_cmd; from tier_c.model import Variant; print(' '.join(build_codex_cmd(Variant('gpt-5.5',False), repo='.')))"`
Expected: prints a `codex exec -m gpt-5.5 ...` line. (Live model calls are exercised in the end-to-end run, Task 11; flags verified against `codex exec --help` / `claude --help`.)

- [ ] **Step 6: Commit**

```bash
git add eval/tier_c/prompts.py eval/tier_c/arm_runner.py eval/tests/test_tc_arm_runner.py
git commit -m "feat(tier-c): arm-runner prompts, command builders, fake runner"
```

---

### Task 9: Stage orchestration (one stage, 4 variants → cleaned best)

**Files:**
- Create: `eval/tier_c/chain.py`
- Test: `eval/tests/test_tc_chain.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_chain.py
from tier_c.model import Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.planted import PlantedError
from tier_c.investigator import RelevanceAllTrue
from tier_c.chain import run_stage, StageResult

class FakeCo:
    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line): return "x" if rel == "a.py" else None

class FakeRank:
    def rank(self, stage, rubric, candidates):  # best = longest text (proxy)
        return sorted(candidates, key=lambda k: -len(candidates[k]))

def test_run_stage_scores_all_variants_and_picks_clean_best():
    variants = [Variant("opus-4.8", True), Variant("opus-4.8", False),
                Variant("gpt-5.5", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({
        "opus-4.8+prism": "long spec cites a.py:1 and notes ghosttoken is invalid",
        "opus-4.8": "spec a.py:1",
        "gpt-5.5+prism": "spec a.py:1",
        "gpt-5.5": "x",
    })
    res = run_stage(
        stage="spec", variants=variants, runner=runner, co=FakeCo(),
        prompt="p", repo_root="/r", claim_counts={v.id: 1 for v in variants},
        plants=[PlantedError("file", "ghosttoken")],
        judges={"anthropic": FakeRank(), "openai": FakeRank()},
        relevance=RelevanceAllTrue(),
    )
    assert isinstance(res, StageResult)
    assert set(res.investigator.keys()) == {v.id for v in variants}
    # opus+prism caught the planted token and is longest -> consensus best
    assert res.best_variant_id == "opus-4.8+prism"
    # carried frame is sanitized (no planted token)
    assert "ghosttoken" not in res.cleaned_best_text
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_chain.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.chain'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/chain.py
"""Stage orchestration + spec->plan chaining (spec §5). run_stage: 4 variants ->
investigator + planted catch + judge consensus -> sanitized cleaned-best carried
forward. Per-stage prism delta is conditional on the carried frame (provenance logged)."""
from __future__ import annotations
import re
from dataclasses import dataclass, field
from .model import Variant
from .investigator import score_citations, InvestigatorReport
from .planted import PlantedError, score_catch, sanitation_ok, PlantedReport

@dataclass(frozen=True)
class StageResult:
    stage: str
    investigator: dict[str, InvestigatorReport]
    planted: dict[str, PlantedReport]
    consensus: list[str]              # best-first variant ids
    best_variant_id: str
    cleaned_best_text: str
    used_prism: dict[str, bool]
    tokens: dict[str, int]

def _strip_plants(text: str, plants: list[PlantedError]) -> str:
    out = text
    for p in plants:
        out = re.sub(re.escape(p.token), "[removed]", out, flags=re.IGNORECASE)
    return out

def run_stage(*, stage, variants, runner, co, prompt, repo_root, claim_counts,
              plants, judges, relevance) -> StageResult:
    outputs = {v.id: runner.run(v, stage, prompt, repo_root) for v in variants}
    investigator = {
        vid: score_citations(co, o.citations, claim_count=claim_counts[vid],
                             relevance=relevance)
        for vid, o in outputs.items()
    }
    planted = {vid: score_catch(o.text, plants) for vid, o in outputs.items()}
    # blind candidates: anonymized id -> text
    candidates = {vid: o.text for vid, o in outputs.items()}
    from .judges import borda_consensus
    rankings = {fam: j.rank(stage, "rubric", candidates) for fam, j in judges.items()}
    consensus = borda_consensus(rankings)
    best = consensus[0]
    cleaned = _strip_plants(outputs[best].text, plants)
    assert sanitation_ok(cleaned, plants), "sanitation gate failed (codex new-5)"
    return StageResult(
        stage=stage, investigator=investigator, planted=planted, consensus=consensus,
        best_variant_id=best, cleaned_best_text=cleaned,
        used_prism={vid: o.used_prism for vid, o in outputs.items()},
        tokens={vid: o.tokens for vid, o in outputs.items()},
    )
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_chain.py -q`
Expected: PASS (1 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/chain.py eval/tests/test_tc_chain.py
git commit -m "feat(tier-c): single-stage orchestration with sanitized cleaned-best"
```

---

### Task 10: Chain spec→plan (reset-to-best + provenance)

**Files:**
- Modify: `eval/tier_c/chain.py`
- Test: `eval/tests/test_tc_chain.py` (append)

- [ ] **Step 1: Write the failing test**

```python
# append to eval/tests/test_tc_chain.py
from tier_c.chain import run_spec_plan_chain

def test_chain_feeds_cleaned_spec_into_plan_prompt(monkeypatch):
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({
        "opus-4.8+prism": "spec body a.py:1", "gpt-5.5": "x",
    })
    captured = {}
    def fake_prompt(stage, *, issue_text, scoped_slice, upstream=""):
        captured[stage] = upstream
        return "p"
    res = run_spec_plan_chain(
        issue_text="bug", scoped_slice="slice1", variants=variants, runner=runner,
        co=FakeCo(), claim_counts={v.id: 1 for v in variants}, plants=[],
        judges={"anthropic": FakeRank(), "openai": FakeRank()},
        relevance=RelevanceAllTrue(), prompt_fn=fake_prompt,
    )
    assert res.stages[0].stage == "spec" and res.stages[1].stage == "plan"
    # plan stage received the cleaned best spec as upstream
    assert "spec body" in captured["plan"]
    assert res.provenance.spec_best == "opus-4.8+prism"
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_chain.py::test_chain_feeds_cleaned_spec_into_plan_prompt -q`
Expected: FAIL — `ImportError: cannot import name 'run_spec_plan_chain'`.

- [ ] **Step 3: Write the minimal implementation (append to chain.py)**

```python
# append to eval/tier_c/chain.py
@dataclass(frozen=True)
class Provenance:
    spec_best: str
    plan_best: str

@dataclass(frozen=True)
class ChainResult:
    stages: list[StageResult]
    provenance: Provenance

def run_spec_plan_chain(*, issue_text, scoped_slice, variants, runner, co,
                        claim_counts, plants, judges, relevance, prompt_fn) -> ChainResult:
    spec_prompt = prompt_fn("spec", issue_text=issue_text, scoped_slice=scoped_slice)
    spec = run_stage(stage="spec", variants=variants, runner=runner, co=co,
                     prompt=spec_prompt, repo_root=str(getattr(co, "root", ".")),
                     claim_counts=claim_counts, plants=plants, judges=judges,
                     relevance=relevance)
    plan_prompt = prompt_fn("plan", issue_text=issue_text, scoped_slice=scoped_slice,
                            upstream=spec.cleaned_best_text)
    plan = run_stage(stage="plan", variants=variants, runner=runner, co=co,
                     prompt=plan_prompt, repo_root=str(getattr(co, "root", ".")),
                     claim_counts=claim_counts, plants=plants, judges=judges,
                     relevance=relevance)
    return ChainResult(stages=[spec, plan],
                       provenance=Provenance(spec.best_variant_id, plan.best_variant_id))
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_chain.py -q`
Expected: PASS (all chain tests).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/chain.py eval/tests/test_tc_chain.py
git commit -m "feat(tier-c): spec->plan chain with reset-to-best + provenance"
```

---

### Task 11: Report — per-stage prism deltas, family-bias band, GO/NO-GO gate

**Files:**
- Create: `eval/tier_c/report.py`
- Test: `eval/tests/test_tc_report.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_report.py
from tier_c.report import prism_delta, gate_decision

def test_prism_delta_is_within_model_on_minus_off():
    # precision: opus+prism 0.9 vs opus(off) 0.6 -> +0.3
    by_id = {"opus-4.8+prism": 0.9, "opus-4.8": 0.6}
    assert abs(prism_delta(by_id, "opus-4.8") - 0.3) < 1e-9

def test_gate_go_when_material_lift_and_low_failure():
    d = gate_decision(precision_delta=0.25, recall_delta=0.2, planted_delta=0.3,
                      analyze_failure_rate=0.0, cost_ok=True, detectable_judges=False)
    assert d.decision == "GO"

def test_gate_nogo_when_high_analyze_failure():
    d = gate_decision(precision_delta=0.25, recall_delta=0.2, planted_delta=0.3,
                      analyze_failure_rate=0.6, cost_ok=True, detectable_judges=False)
    assert d.decision == "NO-GO"
    assert "coverage" in d.reason.lower()

def test_gate_nogo_when_flat():
    d = gate_decision(precision_delta=0.0, recall_delta=0.0, planted_delta=0.0,
                      analyze_failure_rate=0.0, cost_ok=True, detectable_judges=False)
    assert d.decision == "NO-GO"
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_report.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.report'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/report.py
"""Reporting + investment gate (spec §7, §12). Primary signal = within-model prism
delta on the OBJECTIVE channel; cross-model delta carries the family-bias band; the
GO/NO-GO gate is per role x language and never averaged."""
from __future__ import annotations
from dataclasses import dataclass

def prism_delta(metric_by_id: dict[str, float], model: str) -> float:
    return metric_by_id.get(f"{model}+prism", 0.0) - metric_by_id.get(model, 0.0)

@dataclass(frozen=True)
class Gate:
    decision: str   # GO | NO-GO
    reason: str

_MATERIAL = 0.1            # min objective lift to count
_MAX_ANALYZE_FAIL = 0.34   # above this = coverage debt, fix-first

def gate_decision(*, precision_delta, recall_delta, planted_delta,
                  analyze_failure_rate, cost_ok, detectable_judges) -> Gate:
    if analyze_failure_rate > _MAX_ANALYZE_FAIL:
        return Gate("NO-GO", "high prism analyze-failure / coverage debt — fix maturity first (spec §12)")
    lift = max(precision_delta, recall_delta, planted_delta)
    if lift < _MATERIAL:
        return Gate("NO-GO", "no material objective lift (flat/negative)")
    if not cost_ok:
        return Gate("NO-GO", "lift not net of token/latency cost (non-final, spec §8)")
    note = " (judges detectable -> objective-only)" if detectable_judges else ""
    return Gate("GO", f"material objective lift, acceptable cost & coverage{note}")
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_report.py -q`
Expected: PASS (4 passed).

- [ ] **Step 5: Commit**

```bash
git add eval/tier_c/report.py eval/tests/test_tc_report.py
git commit -m "feat(tier-c): reporting deltas + GO/NO-GO investment gate"
```

---

### Task 12: CLI wiring + fakes end-to-end smoke

**Files:**
- Create: `eval/tier_c/cli.py`
- Test: `eval/tests/test_tc_cli.py`

- [ ] **Step 1: Write the failing test**

```python
# eval/tests/test_tc_cli.py
from tier_c.cli import main

def test_cli_dry_run_lists_issues(tmp_path, capsys):
    p = tmp_path / "issues.toml"
    p.write_text('[[issue]]\nkey="k"\nlanguage="rust"\nrepo="r"\nsha="s"\n'
                 'url="u"\ntext="t"\nscoped_slice="s1"\nfiles_touched_hint=2\n')
    rc = main(["--issues", str(p), "--list"])
    assert rc == 0
    assert "k" in capsys.readouterr().out
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd eval && uv run pytest tests/test_tc_cli.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tier_c.cli'`.

- [ ] **Step 3: Write the minimal implementation**

```python
# eval/tier_c/cli.py
"""tier-c entry (spec). Phase 1 supports --list and (later) a fakes/live run driver.
Mirrors tier_a/cli.py argument-parsing style."""
from __future__ import annotations
import argparse
from .corpus import load_issues

def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="tier-c")
    ap.add_argument("--issues", required=True)
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args(argv)
    issues = load_issues(args.issues)
    if args.list:
        for i in issues:
            print(f"{i.key}\t{i.language}\t{i.scoped_slice}")
        return 0
    print(f"loaded {len(issues)} issues (run driver lands in Task 13 / Phase-1 live run)")
    return 0
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd eval && uv run pytest tests/test_tc_cli.py -q`
Expected: PASS (1 passed).

- [ ] **Step 5: Run the whole tier_c suite + confirm tier_a untouched**

Run: `cd eval && uv run pytest tests/test_tc_*.py -q && uv run pytest tests/test_model.py -q`
Expected: all tier_c green; tier_a still green.

- [ ] **Step 6: Commit**

```bash
git add eval/tier_c/cli.py eval/tests/test_tc_cli.py
git commit -m "feat(tier-c): cli entry + end-to-end fakes smoke"
```

---

## Self-Review

**1. Spec coverage:**
- §2 oracle/leakage → corpus (Task 2) holds pinned-SHA open issues; cold leakage probe is a Phase-1 *run-time* checklist item (manual; not code) — noted, not a gap for the harness.
- §3 arms + citation parity → `prompts.py` parity prompt (Task 8); stage-specific models are data (`Variant`), develop pair is Phase 2.
- §4 Goldilocks selection → corpus validation (Task 2).
- §5 chain / reset-to-best / sanitation → chain (Tasks 9–10), `_strip_plants` + `sanitation_ok` gate.
- §6a investigator (neutral, precision/recall, no-prism) → investigator (Task 5); planted diagnostic (Task 6).
- §6b dual judges / consensus / family-bias / detectability → judges (Task 7), wired in chain (Task 9).
- §7 reporting / conditional delta → report (Task 11); provenance (Task 10).
- §12 GO/NO-GO gate → `gate_decision` (Task 11).
- **Deferred to Phase 2 (out of scope, by design):** develop+review stages, per-repo build/test sandboxes, seeded-bug review, live model-call end-to-end (Task 8 builds the commands; the live multi-model run is the first Phase-1 *execution*, run after this plan lands).

**2. Placeholder scan:** no TBD/TODO; every code step has complete code; the one manual step (Task 8 Step 5 integration check) is an explicit verification, not a code placeholder.

**3. Type consistency:** `Variant.id`/`.family`, `ArmOutput`, `Citation(file,line,symbol)`, `InvestigatorReport(precision,recall,hallucinations)`, `PlantedReport(caught,total)`, `StageResult`, `ChainResult/Provenance`, `Gate(decision,reason)` are used consistently across Tasks 1–12. `RelevanceJudge`/`RankJudge`/`ArmRunner` Protocols match their fakes (`RelevanceAllTrue`, `FakeRank`, `FakeArmRunner`).

---

## Notes for the executor
- **Run from `eval/`:** all tests are `cd eval && uv run pytest tests/test_tc_*.py`.
- **Investigator must never import prism** (codex new-3) — it uses `Checkout` (git/file) only. Keep it that way.
- **Live model flags:** Task 8's `build_codex_cmd`/`build_claude_cmd` encode the on/off MCP toggle; verify exact flags against `codex exec --help` and `claude --help` before the first live run, and keep the unit tests asserting the on/off toggle.
- **NEVER stage** `eval/snapshots/` or `docs/eval/`. Commit only the explicit `tier_c` + test paths shown.

---

## Phase-1 known gaps (from the opus final review — what the next phase must build)

The Phase-1 package is the **objective backbone + spec→plan orchestration, fakes-tested**. These are
deliberately deferred and must be built before the harness produces a real verdict:

1. **Live model-call driver (the first execution after this lands).** `build_codex_cmd`/`build_claude_cmd`
   assemble argv and are unit-tested on the on/off MCP toggle, but **nothing spawns the subprocess or parses
   real `codex exec`/`claude -p --output-format json` output** (capture text, citations, tokens, tool-calls,
   `used_prism`). Re-verify exact flags against `codex exec --help` / `claude --help` first. The `claim_count`
   per output (denominator of citation recall) also needs a real extractor (count substantive repo-claims).
2. **Detectability test is load-bearing and unwired.** `judges.detectable()` is a pure function; there is no
   pooling across issues, no pre-registered permutation test/threshold, and nothing computes
   `detectable_judges` to feed `gate_decision`. Spec §6b makes this decisive ("if detectable, the judge
   prism-delta is INVALID"). **Label-blinding alone is insufficient** — a live arm's *text* can still leak the
   prism condition (prism phrasing / citation density); the detectability gate is the real safety net.
3. **Relevance-judge audit-sampling + κ adjudication.** `RelevanceJudge` has only `RelevanceAllTrue/None`
   fakes; the spec's audit-sampled, blind, adjudicated relevance pass (§6a) and κ-style judge reconciliation
   (§6b) are unbuilt.
4. **Report assembly is partial.** `prism_delta` + `gate_decision` exist, but there is **no per-(stage ×
   language) table generator**, `family_bias` is never called by a report, and there is no
   evidence-availability split or ITT/per-protocol usage rollup (`StageResult` carries `used_prism`/`tokens`
   but nothing aggregates them). Spec §7/§12.
5. **Sanitation gate is structural-only.** `_strip_plants` removes all tokens then `sanitation_ok` checks the
   same set, so the `raise` is fail-closed defense-in-depth but cannot fire on token residue; the spec §5
   *semantic* survival check ("planted falsehood surviving as a corrected-looking claim") and the **re-run
   loop** are not implemented.
6. **Tie-handling.** `borda_consensus` tie-breaks deterministically by id (favors lexically-smaller ids); spec
   §5/§8 wants a **pre-registered random** tie-break with tied cells flagged non-identifiable.
7. **Cold leakage probe** (spec §2 — ask the model if it already knows the fix; drop reproducible issues) is a
   run-time checklist item with no harness support yet.
8. **Develop + Review stages = Phase 2** (separate plan): per-repo build/test/lint sandbox, repro resolution,
   seeded-bug recall. The chain stops at spec→plan.
9. **Two human-in-loop inputs before the first live run:** select + pin the actual **open issues** (the
   Goldilocks corpus → `eval/tier_c/issues/issues.toml`), and verify the live `codex`/`claude` flags.

Minor/diagnostic (acceptable as-is, noted for calibration): `score_catch` under-counts (±80-char window, first
occurrence only) — it's an explicit diagnostic metric; `ArmRunner`/`RankJudge` Protocols aren't used as type
annotations on `run_stage` (tightening would catch fake-drift); the unrealized `JudgeRanking` type name (folded
into `dict[str, list[str]]`).
