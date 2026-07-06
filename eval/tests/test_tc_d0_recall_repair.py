"""D0 — recall-denominator repair (cli.py _LivePartCComps.score).

Bug (pre-fix): cli.py passed claim_count=max(len(citations), 1) into
score_citations (investigator.py), which makes recall = valid/claim_count
IDENTICALLY equal to precision whenever the arm made >=1 citation — the
under-citing penalty (spec §6a) was silently disabled, and Part-C never
surfaced recall as an independent axis (run_partc_cell hardcoded
recall_on=None / recall_base=None even though InvestigatorReport.recall was
already computed).

Fix: thread the arm TEXT (self._last_off / self._last_on, populated by
run_off_arm / run_on_arm before score() is called) through
claims.count_claims() as the recall denominator, and surface the real
recall_on/recall_base on PartCCell instead of discarding them.
"""
from __future__ import annotations

import types

import pytest

from tier_c.cli import _LivePartCComps
from tier_c.claims import count_claims
from tier_c.model import ArmOutput, Citation, Dose, Variant
from tier_c.partc import run_partc_cell


class _Co:
    """Fake pinned checkout: every cite resolves; read_line/read_window are non-empty."""

    root = "/tmp/x"

    def file_exists(self, rel: str) -> bool:
        return True

    def resolve_rel(self, rel: str) -> str:
        return rel

    def read_line(self, rel: str, line: int) -> str:
        return "def f(): ..."

    def read_window(self, rel: str, line: int) -> str:
        return "def f(): ..."


_ARM_TEXT = (
    "The root cause is in parse_config(). "
    "It calls load_file() which reads config.py before validating input. "
    "The db_conn handle is never closed after use, leaking a file descriptor. "
    "process_data() mutates the shared_state global unexpectedly. "
    "Finally emit_metrics() logs an incomplete result to metrics.log."
)


def _issue() -> types.SimpleNamespace:
    return types.SimpleNamespace(text="ISSUE", scoped_slice="s", repo="ruff", sha="abc")


def _batch_yes_ask(calls: list):
    """Batch judging (perf fix): score() now classifies ALL citations for an arm in
    ONE relevance_batch call instead of one ensemble call per citation, so a fake ask
    must answer every numbered "--- Item #n ---" block, not just return a bare "YES".
    `calls` collects every prompt seen so tests can assert the call count."""
    def ask(model: str, prompt: str) -> str:
        calls.append(prompt)
        n = prompt.count("--- Item #")
        return "\n".join(f"#{i} YES" for i in range(1, n + 1)) if n else "YES"
    return ask


def test_fixture_text_has_several_substantive_code_claims():
    """Sanity check on the fixture itself — must have >1 code claim so the repair is
    actually exercised (a single-claim text can't distinguish old vs new behaviour)."""
    n = count_claims(_ARM_TEXT)
    assert n >= 4, f"fixture text must assert several code claims, got count_claims={n}"


def test_score_recall_uses_claim_count_not_citation_count():
    """The core D0 assertion: an arm with N substantive code claims but only 2 valid
    citations must show recall ~= 2/N, NOT 1.0 (which is what max(len(citations),1)
    as the denominator would produce — recall collapsing onto precision)."""
    n_claims = count_claims(_ARM_TEXT)
    cites = [Citation("a.py", 1, "f"), Citation("b.py", 2, "f")]

    calls: list = []
    comps = _LivePartCComps(co=_Co(), issue=_issue(), model="opus-4.8", base_root="x",
                            ask=_batch_yes_ask(calls))
    comps._last_on = ArmOutput(
        variant=Variant("opus-4.8", True), text=_ARM_TEXT, citations=cites,
        tokens=10, tool_calls=2, wall_s=0.0, used_prism=True, prism_calls=2,
        dose=Dose(count=2), low_dose=False,
    )

    rep = comps.score(cites, cell=("ruff", "spec", "opus-4.8"), arm="on")

    assert len(calls) <= 1, (
        f"batch judging: relevance for both citations must cost <=1 ask call, got {len(calls)}"
    )
    assert rep.precision == 1.0, "both citations resolve + judge YES -> precision 1.0"
    expected_recall = 2 / n_claims
    assert rep.recall == pytest.approx(expected_recall), (
        f"recall must be valid/count_claims(text) = 2/{n_claims} = {expected_recall}, "
        f"got {rep.recall} — looks like claim_count regressed to max(len(citations),1)"
    )
    assert rep.recall < rep.precision, (
        "under-citing penalty must be VISIBLE: recall must differ from (and be lower "
        "than) precision once the claim text asserts more than it cites"
    )


