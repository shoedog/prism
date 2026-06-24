from tier_c.model import Citation
from tier_c.judges_live import LlmRankJudge, LlmRelevanceJudge, LlmConditionGuesser

def test_rank_judge_parses_permutation():
    j = LlmRankJudge(ask=lambda m, p: "cand2, cand0, cand1", model="opus-4.8")
    order = j.rank("spec", "rubric", {"cand0":"a","cand1":"b","cand2":"c"})
    assert order == ["cand2","cand0","cand1"]

def test_rank_judge_repairs_missing_labels():
    # model omits cand1 -> appended in input order so result is always a full permutation
    j = LlmRankJudge(ask=lambda m, p: "cand2,cand0", model="opus-4.8")
    order = j.rank("spec", "r", {"cand0":"a","cand1":"b","cand2":"c"})
    assert sorted(order) == ["cand0","cand1","cand2"]
    assert order[:2] == ["cand2","cand0"]

def test_relevance_judge_yes_no():
    yes = LlmRelevanceJudge(ask=lambda m,p: "YES, clearly relevant", model="opus-4.8")
    no = LlmRelevanceJudge(ask=lambda m,p: "No, unrelated", model="opus-4.8")
    assert yes.is_relevant(Citation("a.py",1,"f"), "issue") is True
    assert no.is_relevant(Citation("a.py",1,"f"), "issue") is False

def test_condition_guesser_returns_bool():
    g = LlmConditionGuesser(ask=lambda m,p: "YES", model="opus-4.8")
    assert g.guess_used_prism("some output text") is True
