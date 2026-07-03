"""Task P6a: confidence-stratified M2 reporting.

Prism nav emits a `score` on every Evidence item at M2's depth-1 hop: Exact
confidence = 1.0, NameOnly = 0.6. The upcoming P3 change emits capped NameOnly
"candidate" edges for Python/JS/TS unknown receivers instead of silently
dropping them; these tests pin that the harness can tell exact edges from
candidate ones without disturbing any existing (legacy) M2 field.
"""
from tier_a.cli import (
    _pending_for_probe,
    compute_m2_from_probes,
    recompute_metrics_from_stored,
)
from tier_a.report import render_markdown


def _probes(prism_sites, oracle_sites, stratum="U-free"):
    return {
        "_corpus": "prism",
        "callers:src/s.rs:5": {
            "outcome": "ok",
            "direction": "callers",
            "stratum": stratum,
            "seed_def": "src/s.rs:5",
            "prism_sites": prism_sites,
            "oracle_sites": oracle_sites,
        },
    }


# (a) legacy fields byte-identical to a no-score run --------------------------


def test_legacy_fields_byte_identical_with_and_without_score_metadata():
    # Same sites, same oracle -- the only difference is whether prism_sites carry
    # a score=1.0 (Exact) metadata dict. Adding score capture (P3 groundwork) must
    # not perturb raw/corrected/function/pending/shortfall for existing corpora
    # where every edge stays Exact confidence -- these are what docs/eval/tier-a/
    # baselines compare against.
    no_score = _probes(
        [["src/a.rs", 10, 10], ["src/b.rs", 99, 99]],
        [["src/a.rs", 9, 11]],
    )
    with_score = _probes(
        [
            ["src/a.rs", 10, 10, {"score": 1.0}],
            ["src/b.rs", 99, 99, {"score": 1.0}],
        ],
        [["src/a.rs", 9, 11]],
    )

    m_no_score = compute_m2_from_probes(no_score, [])["callers"]["U-free"]
    m_with_score = compute_m2_from_probes(with_score, [])["callers"]["U-free"]

    for key in ("raw", "corrected", "function", "pending", "shortfall"):
        assert m_no_score[key] == m_with_score[key], key

    # both runs still report the SAME legacy values whether or not score was
    # captured (regression pin, not just mutual equality)
    assert m_no_score["raw"]["tp"] == 1
    assert m_no_score["raw"]["fp"] == 1
    assert m_no_score["raw"]["fn"] == 0


# (b) exact / candidate tiers split correctly ---------------------------------


def test_exact_and_candidate_tiers_split_correctly():
    # oracle has two sites: A confirmed by an Exact edge, B confirmed only by a
    # NameOnly candidate edge. Each tier also has one edge that finds nothing.
    prism_sites = [
        ["src/a.rs", 10, 10, {"score": 1.0}],   # exact, matches oracle A -> tp
        ["src/x.rs", 99, 99, {"score": 1.0}],   # exact, no oracle match -> fp
        ["src/b.rs", 20, 20, {"score": 0.6}],   # candidate, matches oracle B
        ["src/y.rs", 77, 77, {"score": 0.6}],   # candidate, no oracle match
    ]
    oracle_sites = [["src/a.rs", 10, 10], ["src/b.rs", 20, 20]]

    m = compute_m2_from_probes(_probes(prism_sites, oracle_sites), [])["callers"]["U-free"]

    # legacy raw stays ALL-EDGE (candidates included) -- unchanged semantics,
    # baseline comparability preserved.
    assert (m["raw"]["tp"], m["raw"]["fp"], m["raw"]["fn"]) == (2, 2, 0)

    # exact_tier: oracle set unchanged, but only exact-tier prism edges compete --
    # B is no longer matched (only a candidate matched it) so it becomes a hole.
    exact_raw = m["exact_tier"]["raw"]
    assert (exact_raw["tp"], exact_raw["fp"], exact_raw["fn"]) == (1, 1, 1)

    # candidate_tier: informational count/confirmed/unconfirmed only (no P/R).
    assert m["candidate_tier"] == {
        "count": 2,
        "oracle_confirmed": 1,
        "oracle_unconfirmed": 1,
    }


