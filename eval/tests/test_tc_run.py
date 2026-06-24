# eval/tests/test_tc_run.py
from tier_c.model import Issue, Variant
from tier_c.arm_runner import FakeArmRunner
from tier_c.investigator import RelevanceAllTrue
from tier_c.run import run_issue

class FakeCo:
    root = "."
    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line): return "x" if rel == "a.py" else None

class FakeRank:
    def rank(self, stage, rubric, candidates): return sorted(candidates, key=lambda k: -len(candidates[k]))

def test_run_issue_returns_chain_with_two_stages():
    issue = Issue("k","python","r","sha","u","bug text","slice 1")
    variants = [Variant("opus-4.8", True), Variant("gpt-5.5", False)]
    runner = FakeArmRunner({"opus-4.8+prism": "spec a.py:1 with detail", "gpt-5.5": "x"})
    res = run_issue(issue, variants=variants, runner=runner, co=FakeCo(),
                    judges={"anthropic": FakeRank(), "openai": FakeRank()},
                    relevance=RelevanceAllTrue(), plants=[])
    assert [s.stage for s in res.stages] == ["spec", "plan"]
    assert res.provenance.spec_best in {v.id for v in variants}
