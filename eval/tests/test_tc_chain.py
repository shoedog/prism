# eval/tests/test_tc_chain.py
from tier_c.model import Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.planted import PlantedError
from tier_c.investigator import RelevanceAllTrue
from tier_c.chain import run_stage, StageResult, run_spec_plan_chain

class FakeCo:
    root = "."
    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line): return "x" if rel == "a.py" else None
    def read_window(self, rel, line): return "x" if rel == "a.py" else None

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


def test_run_stage_blinds_variant_ids_from_judges():
    seen = {}
    class RecordingRank:
        def rank(self, stage, rubric, candidates):
            seen.update(candidates)
            return sorted(candidates, key=lambda k: -len(candidates[k]))
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "longer spec a.py:1", "gpt-5.5": "x"})
    run_stage(stage="spec", variants=variants, runner=runner, co=FakeCo(),
              prompt="p", repo_root="/r", claim_counts={v.id: 1 for v in variants},
              plants=[], judges={"anthropic": RecordingRank(), "openai": RecordingRank()},
              relevance=RelevanceAllTrue())
    assert seen, "judges saw no candidates"
    assert all(k.startswith("cand") for k in seen)
    assert all(("prism" not in k and "opus" not in k and "gpt" not in k) for k in seen)

def test_chain_salts_frames_when_plants_present():
    from tier_c.planted import PlantedError
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec body a.py:1", "gpt-5.5": "x"})
    captured = {}
    def fake_prompt(stage, *, issue_text, scoped_slice, upstream=""):
        captured[stage] = (issue_text, upstream); return "p"
    run_spec_plan_chain(issue_text="bug", scoped_slice="s1", variants=variants, runner=runner,
        co=FakeCo(), claim_counts={v.id: 1 for v in variants},
        plants=[PlantedError("file", "ghostref")],
        judges={"anthropic": FakeRank(), "openai": FakeRank()},
        relevance=RelevanceAllTrue(), prompt_fn=fake_prompt)
    assert "ghostref" in captured["spec"][0]   # spec issue frame was salted
    assert "ghostref" in captured["plan"][1]   # plan upstream (cleaned spec) was salted


class _RecordingRelevance:
    """Records every (issue_text, code) pair seen by is_relevant."""
    def __init__(self):
        self.calls: list[tuple[str, str]] = []

    def is_relevant(self, cite, issue_text, code: str = "") -> bool:
        self.calls.append((issue_text, code))
        return True


class FakeCoWithWindow:
    """FakeCo that also implements read_window (Task 1 seam)."""
    root = "."

    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line): return "x" if rel == "a.py" else None
    def read_window(self, f, l): return f"CODE@{f}:{l}"


def test_chain_passes_issue_and_code_to_oracle():
    """Task 5: score_citations must receive issue_text + read_code.

    Spec stage: relevance sees issue text only (no spec upstream).
    Plan stage: relevance sees issue text + '## Upstream spec' section.
    read_code is bound to co.read_window so code snippets reach the oracle.
    """
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    # Both variants cite a.py:1 so is_relevant is actually called
    runner = FakeArmRunner({
        "opus-4.8+prism": "spec body a.py:1",
        "gpt-5.5": "also a.py:1",
    })
    rec = _RecordingRelevance()
    co = FakeCoWithWindow()

    def fake_prompt(stage, *, issue_text, scoped_slice, upstream=""):
        return "p"

    run_spec_plan_chain(
        issue_text="fix the crash",
        scoped_slice="slice1",
        variants=variants,
        runner=runner,
        co=co,
        claim_counts={v.id: 1 for v in variants},
        plants=[],
        judges={"anthropic": FakeRank(), "openai": FakeRank()},
        relevance=rec,
        prompt_fn=fake_prompt,
    )

    assert rec.calls, "is_relevant was never called — citations not reaching oracle"

    # Partition calls by stage: spec calls come first (2 variants × cites),
    # plan calls follow. We need at least one spec call and one plan call.
    # Easier: check that some calls have the issue text only,
    # and some have BOTH issue text AND spec upstream marker.
    spec_calls = [c for c in rec.calls if "## Upstream spec" not in c[0]]
    plan_calls = [c for c in rec.calls if "## Upstream spec" in c[0]]

    assert spec_calls, "spec stage produced no relevance calls"
    assert plan_calls, "plan stage produced no relevance calls"

    # (a) spec-stage calls saw the issue text
    for issue_seen, code_seen in spec_calls:
        assert "fix the crash" in issue_seen, f"issue text missing from spec call: {issue_seen!r}"
        # code comes from co.read_window, which returns "CODE@<f>:<l>"
        assert "CODE@" in code_seen, f"read_code not wired: code={code_seen!r}"

    # (b) plan-stage calls saw issue + spec text in the composed context
    for issue_seen, code_seen in plan_calls:
        assert "fix the crash" in issue_seen
        assert "## Upstream spec" in issue_seen
        assert "spec body" in issue_seen  # the cleaned best from the spec stage
        assert "CODE@" in code_seen
