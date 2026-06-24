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
