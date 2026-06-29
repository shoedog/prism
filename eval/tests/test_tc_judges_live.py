from tier_c.model import Citation
from tier_c.judges_live import LlmRankJudge, LlmRelevanceJudge, LlmConditionGuesser, _RecordingRelevanceJudge, SpecQualityJudge

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

def test_condition_guesser_false_on_no_and_hedge():
    assert LlmConditionGuesser(ask=lambda m,p: "NO", model="opus-4.8").guess_used_prism("x") is False
    assert LlmConditionGuesser(ask=lambda m,p: "Probably not", model="opus-4.8").guess_used_prism("x") is False

def _cite(file, line, symbol):
    return Citation(file, line, symbol)

def test_relevance_prompt_includes_issue_and_code():
    seen = {}
    def ask(m, p):
        seen["p"] = p
        return "YES"
    j = LlmRelevanceJudge(ask=ask, model="opus-4.8")
    assert j.is_relevant(cite=_cite("a.py", 10, "f"), issue_text="ISSUE-XYZ", code="def f(): ...") is True
    assert "ISSUE-XYZ" in seen["p"] and "def f()" in seen["p"]


def _rcite():
    return Citation(file="model/labels/regexp.go", line=96, symbol=None)


def test_relevance_ensemble_yes_when_both_sonnet_yes(monkeypatch):
    j = LlmRelevanceJudge(lambda m, p: "YES because it installs trueMatcher")
    ev = j.relevance(_rcite(), "issue", "code")
    assert ev.verdict == "YES" and ev.escalated is False and len(ev.votes) == 2
    assert j.is_relevant(_rcite(), "issue", "code") is True


def test_relevance_ensemble_escalates(monkeypatch):
    seq = iter(["YES x", "NO y"])
    def ask(m, p):
        return "NO opus" if m == "opus-4.8" else next(seq)
    j = LlmRelevanceJudge(ask)
    ev = j.relevance(_rcite(), "issue", "code")
    assert ev.escalated is True and ev.verdict == "NO"


def test_recording_relevance_captures_votes():
    records = []
    inner = LlmRelevanceJudge(lambda m, p: "YES it is the root cause")
    rec = _RecordingRelevanceJudge(inner, records)
    assert rec.is_relevant(_rcite(), "issue", "code") is True
    r = records[0]
    assert r["file"] == "model/labels/regexp.go" and r["line"] == 96
    assert r["relevant"] is True and r["escalated"] is False
    assert len(r["votes"]) == 2 and "reason" in r["votes"][0]


def test_spec_quality_deterministic_assignment():
    """Same (off,on) inputs -> same A/B assignment (rescore reproducibility)."""
    j = SpecQualityJudge(lambda m, p: "A is better")
    r1 = j.compare("issue", "OFF spec body", "ON spec body")
    r2 = j.compare("issue", "OFF spec body", "ON spec body")
    assert r1["swap"] == r2["swap"]


def test_spec_quality_winner_maps_back_to_arm():
    calls = {}
    def ask(m, p):
        calls["prompt"] = p
        return "A wins, clearer root cause"
    j = SpecQualityJudge(ask)
    r = j.compare("issue", "OFF", "ON")
    a_is = "on" if r["swap"] else "off"
    assert r["winner"] == a_is


def test_spec_quality_tie():
    j = SpecQualityJudge(lambda m, p: "TIE, both miss it")
    r = j.compare("issue", "OFF", "ON")
    assert r["winner"] == "tie" and r["escalated"] is False


def test_spec_quality_escalates_on_split():
    seq = iter(["A better", "B better"])
    def ask(m, p):
        return "TIE final" if m == "opus-4.8" else next(seq)
    r = SpecQualityJudge(ask).compare("issue", "OFF", "ON")
    assert r["escalated"] is True and r["winner"] == "tie"
