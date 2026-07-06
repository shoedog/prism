import subprocess
from pathlib import Path
from tier_c.checkout import ABSENT, AMBIGUOUS, RESOLVED, Checkout

def _init_repo(p: Path) -> str:
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    (p / "a.py").write_text("def foo():\n    return 1\n")
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                          capture_output=True, text=True, check=True).stdout.strip()

def test_checkout_reads_file_at_sha(tmp_path):
    sha = _init_repo(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.read_line("a.py", 1) == "def foo():"
        assert co.file_exists("a.py")
        assert not co.file_exists("missing.py")

def test_no_tempdir_leak_on_bad_sha(tmp_path):
    import glob, tempfile, pytest
    _init_repo(tmp_path / "repo")
    before = set(glob.glob(str(Path(tempfile.gettempdir()) / "tc-co-*")))
    with pytest.raises(subprocess.CalledProcessError):
        with Checkout(str(tmp_path / "repo"), "deadbeefdeadbeef"):
            pass
    after = set(glob.glob(str(Path(tempfile.gettempdir()) / "tc-co-*")))
    assert after == before  # no leaked worktree dir


# --- read_window unit tests (no git worktree needed; set _dir directly) ---

def _make_checkout_at(root: Path) -> Checkout:
    """Return a Checkout whose .root is *root* without running git worktree."""
    co = Checkout.__new__(Checkout)
    co.repo = ""
    co.sha = ""
    co._dir = root
    return co


def test_read_window_centered(tmp_path):
    f = tmp_path / "a.py"
    f.write_text("\n".join(f"L{i}" for i in range(1, 21)))  # L1..L20
    co = _make_checkout_at(tmp_path)
    result = co.read_window("a.py", 10, ctx=2)
    assert result == "L8\nL9\nL10\nL11\nL12"


def test_read_window_clamps_at_top(tmp_path):
    f = tmp_path / "a.py"
    f.write_text("\n".join(f"L{i}" for i in range(1, 21)))
    co = _make_checkout_at(tmp_path)
    result = co.read_window("a.py", 1, ctx=2)
    assert result == "L1\nL2\nL3"


def test_read_window_missing_file_returns_none(tmp_path):
    co = _make_checkout_at(tmp_path)
    assert co.read_window("missing.py", 5, ctx=2) is None


# --- resolve_rel tests ---

def _init_repo_multi(p: Path) -> str:
    """Repo with files at various paths including same-basename duplicates."""
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    # unique basename: model/labels/regexp.go
    (p / "model").mkdir()
    (p / "model" / "labels").mkdir()
    (p / "model" / "labels" / "regexp.go").write_text("package labels\n")
    # duplicate basename: util/parser.go AND core/parser.go
    (p / "util").mkdir()
    (p / "util" / "parser.go").write_text("package util\n")
    (p / "core").mkdir()
    (p / "core" / "parser.go").write_text("package core\n")
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                          capture_output=True, text=True, check=True).stdout.strip()


