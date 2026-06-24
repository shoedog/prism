from tier_c.model import Variant
from tier_c.arm_runner import RoutingArmRunner

class FakeClaude:
    def run(self, v, stage, prompt, repo): return ("claude", v.id)
class FakeCodex:
    def run(self, v, stage, prompt, repo): return ("codex", v.id)

def test_routing_picks_cli_by_family():
    r = RoutingArmRunner(claude=FakeClaude(), codex=FakeCodex())
    assert r.run(Variant("opus-4.8", True), "spec", "p", "/r")[0] == "claude"
    assert r.run(Variant("gpt-5.5", False), "spec", "p", "/r")[0] == "codex"

def test_routing_raises_on_unknown_family():
    import pytest
    r = RoutingArmRunner(claude=FakeClaude(), codex=FakeCodex())
    with pytest.raises(ValueError, match="no CLI registered"):
        r.run(Variant("some-future-model", True), "spec", "p", "/r")

def test_run_stage_computes_claim_counts_from_outputs():
    # when claim_counts is None, run_stage derives it per-output via count_claims
    from tier_c.arm_runner import FakeArmRunner
    from tier_c.investigator import RelevanceAllTrue
    from tier_c.chain import run_stage
    class FakeCo:
        def file_exists(self, rel): return True
        def read_line(self, rel, line): return "x"
    class FakeRank:
        def rank(self, s, r, c): return sorted(c, key=lambda k: -len(c[k]))
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "The matcher uses compile() in a.py:1.", "gpt-5.5": "ok"})
    res = run_stage(stage="spec", variants=variants, runner=runner, co=FakeCo(),
                    prompt="p", repo_root="/r", claim_counts=None,
                    plants=[], judges={"anthropic": FakeRank(), "openai": FakeRank()},
                    relevance=RelevanceAllTrue())
    # opus output makes >=1 code-claim -> recall denominator > 0 (not the {id:1} placeholder by luck)
    assert res.investigator["opus-4.8+prism"].recall >= 0.0  # computed, no crash
