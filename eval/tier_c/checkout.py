"""Read-only pinned checkout via `git worktree` (spec §4 pinning). The investigator
verifies citations against THIS, using neutral git/file primitives — never prism."""
from __future__ import annotations
import os, re, subprocess, tempfile, shutil
from dataclasses import dataclass
from pathlib import Path

# Resolution statuses (resolver-fix-spec.md R1). RESOLVED means "pinned to exactly one
# real file"; AMBIGUOUS means "the cited line is real in >=2 candidate files and no
# layer could pin it to one" (load-bearing: NEVER conflate with ABSENT — a citation to a
# real line must never score as fabrication just because its path is bare/ambiguous);
# ABSENT means the cited line exists in NO candidate file (true fabrication).
RESOLVED, AMBIGUOUS, ABSENT = "RESOLVED", "AMBIGUOUS", "ABSENT"


@dataclass(frozen=True)
class ResolveResult:
    status: str            # RESOLVED | AMBIGUOUS | ABSENT
    path: str | None       # repo-relative path when RESOLVED, else None
    layer: str = ""        # which layer resolved it (R4 resolvability axis):
                           # "exact" | "unique_basename" | "line_range" | "line_symbol"
                           # | "line_tokens" | "llm_disambiguated" | "" (not RESOLVED)


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

    def read_file(self, rel: str) -> str | None:
        """Full text of a repo-relative file (D2 relational.py: neutral per-language
        import-text parsing needs the whole file, not a windowed excerpt)."""
        p = self.root / rel
        if not p.is_file():
            return None
        return p.read_text(errors="replace")

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
        """Thin back-compat shim over resolve_cite (R1): RESOLVED -> path, else None.

        Existing callers with no line/symbol context (e.g. relational.py's
        module-name resolution) keep exactly their old behavior: exact match wins,
        else UNIQUE basename, else None for anything the line-aware layers would need
        a line number to disambiguate (ambiguous or absent alike collapse to None
        here — the whole point of resolve_cite is that CALLERS WHO HAVE A LINE NUMBER
        get the richer RESOLVED/AMBIGUOUS/ABSENT distinction instead of using this).
        """
        result = self.resolve_cite(rel)
        return result.path if result.status == RESOLVED else None

    def _linecount(self, rel: str) -> int | None:
        p = self.root / rel
        if not p.is_file():
            return None
        return len(p.read_text(errors="replace").splitlines())

    def _line_has_symbol(self, rel: str, line: int | None, symbol: str) -> bool:
        if line is None:
            return False
        ln = self.read_line(rel, line)
        return bool(ln and symbol in ln)

    def _tokens_match(self, rel: str, line: int | None, claim_text: str) -> bool:
        """Light tie-break (R1 layer 3): do the claim sentence's salient tokens
        (identifier-ish words, len>=4, skip common stopwords) appear in the
        candidate's read_window? Best-effort — returns False (no signal) when there's
        no line or no usable tokens, so callers must treat a False as "inconclusive",
        not "rejected"."""
        if line is None or not claim_text:
            return False
        window = self.read_window(rel, line)
        if not window:
            return False
        tokens = _salient_tokens(claim_text)
        if not tokens:
            return False
        window_lower = window.lower()
        return any(tok in window_lower for tok in tokens)

    def resolve_cite(self, file: str, line: int | None = None, symbol: str | None = None,
                     claim_text: str = "", *, disambiguate=None) -> ResolveResult:
        """Layered citation resolver (resolver-fix-spec.md R1).

        1. Exact path exists -> RESOLVED (layer "exact") — a FULL-PATH citation.
        2. Unique basename among tracked files -> RESOLVED (layer "unique_basename").
        3. Ambiguous basename: among same-basename candidates, keep those where the
           cited LINE is in range, then (if the citation carries a symbol) narrow to
           candidates where the symbol appears on that line, then (best-effort) narrow
           by the claim's salient tokens appearing in the candidate's read_window.
           Exactly one survivor at any narrowing step -> RESOLVED to it.
        4. Still >=2 candidates and a `disambiguate` callable was injected -> ask it
           (the R2 Q3 LLM disambiguator, `(claim_text, [window, ...]) -> int | None`).
           A picked index -> RESOLVED (layer "llm_disambiguated"); None (abstain) ->
           AMBIGUOUS.
        5. No candidate has the line in range (or no basename match at all) -> ABSENT
           (line exists in NO candidate file — true fabrication, the ONLY case this
           resolver ever reports as absent).

        The load-bearing principle: a real-but-unpinnable citation returns AMBIGUOUS,
        NEVER ABSENT — callers must never score AMBIGUOUS as a hallucination.
        """
        if (self.root / file).is_file():
            return ResolveResult(status=RESOLVED, path=file, layer="exact")

        base = os.path.basename(file)
        candidates = self._basename_index().get(base, [])
        if not candidates:
            return ResolveResult(status=ABSENT, path=None, layer="")

        if len(candidates) == 1:
            return ResolveResult(status=RESOLVED, path=candidates[0], layer="unique_basename")

        # Layer 3a: line-in-range filter.
        if line is not None:
            pool = [c for c in candidates if (self._linecount(c) or 0) >= line]
        else:
            # No line given at all: can't apply the line-range filter — fall through
            # with the full candidate pool (Q3/AMBIGUOUS below, never silently pick).
            pool = list(candidates)

        if not pool:
            return ResolveResult(status=ABSENT, path=None, layer="")
        if len(pool) == 1:
            return ResolveResult(status=RESOLVED, path=pool[0], layer="line_range")

        # Layer 3b: symbol-on-line filter (only narrows; never used alone to reject).
        if symbol:
            sym_pool = [c for c in pool if self._line_has_symbol(c, line, symbol)]
            if len(sym_pool) == 1:
                return ResolveResult(status=RESOLVED, path=sym_pool[0], layer="line_symbol")
            if sym_pool:
                pool = sym_pool

        # Layer 3c: salient-token tie-break (best-effort; only narrows).
        if claim_text:
            tok_pool = [c for c in pool if self._tokens_match(c, line, claim_text)]
            if len(tok_pool) == 1:
                return ResolveResult(status=RESOLVED, path=tok_pool[0], layer="line_tokens")
            if tok_pool:
                pool = tok_pool

        if len(pool) == 1:
            return ResolveResult(status=RESOLVED, path=pool[0], layer="line_range")

        # Layer 4: Q3 LLM disambiguator, only on a genuine >=2-candidate tie.
        if disambiguate is not None and len(pool) >= 2:
            windows = [self.read_window(c, line) or "" for c in pool]
            idx = disambiguate(claim_text, windows)
            if idx is not None and 0 <= idx < len(pool):
                return ResolveResult(status=RESOLVED, path=pool[idx], layer="llm_disambiguated")

        return ResolveResult(status=AMBIGUOUS, path=None, layer="")


_STOPWORDS = frozenset({
    "this", "that", "with", "from", "have", "been", "into", "code", "file",
    "line", "cited", "citation", "does", "which", "there", "their", "about",
})


def _salient_tokens(claim_text: str) -> list[str]:
    """Cheap tokenizer for the R1 layer-3 tie-break: lowercase identifier-ish words of
    length >= 4, minus a small stopword list. Best-effort/heuristic by design."""
    words = re.findall(r"[A-Za-z_][A-Za-z0-9_]{3,}", claim_text)
    return [w.lower() for w in words if w.lower() not in _STOPWORDS]
