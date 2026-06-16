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
    """Verify the FP rule with synthetic data.

    Class 'type_assertion' has 4 sites with these adjudications:
      site :10 -> prism_fp       -> raw_fp=1, corrected_fp=1
      site :20 -> ambiguous      -> ambiguous=1, NOT in raw/corrected
      site :30 -> oracle_artifact -> excluded from corrected_fp; not a real prism FP
                                    (neither raw_fp nor corrected_fp)
      site :40 -> (no record)    -> pending=1
    Expected: raw_fp=1, corrected_fp=1, pending=1, ambiguous=1, fanout_width=2.0
    """
    sites = [
        _site(line=10, start_byte=100, end_byte=110, fanout=2),  # prism_fp
        _site(line=20, start_byte=200, end_byte=210, fanout=2),  # ambiguous
        _site(line=30, start_byte=300, end_byte=310, fanout=2),  # oracle_artifact
        _site(line=40, start_byte=400, end_byte=410, fanout=2),  # pending (no adj)
    ]
    adjs = [
        _adj(site="main.go:10", verdict="prism_fp"),
        _adj(site="main.go:20", verdict="ambiguous"),
        _adj(site="main.go:30", verdict="oracle_artifact"),
        # no record for main.go:40
    ]
    rows = gate_report(sites, adjs, corpus="caddy", direction="callers")

    assert len(rows) == 1
    row = rows[0]
    assert row["receiver_class"] == "type_assertion"
    assert row["raw_fp"] == 1
    assert row["corrected_fp"] == 1
    assert row["pending"] == 1
    assert row["ambiguous"] == 1
    assert row["fanout_width"] == pytest.approx(2.0)
    assert row["corpus"] == "caddy"
    assert row["direction"] == "callers"


def test_gate_report_oracle_artifact_excluded_from_both_raw_and_corrected():
    """oracle_artifact contributes to NEITHER raw_fp NOR corrected_fp."""
    sites = [_site(line=10, fanout=1)]
    adjs = [_adj(site="main.go:10", verdict="oracle_artifact")]
    rows = gate_report(sites, adjs, corpus="caddy", direction="callers")
    row = rows[0]
    assert row["raw_fp"] == 0
    assert row["corrected_fp"] == 0
    assert row["pending"] == 0
    assert row["ambiguous"] == 0


def test_gate_report_fanout_width_is_mean():
    """fanout_width is mean fanout across all sites in the class."""
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
    """Adjudications from a different corpus do NOT apply to sites."""
    sites = [_site(line=10, fanout=1)]
    adjs = [
        _adj(site="main.go:10", verdict="prism_fp", corpus="tokio"),  # wrong corpus
    ]
    rows = gate_report(sites, adjs, corpus="caddy", direction="callers")
    # should be pending since the adj is for a different corpus
    assert rows[0]["pending"] == 1
    assert rows[0]["raw_fp"] == 0


def test_gate_report_direction_filter_ignores_wrong_measurement():
    """Adjudications with wrong measurement (direction) do NOT apply."""
    sites = [_site(line=10, fanout=1)]
    adjs = [
        _adj(site="main.go:10", verdict="prism_fp", measurement="callees"),  # wrong direction
    ]
    rows = gate_report(sites, adjs, corpus="caddy", direction="callers")
    assert rows[0]["pending"] == 1
    assert rows[0]["raw_fp"] == 0


def test_gate_report_empty_sites_returns_empty():
    rows = gate_report([], [], corpus="caddy", direction="callers")
    assert rows == []


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
