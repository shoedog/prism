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


# ---- Task: lenient citation path resolution (basename resolve) ----

class _ResolvingCo:
    """FakeCo that exposes resolve_rel (simulating unique-basename resolution).

    resolve_rel("regexp.go")         -> "model/labels/regexp.go"  (unique basename)
    resolve_rel("parser.go")         -> None                       (ambiguous)
    resolve_rel("no_such.go")        -> None                       (absent)
    resolve_rel("model/labels/regexp.go") -> "model/labels/regexp.go"  (exact)

    read_line and read_window answer for the resolved path only.
    """
    _RESOLVED_TEXT = "var re = regexp.MustCompile"

    def resolve_rel(self, rel: str) -> str | None:
        mapping = {
            "regexp.go": "model/labels/regexp.go",
            "model/labels/regexp.go": "model/labels/regexp.go",
        }
        return mapping.get(rel, None)

    def file_exists(self, rel: str) -> bool:
        # fallback used by old hasattr path — not exercised when resolve_rel present
        return rel == "model/labels/regexp.go"

    def read_line(self, rel: str, line: int) -> str | None:
        if rel == "model/labels/regexp.go" and line == 76:
            return self._RESOLVED_TEXT
        return None

    def read_window(self, rel: str, line: int, ctx: int = 3) -> str | None:
        return self.read_line(rel, line)


def test_lenient_basename_cite_not_hallucination():
    """A basename-only cite that uniquely resolves is NOT a hallucination."""
    co = _ResolvingCo()
    cite = Citation("regexp.go", 76, "MustCompile")
    v = verify_citation(co, cite)
    assert v.file_ok, "basename resolved → file_ok must be True"
    assert v.line_ok, "resolved file has line 76 → line_ok must be True"
    assert v.symbol_ok, "'MustCompile' appears on line 76 → symbol_ok True"
    assert not v.is_hallucination


def test_ambiguous_basename_is_hallucination():
    """An ambiguous basename (resolve_rel returns None) is a hallucination."""
    co = _ResolvingCo()
    cite = Citation("parser.go", 10, "Parse")
    v = verify_citation(co, cite)
    assert not v.file_ok
    assert v.is_hallucination


def test_absent_basename_is_hallucination():
    """A non-existent basename is a hallucination."""
    co = _ResolvingCo()
    cite = Citation("no_such.go", 1, "Foo")
    v = verify_citation(co, cite)
    assert not v.file_ok
    assert v.is_hallucination


def test_lenient_cite_code_uses_resolved_path():
    """The oracle receives the RESOLVED path's code, not cite.file."""
    code_calls: list[tuple[str, int | None]] = []

    def record_read_code(file: str, line: int | None) -> str:
        code_calls.append((file, line))
        return "resolved code"

    co = _ResolvingCo()
    cite = Citation("regexp.go", 76, "MustCompile")
    v = verify_citation(co, cite, relevance=RelevanceAllTrue(),
                        read_code=record_read_code)
    # read_code must be called with the RESOLVED path, not "regexp.go"
    assert code_calls == [("model/labels/regexp.go", 76)]


def test_exact_path_still_works_with_resolve_rel():
    """Full repo-relative path still resolves correctly via resolve_rel."""
    co = _ResolvingCo()
    cite = Citation("model/labels/regexp.go", 76, "MustCompile")
    v = verify_citation(co, cite)
    assert v.file_ok and v.line_ok and v.symbol_ok
    assert not v.is_hallucination


def test_old_fake_co_without_resolve_rel_unbroken():
    """FakeCo without resolve_rel still works via hasattr fallback."""
    # FakeCo (defined at top of file) has no resolve_rel
    v = verify_citation(FakeCo(), Citation("a.py", 1, "foo"))
    assert v.file_ok and v.line_ok and v.symbol_ok
    assert not v.is_hallucination

    v2 = verify_citation(FakeCo(), Citation("ghost.py", 1, "x"))
    assert v2.is_hallucination
