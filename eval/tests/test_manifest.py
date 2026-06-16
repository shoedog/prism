"""Tests for Phase-IP PR-2 manifest reader + gate report (spec §8a/§8b).

TDD: these tests were written BEFORE the implementation in eval/tier_a/manifest.py.
"""
import json

import pytest

from tier_a.adjudication import Adjudication
from tier_a.manifest import ManifestSite, gate_report, load_manifest, stratify


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _site(
    file="main.go",
    start_byte=100,
    end_byte=115,
    line=10,
    receiver_class="type_assertion",
    method="Run",
    fanout=2,
) -> ManifestSite:
    return ManifestSite(
        file=file,
        start_byte=start_byte,
        end_byte=end_byte,
        line=line,
        receiver_class=receiver_class,
        method=method,
        fanout=fanout,
    )


def _adj(
    site="main.go:10",
    verdict="prism_fp",
    direction="prism_only",
    corpus="caddy",
    measurement="callers",
) -> Adjudication:
    return Adjudication(
        corpus=corpus,
        measurement=measurement,
        direction=direction,
        seed_def="main.go:5",
        site=site,
        verdict=verdict,
        reason="test",
        adjudicated_by="t",
        date="2026-06-16",
    )


# ---------------------------------------------------------------------------
# Test 1: load_manifest parses JSON into ManifestSite; byte_key correctness
# ---------------------------------------------------------------------------

def test_load_manifest_and_byte_key(tmp_path):
    doc = {
        "sites": [
            {
                "file": "server/server.go",
                "start_byte": 120,
                "end_byte": 135,
                "line": 14,
                "receiver_class": "type_assertion",
                "method": "Go",
                "fanout": 2,
            },
            {
                "file": "cmd/main.go",
                "start_byte": 200,
                "end_byte": 210,
                "line": 25,
                "receiver_class": "typed_param",
                "method": "ServeHTTP",
                "fanout": 1,
            },
        ]
    }
    p = tmp_path / "manifest.json"
    p.write_text(json.dumps(doc))

    sites = load_manifest(p)

    assert len(sites) == 2
    s0 = sites[0]
    assert s0.file == "server/server.go"
    assert s0.start_byte == 120
    assert s0.end_byte == 135
    assert s0.line == 14
    assert s0.receiver_class == "type_assertion"
    assert s0.method == "Go"
    assert s0.fanout == 2
    # byte_key is the primary identity
    assert s0.byte_key == "server/server.go:120:135"
    # line_key is the adjudication join key
    assert s0.line_key == "server/server.go:14"

    s1 = sites[1]
    assert s1.byte_key == "cmd/main.go:200:210"
    assert s1.line_key == "cmd/main.go:25"


def test_load_manifest_empty_sites(tmp_path):
    p = tmp_path / "empty.json"
    p.write_text(json.dumps({"sites": []}))
    assert load_manifest(p) == []


def test_load_manifest_missing_sites_key(tmp_path):
    """A document with no 'sites' key returns an empty list gracefully."""
    p = tmp_path / "no_sites.json"
    p.write_text(json.dumps({}))
    assert load_manifest(p) == []


# ---------------------------------------------------------------------------
# Test 2: stratify groups by receiver_class
# ---------------------------------------------------------------------------

def test_stratify_groups_by_receiver_class():
    sites = [
        _site(receiver_class="type_assertion", start_byte=10, end_byte=20),
        _site(receiver_class="var_local", start_byte=30, end_byte=40),
        _site(receiver_class="typed_param", start_byte=50, end_byte=60),
        _site(receiver_class="type_assertion", start_byte=70, end_byte=80),  # second type_assertion
    ]
    groups = stratify(sites)

    assert set(groups.keys()) == {"type_assertion", "var_local", "typed_param"}
    assert len(groups["type_assertion"]) == 2
    assert len(groups["var_local"]) == 1
    assert len(groups["typed_param"]) == 1


def test_stratify_empty_returns_empty_dict():
    assert stratify([]) == {}


def test_stratify_preserves_all_four_classes():
    classes = ["typed_param", "constructor_local", "type_assertion", "var_local"]
    sites = [
        _site(receiver_class=c, start_byte=i * 20, end_byte=i * 20 + 10)
        for i, c in enumerate(classes)
    ]
    groups = stratify(sites)
    assert set(groups.keys()) == set(classes)
    for cls in classes:
        assert len(groups[cls]) == 1


# ---------------------------------------------------------------------------
# Test 3: gate_report FP rule — prism_fp / pending / ambiguous / oracle_artifact
# ---------------------------------------------------------------------------

