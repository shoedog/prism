import subprocess
from pathlib import Path
from tier_c.checkout import Checkout

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
