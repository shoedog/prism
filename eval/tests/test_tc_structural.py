"""P3/P6: structural.py set-math scorer (spec §5). Pure set arithmetic against a
FROZEN adjudicated gold set — no LLM, no prism. P6 (dry-run harness check) lives
at the bottom: a hand-written fake arm impact JSON scored against a small
hand-written frozen gold.json fixture, asserting exact metric values."""
from __future__ import annotations
from tier_c.structural import (
    ClaimedSite, norm_symbol, score_structural, verify_site_exists, load_gold,
)


# ---------------------------------------------------------------------------
# norm_symbol
# ---------------------------------------------------------------------------

def test_norm_symbol_strips_go_receiver():
    assert norm_symbol("(*FastRegexMatcher).MatchString") == "matchstring"


def test_norm_symbol_strips_rust_scope():
    assert norm_symbol("TypeChecker::match_annotation") == "match_annotation"


def test_norm_symbol_strips_generics():
    assert norm_symbol("Foo<T>::bar") == "bar"


def test_norm_symbol_casefolds():
    assert norm_symbol("MatchString") == norm_symbol("matchstring") == "matchstring"


def test_norm_symbol_bare_name_unchanged():
    assert norm_symbol("match_annotation") == "match_annotation"


# ---------------------------------------------------------------------------
# score_structural: perfect / partial / phantom / unmatched_extra / d_recall
# ---------------------------------------------------------------------------

def _gold(sites):
    return {"task_id": "t", "repo": "r", "sha": "s", "symbol": "Sym", "sites": sites}


def _site(file, symbol, line, *, provenance="both", adjudication="real",
          d_member="none", reason=""):
    return {"file": file, "symbol": symbol, "line": line, "provenance": provenance,
            "adjudication": adjudication, "d_member": d_member, "reason": reason}


def test_perfect_match_f1_1_d_recall_1_phantom_0():
    gold = _gold([
        _site("a.go", "Foo", 10, d_member="D1"),
        _site("b.go", "Bar", 20, d_member="D2"),
    ])
    claimed = [ClaimedSite("a.go", "Foo"), ClaimedSite("b.go", "Bar")]
    r = score_structural(claimed, gold)
    assert r.file_f1 == 1.0
    assert r.symbol_f1 == 1.0
    assert r.d_recall == 1.0
    assert r.phantom == 0
    assert r.unmatched_extra == 0


def test_partial_match_recall_below_1():
    gold = _gold([
        _site("a.go", "Foo", 10, d_member="D1"),
        _site("b.go", "Bar", 20, d_member="D1"),
    ])
    claimed = [ClaimedSite("a.go", "Foo")]
    r = score_structural(claimed, gold)
    assert r.file_recall == 0.5
    assert r.symbol_recall == 0.5
    assert r.d_recall == 0.5
    assert r.file_precision == 1.0


def test_phantom_nonexistent_file_or_symbol():
    """A claimed site whose file/symbol is verified NOT to exist in the checkout
    is a phantom, not merely 'unmatched_extra'."""
    gold = _gold([_site("a.go", "Foo", 10)])
    claimed = [ClaimedSite("a.go", "Foo"), ClaimedSite("ghost.go", "Nope")]

    def verify_exists(file, symbol):
        return file != "ghost.go"

    r = score_structural(claimed, gold, verify_exists=verify_exists)
    assert r.phantom == 1
    assert r.unmatched_extra == 0


def test_claimed_real_extra_is_unmatched_extra_not_phantom():
    """A claimed site NOT in gold but verified to be REAL is unmatched_extra —
    never counted as a phantom (spec: novel-but-real claims route through
    adjudication, not a phantom penalty)."""
    gold = _gold([_site("a.go", "Foo", 10)])
    claimed = [ClaimedSite("a.go", "Foo"), ClaimedSite("c.go", "RealButUncounted")]

    def verify_exists(file, symbol):
        return True  # everything claimed is confirmed real in the checkout

    r = score_structural(claimed, gold, verify_exists=verify_exists)
    assert r.phantom == 0
    assert r.unmatched_extra == 1


