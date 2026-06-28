# eval/tests/test_tc_run_partc.py
"""Task 11: single-cell Part-C runner + report + CLI (fakes only — NO live spend).

Tests:
  (a) main target: run_partc_cell scores on vs base with fakes (both-live flow).
  (b) 0-prism on-arm → administered == False.
  (c) leaked text → leaked == True.
  (d) render_partc includes the cell row + "pilot signal" label.
  (e) live comps score() threads issue.text + code window to the real oracle.
  (f) _upstream_spec returns "" for spec cell, recovered spec body for plan cell (FIX 2).
  (g) score() for a PLAN cell threads both issue.text and the upstream spec to the oracle
      (run_on_arm also passes upstream); spec cell does NOT receive a spec section (FIX 2).
  (h) administered reflects the ON arm's used_prism (not the OFF arm).
  (i) leaked reflects the ON arm's text (not the OFF arm's text).
  (j) run_partc_cell threads tokens/cost/wall from ArmOutputs into PartCCell.
  (k) render_partc shows token + cost columns + signed Δtokens.
  (l) run_partc_cell extracts per-arm citation breakdown (valid/halluc/irrelevant/fails).
  (m) run_partc_cell threads in/out token split into PartCCell.
  (n) render_partc shows in/out token split line and cite-breakdown lines.
"""
from __future__ import annotations
import types
import pytest
from tier_c.model import Dose, ArmOutput, Variant, Citation
from tier_c.partc import PartCCell, run_partc_cell, render_partc
from tier_c.investigator import InvestigatorReport, CitationVerdict


# ---------------------------------------------------------------------------
# Helpers — fake composites
# ---------------------------------------------------------------------------

def _cell(repo: str, stage: str, model: str):
    """Tiny descriptor triple for run_partc_cell."""
    return (repo, stage, model)


def _fake_arm_output(*, used_prism: bool, prism_calls: int, text: str,
                     citations: list | None = None,
                     in_tokens: int = 0, cost_usd: float = 0.0) -> ArmOutput:
    """Build a minimal ArmOutput for fake comps."""
    dose = Dose(count=prism_calls)
    if citations is None:
        citations = [Citation(file="src/a.go", line=1, symbol=None)]
    return ArmOutput(
        variant=Variant("opus-4.8", True),
        text=text,
        citations=citations,
        tokens=10,
        tool_calls=prism_calls,
        wall_s=0.1,
        used_prism=used_prism,
        prism_calls=prism_calls,
        dose=dose,
        low_dose=(prism_calls > 0 and prism_calls <= 1),
        in_tokens=in_tokens,
        cost_usd=cost_usd,
    )


def _make_fake_report(precision: float, *, verdicts: list | None = None) -> InvestigatorReport:
    """Build a minimal InvestigatorReport for fake comps."""
    if verdicts is None:
        verdicts = []
    halluc = sum(v.is_hallucination for v in verdicts)
    return InvestigatorReport(
        precision=precision,
        recall=precision,
        hallucinations=halluc,
        verdicts=verdicts,
    )


class _FakeComps:
    """Injectable component bundle for run_partc_cell (no live calls).

    Both-live flow: run_off_arm supplies base citations; run_on_arm supplies on citations.
    score() returns an InvestigatorReport — base_precision on first call (off-arm) and
    on_precision on second (on-arm).
    load_base / extract_citations are REMOVED — the off-arm provides citations directly.
    """

    def __init__(self, *, on_precision: float, base_precision: float,
                 arm_out: ArmOutput, off_arm_out: ArmOutput,
                 on_verdicts: list | None = None,
                 off_verdicts: list | None = None):
        self._on_precision = on_precision
        self._base_precision = base_precision
        self._arm_out = arm_out          # prism-ON arm
        self._off_arm_out = off_arm_out  # prism-OFF arm (fresh live run)
        self._on_verdicts = on_verdicts or []
        self._off_verdicts = off_verdicts or []

    def score(self, citations, **kwargs) -> InvestigatorReport:
        # off-arm scored first (base), on-arm scored second.
        if not hasattr(self, "_score_call_count"):
            self._score_call_count = 0
        self._score_call_count += 1
        if self._score_call_count == 1:
            return _make_fake_report(self._base_precision, verdicts=self._off_verdicts)
        return _make_fake_report(self._on_precision, verdicts=self._on_verdicts)

    def run_off_arm(self, cell) -> ArmOutput:
        return self._off_arm_out

    def run_on_arm(self, cell) -> ArmOutput:
        return self._arm_out


