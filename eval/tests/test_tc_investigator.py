from tier_c.model import Citation
from tier_c.investigator import (
    CitationVerdict,
    RelevanceAllTrue,
    RelevanceNone,
    resolvability_breakdown,
    score_citations,
    verify_citation,
)

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


# ---------------------------------------------------------------------------
# resolver-fix-spec.md R3 — three-way classification (RESOLVED/AMBIGUOUS/ABSENT via
# Checkout.resolve_cite): valid / hallucinated / ambiguous are now mutually exclusive.
# ---------------------------------------------------------------------------

class _ResolveCiteCo:
    """FakeCo exposing resolve_cite directly (bypassing git), so these tests exercise
    verify_citation's classification logic in isolation from Checkout's git plumbing
    (that integration is covered by test_tc_checkout.py's noqa.rs fixtures)."""

    def __init__(self, result):
        self._result = result

    def resolve_cite(self, file, line=None, symbol=None, claim_text="", *, disambiguate=None):
        return self._result

    def read_line(self, rel, line):
        return "fn handle() {}" if rel == "real/noqa.rs" else None

    def read_window(self, rel, line, ctx=3):
        return self.read_line(rel, line)


def test_absent_status_scores_hallucinated_not_ambiguous():
    from tier_c.checkout import ABSENT, ResolveResult
    co = _ResolveCiteCo(ResolveResult(status=ABSENT, path=None))
    v = verify_citation(co, Citation("noqa.rs", 99999, None))
    assert v.is_hallucination is True
    assert v.is_ambiguous is False
    assert v.is_valid is False


def test_ambiguous_status_scores_ambiguous_not_hallucinated():
    from tier_c.checkout import AMBIGUOUS, ResolveResult
    co = _ResolveCiteCo(ResolveResult(status=AMBIGUOUS, path=None))
    v = verify_citation(co, Citation("noqa.rs", 10, None))
    assert v.is_ambiguous is True, "AMBIGUOUS status must set is_ambiguous"
    assert v.is_hallucination is False, "AMBIGUOUS must NEVER be scored as hallucination"
    assert v.is_valid is False, "AMBIGUOUS is also excluded from valid (unpinnable)"


def test_resolved_status_bare_but_real_scores_valid():
    """THE ARTIFACT FIX, at the verify_citation layer: a RESOLVED (via line-range
    layer) bare citation scores valid — not hallucinated."""
    from tier_c.checkout import ResolveResult
    co = _ResolveCiteCo(ResolveResult(status="RESOLVED", path="real/noqa.rs", layer="line_range"))
    v = verify_citation(co, Citation("noqa.rs", 1014, None))
    assert v.file_ok and v.line_ok and v.symbol_ok
    assert v.is_hallucination is False
    assert v.is_ambiguous is False
    assert v.is_valid is True
    assert v.resolve_layer == "line_range"


def test_score_citations_excludes_ambiguous_from_precision_denominator():
    """1 valid + 1 ambiguous: precision must be 1.0 (ambiguous excluded), NOT 0.5 (which
    would happen if ambiguous were counted against precision like a hallucination)."""
    from tier_c.checkout import AMBIGUOUS, ResolveResult

    class _MixedCo:
        def resolve_cite(self, file, line=None, symbol=None, claim_text="", *, disambiguate=None):
            if file == "valid.py":
                return ResolveResult(status="RESOLVED", path="valid.py", layer="exact")
            return ResolveResult(status=AMBIGUOUS, path=None)

        def read_line(self, rel, line):
            return "code" if rel == "valid.py" else None

        def read_window(self, rel, line, ctx=3):
            return self.read_line(rel, line)

    cites = [Citation("valid.py", 1, None), Citation("ambiguous.py", 10, None)]
    rep = score_citations(_MixedCo(), cites, claim_count=2, relevance=RelevanceAllTrue())
    assert rep.valid == 1
    assert rep.hallucinations == 0
    assert rep.ambiguous == 1
    assert rep.precision == 1.0, f"expected 1.0 (ambiguous excluded), got {rep.precision}"
    assert rep.ambiguous_rate == 0.5