def test_gate_report_fp_rule():
    """FP rule (review BLOCKER 1 — positive selection) over 4 dispatch sites (fanout>0),
    provisional (no prism_only_keys → all are raw candidates):
      :10 prism_fp   :20 ambiguous   :30 oracle_artifact   :40 (no record)
    raw_fp=4 (all dispatch), ambiguous=1, pending=1, corrected_fp=1 (only the prism_fp).
    """
    sites = [
        _site(line=10, start_byte=100, end_byte=110, fanout=2),
        _site(line=20, start_byte=200, end_byte=210, fanout=2),
        _site(line=30, start_byte=300, end_byte=310, fanout=2),
        _site(line=40, start_byte=400, end_byte=410, fanout=2),
    ]
    adjs = [
        _adj(site="main.go:10", verdict="prism_fp"),
        _adj(site="main.go:20", verdict="ambiguous"),
        _adj(site="main.go:30", verdict="oracle_artifact"),
    ]
    rows = gate_report(sites, adjs, corpus="caddy", direction="callers")
    assert len(rows) == 1
    row = rows[0]
    assert row["receiver_class"] == "type_assertion"
    assert row["dispatch_sites"] == 4
    assert row["concrete_sites"] == 0
    assert row["raw_fp"] == 4          # all dispatch sites are raw FP candidates
    assert row["ambiguous"] == 1
    assert row["pending"] == 1         # :40 has no record
    assert row["corrected_fp"] == 1    # ONLY the prism_fp (positive selection)
    assert row["fanout_width"] == pytest.approx(2.0)
    assert row["corpus"] == "caddy"
    assert row["direction"] == "callers"


def test_gate_report_oracle_miss_is_not_a_corrected_fp():
    """oracle_miss = prism found a real edge the oracle missed → a TP, NOT a corrected FP.
    It is a legal prism_only verdict, so it must be joined (not left pending)."""
    sites = [_site(line=10, fanout=2)]
    adjs = [_adj(site="main.go:10", verdict="oracle_miss")]
    row = gate_report(sites, adjs, corpus="caddy", direction="callers")[0]
    assert row["corrected_fp"] == 0    # prism was right
    assert row["raw_fp"] == 1          # still a prism-only dispatch candidate
    assert row["pending"] == 0         # oracle_miss is a recorded (legal) verdict
    assert row["ambiguous"] == 0


def test_gate_report_alias_site_is_not_a_corrected_fp():
    """alias_site is explicitly not a FP (adjudication truth table)."""
    sites = [_site(line=10, fanout=2)]
    adjs = [_adj(site="main.go:10", verdict="alias_site")]
    row = gate_report(sites, adjs, corpus="caddy", direction="callers")[0]
    assert row["corrected_fp"] == 0
    assert row["raw_fp"] == 1
    assert row["pending"] == 0         # alias_site is a recorded verdict


def test_gate_report_pending_not_counted_as_corrected_fp():
    """An unadjudicated dispatch site is pending (unknown), never a corrected FP."""
    sites = [_site(line=10, fanout=2)]
    row = gate_report(sites, [], corpus="caddy", direction="callers")[0]
    assert row["pending"] == 1
    assert row["corrected_fp"] == 0
    assert row["raw_fp"] == 1


def test_gate_report_oracle_artifact_excused_from_corrected_but_counts_raw():
    """oracle_artifact IS raw prism-only membership (raw_fp) but excused from corrected_fp."""
    sites = [_site(line=10, fanout=1)]
    adjs = [_adj(site="main.go:10", verdict="oracle_artifact")]
    row = gate_report(sites, adjs, corpus="caddy", direction="callers")[0]
    assert row["raw_fp"] == 1          # raw membership counts (review BLOCKER 1)
    assert row["corrected_fp"] == 0    # excused (not prism_fp)
    assert row["pending"] == 0         # oracle_artifact is a recorded verdict
    assert row["ambiguous"] == 0


def test_gate_report_concrete_sites_excluded_from_fp():
    """fanout==0 (concrete owner-resolved) sites are concrete_sites, never FP (review MAJOR 4)."""
    sites = [
        _site(line=10, start_byte=10, end_byte=20, fanout=0),  # concrete
        _site(line=20, start_byte=20, end_byte=30, fanout=3),  # dispatch
    ]
    row = gate_report(sites, [], corpus="caddy", direction="callers")[0]
    assert row["concrete_sites"] == 1
    assert row["dispatch_sites"] == 1
    assert row["raw_fp"] == 1          # only the dispatch site
    assert row["fanout_width"] == pytest.approx(3.0)