def _fake_comps(*, on_precision: float, base_precision: float,
                prism_calls: int = 2, text: str = "clean spec, no tool names",
                citations: list | None = None,
                off_text: str = "off arm clean text src/b.go:5",
                off_citations: list | None = None,
                on_in_tokens: int = 50, on_cost_usd: float = 0.002,
                off_in_tokens: int = 30, off_cost_usd: float = 0.001,
                on_verdicts: list | None = None,
                off_verdicts: list | None = None) -> _FakeComps:
    """Build a FakeComps bundle for the both-live flow.

    on-arm: prism=True, prism_calls=prism_calls, text=text.
    off-arm: prism=False, prism_calls=0, text=off_text.
    in_tokens/cost_usd are distinct per arm so threading is actually exercised.
    """
    arm_out = _fake_arm_output(
        used_prism=(prism_calls > 0),
        prism_calls=prism_calls,
        text=text,
        citations=citations,
        in_tokens=on_in_tokens,
        cost_usd=on_cost_usd,
    )
    if off_citations is None:
        off_citations = [Citation(file="src/b.go", line=5, symbol=None)]
    off_arm_out = ArmOutput(
        variant=Variant("opus-4.8", False),
        text=off_text,
        citations=off_citations,
        tokens=8,
        tool_calls=0,
        wall_s=0.05,
        used_prism=False,
        prism_calls=0,
        dose=Dose(count=0),
        low_dose=False,
        in_tokens=off_in_tokens,
        cost_usd=off_cost_usd,
    )
    return _FakeComps(on_precision=on_precision, base_precision=base_precision,
                      arm_out=arm_out, off_arm_out=off_arm_out,
                      on_verdicts=on_verdicts, off_verdicts=off_verdicts)


# ---------------------------------------------------------------------------
# (a) Main target test — both-live flow: off-arm provides base, on-arm provides on
# ---------------------------------------------------------------------------

def test_run_partc_cell_scores_on_vs_base_with_fakes():
    """Both arms are fresh live runs; base comes from run_off_arm, on from run_on_arm."""
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(on_precision=0.8, base_precision=0.4),
    )
    assert cell.bundle_delta == pytest.approx(0.4)
    assert cell.precision_base == pytest.approx(0.4)
    assert cell.precision_on == pytest.approx(0.8)
    assert cell.dose.count >= 1
    assert not cell.leaked


# ---------------------------------------------------------------------------
# (b) 0-prism arm → administered == False
# ---------------------------------------------------------------------------

def test_run_partc_cell_zero_prism_sets_not_administered():
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(on_precision=0.8, base_precision=0.4, prism_calls=0),
    )
    assert not cell.administered, "0 real prism calls must mark administered=False"
    # bundle_delta is still computed (so caller can see the scores), but administered=False
    # signals the Verify Gate to discard/re-run this cell.


# ---------------------------------------------------------------------------
# (c) Leaked text → leaked == True
# ---------------------------------------------------------------------------

def test_run_partc_cell_leaked_when_on_arm_text_contains_nav_callers():
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=0.7,
            base_precision=0.3,
            prism_calls=2,
            text="I used nav_callers to find the issue in src/a.go:1",
        ),
    )
    assert cell.leaked, "on-arm text naming nav_callers must set leaked=True"


# ---------------------------------------------------------------------------
# (h) administered reflects the ON arm's used_prism (not the OFF arm's)
# ---------------------------------------------------------------------------

def test_run_partc_cell_administered_reflects_on_arm_used_prism():
    """administered is set from on-arm (prism=True arm); off-arm's used_prism is irrelevant."""
    # off-arm: prism=False, so used_prism=False — but ON arm has 2 prism calls → administered=True
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(on_precision=0.8, base_precision=0.4, prism_calls=2),
    )
    assert cell.administered, "administered must come from the on-arm's used_prism"


