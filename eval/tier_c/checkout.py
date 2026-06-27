"""Read-only pinned checkout via `git worktree` (spec §4 pinning). The investigator
verifies citations against THIS, using neutral git/file primitives — never prism."""
from __future__ import annotations
import subprocess, tempfile, shutil
from pathlib import Path

class Checkout:
    def __init__(self, repo: str, sha: str):
        self.repo, self.sha = repo, sha
        self._dir: Path | None = None
    def __enter__(self) -> "Checkout":
        self._dir = Path(tempfile.mkdtemp(prefix="tc-co-"))
        try:
            subprocess.run(["git", "worktree", "add", "--detach", "-q", str(self._dir), self.sha],
                           cwd=self.repo, check=True)
        except BaseException:
            shutil.rmtree(self._dir, ignore_errors=True)
            self._dir = None
            raise
        return self
    def __exit__(self, *exc) -> None:
        if self._dir:
            subprocess.run(["git", "worktree", "remove", "--force", str(self._dir)],
                           cwd=self.repo, check=False)
            shutil.rmtree(self._dir, ignore_errors=True)
    @property
    def root(self) -> Path:
        assert self._dir is not None
        return self._dir
    def file_exists(self, rel: str) -> bool:
        return (self.root / rel).is_file()
    def read_line(self, rel: str, line: int) -> str | None:
        p = self.root / rel
        if not p.is_file(): return None
        lines = p.read_text(errors="replace").splitlines()
        return lines[line - 1] if 1 <= line <= len(lines) else None
    def read_window(self, rel: str, line: int, ctx: int = 3) -> str | None:
        p = self.root / rel
        if not p.is_file():
            return None
        lines = p.read_text(errors="replace").splitlines()
        lo, hi = max(0, line - 1 - ctx), min(len(lines), line + ctx)
        return "\n".join(lines[lo:hi]) if lo < hi else None
