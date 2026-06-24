# eval/tests/test_tc_run_live.py
"""Task 5: run_live + Report + LiveComponents (fakes only — NO live calls)."""
from tier_c.model import Issue, Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.investigator import RelevanceAllTrue
from tier_c.run import run_live, LiveComponents, Report


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


def _make_comps(variants, runner):
    return LiveComponents(
        variants=variants,
        runner=runner,
        judges={"anthropic": FakeRank(), "openai": FakeRank()},
        relevance=RelevanceAllTrue(),
        guesser=FakeGuess(),
        plants=[],
        open_checkout=lambda repo, sha: FakeCo(),
    )


def test_run_live_produces_report_cells():
    issues = [Issue("ruff-1", "rust", "ruff", "sha", "u", "bug: matcher in a.py:1 wrong", "slice")]
    variants = [Variant("opus-4.8", True), Variant("opus-4.8", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec a.py:1 detailed", "opus-4.8": "spec"})
    comps = _make_comps(variants, runner)
    report = run_live(issues, comps)
    assert ("spec", "rust") in report.cells
    assert ("plan", "rust") in report.cells
    assert report.cells[("spec", "rust")].stage == "spec"


def test_run_live_returns_report_type():
    issues = [Issue("ruff-1", "rust", "ruff", "sha", "u", "bug: matcher in a.py:1 wrong", "slice")]
    variants = [Variant("opus-4.8", True), Variant("opus-4.8", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec a.py:1 detailed", "opus-4.8": "spec"})
    comps = _make_comps(variants, runner)
    report = run_live(issues, comps)
    assert isinstance(report, Report)
    assert report.detectability is not None


def test_run_live_pooled_detectability_count():
    """n == variants × stages × issues — proves pooling, not per-stage."""
    issues = [
        Issue("ruff-1", "rust", "ruff", "sha1", "u", "bug a.py:1", "s1"),
        Issue("ruff-2", "rust", "ruff", "sha2", "u", "bug a.py:2", "s2"),
    ]
    variants = [Variant("opus-4.8", True), Variant("opus-4.8", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec a.py:1 detailed", "opus-4.8": "spec"})
    comps = _make_comps(variants, runner)
    report = run_live(issues, comps)
    # 2 variants × 2 stages × 2 issues = 8
    assert report.detectability.n == len(variants) * 2 * len(issues)


def test_run_live_cell_language_from_issue():
    """Cell language matches the issue's language field."""
    issues = [Issue("go-1", "go", "caddy", "sha", "u", "bug a.go:1", "slice")]
    variants = [Variant("opus-4.8", True), Variant("opus-4.8", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec a.go:1 detailed", "opus-4.8": "spec"})
    comps = _make_comps(variants, runner)
    report = run_live(issues, comps)
    assert ("spec", "go") in report.cells
    assert ("plan", "go") in report.cells


def test_run_live_stage_outputs_pooled_from_stageresult():
    """Pool is built from stage_result.outputs, not reconstructed separately."""
    issues = [Issue("ruff-1", "rust", "ruff", "sha", "u", "bug a.py:1", "slice")]
    variants = [Variant("opus-4.8", True), Variant("opus-4.8", False),
                Variant("gpt-5.5", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({
        "opus-4.8+prism": "spec a.py:1 detailed long",
        "opus-4.8": "spec",
        "gpt-5.5+prism": "spec a.py:1",
        "gpt-5.5": "x",
    })
    comps = _make_comps(variants, runner)
    report = run_live(issues, comps)
    # 4 variants × 2 stages × 1 issue = 8
    assert report.detectability.n == 4 * 2 * 1