# ---------------------------------------------------------------------------
# (i) leaked reflects the ON arm's text (not the OFF arm's text)
# ---------------------------------------------------------------------------

def test_run_partc_cell_leaked_from_on_arm_not_off_arm():
    """leaked is derived from on-arm text only; a clean on-arm with a leaky off-arm → not leaked."""
    # off-arm text contains nav_callers (a leak signal), on-arm text is clean
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=0.7,
            base_precision=0.3,
            prism_calls=2,
            text="clean on-arm spec src/a.go:1",
            off_text="I used nav_callers to look at src/b.go:5",  # leaky off-arm
        ),
    )
    assert not cell.leaked, "leaked must come from the on-arm text only (off-arm leak is irrelevant)"


# ---------------------------------------------------------------------------
# (d) render_partc includes row + pilot signal label
# ---------------------------------------------------------------------------

def _make_cell(**kwargs) -> PartCCell:
    """Build a PartCCell with sensible defaults; override via kwargs."""
    defaults = dict(
        repo="ruff",
        stage="spec",
        model="opus-4.8",
        precision_on=0.8,
        precision_base=0.4,
        bundle_delta=0.4,
        dose=Dose(count=2),
        low_dose=False,
        administered=True,
        leaked=False,
        recall_on=None,
        recall_base=None,
        tokens_off=38,
        tokens_on=60,
        cost_off=0.001,
        cost_on=0.002,
        wall_off=0.05,
        wall_on=0.10,
    )
    defaults.update(kwargs)
    return PartCCell(**defaults)


def test_render_partc_includes_row_and_pilot_signal_label():
    cell = _make_cell()
    report = render_partc([cell])
    # Must contain cell identifier
    assert "ruff/spec/opus-4.8" in report or ("ruff" in report and "spec" in report)
    # Must contain precision values or delta
    assert "0.8" in report or "0.40" in report
    # Must contain "pilot signal" label
    assert "pilot signal" in report.lower()


# ---------------------------------------------------------------------------
# (e) live comps score() threads issue.text + code window to the real oracle
# ---------------------------------------------------------------------------

def _cite(file: str, line: int, symbol: str | None) -> Citation:
    """Build a Citation for test use."""
    return Citation(file=file, line=line, symbol=symbol)


def test_live_partc_comps_score_threads_issue_and_code():
    """score() must use LlmRelevanceJudge with real issue.text + code, not RelevanceAllTrue."""
    seen: dict = {}

    def fake_ask(model: str, prompt: str) -> str:
        seen.setdefault("p", prompt)
        return "YES"

    class _Co:
        """Fake pinned checkout — file/line resolution always succeeds."""
        root = "/tmp/x"

        def file_exists(self, rel: str) -> bool:
            return True

        def read_line(self, rel: str, line: int) -> str:
            # symbol "f" must appear in the line so symbol_ok passes in verify_citation
            return "def f(): ..."

        def read_window(self, rel: str, line: int) -> str:
            return "def f(): ..."

    issue = types.SimpleNamespace(
        text="ISSUE-XYZ",
        scoped_slice="s",
        repo="ruff",
        sha="deadbeef",
    )
    from tier_c.cli import _LivePartCComps
    comps = _LivePartCComps(
        co=_Co(),
        issue=issue,
        model="opus-4.8",
        base_root="x",
        ask=fake_ask,
    )
    p = comps.score(
        [_cite("a.py", 10, "f")],
        cell=("ruff", "spec", "opus-4.8"),
        arm="on",
    )
    assert "p" in seen, "fake_ask was never called — relevance judge did not fire"
    assert "ISSUE-XYZ" in seen["p"], "issue.text must appear in the relevance judge prompt"
    assert "def f()" in seen["p"], "code window must appear in the relevance judge prompt"


# ---------------------------------------------------------------------------
# (f) _upstream_spec helper (FIX 2)
# ---------------------------------------------------------------------------

