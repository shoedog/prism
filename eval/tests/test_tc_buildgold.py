"""P2: `tier-c build-gold` candidate generator (spec §5 + design §5a LSP-fallibility).
Emits CANDIDATE sites, provenance-tagged, WITHOUT deciding truth — the controller
adjudicates the disagreement band and freezes gold.json separately.

Unit-tested with a FAKE LspOracle + a FAKE prism runner injected via seams — no
real servers/binaries. Uses a REAL git temp repo (same pattern as test_tc_checkout.py)
so D-membership (git grep) and read_window() snippets exercise real git primitives.
"""
from __future__ import annotations
import json
import subprocess
from pathlib import Path

import pytest

from tier_a.model import CallEdge, FunctionDef, Location
from tier_c.checkout import Checkout
from tier_c.structural_corpus import StructuralTask
from tier_c.buildgold import (
    CandidateSite, build_gold, lsp_candidates, prism_candidates,
    render_adjudicate_md, write_gold_files,
)


# ---------------------------------------------------------------------------
# Fakes
# ---------------------------------------------------------------------------

class _FakeLspOracle:
    def __init__(self, doc_syms: dict, edges_by_name: dict, *, fail_start: bool = False):
        self._doc_syms = doc_syms
        self._edges = edges_by_name
        self._fail_start = fail_start
        self.started = False
        self.stopped = False

    def start(self):
        if self._fail_start:
            raise RuntimeError("server failed to spawn")
        self.started = True

    def stop(self):
        self.stopped = True

    def document_symbols(self, rel):
        return self._doc_syms.get(rel, [])

    def callers(self, fd):
        return self._edges.get(fd.name, [])


def _seed_fd(name="MatchString", file="model/labels/regexp.go", start=328, end=340):
    return FunctionDef(name, "method", "FastRegexMatcher",
                       Location(file, start, end), start)


def _caller_edge(file, name, def_start, def_end, call_line):
    return CallEdge("caller", _seed_fd(), Location(file, def_start, def_end), name,
                    Location(file, call_line, call_line))


# ---------------------------------------------------------------------------
# Real temp git repo fixture (D-membership + read_window need real git/files)
# ---------------------------------------------------------------------------

def _init_repo(root: Path) -> str:
    root.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "model").mkdir()
    (root / "model" / "labels").mkdir()
    (root / "model" / "labels" / "regexp.go").write_text(
        "\n".join(f"// line {i}" for i in range(1, 328))
        + "\nfunc (m *FastRegexMatcher) MatchString(s string) bool {\n"
        + "    return m.matchString(s)\n}\n"
    )
    (root / "labels").mkdir()
    (root / "labels" / "matcher.go").write_text(
        "package labels\n\nfunc (m *Matcher) Matches(s string) bool {\n"
        "    return m.re.MatchString(s)\n}\n"  # D2-ish: the NAME appears here (present)
    )
    (root / "labels" / "dispatch.go").write_text(
        "package labels\n\nfunc CallSite(m *Matcher, s string) bool {\n"
        "    return m.Matches(s)\n}\n"  # name-absent caller: never says MatchString
    )
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-q", "-m", "init"], cwd=root, check=True)
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=root,
                          capture_output=True, text=True, check=True).stdout.strip()


def _task(sha: str) -> StructuralTask:
    return StructuralTask(
        id="prometheus-matchstring", repo="prometheus", lang="go", sha=sha,
        symbol="MatchString", receiver="(*FastRegexMatcher)",
        def_site=("model/labels/regexp.go", 328),
        dispatch="labels.Matcher.Matches -> matchString field",
        prompt_change="We are changing the matching semantics...",
        grep_name_stats="git grep -lw MatchString = 21 files",
        notes="",
    )


@pytest.fixture()
def repo_checkout(tmp_path):
    sha = _init_repo(tmp_path / "repo")
    with Checkout(str(tmp_path / "repo"), sha) as co:
        yield co, sha


# ---------------------------------------------------------------------------
# lsp_candidates
# ---------------------------------------------------------------------------

def test_lsp_candidates_matches_seed_by_name_and_containment(repo_checkout):
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}
    edges = {"MatchString": [_caller_edge("labels/matcher.go", "Matches", 3, 5, 4)]}

    def factory(lang, root):
        return _FakeLspOracle(doc_syms, edges)

    sites, health = lsp_candidates(task, co, oracle_factory=factory)
    assert health == "ok"
    assert sites == [("labels/matcher.go", "Matches", 4)]


def test_lsp_candidates_start_failure_degrades_gracefully(repo_checkout):
    co, sha = repo_checkout
    task = _task(sha)

    def factory(lang, root):
        return _FakeLspOracle({}, {}, fail_start=True)

    sites, health = lsp_candidates(task, co, oracle_factory=factory)
    assert sites == []
    assert "unavailable" in health


