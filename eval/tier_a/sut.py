"""PrismCli SystemUnderTest adapter.

Implements spec §2.3 discovery/freshness and §2.5 wire extraction. Build identity
parsing cites the single owner of the identity grammar: the "Build identity"
comment in the repository's build.rs.
"""
from __future__ import annotations

import json
import os
import re
import subprocess

from .model import CallEdge, FunctionDef, Location


class SutError(Exception):
    """SUT-side failure consumed by the accounting layer."""


class SutStale(SutError):
    """SUT binary is not verifiably built from the current clean repo."""


class SutAmbiguous(SutError):
    """prism's AmbiguousSymbol safe-fail contract for ambiguous symbol seeds."""


class SutSeedMiss(SutError):
    """prism could not resolve the harness seed to a function."""


def parse_version(out: str) -> tuple[str, bool]:
    """Parse prism's build identity.

    Identity grammar is owned by the "Build identity" comment in build.rs:
    ``<sha:[0-9a-f]{12,40}>["-dirty"] | "unknown"``.
    """
    m = re.search(r"\((?:([0-9a-f]{12,40})(-dirty)?|unknown)\)", out)
    if not m:
        raise SutStale(f"no build identity in --version output: {out!r} (rebuild prism)")
    sha = m.group(1)
    if sha is None:
        raise SutStale("gitless build -- rebuild from a git checkout")
    return sha, bool(m.group(2))


def extract_functions(arr: list[dict]) -> list[FunctionDef]:
    return [
        FunctionDef(
            name=r["name"],
            kind=r["kind"],
            container=None,
            location=Location(r["file"], r["start_line"], r["end_line"]),
            selection_line=r["start_line"],
        )
        for r in arr
    ]


def _why(item: dict, tag: str) -> dict | None:
    for reason in item.get("why", []):
        if isinstance(reason, dict) and tag in reason:
            value = reason[tag]
            return value if isinstance(value, dict) else None
    return None


def _symbol_name(symbol: dict | None) -> str | None:
    if not symbol:
        return None
    fn = symbol.get("Function")
    if isinstance(fn, dict):
        return fn.get("name")
    return symbol.get("name")


def extract_callers(seed: FunctionDef, ev: dict) -> list[CallEdge]:
    edges = []
    for it in ev.get("items", []):
        called_by = _why(it, "CalledBy")
        if called_by is None:
            continue
        loc = it["location"]
        other = Location(loc["file"], loc["start_line"], loc["end_line"])
        site = Location(loc["file"], called_by["call_site_line"], called_by["call_site_line"])
        name = _symbol_name(it.get("symbol")) or called_by.get("caller")
        edges.append(CallEdge("caller", seed, other, name, site))
    return edges


def extract_callees(seed: FunctionDef, ev: dict) -> list[CallEdge]:
    edges = []
    for it in ev.get("items", []):
        calls = _why(it, "Calls")
        if calls is None:
            continue
        site = Location(seed.location.file, calls["call_site_line"], calls["call_site_line"])
        if it.get("symbol"):
            loc = it["location"]
            other = Location(loc["file"], loc["start_line"], loc["end_line"])
        else:
            other = None
        edges.append(CallEdge("callee", seed, other, calls.get("callee"), site))
    return edges


class PrismCli:
    def __init__(
        self,
        prism_repo: str,
        sut_bin: str | None = None,
        allow_stale: bool = False,
    ):
        self.repo = prism_repo
        self.bin = sut_bin or os.environ.get("PRISM_BIN") or os.path.join(
            prism_repo, "target/release/prism"
        )
        self.allow_stale = allow_stale
        # When True, nav calls pass `--no-cache` so the per-repo nav store is
        # bypassed. The matrix runner sets this for deterministic fixture eval —
        # a stale fixture cache silently served pre-S3 results and produced false
        # regressions (S3 Task 13). Corpus measurements leave it False for speed.
        self.no_cache = False
        self.sha, self.dirty = self._check_freshness()

    def _check_freshness(self) -> tuple[str, bool]:
        out = subprocess.run(
            [self.bin, "--version"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        sha, dirty = parse_version(out)
        head = subprocess.run(
            ["git", "-C", self.repo, "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "-C", self.repo, "status", "--porcelain", "-uno"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        repo_dirty = bool(status)
        if (not head.startswith(sha) or dirty or repo_dirty) and not self.allow_stale:
            flags = []
            if not head.startswith(sha):
                flags.append(f"binary {sha} != HEAD {head}")
            if dirty:
                flags.append("binary was built from a dirty tree")
            if repo_dirty:
                flags.append("repo has uncommitted changes")
            reason = "; ".join(flags)
            raise SutStale(f"{reason}; rebuild (cargo build --release) or pass allow_stale=True")
        return sha, dirty

    def _run(self, args: list[str]) -> dict | list:
        # getattr default keeps objects built via __new__ in tests working
        # (and defaults to cache-on, the safe behavior).
        cache_args = ["--no-cache"] if getattr(self, "no_cache", False) else []
        p = subprocess.run(
            [self.bin, "nav", *cache_args, *args, "--format", "json"],
            capture_output=True,
            text=True,
        )
        if p.returncode != 0:
            blob = p.stdout or p.stderr
            if "AmbiguousSymbol" in blob:
                raise SutAmbiguous(blob)
            if "SymbolNotFound" in blob or "LocationOutOfRange" in blob:
                raise SutSeedMiss(blob)
            raise SutError(f"prism nav {args[0]} failed ({p.returncode}): {blob}")
        try:
            return json.loads(p.stdout)
        except json.JSONDecodeError as exc:
            raise SutError(f"prism nav {args[0]} returned invalid JSON: {exc}") from exc

    def inventory(self, corpus_root: str) -> list[FunctionDef]:
        return extract_functions(self._run(["functions", "--repo", corpus_root]))

    def callers(self, corpus_root: str, seed: FunctionDef) -> list[CallEdge]:
        loc = f"{seed.location.file}:{seed.location.start_line}"
        try:
            ev = self._run(["callers", "--repo", corpus_root, "--location", loc, "--depth", "1"])
        except SutSeedMiss:
            return []
        return extract_callers(seed, ev)

    def callees(self, corpus_root: str, seed: FunctionDef) -> list[CallEdge]:
        loc = f"{seed.location.file}:{seed.location.start_line}"
        try:
            ev = self._run(["callees", "--repo", corpus_root, "--location", loc, "--depth", "1"])
        except SutSeedMiss:
            return []
        return extract_callees(seed, ev)

    def callers_by_symbol(
        self,
        corpus_root: str,
        symbol: str,
        file: str | None = None,
    ) -> list[CallEdge]:
        args = ["callers", "--repo", corpus_root, "--symbol", symbol, "--depth", "1"]
        if file is not None:
            args += ["--file", file]
        # Symbol probes do not carry an oracle FunctionDef. The placeholder seed is
        # used only to preserve CallEdge.seed identity for this safe-fail pathway;
        # location-seeded callers()/callees() retain the real sampled definition.
        seed = FunctionDef(symbol, "function", None, Location(file or "?", 1, 1), 1)
        try:
            ev = self._run(args)
        except SutSeedMiss:
            return []
        return extract_callers(seed, ev)

    def version(self) -> str:
        return f"prism {self.sha}{'-dirty' if self.dirty else ''}"