def test_upstream_spec_returns_empty_for_spec_cell(tmp_path):
    """_upstream_spec must return '' for a spec cell (no upstream exists yet)."""
    import types
    from tier_c.cli import _LivePartCComps

    issue = types.SimpleNamespace(text="ISSUE", scoped_slice="s", repo="ruff", sha="abc")
    comps = _LivePartCComps(co=None, issue=issue, model="opus-4.8",
                            base_root=str(tmp_path), ask=lambda m, p: "YES")
    result = comps._upstream_spec(("ruff", "spec", "opus-4.8"))
    assert result == "", f"expected '' for spec cell, got {result!r}"


def test_upstream_spec_returns_stripped_body_for_plan_cell(tmp_path):
    """_upstream_spec for a plan cell must load and strip the recovered prism-off spec body."""
    import types
    from tier_c.cli import _LivePartCComps

    # Write a fake recovered spec file: <base_root>/<model>/<repo>/spec/<model>.md
    model, repo = "opus-4.8", "ruff"
    spec_dir = tmp_path / model / repo / "spec"
    spec_dir.mkdir(parents=True)
    spec_body = "## Spec body\nSome spec content here."
    spec_file = spec_dir / f"{model}.md"
    spec_file.write_text(f"prism=False\nsession: test\n---\n{spec_body}")

    issue = types.SimpleNamespace(text="ISSUE", scoped_slice="s", repo=repo, sha="abc")
    comps = _LivePartCComps(co=None, issue=issue, model=model,
                            base_root=str(tmp_path), ask=lambda m, p: "YES")
    result = comps._upstream_spec((repo, "plan", model))
    assert result == spec_body, f"expected spec body, got {result!r}"


# ---------------------------------------------------------------------------
# (g) score() for plan/spec cells — upstream spec threading (FIX 2)
# ---------------------------------------------------------------------------

def _make_live_comps_for_score_test(issue_text: str, upstream_spec: str, ask_fn):
    """Build a _LivePartCComps with a fake checkout and the given ask function."""
    import types
    from tier_c.cli import _LivePartCComps

    class _Co:
        root = "/tmp/x"

        def file_exists(self, rel):
            return True

        def read_line(self, rel, line):
            return "def f(): ..."

        def read_window(self, rel, line):
            return "def f(): ..."

    issue = types.SimpleNamespace(
        text=issue_text,
        scoped_slice="s",
        repo="ruff",
        sha="deadbeef",
    )
    comps = _LivePartCComps(co=_Co(), issue=issue, model="opus-4.8",
                            base_root="x", ask=ask_fn)
    # Patch _upstream_spec to avoid filesystem access
    comps._upstream_spec = lambda cell: upstream_spec if cell[1] == "plan" else ""
    return comps


def test_score_plan_cell_includes_upstream_spec_in_oracle_prompt():
    """score() for a plan cell must pass issue_text + upstream spec to the oracle."""
    seen: dict = {}

    def fake_ask(model: str, prompt: str) -> str:
        seen["prompt"] = prompt
        return "YES"

    spec_text = "## Recovered spec body\nSome detail."
    comps = _make_live_comps_for_score_test(
        issue_text="ISSUE-PLAN",
        upstream_spec=spec_text,
        ask_fn=fake_ask,
    )
    comps.score(
        [Citation(file="a.py", line=10, symbol="f")],
        cell=("ruff", "plan", "opus-4.8"),
        arm="on",
    )
    assert "ISSUE-PLAN" in seen["prompt"], "issue.text must appear in oracle prompt for plan cell"
    assert "Recovered spec body" in seen["prompt"], "upstream spec must appear in oracle prompt for plan cell"


def test_score_spec_cell_does_not_include_spec_section():
    """score() for a spec cell must NOT inject an 'Upstream spec' section."""
    seen: dict = {}

    def fake_ask(model: str, prompt: str) -> str:
        seen["prompt"] = prompt
        return "YES"

    comps = _make_live_comps_for_score_test(
        issue_text="ISSUE-SPEC",
        upstream_spec="",
        ask_fn=fake_ask,
    )
    comps.score(
        [Citation(file="a.py", line=10, symbol="f")],
        cell=("ruff", "spec", "opus-4.8"),
        arm="on",
    )
    assert "ISSUE-SPEC" in seen["prompt"]
    assert "Upstream spec" not in seen["prompt"], "spec cell must NOT inject upstream spec section"