def test_resolve_rel_exact_match(tmp_path):
    """Exact repo-relative path resolves to itself."""
    sha = _init_repo_multi(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.resolve_rel("model/labels/regexp.go") == "model/labels/regexp.go"


def test_resolve_rel_unique_basename(tmp_path):
    """Basename-only cite resolves when unique among tracked files."""
    sha = _init_repo_multi(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.resolve_rel("regexp.go") == "model/labels/regexp.go"


def test_resolve_rel_ambiguous_basename_returns_none(tmp_path):
    """Ambiguous basename (two files) returns None — treated as hallucination."""
    sha = _init_repo_multi(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.resolve_rel("parser.go") is None


def test_resolve_rel_absent_returns_none(tmp_path):
    """Non-existent path and non-existent basename both return None."""
    sha = _init_repo_multi(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.resolve_rel("no_such_file.go") is None
        assert co.resolve_rel("deep/path/no_such_file.go") is None


def test_resolve_rel_basename_index_cached(tmp_path):
    """_basename_index is built once and reused (smoke-test idempotency)."""
    sha = _init_repo_multi(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        idx1 = co._basename_index()
        idx2 = co._basename_index()
        assert idx1 is idx2  # same object — cached


# ---------------------------------------------------------------------------
# resolve_cite — R1 layered resolver (resolver-fix-spec.md). Reproduces the CONFIRMED
# pilot artifact: bare filename + non-unique basename used to collapse straight to
# "unresolved" (scored as hallucination) even when the cited line was real in exactly
# one candidate. noqa.rs shape: several short decoy files + one long file that alone
# has the cited line in range.
# ---------------------------------------------------------------------------

_DECOY_DIRS = ["a", "b", "c", "d", "e"]
_REAL_LINE = 1014  # 1-indexed, matches the pilot's ruff:noqa.rs:1014 citation


def _init_repo_noqa(p: Path) -> str:
    """6 files named noqa.rs (pilot shape): 5 short decoys (2 lines each) + 1 long file
    (1200 lines) — ONLY the long file has line 1014 in range."""
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    for d in _DECOY_DIRS:
        (p / d).mkdir()
        (p / d / "noqa.rs").write_text("// decoy noqa handling\nfn short() {}\n")
    (p / "real").mkdir()
    lines = [f"// line {i}" for i in range(1, 1201)]
    lines[_REAL_LINE - 1] = "fn handle_noqa_directive() { /* the real fix */ }"
    (p / "real" / "noqa.rs").write_text("\n".join(lines) + "\n")
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                          capture_output=True, text=True, check=True).stdout.strip()


def test_resolve_cite_exact_path_resolves_full_path_layer(tmp_path):
    sha = _init_repo_noqa(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        r = co.resolve_cite("real/noqa.rs", _REAL_LINE)
        assert r.status == RESOLVED
        assert r.path == "real/noqa.rs"
        assert r.layer == "exact"


def test_resolve_cite_unique_basename_resolves_bare_layer(tmp_path):
    sha = _init_repo_multi(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        r = co.resolve_cite("regexp.go", 1)
        assert r.status == RESOLVED
        assert r.path == "model/labels/regexp.go"
        assert r.layer == "unique_basename"


def test_resolve_cite_bare_filename_disambiguates_by_line_range(tmp_path):
    """THE ARTIFACT FIX: a bare `noqa.rs:1014` citation among 6 same-basename files
    must RESOLVE to the one candidate whose line is in range — this alone is what the
    old resolve_rel-only resolver marked unresolved/hallucinated for 19/20 real
    off-arm citations in the pilot's ruff cell."""
    sha = _init_repo_noqa(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        r = co.resolve_cite("noqa.rs", _REAL_LINE)
        assert r.status == RESOLVED, "a real line in exactly one candidate must RESOLVE"
        assert r.path == "real/noqa.rs"
        assert r.layer == "line_range"


def test_resolve_cite_line_out_of_range_everywhere_is_absent(tmp_path):
    """A cited line that exists in NO candidate (decoys are 2 lines, real is 1200) is
    true fabrication -> ABSENT, never AMBIGUOUS."""
    sha = _init_repo_noqa(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        r = co.resolve_cite("noqa.rs", 99999)
        assert r.status == ABSENT
        assert r.path is None


def test_resolve_cite_no_basename_match_is_absent(tmp_path):
    sha = _init_repo_noqa(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        r = co.resolve_cite("nonexistent_file.rs", 1)
        assert r.status == ABSENT


def test_resolve_cite_genuinely_ambiguous_two_real_candidates_no_disambiguator(tmp_path):
    """Two candidates BOTH have the cited line in range, no symbol/claim-text signal,
    and no disambiguator injected -> AMBIGUOUS. The load-bearing principle: never
    silently guess one — AMBIGUOUS must be distinct from ABSENT (the caller must never
    score this as a hallucination)."""
    p = tmp_path / "repo"
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    (p / "util").mkdir()
    (p / "util" / "parser.go").write_text("\n".join(f"line {i}" for i in range(1, 30)))
    (p / "core").mkdir()
    (p / "core" / "parser.go").write_text("\n".join(f"line {i}" for i in range(1, 30)))
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                         capture_output=True, text=True, check=True).stdout.strip()
    with Checkout(str(p), sha) as co:
        r = co.resolve_cite("parser.go", 10)
        assert r.status == AMBIGUOUS
        assert r.path is None


def test_resolve_cite_symbol_narrows_ambiguous_pool_to_one(tmp_path):
    """When both same-basename candidates have the line in range but only ONE has the
    cited symbol on that line, the symbol layer alone resolves it."""
    p = tmp_path / "repo"
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    (p / "util").mkdir()
    (p / "util" / "parser.go").write_text("\n".join(
        "func Parse() {}" if i == 10 else f"// line {i}" for i in range(1, 30)))
    (p / "core").mkdir()
    (p / "core" / "parser.go").write_text("\n".join(
        "func Other() {}" if i == 10 else f"// line {i}" for i in range(1, 30)))
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                         capture_output=True, text=True, check=True).stdout.strip()
    with Checkout(str(p), sha) as co:
        r = co.resolve_cite("parser.go", 10, "Parse")
        assert r.status == RESOLVED
        assert r.path == "util/parser.go"
        assert r.layer == "line_symbol"


def test_resolve_cite_llm_disambiguator_invoked_only_on_genuine_tie(tmp_path):
    """R2 integration point: the injected `disambiguate` callable is called ONLY when
    layers 1-3 leave >=2 candidates, receives (claim_text, [window, ...]) in candidate
    order, and a picked index RESOLVES."""
    p = tmp_path / "repo"
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    (p / "util").mkdir()
    (p / "util" / "parser.go").write_text("\n".join(f"line {i}" for i in range(1, 30)))
    (p / "core").mkdir()
    (p / "core" / "parser.go").write_text("\n".join(f"line {i}" for i in range(1, 30)))
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                         capture_output=True, text=True, check=True).stdout.strip()
    with Checkout(str(p), sha) as co:
        candidates = co._basename_index()["parser.go"]
        assert len(candidates) == 2, "fixture must have exactly 2 same-basename candidates"
        calls = []
        def fake_disambiguate(claim_text, windows):
            calls.append((claim_text, windows))
            return 1  # picks the SECOND candidate, by index
        r = co.resolve_cite("parser.go", 10, None, "the claim", disambiguate=fake_disambiguate)
        assert r.status == RESOLVED
        assert len(calls) == 1, "disambiguator must be called exactly once on a genuine tie"
        assert calls[0][0] == "the claim"
        assert len(calls[0][1]) == 2, "must receive one window per candidate"
        # The picked index must map to the candidate pool's own order, not a guess.
        assert r.path == candidates[1]


def test_resolve_cite_llm_disambiguator_abstain_gives_ambiguous(tmp_path):
    p = tmp_path / "repo"
    p.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=p, check=True)
    (p / "util").mkdir()
    (p / "util" / "parser.go").write_text("\n".join(f"line {i}" for i in range(1, 30)))
    (p / "core").mkdir()
    (p / "core" / "parser.go").write_text("\n".join(f"line {i}" for i in range(1, 30)))
    subprocess.run(["git", "add", "-A"], cwd=p, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=p, check=True)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=p,
                         capture_output=True, text=True, check=True).stdout.strip()
    with Checkout(str(p), sha) as co:
        r = co.resolve_cite("parser.go", 10, None, "the claim",
                            disambiguate=lambda claim, windows: None)
        assert r.status == AMBIGUOUS
        assert r.path is None


def test_resolve_rel_shim_stays_lossy_for_bare_ambiguous_noqa(tmp_path):
    """resolve_rel(str) has no line parameter, so it CANNOT apply the line-range fix —
    it stays exactly as lossy as before for genuinely ambiguous basenames. This is the
    documented, intentional back-compat-shim limitation (R1): callers who have a line
    number must use resolve_cite instead to get the fix."""
    sha = _init_repo_noqa(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        assert co.resolve_rel("noqa.rs") is None