def test_score_falls_back_to_old_behavior_when_no_arm_text_set():
    """Back-compat: a direct caller that never ran run_off_arm/run_on_arm (self._last_off/
    self._last_on stay None) keeps the pre-fix max(len(citations),1) denominator so
    existing direct-score() callers/tests are unaffected."""
    cites = [Citation("a.py", 1, "f")]
    comps = _LivePartCComps(co=_Co(), issue=_issue(), model="opus-4.8", base_root="x",
                            ask=lambda m, p: "YES")
    rep = comps.score(cites, cell=("ruff", "spec", "opus-4.8"), arm="on")
    assert rep.recall == rep.precision == 1.0


def test_run_partc_cell_surfaces_real_recall_not_none():
    """run_partc_cell must populate recall_on/recall_base from the InvestigatorReport —
    previously these were hardcoded to None even though comps.score() already computed
    a real recall value (owner rubric-a: precision and recall are separate axes)."""
    from tier_c.investigator import InvestigatorReport

    def _arm(prism: bool) -> ArmOutput:
        return ArmOutput(
            variant=Variant("opus-4.8", prism), text="spec text",
            citations=[Citation("a.go", 1, None)], tokens=10, tool_calls=0,
            wall_s=1.0, used_prism=prism, prism_calls=1 if prism else 0,
            dose=Dose(count=1 if prism else 0), low_dose=prism, in_tokens=5,
            cost_usd=0.0, raw_stdout="",
        )

    class _Comps:
        def run_off_arm(self, cell):
            return _arm(False)

        def run_on_arm(self, cell):
            return _arm(True)

        def score(self, citations, *, cell, arm):
            r = 0.3 if arm == "base" else 0.6
            return InvestigatorReport(precision=1.0, recall=r, hallucinations=0, verdicts=[])

    cell = run_partc_cell(("ruff", "spec", "opus-4.8"), _Comps())
    assert cell.recall_base == 0.3, f"recall_base must come from off_rep.recall, got {cell.recall_base!r}"
    assert cell.recall_on == 0.6, f"recall_on must come from on_rep.recall, got {cell.recall_on!r}"


def test_rescore_cached_comps_threads_arm_text_into_recall(monkeypatch):
    """The rescore path builds a _CachedComps wrapper around _LivePartCComps that returns
    pre-built ArmOutputs without ever calling run_off_arm/run_on_arm — so live._last_off/
    live._last_on must be set by the wrapper itself, or the D0 fix silently reverts to the
    old max(len(citations),1) fallback under rescore (the ONE path this whole rubric-v2
    pass must stay rescorable through)."""
    from tier_c.rescore import rescore_cell

    class _FakeIssue:
        repo = "ruff"
        sha = "abc"
        text = "ISSUE"
        scoped_slice = "s"
        language = "python"

    off = ArmOutput(variant=Variant("opus-4.8", False), text="off body no citations claims",
                    citations=[], tokens=1, tool_calls=0, wall_s=0.0, used_prism=False)
    on = ArmOutput(
        variant=Variant("opus-4.8", True), text=_ARM_TEXT,
        citations=[Citation("a.py", 1, "f"), Citation("b.py", 2, "f")],
        tokens=10, tool_calls=2, wall_s=0.0, used_prism=True, prism_calls=2,
        dose=Dose(count=2), low_dose=False,
    )

    calls: list = []
    cell, _, _ = rescore_cell(
        ("ruff", "spec", "opus-4.8"), off_out=off, on_out=on, co=_Co(),
        issue=_FakeIssue(), base_root="", ask=_batch_yes_ask(calls),
    )
    n_claims = count_claims(_ARM_TEXT)
    assert cell.recall_on == pytest.approx(2 / n_claims), (
        "rescore must recompute recall from the ON arm's real text via count_claims — "
        f"got {cell.recall_on!r} (n_claims={n_claims})"
    )