# ---------------------------------------------------------------------------
# (j) run_partc_cell threads tokens/cost/wall from ArmOutputs into PartCCell
# ---------------------------------------------------------------------------

def test_run_partc_cell_threads_tokens_cost_wall():
    """PartCCell must carry tokens (in+out per arm), cost, and wall from each ArmOutput."""
    # on-arm: in_tokens=50, tokens(out)=10 → total=60; cost_usd=0.002; wall_s=0.10
    # off-arm: in_tokens=30, tokens(out)=8  → total=38; cost_usd=0.001; wall_s=0.05
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=0.8,
            base_precision=0.4,
            prism_calls=2,
            on_in_tokens=50,
            on_cost_usd=0.002,
            off_in_tokens=30,
            off_cost_usd=0.001,
        ),
    )
    # tokens = in + out per arm
    assert cell.tokens_off == 38, f"expected 38 (30+8), got {cell.tokens_off}"
    assert cell.tokens_on == 60, f"expected 60 (50+10), got {cell.tokens_on}"
    # cost threaded directly
    assert cell.cost_off == pytest.approx(0.001)
    assert cell.cost_on == pytest.approx(0.002)
    # wall threaded directly (off-arm ArmOutput has wall_s=0.05, on-arm has 0.1)
    assert cell.wall_off == pytest.approx(0.05)
    assert cell.wall_on == pytest.approx(0.10)


# ---------------------------------------------------------------------------
# (k) render_partc shows token + cost columns + signed Δtokens
# ---------------------------------------------------------------------------

def test_render_partc_shows_tokens_cost_and_signed_delta():
    """render_partc must include token counts, costs, and a signed Δtokens column."""
    # tokens_off=38, tokens_on=60 → Δ = +22
    cell = _make_cell(
        tokens_off=38,
        tokens_on=60,
        cost_off=0.001,
        cost_on=0.002,
    )
    report = render_partc([cell])
    # Token counts must appear
    assert "38" in report, "off-arm token count (38) must appear in report"
    assert "60" in report, "on-arm token count (60) must appear in report"
    # Signed delta: +22 (on > off)
    assert "+22" in report, "positive Δtokens (+22) must appear signed in report"
    # Cost values must appear (some form of $0.001 and $0.002)
    assert "$0.001" in report or "0.001" in report, "off-arm cost must appear in report"
    assert "$0.002" in report or "0.002" in report, "on-arm cost must appear in report"


def test_render_partc_shows_negative_delta_tokens_when_on_arm_cheaper():
    """When on-arm uses fewer tokens than off, Δtokens is negative."""
    cell = _make_cell(
        tokens_off=80,
        tokens_on=60,
        cost_off=0.003,
        cost_on=0.002,
    )
    report = render_partc([cell])
    assert "-20" in report, "negative Δtokens (-20) must appear signed in report"


# ---------------------------------------------------------------------------
# Helpers for breakdown tests — build CitationVerdict directly
# ---------------------------------------------------------------------------

def _valid_verdict(file: str, line: int) -> CitationVerdict:
    return CitationVerdict(
        cite=Citation(file=file, line=line, symbol=None),
        file_ok=True, line_ok=True, symbol_ok=True, relevant=True,
    )


def _halluc_verdict(file: str, line: int) -> CitationVerdict:
    """Structural failure: file not found."""
    return CitationVerdict(
        cite=Citation(file=file, line=line, symbol=None),
        file_ok=False, line_ok=False, symbol_ok=True, relevant=True,
    )


def _irrelevant_verdict(file: str, line: int) -> CitationVerdict:
    """Structurally ok but relevant=False."""
    return CitationVerdict(
        cite=Citation(file=file, line=line, symbol=None),
        file_ok=True, line_ok=True, symbol_ok=True, relevant=False,
    )