def test_d_recall_isolates_d_subset():
    """d_recall must be computed ONLY over gold sites with d_member in {D1,D2};
    a claimed site outside the D subset does not inflate it, and a missed
    non-D gold site does not deflate it."""
    gold = _gold([
        _site("d1.go", "A", 1, d_member="D1"),
        _site("d2.go", "B", 2, d_member="D2"),
        _site("plain.go", "C", 3, d_member="none"),
    ])
    # Claim only the D1 file; miss D2 and the non-D site entirely.
    claimed = [ClaimedSite("d1.go", "A")]
    r = score_structural(claimed, gold)
    assert r.d_recall == 0.5   # 1 of 2 D-subset files recalled
    assert r.file_recall == 1 / 3


def test_excluded_gold_sites_are_not_counted():
    """adjudication == 'excluded' sites must be dropped from gold entirely."""
    gold = _gold([
        _site("a.go", "Foo", 10, adjudication="real"),
        _site("junk.go", "Junk", 1, adjudication="excluded"),
    ])
    claimed = [ClaimedSite("a.go", "Foo")]
    r = score_structural(claimed, gold)
    assert r.gold_size == 1
    assert r.file_recall == 1.0


def test_empty_claimed_gives_zero_recall_and_zero_precision():
    gold = _gold([_site("a.go", "Foo", 10)])
    r = score_structural([], gold)
    assert r.file_recall == 0.0
    assert r.file_precision == 0.0
    assert r.file_f1 == 0.0


def test_no_verify_exists_disables_phantom_detection():
    """Without an injected verify_exists, everything not in gold is unmatched_extra
    (no phantom claim without an existence oracle to confirm it)."""
    gold = _gold([_site("a.go", "Foo", 10)])
    claimed = [ClaimedSite("a.go", "Foo"), ClaimedSite("mystery.go", "X")]
    r = score_structural(claimed, gold)
    assert r.phantom == 0
    assert r.unmatched_extra == 1


# ---------------------------------------------------------------------------
# verify_site_exists (Checkout-backed phantom oracle)
# ---------------------------------------------------------------------------

class _FakeCheckout:
    def __init__(self, files: dict[str, str]):
        self._files = files

    def file_exists(self, rel):
        return rel in self._files

    def read_text(self, rel):
        return self._files.get(rel, "")


def test_verify_site_exists_false_for_missing_file():
    co = _FakeCheckout({"a.go": "func Foo() {}"})
    assert verify_site_exists(co, "b.go", "Foo") is False


def test_verify_site_exists_false_for_symbol_not_in_file_text():
    co = _FakeCheckout({"a.go": "func Bar() {}"})
    assert verify_site_exists(co, "a.go", "Foo") is False


def test_verify_site_exists_true_when_symbol_text_present():
    co = _FakeCheckout({"a.go": "func (*T) Foo() { return }"})
    assert verify_site_exists(co, "a.go", "(*T).Foo") is True


# ---------------------------------------------------------------------------
# load_gold
# ---------------------------------------------------------------------------

def test_load_gold_reads_json(tmp_path):
    import json
    p = tmp_path / "gold.json"
    payload = _gold([_site("a.go", "Foo", 10)])
    p.write_text(json.dumps(payload))
    loaded = load_gold(p)
    assert loaded["task_id"] == "t"
    assert loaded["sites"][0]["file"] == "a.go"


# ---------------------------------------------------------------------------
# P6 — dry-run the scorer against a hand-written FROZEN gold.json fixture
# (design §8.3: score a hand-written fake arm impact JSON against a small
# hand-written frozen gold fixture; assert exact metric values.)
# ---------------------------------------------------------------------------