def test_candidate_tier_pending_records_are_tagged():
    # The unconfirmed candidate site (src/y.rs:77) is a live prism_only diff site;
    # it must surface as a pending record tagged tier="candidate" so adjudication
    # can filter it, while a same-shaped exact-tier pending keeps NO tier key at
    # all (existing pendings must keep their shape byte-for-byte).
    probe = _probes(
        [
            ["src/x.rs", 99, 99, {"score": 1.0}],
            ["src/y.rs", 77, 77, {"score": 0.6}],
        ],
        [],
    )["callers:src/s.rs:5"]
    pending = _pending_for_probe(probe, "prism", [])
    by_site = {p["site"]: p for p in pending}
    assert "tier" not in by_site["src/x.rs:99"]
    assert by_site["src/y.rs:77"]["tier"] == "candidate"


# (c) None-score edges classify exact -----------------------------------------


def test_none_score_edges_classify_as_exact_tier():
    # A prism site with no metadata dict at all (the plain legacy [file, start,
    # end] triple) has score=None -- must land in exact_tier, not be silently
    # dropped from either bucket.
    m = compute_m2_from_probes(
        _probes([["src/a.rs", 10, 10]], [["src/a.rs", 10, 10]]), []
    )["callers"]["U-free"]
    assert m["exact_tier"]["raw"]["tp"] == 1
    assert m["candidate_tier"] == {"count": 0, "oracle_confirmed": 0, "oracle_unconfirmed": 0}


# (d) --report-only on an old-format run JSON doesn't crash -------------------


def test_report_only_renders_old_run_json_missing_stratified_fields():
    # A genuinely old run JSON (pre-dates even the "probes" G3 replay block) has
    # m2 stratum dicts with no exact_tier/candidate_tier keys at all. recompute is
    # an identity in this case (no "probes"); render_markdown must still render
    # (guarded with .get defaults) instead of KeyError-ing on the new keys.
    old_run = {
        "meta": {
            "corpus": "prism",
            "corpus_sha": "abc123def456",
            "corpus_dirty": False,
            "prism_sha": "abc123def456",
            "oracle": "rust-analyzer 1.94.0",
            "seed": 42,
            "harness_sha": "deadbeef",
            "date": "2026-05-01",
            "wall_s": {},
            "oracle_error_rate": 0.0,
            "sut_error_rate": 0.0,
            "baseline_invalid": False,
            "oracle_not_quiescent": False,
        },
        "m2": {
            "callers": {
                "U-free": {
                    "raw": {
                        "precision": (1.0, 0.7, 1.0),
                        "recall": (1.0, 0.7, 1.0),
                        "tp": 5, "fp": 0, "fn": 0,
                    },
                    "corrected": {
                        "precision": (1.0, 0.7, 1.0),
                        "recall": (1.0, 0.7, 1.0),
                        "tp": 5, "fp": 0, "fn": 0,
                    },
                    "pending": 0,
                    "shortfall": 0,
                },
            },
        },
    }
    replayed = recompute_metrics_from_stored(old_run)
    assert replayed == old_run  # no "probes" -> identity, exactly as today

    md = render_markdown(replayed)  # must not raise
    assert "U-free" in md
    assert "exact/candidate tier" not in md  # nothing to show for this stratum


def test_report_only_replay_on_probes_without_score_recomputes_stratified_fields():
    # The more common old-run case: probes ARE present (G3 replay applies) but
    # every stored site predates score capture. Replay must still produce the new
    # fields (all edges classify exact) and render the stratified block.
    stored = {"meta": {"corpus": "prism"}, "probes": _probes(
        [["src/a.rs", 10, 10]], [["src/a.rs", 10, 10]]
    )}
    run = recompute_metrics_from_stored(stored)
    m = run["m2"]["callers"]["U-free"]
    assert m["exact_tier"]["raw"]["tp"] == 1
    assert m["candidate_tier"]["count"] == 0

    md = render_markdown({**run, "meta": {
        "corpus": "prism", "corpus_sha": "a", "corpus_dirty": False,
        "prism_sha": "a", "oracle": "rust-analyzer", "seed": 42,
        "harness_sha": "a", "date": "2026-07-02", "wall_s": {},
        "oracle_error_rate": 0.0, "sut_error_rate": 0.0,
        "baseline_invalid": False, "oracle_not_quiescent": False,
    }})
    assert "exact/candidate tier" in md
