# eval/tests/test_tc_chain.py
from tier_c.model import Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.planted import PlantedError
from tier_c.investigator import RelevanceAllTrue
from tier_c.chain import run_stage, StageResult, run_spec_plan_chain

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