def test_gate_report_prism_only_keys_narrows_raw():
    """When the oracle-derived prism-only set is supplied (Slice E), the FP numerator narrows
    to it — but the denominator (dispatch_sites) is NOT narrowed (review MAJOR 2)."""
    sites = [
        _site(line=10, start_byte=10, end_byte=20, fanout=2),
        _site(line=20, start_byte=20, end_byte=30, fanout=2),
    ]
    only = {"main.go:10:20"}  # byte_key of the first site only
    row = gate_report(sites, [], corpus="caddy", direction="callers", prism_only_keys=only)[0]
    assert row["raw_fp"] == 1          # FP numerator: site :20 is not in the prism-only set
    assert row["dispatch_sites"] == 2  # denominator over ALL class dispatch sites (MAJOR 2)


def test_gate_report_fanout_width_is_mean():
    sites = [
        _site(line=10, start_byte=10, end_byte=20, fanout=4),
        _site(line=20, start_byte=20, end_byte=30, fanout=2),
    ]
    rows = gate_report(sites, [], corpus="caddy", direction="callers")
    assert rows[0]["fanout_width"] == pytest.approx(3.0)


def test_gate_report_multiple_classes_sorted():
    """gate_report returns one row per receiver_class, sorted by class name."""
    sites = [
        _site(line=10, start_byte=10, end_byte=20, receiver_class="var_local"),
        _site(line=20, start_byte=20, end_byte=30, receiver_class="type_assertion"),
        _site(line=30, start_byte=30, end_byte=40, receiver_class="typed_param"),
    ]
    rows = gate_report(sites, [], corpus="caddy", direction="callers")
    class_names = [r["receiver_class"] for r in rows]
    assert class_names == sorted(class_names)
    assert len(rows) == 3


def test_gate_report_corpus_filter_ignores_other_corpus():
    """A different-corpus adjudication does not apply; the site stays pending (still raw)."""
    sites = [_site(line=10, fanout=1)]
    adjs = [_adj(site="main.go:10", verdict="prism_fp", corpus="tokio")]
    row = gate_report(sites, adjs, corpus="caddy", direction="callers")[0]
    assert row["pending"] == 1
    assert row["raw_fp"] == 1          # raw dispatch membership regardless of adjudication


def test_gate_report_direction_filter_ignores_wrong_measurement():
    sites = [_site(line=10, fanout=1)]
    adjs = [_adj(site="main.go:10", verdict="prism_fp", measurement="callees")]
    row = gate_report(sites, adjs, corpus="caddy", direction="callers")[0]
    assert row["pending"] == 1
    assert row["raw_fp"] == 1


def test_gate_report_ignores_oracle_only_direction_record():
    """An oracle_only-direction record at the same line must not steal the verdict (MAJOR 3)."""
    sites = [_site(line=10, fanout=1)]
    adjs = [_adj(site="main.go:10", verdict="ambiguous", direction="oracle_only")]
    row = gate_report(sites, adjs, corpus="caddy", direction="callers")[0]
    assert row["pending"] == 1         # the oracle_only record is ignored
    assert row["ambiguous"] == 0


def test_gate_report_empty_sites_returns_empty():
    assert gate_report([], [], corpus="caddy", direction="callers") == []


# ---------------------------------------------------------------------------
# Test 4: byte-span vs line — distinct byte_key, same line_key
# ---------------------------------------------------------------------------

def test_byte_span_vs_line_identity():
    """Two sites with the same file:line but different byte spans are DISTINCT identities.

    byte_key (primary identity) differs; line_key (adjudication join) is the same.
    This is the core design property of ManifestSite.
    """
    # Same file, same line — different byte spans (two call expressions on the same source line)
    s1 = ManifestSite(
        file="pkg/handler.go",
        start_byte=1000,
        end_byte=1015,
        line=42,
        receiver_class="type_assertion",
        method="Handle",
        fanout=1,
    )
    s2 = ManifestSite(
        file="pkg/handler.go",
        start_byte=1020,
        end_byte=1035,
        line=42,
        receiver_class="type_assertion",
        method="Handle",
        fanout=1,
    )

    # byte_key must differ (byte-span is the primary identity)
    assert s1.byte_key != s2.byte_key
    assert s1.byte_key == "pkg/handler.go:1000:1015"
    assert s2.byte_key == "pkg/handler.go:1020:1035"

    # line_key is shared — both join to the same adjudication record
    assert s1.line_key == s2.line_key == "pkg/handler.go:42"
