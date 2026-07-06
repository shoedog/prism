"""`tier-c build-gold` (design-of-record §5, spec P2): emit gold CANDIDATE sites
for a Part-D structural task, provenance-tagged {lsp, prism, both}, WITHOUT
deciding truth. This module only PROPOSES — the controller adjudicates the
disagreement band (`adjudicate.md`) and freezes the source-verified `gold.json`
separately (structural.py reads only that frozen file; never this module's
output directly).

Candidates = LSP(S) UNION prism(S) (design §5.1), merged on a normalized
`(file, norm_symbol)` key. LSP is a candidate source and cross-check, never
ground truth on its own (§5a) — `both` sites still need the controller's
source-verification pass before entering gold, this module just marks them
auto-acceptable to save adjudication effort.

Graceful degradation: ANY oracle/binary failure -> empty candidates for that
source + a recorded `oracle_health` string, never a crash — build-gold must
always emit `candidates.json` + `adjudicate.md`.
"""
from __future__ import annotations

import dataclasses
import json
import os
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

from tier_a.model import FunctionDef, Location, tie_break

from .structural import norm_symbol
from .structural_corpus import StructuralTask

# LSP server command table (mirrors tier_a/cli.py's make_oracle; Part-D's first
# slice is Rust/Go only — design §7 defers Python to slice 2).
_LSP_CMD = {
    "rust": ["rust-analyzer"],
    "go": ["gopls", "serve"],
    "python": ["pyright-langserver", "--stdio"],
}


@dataclass(frozen=True)
class CandidateSite:
    file: str
    symbol: str
    line: int
    provenance: str    # "both" | "lsp" | "prism"
    d_member: str      # "D1" | "D2" | "none"


@dataclass
class BuildGoldResult:
    task_id: str
    repo: str
    sha: str
    symbol: str
    oracle_health: dict   # {"lsp": "ok"|"unavailable: ..."|"seed-miss: ...", "prism": ...}
    sites: list = field(default_factory=list)   # list[CandidateSite]


def _safe_stop(oracle) -> None:
    try:
        oracle.stop()
    except Exception:
        pass


def _default_lsp_oracle_factory(lang: str, repo_root: str):
    from tier_a.oracles import LspOracle

    cmd = _LSP_CMD.get(lang)
    if cmd is None:
        raise ValueError(f"no LSP server configured for lang={lang!r}")
    return LspOracle(cmd, repo_root, lang)


def lsp_candidates(task: StructuralTask, co, *, oracle_factory=None):
    """LSP incoming-callers candidates for task.symbol at task.def_site.

    Returns (sites, health) where sites is a list of (file, symbol, line) tuples
    (direct callers only — design §5 step 2: transitive/name-absent sites come
    from prism + adjudication, not here) and health is "ok", "seed-miss: ...",
    or "unavailable: ...". Never raises.
    """
    factory = oracle_factory or _default_lsp_oracle_factory
    try:
        oracle = factory(task.lang, str(co.root))
    except Exception as exc:
        return [], f"unavailable: {exc}"

    try:
        oracle.start()
    except Exception as exc:
        _safe_stop(oracle)
        return [], f"unavailable: start failed: {exc}"

    try:
        file, line = task.def_site
        fds = oracle.document_symbols(file)
        cands = [
            fd for fd in fds
            if fd.name == task.symbol
            and fd.location.start_line <= line <= fd.location.end_line
        ]
        if not cands:
            return [], "seed-miss: no matching FunctionDef at def_site"
        seed = tie_break(cands)
        edges = oracle.callers(seed)
    except Exception as exc:
        return [], f"unavailable: {exc}"
    finally:
        _safe_stop(oracle)

    sites = sorted({
        (e.call_site.file, e.other_name or "?", e.call_site.start_line)
        for e in edges
    })
    return sites, "ok"


def _default_prism_runner(repo_root: str, symbol: str) -> dict:
    from .arm_runner import _prism_bin

    argv = [_prism_bin(), "nav", "callers", "--repo", repo_root,
            "--symbol", symbol, "--format", "json"]
    p = subprocess.run(argv, capture_output=True, text=True, timeout=300)
    if p.returncode != 0:
        raise RuntimeError(
            f"prism nav callers failed ({p.returncode}): {(p.stdout or p.stderr)[:2000]}")
    return json.loads(p.stdout)


def prism_candidates(repo_root: str, symbol: str, *, runner=None):
    """prism incoming-callers candidates via `prism nav callers --symbol ... --format json`.

    Reuses tier_a.sut.extract_callers (the pure Evidence-JSON -> CallEdge mapping,
    already exercised against the real binary's wire shape) with a placeholder
    seed, mirroring PrismCli.callers_by_symbol's safe-fail pattern. `runner` is
    injectable: callable(repo_root, symbol) -> Evidence dict (tests fake this;
    production shells the release binary). Never raises.
    """
    from tier_a.sut import extract_callers

    run = runner or _default_prism_runner
    try:
        ev = run(repo_root, symbol)
    except Exception as exc:
        return [], f"unavailable: {exc}"

    try:
        placeholder = FunctionDef(symbol, "function", None, Location("?", 1, 1), 1)
        edges = extract_callers(placeholder, ev)
    except Exception as exc:
        return [], f"parse-error: {exc}"

    sites = sorted({
        (e.call_site.file, e.other_name or "?", e.call_site.start_line)
        for e in edges
    })
    return sites, "ok"