# ---------------------------------------------------------------------------
# (l) run_partc_cell extracts per-arm citation breakdown
# ---------------------------------------------------------------------------

def test_run_partc_cell_extracts_breakdown_from_on_report():
    """on_breakdown must reflect the verdicts returned by score() for the on-arm."""
    on_verdicts = [
        _valid_verdict("src/a.go", 1),
        _halluc_verdict("src/missing.go", 99),
        _irrelevant_verdict("src/b.go", 5),
    ]
    off_verdicts = [
        _valid_verdict("src/c.go", 10),
    ]
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=1 / 3,
            base_precision=1.0,
            prism_calls=2,
            on_verdicts=on_verdicts,
            off_verdicts=off_verdicts,
        ),
    )
    bd = cell.on_breakdown
    assert bd["n"] == 3, f"expected 3 on-verdicts, got {bd['n']}"
    assert bd["valid"] == 1, f"expected 1 valid, got {bd['valid']}"
    assert bd["halluc"] == 1, f"expected 1 halluc, got {bd['halluc']}"
    assert bd["irrelevant"] == 1, f"expected 1 irrelevant, got {bd['irrelevant']}"
    assert len(bd["fails"]) == 2, f"expected 2 fail entries, got {bd['fails']}"
    # halluc entry must mention 'halluc' label, irrelevant must mention 'irrelevant'
    fail_labels = " ".join(bd["fails"])
    assert "halluc" in fail_labels, f"halluc label missing from fails: {bd['fails']}"
    assert "irrelevant" in fail_labels, f"irrelevant label missing from fails: {bd['fails']}"


def test_run_partc_cell_extracts_breakdown_from_off_report():
    """off_breakdown must reflect the verdicts from the off-arm score() call."""
    off_verdicts = [
        _valid_verdict("src/x.go", 1),
        _halluc_verdict("src/ghost.go", 42),
    ]
    on_verdicts = [
        _valid_verdict("src/y.go", 7),
    ]
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=1.0,
            base_precision=0.5,
            prism_calls=2,
            off_verdicts=off_verdicts,
            on_verdicts=on_verdicts,
        ),
    )
    bd = cell.off_breakdown
    assert bd["n"] == 2
    assert bd["valid"] == 1
    assert bd["halluc"] == 1
    assert bd["irrelevant"] == 0
    assert len(bd["fails"]) == 1


def test_run_partc_cell_breakdown_all_valid():
    """When all verdicts are valid, fails is empty and halluc/irrelevant are 0."""
    on_verdicts = [
        _valid_verdict("src/a.go", 1),
        _valid_verdict("src/b.go", 2),
    ]
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=1.0,
            base_precision=1.0,
            prism_calls=2,
            on_verdicts=on_verdicts,
        ),
    )
    bd = cell.on_breakdown
    assert bd["n"] == 2
    assert bd["valid"] == 2
    assert bd["halluc"] == 0
    assert bd["irrelevant"] == 0
    assert bd["fails"] == []


def test_run_partc_cell_breakdown_empty_verdicts():
    """Empty verdicts (no citations scored) produces a zero-filled breakdown."""
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=0.0,
            base_precision=0.0,
            prism_calls=2,
            on_verdicts=[],
            off_verdicts=[],
        ),
    )
    bd = cell.on_breakdown
    assert bd["n"] == 0
    assert bd["valid"] == 0
    assert bd["fails"] == []


# ---------------------------------------------------------------------------
# (m) run_partc_cell threads in/out token split into PartCCell
# ---------------------------------------------------------------------------