FROZEN_GOLD = {
    "task_id": "example-task",
    "repo": "example-repo",
    "sha": "deadbeef",
    "symbol": "DoThing",
    "sites": [
        {"file": "src/core/engine.go", "symbol": "Run", "line": 42,
         "provenance": "both", "adjudication": "real", "d_member": "D1",
         "reason": "dispatches via interface, name never appears"},
        {"file": "src/core/engine.go", "symbol": "Stop", "line": 88,
         "provenance": "lsp", "adjudication": "real", "d_member": "D1",
         "reason": "lsp-only, confirmed real by source read"},
        {"file": "src/api/handler.go", "symbol": "Handle", "line": 15,
         "provenance": "prism", "adjudication": "real", "d_member": "D2",
         "reason": "prism-only oracle_miss, name-present but 300+ repo hits"},
        {"file": "src/util/log.go", "symbol": "Debugf", "line": 5,
         "provenance": "both", "adjudication": "real", "d_member": "none",
         "reason": "grep-findable direct caller"},
        {"file": "src/api/junk.go", "symbol": "Nope", "line": 1,
         "provenance": "prism", "adjudication": "excluded", "d_member": "none",
         "reason": "prism_fp — not actually a caller"},
    ],
}

# Fake arm output (as if produced by tier_c.impact.parse_impact_block on an
# agent's ```json impact block): recalls 3 of 4 real gold sites, misses Handle
# (the D2 site), and claims one nonexistent file (a true phantom) plus one
# extra real-but-uncounted site (unmatched_extra).
FAKE_ARM_CLAIMED = [
    ClaimedSite(file="src/core/engine.go", symbol="Run", reason="dispatch target"),
    ClaimedSite(file="src/core/engine.go", symbol="Stop", reason="lifecycle pair"),
    ClaimedSite(file="src/util/log.go", symbol="Debugf", reason="direct caller"),
    ClaimedSite(file="src/core/ghost_module.go", symbol="Phantom", reason="hallucinated"),
    ClaimedSite(file="src/core/engine.go", symbol="Restart", reason="plausible but not in gold"),
]


class _FrozenCheckout:
    """Fixed file/text universe backing the P6 phantom check: ghost_module.go
    does not exist; every other claimed file/symbol combination is real text."""
    _FILES = {
        "src/core/engine.go": "func Run() {}\nfunc Stop() {}\nfunc Restart() {}\n",
        "src/api/handler.go": "func Handle() {}\n",
        "src/util/log.go": "func Debugf(f string, a ...any) {}\n",
    }

    def file_exists(self, rel):
        return rel in self._FILES

    def read_text(self, rel):
        return self._FILES.get(rel, "")


def test_p6_dry_run_scorer_against_frozen_gold_fixture():
    co = _FrozenCheckout()
    r = score_structural(
        FAKE_ARM_CLAIMED, FROZEN_GOLD,
        verify_exists=lambda f, s: verify_site_exists(co, f, s),
    )

    # gold has 4 "real" sites (the 5th is excluded); 2 files total (engine.go, handler.go)
    # plus log.go -> 3 distinct gold files: engine.go, handler.go, log.go
    assert r.gold_size == 4

    # File-level: claimed files = {engine.go, log.go, ghost_module.go}
    # gold files = {engine.go, handler.go, log.go}
    # TP=2 (engine.go, log.go), claimed=3 distinct files, gold=3 distinct files
    assert r.file_precision == 2 / 3
    assert r.file_recall == 2 / 3
    assert r.file_f1 == 2 / 3

    # Symbol-level: claimed (file,symbol) = {(engine.go,run),(engine.go,stop),
    #   (log.go,debugf),(ghost_module.go,phantom),(engine.go,restart)} = 5 distinct
    # gold (file,symbol) = {(engine.go,run),(engine.go,stop),(handler.go,handle),
    #   (log.go,debugf)} = 4 distinct
    # TP = 3 (run, stop, debugf)
    assert r.symbol_precision == 3 / 5
    assert r.symbol_recall == 3 / 4

    # D-recall: D-subset gold files = {engine.go (D1 x2), handler.go (D2)} = 2 files
    # claimed files hit engine.go but NOT handler.go -> 1/2
    assert r.d_recall == 0.5

    # Phantom: ghost_module.go does not exist in the checkout -> 1 phantom.
    assert r.phantom == 1
    # unmatched_extra: engine.go/Restart is real (file+symbol text both exist)
    # but not in gold -> 1 unmatched_extra.
    assert r.unmatched_extra == 1