def test_lsp_candidates_factory_construction_failure_degrades_gracefully(repo_checkout):
    co, sha = repo_checkout
    task = _task(sha)

    def factory(lang, root):
        raise ValueError(f"no server configured for {lang}")

    sites, health = lsp_candidates(task, co, oracle_factory=factory)
    assert sites == []
    assert "unavailable" in health


def test_lsp_candidates_seed_miss_when_no_matching_symbol(repo_checkout):
    co, sha = repo_checkout
    task = _task(sha)
    # document_symbols returns a FunctionDef with the WRONG name -> no containment match
    doc_syms = {"model/labels/regexp.go": [
        FunctionDef("OtherMethod", "method", None, Location("model/labels/regexp.go", 328, 340), 328)
    ]}

    def factory(lang, root):
        return _FakeLspOracle(doc_syms, {})

    sites, health = lsp_candidates(task, co, oracle_factory=factory)
    assert sites == []
    assert "seed-miss" in health


def test_lsp_candidates_stops_oracle_even_on_callers_failure(repo_checkout):
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}

    class _BoomOracle(_FakeLspOracle):
        def callers(self, fd):
            raise RuntimeError("boom")

    oracle_holder = {}

    def factory(lang, root):
        o = _BoomOracle(doc_syms, {})
        oracle_holder["o"] = o
        return o

    sites, health = lsp_candidates(task, co, oracle_factory=factory)
    assert sites == []
    assert "unavailable" in health
    assert oracle_holder["o"].stopped, "oracle.stop() must run even when callers() raises"


# ---------------------------------------------------------------------------
# prism_candidates
# ---------------------------------------------------------------------------

_EVIDENCE_SHAPE = {
    "query": "callers:MatchString@model/labels/regexp.go",
    "items": [
        {
            "symbol": {"Function": {"file": "labels/dispatch.go", "name": "CallSite",
                                    "start_line": 3, "end_line": 5}},
            "location": {"file": "labels/dispatch.go", "start_line": 3, "end_line": 5},
            "score": 1.0,
            "source": "PrismCpg",
            "fallback": False,
            "why": [{"CalledBy": {"caller": "CallSite", "call_site_line": 4}},
                    {"Resolution": {"kind": "local_def"}}],
            "snippet": None,
        }
    ],
    "truncated": False,
    "warnings": [],
}


def test_prism_candidates_parses_real_evidence_shape(repo_checkout):
    co, sha = repo_checkout

    def runner(repo_root, symbol):
        assert symbol == "MatchString"
        return _EVIDENCE_SHAPE

    sites, health = prism_candidates(str(co.root), "MatchString", runner=runner)
    assert health == "ok"
    assert sites == [("labels/dispatch.go", "CallSite", 4)]


def test_prism_candidates_runner_error_degrades_gracefully(repo_checkout):
    co, sha = repo_checkout

    def runner(repo_root, symbol):
        raise RuntimeError("prism binary not found")

    sites, health = prism_candidates(str(co.root), "MatchString", runner=runner)
    assert sites == []
    assert "unavailable" in health


def test_prism_candidates_malformed_json_degrades_gracefully(repo_checkout):
    co, sha = repo_checkout

    def runner(repo_root, symbol):
        return {"items": [{"symbol": {}, "location": {}, "why": [{"CalledBy": {}}]}]}

    sites, health = prism_candidates(str(co.root), "MatchString", runner=runner)
    assert sites == []
    assert health != "ok"


# ---------------------------------------------------------------------------
# build_gold: merge / provenance / D-membership / file emission
# ---------------------------------------------------------------------------

def test_build_gold_merges_provenance_both_lsp_prism(repo_checkout, tmp_path):
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}
    # LSP finds Matches (matcher.go) AND CallSite (dispatch.go); prism finds only CallSite.
    edges = {"MatchString": [
        _caller_edge("labels/matcher.go", "Matches", 3, 5, 4),
        _caller_edge("labels/dispatch.go", "CallSite", 3, 5, 4),
    ]}

    def oracle_factory(lang, root):
        return _FakeLspOracle(doc_syms, edges)

    def prism_runner(repo_root, symbol):
        return _EVIDENCE_SHAPE  # only CallSite/dispatch.go

    result, cand_path, adjudicate_path = build_gold(
        task, co, out_root=str(tmp_path / "gold"),
        oracle_factory=oracle_factory, prism_runner=prism_runner,
    )

    by_key = {(s.file, s.symbol): s for s in result.sites}
    assert by_key[("labels/matcher.go", "Matches")].provenance == "lsp"
    assert by_key[("labels/dispatch.go", "CallSite")].provenance == "both"
    assert result.oracle_health == {"lsp": "ok", "prism": "ok"}


