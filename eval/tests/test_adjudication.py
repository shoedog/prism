import dataclasses
import json

import pytest

from tier_a.adjudication import (
    Adjudication,
    IllegalAdjudication,
    apply_verdicts,
    fingerprint,
    load_records,
    validate,
)


def rec(
    direction="prism_only",
    verdict="oracle_miss",
    site="src/a.rs:10",
    seed_def="src/s.rs:5",
    measurement="callers",
):
    return Adjudication(
        corpus="prism",
        measurement=measurement,
        direction=direction,
        seed_def=seed_def,
        site=site,
        verdict=verdict,
        reason="r",
        adjudicated_by="t",
        date="2026-06-11",
    )


def test_illegal_direction_verdict_combos_rejected():
    with pytest.raises(IllegalAdjudication):
        validate(rec(direction="prism_only", verdict="prism_fn"))
    with pytest.raises(IllegalAdjudication):
        validate(rec(direction="oracle_only", verdict="oracle_miss"))
    with pytest.raises(IllegalAdjudication):
        validate(rec(direction="oracle_only", verdict="prism_fp"))


def test_truth_table_transforms():
    # raw: 1 TP, prism_only sites {a,b,c,d}, oracle_only sites {e,f}
    fp = {("src/a.rs", 10), ("src/b.rs", 11), ("src/c.rs", 12), ("src/d.rs", 13)}
    fn = {("src/e.rs", 20), ("src/f.rs", 21)}
    records = [
        rec(site="src/a.rs:10", verdict="oracle_miss"),
        rec(site="src/b.rs:11", verdict="prism_fp"),
        rec(site="src/c.rs:12", verdict="oracle_artifact"),
        # src/d.rs:13 unadjudicated -> pending, excluded from corrected
        rec(site="src/e.rs:20", direction="oracle_only", verdict="prism_fn"),
        # src/f.rs:21 unadjudicated -> pending
    ]
    out = apply_verdicts(
        tp=1,
        fp_sites=fp,
        fn_sites=fn,
        records=records,
        corpus="prism",
        measurement="callers",
        seed_def="src/s.rs:5",
    )
    assert (out.tp, out.fp, out.fn) == (2, 1, 1)
    assert out.pending == 2
    assert out.oracle_miss_count == 1
    assert out.excluded == 1


def test_ambiguous_and_alias_site_are_excluded_listed():
    # the two §2.6/§2.8 routing verdicts must land in `excluded`, not FP/FN
    fp = {("src/a.rs", 10), ("src/b.rs", 11)}
    records = [
        rec(site="src/a.rs:10", verdict="ambiguous"),
        rec(site="src/b.rs:11", verdict="alias_site"),
    ]
    out = apply_verdicts(
        tp=0,
        fp_sites=fp,
        fn_sites=set(),
        records=records,
        corpus="prism",
        measurement="callers",
        seed_def="src/s.rs:5",
    )
    assert (out.fp, out.excluded, out.pending) == (0, 2, 0)


def test_stale_records_flagged_not_deleted():
    records = [rec(site="src/gone.rs:99", verdict="prism_fp")]
    out = apply_verdicts(
        tp=0,
        fp_sites=set(),
        fn_sites=set(),
        records=records,
        corpus="prism",
        measurement="callers",
        seed_def="src/s.rs:5",
    )
    assert out.stale == 1 and out.fp == 0


def test_fingerprint_is_line_number_independent():
    # Same call code, different surrounding whitespace -> same fingerprint (drift-stable).
    a = fingerprint(["  x();", "  call(arg);", "  y();"])
    b = fingerprint(["x();", "call(arg);", "y();"])
    assert a == b
    # Different call code -> different fingerprint.
    assert a != fingerprint(["x();", "DIFFERENT(arg);", "y();"])


def test_fingerprint_reanchors_stale_verdict_to_moved_site():
    # A verdict adjudicated at line 10 (now stale) re-anchors to the same call at line 14.
    fp_match = fingerprint(["    a();", "    x.foo(y);", "    b();"])
    records = [
        dataclasses.replace(
            rec(site="src/a.rs:10", verdict="prism_fp"), site_fingerprint=fp_match
        )
    ]
    out = apply_verdicts(
        tp=0,
        fp_sites={("src/a.rs", 14)},  # the call moved 10 -> 14 under churn
        fn_sites=set(),
        records=records,
        corpus="prism",
        measurement="callers",
        seed_def="src/s.rs:5",
        site_fps={"src/a.rs:14": fp_match},
    )
    assert out.fp == 1  # verdict applied at the moved site
    assert out.reanchored == 1
    assert out.stale == 0  # NOT counted stale
    assert out.pending == 0


def test_no_reanchor_when_fingerprint_differs():
    records = [
        dataclasses.replace(
            rec(site="src/a.rs:10", verdict="prism_fp"), site_fingerprint="aaaaaaaaaaaaaaaa"
        )
    ]
    out = apply_verdicts(
        tp=0,
        fp_sites={("src/a.rs", 14)},
        fn_sites=set(),
        records=records,
        corpus="prism",
        measurement="callers",
        seed_def="src/s.rs:5",
        site_fps={"src/a.rs:14": "bbbbbbbbbbbbbbbb"},
    )
    assert out.stale == 1 and out.pending == 1 and out.reanchored == 0


def test_no_reanchor_when_two_live_sites_share_one_fingerprint():
    # The review BLOCKER: one stale record must NOT re-anchor to (and double-count against)
    # two live sites sharing its fingerprint. Unique 1:1 only — else both fall to pending.
    fp = fingerprint(["    a();", "    dup();", "    b();"])
    records = [
        dataclasses.replace(
            rec(site="src/a.rs:10", verdict="prism_fp"), site_fingerprint=fp
        )
    ]
    out = apply_verdicts(
        tp=0,
        fp_sites={("src/a.rs", 20), ("src/a.rs", 30)},  # two moved sites, same fingerprint
        fn_sites=set(),
        records=records,
        corpus="prism",
        measurement="callers",
        seed_def="src/s.rs:5",
        site_fps={"src/a.rs:20": fp, "src/a.rs:30": fp},
    )
    assert out.reanchored == 0  # ambiguous live side -> no re-anchor (no double-apply)
    assert out.pending == 2  # both sites pending (conservative)
    assert out.fp == 0
    assert out.stale == 1  # the record stays stale (not consumed)


def test_jsonl_roundtrip(tmp_path):
    p = tmp_path / "adj.jsonl"
    p.write_text(json.dumps(dataclasses.asdict(rec())) + "\n")
    [r] = load_records(p)
    assert r.verdict == "oracle_miss"


def test_jsonl_accepts_optional_site_fingerprint(tmp_path):
    p = tmp_path / "adj.jsonl"
    old = dataclasses.asdict(rec())
    new = dataclasses.asdict(rec(site="src/b.rs:12"))
    new["site_fingerprint"] = "sha256:abc"
    p.write_text(json.dumps(old) + "\n" + json.dumps(new) + "\n")

    r1, r2 = load_records(p)
    assert r1.site_fingerprint is None
    assert r2.site_fingerprint == "sha256:abc"
