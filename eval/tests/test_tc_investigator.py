from tier_c.model import Citation
from tier_c.investigator import verify_citation, score_citations, RelevanceAllTrue, RelevanceNone

class FakeCo:
    def file_exists(self, rel): return rel == "a.py"
    def read_line(self, rel, line):
        return "def foo():" if (rel == "a.py" and line == 1) else (None if rel == "a.py" else None)

def test_verify_existing_symbol_line():
    v = verify_citation(FakeCo(), Citation("a.py", 1, "foo"))
    assert v.file_ok and v.line_ok and v.symbol_ok

def test_verify_nonexistent_file_is_hallucination():
    v = verify_citation(FakeCo(), Citation("ghost.py", 1, "x"))
    assert not v.file_ok and v.is_hallucination

def test_symbol_not_on_line_fails_symbol():
    v = verify_citation(FakeCo(), Citation("a.py", 1, "bar"))
    assert v.file_ok and v.line_ok and not v.symbol_ok

def test_precision_recall_penalize_undercite():
    # 1 valid citation, but 3 substantive claims -> recall 1/3 (under-citing penalized)
    cites = [Citation("a.py", 1, "foo")]
    rep = score_citations(FakeCo(), cites, claim_count=3, relevance=RelevanceAllTrue())
    assert rep.precision == 1.0
    assert abs(rep.recall - 1/3) < 1e-9
    assert rep.hallucinations == 0

def test_precision_counts_irrelevant_against():
    cites = [Citation("a.py", 1, "foo")]
    rep = score_citations(FakeCo(), cites, claim_count=1, relevance=RelevanceNone())
    assert rep.precision == 0.0  # exists but judged irrelevant

# ---- Task 4: read_code threading ----

def _cite(file, line, symbol):
    return Citation(file, line, symbol)

class _FullCo:
    """FakeCo variant that confirms any file/line/symbol as existing."""
    def file_exists(self, rel): return True
    def read_line(self, rel, line): return "f"  # symbol check: "f" in "f" -> True

co_fake = _FullCo()

class _RecordingRelevance:
    """Records (issue_text, code) for each is_relevant call."""
    def __init__(self, calls):
        self._calls = calls
    def is_relevant(self, cite, issue_text, code: str = ""):
        self._calls.append((issue_text, code))
        return True

def test_score_citations_threads_code_to_relevance():
    calls = []
    judge = _RecordingRelevance(calls)
    rep = score_citations(co_fake, [_cite("a.py", 10, "f")], claim_count=1,
                          relevance=judge, issue_text="I", read_code=lambda f, l: "CODE@%s:%s" % (f, l))
    assert calls == [("I", "CODE@a.py:10")]
