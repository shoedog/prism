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