def _grep_name_stats(co, name: str) -> tuple[dict, int]:
    """(per_file_hit_count, total_repo_wide_hits) for `git grep -nw <name>` at the
    checkout's pinned sha (co.root is already a worktree checked out at that sha)."""
    p = subprocess.run(["git", "-C", str(co.root), "grep", "-nw", "-e", name],
                       capture_output=True, text=True)
    if p.returncode not in (0, 1):   # 1 = no matches (not an error)
        raise RuntimeError(f"git grep failed ({p.returncode}): {p.stderr}")
    per_file: dict = {}
    for line in p.stdout.splitlines():
        parts = line.split(":", 2)
        if len(parts) < 2:
            continue
        per_file[parts[0]] = per_file.get(parts[0], 0) + 1
    return per_file, sum(per_file.values())


def _d_member(file: str, per_file_hits: dict, total_hits: int) -> str:
    """D-membership (admission-gate §2 / spec P2 step 5), scoped to ONE candidate
    site's file: D1 = the target name has ZERO textual occurrences in this file
    (grep-invisible dispatch); D2 = name present here AND the name is so common
    repo-wide (>100 hits) that grep is not a practical oracle; else none."""
    if per_file_hits.get(file, 0) == 0:
        return "D1"
    if total_hits > 100:
        return "D2"
    return "none"


def _merge(task: StructuralTask, lsp_sites, prism_sites, per_file_hits, total_hits,
          *, oracle_health: dict) -> BuildGoldResult:
    lsp_keys = {(f, norm_symbol(s)): (f, s, ln) for f, s, ln in lsp_sites}
    prism_keys = {(f, norm_symbol(s)): (f, s, ln) for f, s, ln in prism_sites}
    all_keys = sorted(set(lsp_keys) | set(prism_keys))

    sites = []
    for key in all_keys:
        in_lsp, in_prism = key in lsp_keys, key in prism_keys
        provenance = "both" if (in_lsp and in_prism) else ("lsp" if in_lsp else "prism")
        f, s, ln = lsp_keys.get(key) or prism_keys.get(key)
        sites.append(CandidateSite(file=f, symbol=s, line=ln, provenance=provenance,
                                   d_member=_d_member(f, per_file_hits, total_hits)))
    sites.sort(key=lambda c: (c.file, c.symbol, c.line))

    return BuildGoldResult(task_id=task.id, repo=task.repo, sha=task.sha,
                           symbol=task.symbol, oracle_health=oracle_health, sites=sites)


def build_gold(task: StructuralTask, co, *, out_root: str,
              oracle_factory=None, prism_runner=None) -> tuple:
    """Build candidates for ONE task inside an already-open Checkout `co`, then
    write `candidates.json` + `adjudicate.md` under `<out_root>/<task.id>/`.

    Returns (BuildGoldResult, candidates_json_path, adjudicate_md_path). Always
    writes both files, even when both oracles are unavailable (empty sites).
    """
    lsp_sites, lsp_health = lsp_candidates(task, co, oracle_factory=oracle_factory)
    prism_sites, prism_health = prism_candidates(str(co.root), task.symbol, runner=prism_runner)
    per_file_hits, total_hits = _grep_name_stats(co, task.symbol)

    result = _merge(task, lsp_sites, prism_sites, per_file_hits, total_hits,
                    oracle_health={"lsp": lsp_health, "prism": prism_health})

    out_dir = os.path.join(out_root, task.id)
    cand_path, adjudicate_path = write_gold_files(result, co, out_dir)
    return result, cand_path, adjudicate_path


def render_adjudicate_md(result, co=None) -> str:
    """Render the adjudication file: ONLY the disagreement band gets a `verdict:`
    prompt; `both` (LSP∩prism) sites are listed separately as auto-accepted."""
    both = [s for s in result.sites if s.provenance == "both"]
    band = [s for s in result.sites if s.provenance in ("lsp", "prism")]

    lines = [
        f"# Adjudication — {result.task_id}",
        "",
        f"repo: {result.repo}  sha: {result.sha}  symbol: {result.symbol}",
        f"oracle_health: {json.dumps(result.oracle_health, sort_keys=True)}",
        "",
        "## Disagreement band (needs verdict — source-verify; a tool provenance tag is NOT truth)",
        "",
    ]
    if not band:
        lines.append("(none — every candidate site had lsp/prism agreement)")
        lines.append("")
    for s in band:
        snippet = co.read_window(s.file, s.line) if co is not None else None
        lines.append(f"### {s.file}:{s.line} — {s.symbol}")
        lines.append(f"provenance: {s.provenance}")
        lines.append(f"d_member: {s.d_member}")
        if snippet:
            lines.append("```")
            lines.append(snippet)
            lines.append("```")
        lines.append("verdict: ")
        lines.append("reason: ")
        lines.append("")

    lines.append("## Auto-accepted (both — LSP and prism agree; still source-verify before freezing)")
    lines.append("")
    if not both:
        lines.append("(none)")
    for s in both:
        lines.append(f"- {s.file}:{s.line} — {s.symbol} (d_member={s.d_member})")

    return "\n".join(lines) + "\n"


def write_gold_files(result, co, out_dir: str) -> tuple:
    os.makedirs(out_dir, exist_ok=True)

    cand_path = os.path.join(out_dir, "candidates.json")
    payload = {
        "task_id": result.task_id, "repo": result.repo, "sha": result.sha,
        "symbol": result.symbol, "oracle_health": result.oracle_health,
        "sites": [dataclasses.asdict(s) for s in result.sites],
    }
    with open(cand_path, "w") as f:
        json.dump(payload, f, indent=2, sort_keys=True)
        f.write("\n")

    adjudicate_path = os.path.join(out_dir, "adjudicate.md")
    with open(adjudicate_path, "w") as f:
        f.write(render_adjudicate_md(result, co))

    return cand_path, adjudicate_path
