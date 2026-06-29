"""Read-only pinned checkout via `git worktree` (spec §4 pinning). The investigator
verifies citations against THIS, using neutral git/file primitives — never prism."""
from __future__ import annotations
import os, subprocess, tempfile, shutil
from pathlib import Path

class Checkout:
    def __init__(self, repo: str, sha: str):
        self.repo, self.sha = repo, sha
        self._dir: Path | None = None
        self._bn_index: dict[str, list[str]] | None = None
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

    def _basename_index(self) -> dict[str, list[str]]:
        """Build (and cache) a basename → [repo-relative-path, ...] index from tracked files."""
        if self._bn_index is None:
            out = subprocess.run(["git", "-C", str(self.root), "ls-files"],
                                 capture_output=True, text=True)
            idx: dict[str, list[str]] = {}
            for line in out.stdout.splitlines():
                idx.setdefault(os.path.basename(line), []).append(line)
            self._bn_index = idx
        return self._bn_index

    def resolve_rel(self, rel: str) -> str | None:
        """Resolve a cited path to a real repo-relative path.

        Exact match wins; else if the basename is UNIQUE among tracked files, use that;
        else None (ambiguous or absent — treated as a hallucination by the caller).
        """
        if (self.root / rel).is_file():
            return rel
        base = os.path.basename(rel)
        matches = self._basename_index().get(base, [])
        return matches[0] if len(matches) == 1 else None