def test_build_gold_d_membership_d1_for_name_absent_file(repo_checkout, tmp_path):
    """dispatch.go never literally says 'MatchString' -> D1. matcher.go DOES say
    'MatchString' (the m.re.MatchString(s) line) -> not D1."""
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}
    edges = {"MatchString": [
        _caller_edge("labels/matcher.go", "Matches", 3, 5, 4),
        _caller_edge("labels/dispatch.go", "CallSite", 3, 5, 4),
    ]}

    def oracle_factory(lang, root):
        return _FakeLspOracle(doc_syms, edges)

    result, _, _ = build_gold(task, co, out_root=str(tmp_path / "gold"),
                              oracle_factory=oracle_factory, prism_runner=lambda r, s: {"items": []})

    by_file = {s.file: s.d_member for s in result.sites}
    assert by_file["labels/dispatch.go"] == "D1"
    assert by_file["labels/matcher.go"] == "none"  # name present, repo-wide hits well under 100


def test_build_gold_prism_failure_still_emits_lsp_only_candidates(repo_checkout, tmp_path):
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}
    edges = {"MatchString": [_caller_edge("labels/matcher.go", "Matches", 3, 5, 4)]}

    def oracle_factory(lang, root):
        return _FakeLspOracle(doc_syms, edges)

    def broken_prism_runner(repo_root, symbol):
        raise RuntimeError("prism nav callers exit 1")

    result, _, _ = build_gold(task, co, out_root=str(tmp_path / "gold"),
                              oracle_factory=oracle_factory, prism_runner=broken_prism_runner)

    assert len(result.sites) == 1
    assert result.sites[0].provenance == "lsp"
    assert result.oracle_health["prism"].startswith("unavailable")


def test_build_gold_both_lsp_and_prism_down_emits_empty_but_never_crashes(repo_checkout, tmp_path):
    co, sha = repo_checkout
    task = _task(sha)

    def oracle_factory(lang, root):
        raise RuntimeError("no lsp binary")

    def broken_prism_runner(repo_root, symbol):
        raise RuntimeError("no prism binary")

    result, cand_path, adjudicate_path = build_gold(
        task, co, out_root=str(tmp_path / "gold"),
        oracle_factory=oracle_factory, prism_runner=broken_prism_runner,
    )
    assert result.sites == []
    assert Path(cand_path).exists()
    assert Path(adjudicate_path).exists()


# ---------------------------------------------------------------------------
# candidates.json / adjudicate.md emission
# ---------------------------------------------------------------------------

def test_write_gold_files_candidates_json_shape(repo_checkout, tmp_path):
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}
    edges = {"MatchString": [_caller_edge("labels/matcher.go", "Matches", 3, 5, 4)]}

    def oracle_factory(lang, root):
        return _FakeLspOracle(doc_syms, edges)

    result, cand_path, _ = build_gold(
        task, co, out_root=str(tmp_path / "gold"),
        oracle_factory=oracle_factory, prism_runner=lambda r, s: {"items": []},
    )
    payload = json.loads(Path(cand_path).read_text())
    assert payload["task_id"] == "prometheus-matchstring"
    assert payload["symbol"] == "MatchString"
    assert "oracle_health" in payload
    assert payload["sites"][0]["file"] == "labels/matcher.go"
    assert payload["sites"][0]["provenance"] == "lsp"


def test_adjudicate_md_contains_only_disagreement_band(repo_checkout, tmp_path):
    co, sha = repo_checkout
    task = _task(sha)
    doc_syms = {"model/labels/regexp.go": [_seed_fd()]}
    edges = {"MatchString": [
        _caller_edge("labels/matcher.go", "Matches", 3, 5, 4),   # lsp-only
        _caller_edge("labels/dispatch.go", "CallSite", 3, 5, 4),  # both (also in prism)
    ]}

    def oracle_factory(lang, root):
        return _FakeLspOracle(doc_syms, edges)

    result, _, adjudicate_path = build_gold(
        task, co, out_root=str(tmp_path / "gold"),
        oracle_factory=oracle_factory, prism_runner=lambda r, s: _EVIDENCE_SHAPE,
    )
    md = Path(adjudicate_path).read_text()
    assert "labels/matcher.go:4" in md          # lsp-only -> in disagreement band
    assert "verdict:" in md
    assert "labels/dispatch.go:4" in md          # both -> auto-accepted section
    assert "Auto-accepted" in md
    # The auto-accepted 'both' site must NOT carry a 'verdict:' prompt of its own —
    # only ONE verdict field total (attached to the lsp-only entry).
    assert md.count("verdict:") == 1


def test_render_adjudicate_md_without_checkout_omits_snippet():
    result_sites = [CandidateSite(file="a.go", symbol="Foo", line=1,
                                  provenance="lsp", d_member="D1")]

    class _Result:
        task_id, repo, sha, symbol = "t", "r", "s", "Sym"
        oracle_health = {"lsp": "ok", "prism": "ok"}
        sites = result_sites

    md = render_adjudicate_md(_Result())
    assert "a.go:1" in md
    assert "verdict:" in md