def test_run_partc_cell_threads_in_out_token_split():
    """in_tokens_off/on and out_tokens_off/on must come from the respective ArmOutputs."""
    # on-arm: in_tokens=50, tokens(output)=10
    # off-arm: in_tokens=30, tokens(output)=8
    cell = run_partc_cell(
        _cell("ruff", "spec", "opus-4.8"),
        _fake_comps(
            on_precision=0.8,
            base_precision=0.4,
            prism_calls=2,
            on_in_tokens=50,   # ArmOutput.in_tokens for on-arm
            off_in_tokens=30,  # ArmOutput.in_tokens for off-arm
        ),
    )
    assert cell.in_tokens_on == 50, f"in_tokens_on: expected 50, got {cell.in_tokens_on}"
    assert cell.out_tokens_on == 10, f"out_tokens_on: expected 10, got {cell.out_tokens_on}"
    assert cell.in_tokens_off == 30, f"in_tokens_off: expected 30, got {cell.in_tokens_off}"
    assert cell.out_tokens_off == 8, f"out_tokens_off: expected 8, got {cell.out_tokens_off}"
    # tokens_off/on (totals) must still be correct
    assert cell.tokens_off == 38
    assert cell.tokens_on == 60


# ---------------------------------------------------------------------------
# (n) render_partc shows in/out split line and cite-breakdown lines
# ---------------------------------------------------------------------------

def _make_cell_with_breakdown(**kwargs) -> PartCCell:
    """PartCCell with breakdown + in/out split fields set."""
    defaults = dict(
        repo="ruff",
        stage="spec",
        model="opus-4.8",
        precision_on=0.8,
        precision_base=0.4,
        bundle_delta=0.4,
        dose=Dose(count=2),
        low_dose=False,
        administered=True,
        leaked=False,
        recall_on=None,
        recall_base=None,
        tokens_off=38,
        tokens_on=60,
        in_tokens_off=30,
        out_tokens_off=8,
        in_tokens_on=50,
        out_tokens_on=10,
        cost_off=0.001,
        cost_on=0.002,
        wall_off=0.05,
        wall_on=0.10,
        off_breakdown={"n": 1, "valid": 1, "halluc": 0, "irrelevant": 0, "fails": []},
        on_breakdown={"n": 3, "valid": 1, "halluc": 1, "irrelevant": 1,
                      "fails": ["src/missing.go:99 (halluc)", "src/b.go:5 (irrelevant)"]},
    )
    defaults.update(kwargs)
    return PartCCell(**defaults)


def test_render_partc_shows_in_out_token_split():
    """render_partc must include a tokens line with in/out split values for each arm."""
    cell = _make_cell_with_breakdown(in_tokens_off=30, out_tokens_off=8,
                                     in_tokens_on=50, out_tokens_on=10)
    report = render_partc([cell])
    # The in/out values must appear somewhere in the report
    assert "30" in report, "off-arm in_tokens (30) must appear in report"
    assert "50" in report, "on-arm in_tokens (50) must appear in report"
    # Output tokens too
    assert "8" in report or "out" in report.lower(), "off-arm out_tokens (8) must appear"
    assert "10" in report, "on-arm out_tokens (10) must appear"


def test_render_partc_shows_cite_breakdown_lines():
    """render_partc must include lines with valid/halluc/irrelevant counts for each arm."""
    cell = _make_cell_with_breakdown(
        off_breakdown={"n": 1, "valid": 1, "halluc": 0, "irrelevant": 0, "fails": []},
        on_breakdown={"n": 3, "valid": 1, "halluc": 1, "irrelevant": 1,
                      "fails": ["src/missing.go:99 (halluc)", "src/b.go:5 (irrelevant)"]},
    )
    report = render_partc([cell])
    # breakdown keywords must appear
    assert "valid" in report.lower(), "render must mention 'valid' cite counts"
    assert "halluc" in report.lower(), "render must mention 'halluc' cite counts"
    assert "irrelevant" in report.lower(), "render must mention 'irrelevant' cite counts"
    # numeric counts must appear
    assert "1" in report  # valid count appears in both breakdowns


def test_render_partc_breakdown_with_zero_verdicts():
    """Cells with empty verdicts render without errors (n=0 arms)."""
    cell = _make_cell_with_breakdown(
        off_breakdown={"n": 0, "valid": 0, "halluc": 0, "irrelevant": 0, "fails": []},
        on_breakdown={"n": 0, "valid": 0, "halluc": 0, "irrelevant": 0, "fails": []},
    )
    report = render_partc([cell])
    assert "pilot signal" in report.lower()