def test_score_citations_absent_still_counts_against_precision():
    """A genuinely-absent citation alongside a valid one DOES count against precision
    (0.5) — only AMBIGUOUS is excluded, never ABSENT/hallucinated."""
    from tier_c.checkout import ABSENT, ResolveResult

    class _MixedCo:
        def resolve_cite(self, file, line=None, symbol=None, claim_text="", *, disambiguate=None):
            if file == "valid.py":
                return ResolveResult(status="RESOLVED", path="valid.py", layer="exact")
            return ResolveResult(status=ABSENT, path=None)

        def read_line(self, rel, line):
            return "code" if rel == "valid.py" else None

        def read_window(self, rel, line, ctx=3):
            return self.read_line(rel, line)

    cites = [Citation("valid.py", 1, None), Citation("ghost.py", 10, None)]
    rep = score_citations(_MixedCo(), cites, claim_count=2, relevance=RelevanceAllTrue())
    assert rep.valid == 1
    assert rep.hallucinations == 1
    assert rep.ambiguous == 0
    assert rep.precision == 0.5


# ---------------------------------------------------------------------------
# End-to-end artifact regression: verify_citation against a REAL Checkout (git
# worktree) reproducing the pilot's ruff:noqa.rs shape — the bare citation must
# RESOLVE and score valid, not hallucinated (was hallucinated before the R1 fix).
# ---------------------------------------------------------------------------

def test_real_checkout_bare_ambiguous_but_real_citation_scores_valid_not_hallucinated(tmp_path):
    import subprocess
    from tier_c.checkout import Checkout

    p = tmp_path / "repo"
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    decoys = ["a", "b", "c", "d", "e"]
    for d in decoys:
        (p / d).mkdir()
        (p / d / "noqa.rs").write_text("// decoy\nfn short() {}\n")
    (p / "real").mkdir()
    lines = [f"// line {i}" for i in range(1, 1201)]
    lines[1013] = "fn handle_noqa_directive() { /* the real fix */ }"
    (p / "real" / "noqa.rs").write_text("\n".join(lines) + "\n")
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                         capture_output=True, text=True, check=True).stdout.strip()

    with Checkout(str(p), sha) as co:
        v = verify_citation(co, Citation("noqa.rs", 1014, None))
        assert v.file_ok is True, "the artifact: bare ambiguous-basename must still resolve"
        assert v.is_hallucination is False, (
            "REGRESSION: this exact shape used to score as hallucinated for 19/20 real "
            "off-arm citations in the pilot's ruff cell"
        )
        assert v.is_valid is True
        assert v.resolve_layer == "line_range"


# ---------------------------------------------------------------------------
# resolver-fix-spec.md R4 — resolvability axis: mechanical, kept SEPARATE from
# precision/validity (never a truth signal).
# ---------------------------------------------------------------------------

def _v(file, line, *, ambiguous=False, file_ok=True, layer="") -> CitationVerdict:
    return CitationVerdict(
        cite=Citation(file, line, None), file_ok=file_ok, line_ok=file_ok,
        symbol_ok=True, relevant=True, ambiguous=ambiguous, resolve_layer=layer,
    )


def test_resolvability_breakdown_buckets_are_mutually_exclusive_and_sum_to_total():
    verdicts = [
        _v("a.py", 1, layer="exact"),                       # full-path
        _v("noqa.rs", 1014, layer="line_range"),            # bare-resolved
        _v("parser.go", 10, ambiguous=True, file_ok=False),  # ambiguous
        _v("ghost.py", 99, file_ok=False),                   # absent
    ]
    r = resolvability_breakdown(verdicts)
    assert r["n"] == 4
    assert r["full_path"] == 1
    assert r["bare_resolved"] == 1
    assert r["ambiguous"] == 1
    assert r["absent"] == 1
    assert r["full_path"] + r["bare_resolved"] + r["ambiguous"] + r["absent"] == r["n"]
    assert r["full_path_rate"] == 0.25
    assert r["ambiguous_rate"] == 0.25


def test_resolvability_breakdown_empty_verdicts_is_all_zero():
    r = resolvability_breakdown([])
    assert r["n"] == 0
    assert r["full_path_rate"] == 0.0
    assert r["absent_rate"] == 0.0
