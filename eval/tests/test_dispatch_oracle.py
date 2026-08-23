"""Tests for the gopls interface-satisfaction dispatch oracle (Phase-IP Slice E).

TDD: the pure classification + summary logic (`classify`, `dispatch_precision`,
`compare_site`, `summarize`) is unit-tested here BEFORE the implementation in
`eval/tools/dispatch_oracle.py`. The live gopls query is integration (smoke-run on
caddy), deliberately NOT unit-tested.
"""
import importlib.util
import io
import json
from pathlib import Path

import pytest

# dispatch_oracle.py lives in eval/tools/ (a CLI script, not part of the tier_a
# package), so load it by path the way other tool tests would.
_TOOL = Path(__file__).resolve().parents[1] / "tools" / "dispatch_oracle.py"
_spec = importlib.util.spec_from_file_location("dispatch_oracle", _TOOL)
do = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(do)


# ---------------------------------------------------------------------------
# classify(prism_set, gopls_set) -> sound | over_approx | recall_gap
# ---------------------------------------------------------------------------

def test_classify_sound_exact_equality():
    assert do.classify({"Fast", "Slow"}, {"Fast", "Slow"}) == "sound"


def test_classify_sound_strict_subset():
    # prism ⊂ gopls (RTA pruned to live types) — every minted edge is real => sound.
    assert do.classify({"Fast"}, {"Fast", "Slow", "Idle"}) == "sound"


def test_classify_over_approx_when_prism_has_a_non_satisfier():
    # prism mints `Bogus`, which gopls says does NOT satisfy => over_approx (FP candidate).
    assert do.classify({"Fast", "Bogus"}, {"Fast", "Slow"}) == "over_approx"


def test_classify_over_approx_takes_precedence_over_recall_gap():
    # prism\gopls = {Bogus} (non-empty) AND gopls\prism = {Slow} (non-empty).
    # over_approx wins — a false edge is the precision-relevant verdict.
    assert do.classify({"Fast", "Bogus"}, {"Fast", "Slow"}) == "over_approx"


def test_classify_nonempty_strict_subset_is_sound_not_recall_gap():
    # prism ⊊ gopls with prism non-empty: RTA-pruned but every minted edge is real =>
    # SOUND (the CaddyModule §4 verdict). The strict-subset coverage gap is recorded in
    # gopls_only_types, but the *classification* is sound, never recall_gap.
    assert do.classify({"Fast"}, {"Fast", "Slow"}) == "sound"


def test_classify_empty_prism_with_nonempty_gopls_is_recall_gap():
    # prism minted NOTHING but gopls has satisfiers => recall_gap (a genuine recall hole;
    # the only case recall_gap fires under the resolved precedence).
    assert do.classify(set(), {"Fast"}) == "recall_gap"


def test_classify_both_empty_is_sound():
    # Nothing minted, nothing to satisfy — vacuously sound (no false edges).
    assert do.classify(set(), set()) == "sound"


def test_classify_prism_nonempty_gopls_empty_is_over_approx():
    # gopls found no satisfiers but prism minted some => every minted edge is false.
    assert do.classify({"Fast"}, set()) == "over_approx"


# ---------------------------------------------------------------------------
# dispatch_precision(prism_set, gopls_set) = |prism ∩ gopls| / |prism|
# ---------------------------------------------------------------------------

def test_dispatch_precision_perfect_when_subset():
    assert do.dispatch_precision({"Fast"}, {"Fast", "Slow"}) == pytest.approx(1.0)


def test_dispatch_precision_half_when_one_of_two_is_false():
    assert do.dispatch_precision({"Fast", "Bogus"}, {"Fast", "Slow"}) == pytest.approx(0.5)


def test_dispatch_precision_zero_when_all_false():
    assert do.dispatch_precision({"A", "B"}, {"C"}) == pytest.approx(0.0)


def test_dispatch_precision_empty_prism_is_vacuously_one():
    # |prism| == 0 — no minted edges, no false edges. Vacuous precision = 1.0 so an
    # empty-set site never drags the aggregate down.
    assert do.dispatch_precision(set(), {"Fast"}) == pytest.approx(1.0)


# ---------------------------------------------------------------------------
# compare_site — per-site record assembly from prism + gopls sets
# ---------------------------------------------------------------------------

def test_compare_site_record_shape_over_approx():
    rec = do.compare_site(
        file="caddymodule/caddymodule.go",
        line=42,
        interface="caddy.Module",
        method="CaddyModule",
        prism_set={"Fast", "Bogus"},
        gopls_set={"Fast", "Slow"},
    )
    assert rec["file"] == "caddymodule/caddymodule.go"
    assert rec["line"] == 42
    assert rec["interface"] == "caddy.Module"
    assert rec["method"] == "CaddyModule"
    assert rec["prism_implementers"] == ["Bogus", "Fast"]   # sorted
    assert rec["gopls_satisfiers"] == ["Fast", "Slow"]      # sorted
    assert rec["classification"] == "over_approx"
    assert rec["prism_only_types"] == ["Bogus"]             # the offending types
    assert rec["gopls_only_types"] == ["Slow"]


def test_compare_site_sound_strict_subset_records_gopls_only():
    # Non-empty strict subset => sound; prism_only empty; the coverage gap is still
    # surfaced in gopls_only_types for the recall picture.
    rec = do.compare_site(
        file="a.go", line=1, interface="I", method="M",
        prism_set={"Fast"}, gopls_set={"Fast", "Slow"},
    )
    assert rec["classification"] == "sound"
    assert rec["prism_only_types"] == []
    assert rec["gopls_only_types"] == ["Slow"]


def test_compare_site_with_oracle_timeout_marker():
    # When the (interface, method) group timed out, gopls_set is None: the site is
    # recorded as oracle_timeout, never as a precision verdict.
    rec = do.compare_site(
        file="a.go", line=1, interface="I", method="M",
        prism_set={"Fast"}, gopls_set=None,
    )
    assert rec["classification"] == "oracle_timeout"
    assert rec["gopls_satisfiers"] is None
    assert rec["prism_implementers"] == ["Fast"]


# ---------------------------------------------------------------------------
# #14 slice 1 qualified identities — fake adapter, no live gopls
# ---------------------------------------------------------------------------

def _identity(name, file, span, package_clause="impl"):
    return {
        "name": name,
        "file": file,
        "span": span,
        "package_dir": str(Path(file).parent) if Path(file).parent != Path(".") else "",
        "package_clause": package_clause,
    }


def test_gopls_identity_at_keeps_package_and_method_target_evidence():
    class FakeGoplsAdapter:
        _definition_kind = staticmethod(do.GoplsSatisfiers._definition_kind)
        _identity_with_reason = do.GoplsSatisfiers._identity_with_reason

        def _symbol_at(self, rel, _line):
            assert rel == "good/impl.go"
            return {"container": "Impl", "start_line": 4, "end_line": 7}

        def _package_clause(self, rel):
            assert rel == "good/impl.go"
            return "good"

    identity = do.GoplsSatisfiers._identity_at(FakeGoplsAdapter(), "good/impl.go", 5)
    assert identity == _identity("Impl", "good/impl.go", [5, 8], package_clause="good")


def test_package_clause_ignores_comments_and_string_literals(tmp_path):
    (tmp_path / "impl.go").write_text(
        "/*\npackage decoy\n*/\n"
        "// package another_decoy\n"
        "var note = \"package string_decoy\"\n"
        "var raw = `package raw_decoy`\n"
        "package real\n"
    )

    class FakeGoplsAdapter:
        root = str(tmp_path)

    assert do.GoplsSatisfiers._package_clause(FakeGoplsAdapter(), "impl.go") == "real"


def test_compare_site_qualified_identity_rejects_same_named_other_package():
    rec = do.compare_site(
        file="caller.go",
        line=10,
        interface="Runner",
        method="Go",
        prism_identities=[_identity("Impl", "bad/impl.go", [10, 12], "bad")],
        gopls_identities=[_identity("Impl", "good/impl.go", [10, 12], "good")],
    )
    assert rec["identity_mode"] == "qualified"
    assert rec["classification"] == "over_approx"
    assert rec["prism_only_identities"] == [
        {"package_dir": "bad", "package_clause": "bad", "name": "Impl"}
    ]


def test_compare_site_qualified_identity_requires_exact_method_target():
    rec = do.compare_site(
        file="caller.go",
        line=10,
        interface="Runner",
        method="Go",
        prism_identities=[_identity("Impl", "impl_darwin.go", [3, 5])],
        gopls_identities=[_identity("Impl", "impl_linux.go", [3, 5])],
    )
    assert rec["classification"] == "target_mismatch"
    assert rec["target_mismatches"] == [
        {
            "identity": {"package_dir": "", "package_clause": "impl", "name": "Impl"},
            "prism_targets": [{"file": "impl_darwin.go", "span": [3, 5]}],
            "gopls_targets": [{"file": "impl_linux.go", "span": [3, 5]}],
        }
    ]


def test_compare_site_unknown_package_clause_is_oracle_unresolved():
    unresolved = _identity("Impl", "impl.go", [3, 5], package_clause=None)
    rec = do.compare_site(
        file="caller.go",
        line=10,
        interface="Runner",
        method="Go",
        prism_identities=[unresolved],
        gopls_identities=[_identity("Impl", "impl.go", [3, 5])],
    )
    assert rec["classification"] == "oracle_unresolved"


def test_summarize_marks_legacy_manifest_comparison_name_only():
    legacy = do.compare_site(
        file="caller.go",
        line=10,
        interface="Runner",
        method="Go",
        prism_set={"Impl"},
        gopls_set={"Impl"},
    )
    assert do.summarize([legacy])["identity_mode"] == "name_only"


# ---------------------------------------------------------------------------
# summarize — per-(interface,method) + overall rollup
# ---------------------------------------------------------------------------

def _site(file, line, interface, method, prism_set, gopls_set):
    return do.compare_site(file=file, line=line, interface=interface,
                           method=method, prism_set=prism_set, gopls_set=gopls_set)


def test_summarize_counts_and_overall_precision():
    sites = [
        # CaddyModule: prism = gopls exactly (sound). prism={Fast}, ∩=1, |prism|=1 -> 1.0
        _site("a.go", 1, "caddy.Module", "CaddyModule", {"Fast"}, {"Fast"}),
        # Same group, a sound STRICT subset: prism={Fast} ⊊ gopls (RTA pruned) -> still sound
        _site("a.go", 2, "caddy.Module", "CaddyModule", {"Fast"}, {"Fast", "Slow", "X"}),
        # Handler.ServeHTTP: an over_approx site. prism={H, Bogus}, ∩=1, |prism|=2 -> 0.5
        _site("b.go", 3, "Handler", "ServeHTTP", {"H", "Bogus"}, {"H"}),
    ]
    summary = do.summarize(sites)

    # overall: |prism ∩ gopls| summed / |prism| summed = (1 + 1 + 1) / (1 + 1 + 2) = 3/4
    assert summary["overall"]["dispatch_precision"] == pytest.approx(0.75)
    assert summary["overall"]["sites"] == 3
    assert summary["overall"]["sound"] == 2
    assert summary["overall"]["recall_gap"] == 0
    assert summary["overall"]["over_approx"] == 1
    assert summary["overall"]["oracle_timeout"] == 0

    # per-(interface, method)
    per = {(g["interface"], g["method"]): g for g in summary["groups"]}
    cm = per[("caddy.Module", "CaddyModule")]
    assert cm["sites"] == 2
    assert cm["sound"] == 2
    assert cm["recall_gap"] == 0
    assert cm["over_approx"] == 0
    assert cm["dispatch_precision"] == pytest.approx(1.0)   # (1+1)/(1+1)
    h = per[("Handler", "ServeHTTP")]
    assert h["over_approx"] == 1
    assert h["dispatch_precision"] == pytest.approx(0.5)


def test_summarize_lists_over_approx_sites_for_adjudication():
    sites = [
        _site("good.go", 1, "I", "M", {"Fast"}, {"Fast", "Slow"}),
        _site("bad.go", 9, "I", "M", {"Fast", "Bogus"}, {"Fast"}),
    ]
    summary = do.summarize(sites)
    oa = summary["over_approx_sites"]
    assert len(oa) == 1
    assert oa[0]["file"] == "bad.go"
    assert oa[0]["line"] == 9
    assert oa[0]["prism_only_types"] == ["Bogus"]


def test_summarize_oracle_timeout_excluded_from_precision():
    # A timed-out group contributes neither to the precision ratio nor to the
    # sound/over_approx/recall_gap tallies — only to the oracle_timeout count.
    sites = [
        _site("a.go", 1, "I", "M", {"Fast"}, {"Fast"}),          # sound, prec 1.0
        _site("b.go", 2, "Slow", "Run", {"X", "Y"}, None),       # timed out
    ]
    summary = do.summarize(sites)
    assert summary["overall"]["oracle_timeout"] == 1
    assert summary["overall"]["sound"] == 1
    # precision computed only over the resolved site: 1/1 = 1.0 (the timeout's
    # {X, Y} must NOT count as 2 false edges).
    assert summary["overall"]["dispatch_precision"] == pytest.approx(1.0)
    # the timed-out group is surfaced for re-run
    assert summary["oracle_timeout_groups"] == [{"interface": "Slow", "method": "Run"}]


def test_summarize_empty_sites():
    summary = do.summarize([])
    assert summary["overall"]["sites"] == 0
    assert summary["overall"]["scored_sites"] == 0
    assert summary["overall"]["dispatch_precision"] is None
    assert summary["groups"] == []
    assert summary["over_approx_sites"] == []
    assert summary["oracle_timeout_groups"] == []


def test_load_dispatch_sites_keeps_zero_fanout_and_scores_recall_gap(tmp_path):
    manifest = tmp_path / "manifest.json"
    zero = {
        "file": "caller.go",
        "line": 10,
        "method": "Go",
        "fanout": 0,
        "implementers": [],
        "implementer_identities": [],
    }
    fanned = {
        "file": "caller.go",
        "line": 20,
        "method": "Go",
        "fanout": 1,
        "implementers": ["Impl"],
        "implementer_identities": [_identity("Impl", "impl.go", [3, 5])],
    }
    manifest.write_text(json.dumps({"sites": [zero, fanned]}))
    assert do.load_dispatch_sites(manifest) == [zero, fanned]

    rec = do.compare_site(
        file=zero["file"],
        line=zero["line"],
        interface="Runner",
        method=zero["method"],
        prism_identities=zero["implementer_identities"],
        gopls_identities=[_identity("Impl", "impl.go", [3, 5])],
    )
    summary = do.summarize([rec])
    assert rec["classification"] == "recall_gap"
    assert summary["overall"]["recall_gap"] == 1
    assert summary["overall"]["scored_sites"] == 1


# ---------------------------------------------------------------------------
# #14 fix wave 1 — definition-kind dispatch
# ---------------------------------------------------------------------------

def _run_fake_oracle(tmp_path, *, definition, satisfiers, prism_identities):
    (tmp_path / "caller.go").write_text("adapter.Adapt()\n")
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [{
        "file": "caller.go",
        "line": 1,
        "method": "Adapt",
        "fanout": len(prism_identities),
        "implementers": [identity["name"] for identity in prism_identities],
        "implementer_identities": prism_identities,
        "start_byte": 8,
        "end_byte": 13,
    }]}))

    class FakeGopls:
        implementation_calls = 0

        def __init__(self, *_args, **_kwargs):
            self._settle_s = 0

        def start(self):
            pass

        def stop(self):
            pass

        def _did_open(self, _rel):
            return True

        def _methods(self, _rel):
            return []

        def resettle(self, **_kwargs):
            pass

        def method_decl(self, _rel, _line, _char):
            return definition

        def satisfier_identities(self, _rel, _line, _char):
            type(self).implementation_calls += 1
            return satisfiers, len(satisfiers), []

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    return records[0], FakeGopls.implementation_calls


def test_run_oracle_concrete_definition_is_singleton_ground_truth(tmp_path):
    adapter = _identity("Adapter", "adapter.go", [32, 64], "caddyfile")
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "adapter.go", "line": 31, "character": 4,
            "kind": "concrete", "identity": adapter,
        },
        satisfiers=[_identity("Adapter", "configadapters.go", [27, 27], "caddyconfig")],
        prism_identities=[adapter],
    )
    assert record["classification"] == "sound"
    assert record["definition_kind"] == "concrete"
    assert record["failure_stage"] is None
    assert implementation_calls == 0


def test_run_oracle_interface_definition_still_uses_implementation(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "interface.go", "line": 7, "character": 2,
            "kind": "interface", "identity": None,
        },
        satisfiers=[impl],
        prism_identities=[impl],
    )
    assert record["classification"] == "sound"
    assert record["definition_kind"] == "interface"
    assert record["failure_stage"] is None
    assert implementation_calls == 1


def test_interface_enclosed_location_is_not_a_concrete_satisfier():
    class FakeGoplsAdapter:
        _definition_kind = staticmethod(do.GoplsSatisfiers._definition_kind)

        def _symbol_at(self, _rel, _line):
            return {"container": "Adapter", "start_line": 26, "end_line": 26,
                    "enclosing_kind": 11}

        def _package_clause(self, _rel):
            return "caddyconfig"

    identity, reason = do.GoplsSatisfiers._identity_with_reason(
        FakeGoplsAdapter(), "configadapters.go", 26
    )
    assert identity is None
    assert reason == "interface_location"


def test_mappable_satisfier_set_is_scored_when_extra_non_candidate_is_present():
    impl = _identity("Impl", "impl.go", [3, 5])
    unmappable = {"file": "/go/src/net/http/server.go", "line": 90,
                  "reason": "outside_repo"}
    record = do.compare_site(
        file="caller.go", line=1, interface="Runner", method="Go",
        prism_identities=[impl], gopls_identities=[impl],
        unresolved_locations=[unmappable], failure_stage="mapping",
    )
    assert record["classification"] == "sound"
    assert record["unresolved_locations"] == []
    assert record["non_candidate_locations"] == [unmappable]
    assert record["failure_stage"] == "mapping"


def test_unresolved_location_blocks_only_when_it_can_hide_prism_only_identity():
    matched = _identity("Impl", "impl.go", [3, 5])
    missing = _identity("Other", "other.go", [7, 9])
    record = do.compare_site(
        file="caller.go", line=1, interface="Runner", method="Go",
        prism_identities=[matched, missing], gopls_identities=[matched],
        unresolved_locations=[{"file": "generated.go", "line": 1,
                               "reason": "receiver_unknown"}],
        failure_stage="mapping",
    )
    assert record["classification"] == "oracle_unresolved"


def test_interface_location_is_excluded_when_concrete_prism_target_is_mapped():
    impl = _identity("Impl", "impl.go", [3, 5])
    record = do.compare_site(
        file="caller.go", line=1, interface="Runner", method="Go",
        prism_identities=[impl], gopls_identities=[impl],
        unresolved_locations=[{"file": "interface.go", "line": 7,
                               "reason": "interface_location"}],
        failure_stage="mapping",
    )
    assert record["classification"] == "sound"
    assert record["unresolved_locations"] == []
    assert record["non_candidate_locations"] == [{
        "file": "interface.go", "line": 7, "reason": "interface_location",
    }]


def test_interface_only_first_result_retries_before_scoring_prism_target(tmp_path):
    (tmp_path / "caller.go").write_text("receiver.Go()\n")
    impl = _identity("Impl", "impl.go", [3, 5])
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [{
        "file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
        "implementers": ["Impl"], "implementer_identities": [impl],
        "start_byte": 9, "end_byte": 11,
    }]}))

    class FakeGopls:
        implementation_calls = 0

        def __init__(self, *_args, **_kwargs):
            self._settle_s = 0

        def start(self):
            pass

        def stop(self):
            pass

        def _did_open(self, _rel):
            return True

        def _methods(self, _rel):
            return []

        def resettle(self, **_kwargs):
            pass

        def method_decl(self, _rel, _line, _char):
            return {"file": "iface.go", "line": 3, "character": 1,
                    "kind": "interface", "identity": None}

        def satisfier_identities(self, _rel, _line, _char):
            type(self).implementation_calls += 1
            if type(self).implementation_calls == 1:
                return [], 1, [{
                    "file": "other_interface.go", "line": 3,
                    "reason": "interface_location",
                }]
            return [impl], 1, []

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    assert records[0]["classification"] == "sound"
    assert FakeGopls.implementation_calls == 2


def test_zero_fanout_empty_implementation_is_sound_with_explicit_reason(tmp_path):
    record, _implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "interface.go", "line": 7, "character": 2,
            "kind": "interface", "identity": None,
        },
        satisfiers=[],
        prism_identities=[],
    )
    assert record["classification"] == "sound"
    assert record["oracle_reason"] == "empty_satisfier_set"


def test_run_oracle_uses_each_site_byte_span_and_decl_character_in_cache(tmp_path):
    source = "i.M(); j.M()\n"
    (tmp_path / "caller.go").write_text(source)
    left = _identity("Left", "left.go", [3, 5])
    right = _identity("Right", "right.go", [3, 5])
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [
        {"file": "caller.go", "line": 1, "method": "M", "fanout": 1,
         "implementers": ["Left"], "implementer_identities": [left],
         "start_byte": 0, "end_byte": 5},
        {"file": "caller.go", "line": 1, "method": "M", "fanout": 1,
         "implementers": ["Right"], "implementer_identities": [right],
         "start_byte": 7, "end_byte": 12},
    ]}))

    class FakeGopls:
        definition_chars = []
        implementation_chars = []

        def __init__(self, *_args, **_kwargs):
            self._settle_s = 0

        def start(self):
            pass

        def stop(self):
            pass

        def _did_open(self, _rel):
            return True

        def _methods(self, _rel):
            return []

        def resettle(self, **_kwargs):
            pass

        def method_decl(self, _rel, _line, char):
            type(self).definition_chars.append(char)
            return {"file": "iface.go", "line": 8, "character": char,
                    "kind": "interface", "identity": None}

        def satisfier_identities(self, _rel, _line, char):
            type(self).implementation_chars.append(char)
            return ([left] if char == 2 else [right]), 1, []

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    assert [record["classification"] for record in records] == ["sound", "sound"]
    assert FakeGopls.definition_chars == [2, 9]
    assert FakeGopls.implementation_chars == [2, 9]


def test_run_oracle_fake_manifest_carries_end_to_end_classifications(tmp_path):
    (tmp_path / "caller.go").write_text("a.Go(); b.Go()\n")
    good = _identity("Good", "good.go", [3, 5])
    bad = _identity("Bad", "bad.go", [3, 5])
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
         "implementers": ["Good"], "implementer_identities": [good],
         "start_byte": 0, "end_byte": 6},
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
         "implementers": ["Bad"], "implementer_identities": [bad],
         "start_byte": 8, "end_byte": 14},
    ]}))

    class FakeGopls:
        def __init__(self, *_args, **_kwargs):
            self._settle_s = 0

        def start(self):
            pass

        def stop(self):
            pass

        def _did_open(self, _rel):
            return True

        def _methods(self, _rel):
            return []

        def resettle(self, **_kwargs):
            pass

        def method_decl(self, _rel, _line, char):
            return {"file": "concrete.go", "line": 2, "character": char,
                    "kind": "concrete", "identity": good}

    records, summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    assert [record["classification"] for record in records] == ["sound", "over_approx"]
    assert summary["overall"]["sound"] == 1
    assert summary["overall"]["over_approx"] == 1


def test_same_directory_external_test_package_is_a_distinct_identity():
    prism = _identity("Impl", "p/impl.go", [3, 5], "p")
    gopls = _identity("Impl", "p/impl_test.go", [3, 5], "p_test")
    record = do.compare_site(
        file="caller.go", line=1, interface="Runner", method="Go",
        prism_identities=[prism], gopls_identities=[gopls],
    )
    assert record["classification"] == "over_approx"
    assert record["prism_only_identities"] == [
        {"package_dir": "p", "package_clause": "p", "name": "Impl"}
    ]


# ---------------------------------------------------------------------------
# #14 slice 1 delta gate and environment pins
# ---------------------------------------------------------------------------

_PINS = {
    "corpus_sha": "abc123",
    "go_version": "go version go1.25.0 darwin/arm64",
    "gopls_version": "golang.org/x/tools/gopls v0.22.0",
    "GOOS": "darwin",
    "GOARCH": "arm64",
    "tags": "integration",
    "GOWORK": "/repo/go.work",
}


def _delta_site(*, fanout, identities, classification="sound"):
    gopls = identities or [_identity("Impl", "impl.go", [3, 5])]
    rec = do.compare_site(
        file="caller.go",
        line=10,
        interface="Runner",
        method="Go",
        prism_identities=identities,
        gopls_identities=gopls,
    )
    rec["fanout"] = fanout
    rec["start_byte"] = 100
    rec["end_byte"] = 110
    if classification != rec["classification"]:
        rec["classification"] = classification
    return rec


def test_delta_reports_newly_exact_fanout_and_identity_transitions():
    before = _delta_site(fanout=0, identities=[])
    after = _delta_site(fanout=1, identities=[_identity("Impl", "impl.go", [3, 5])])
    delta = do.delta_summary([after], [before])
    assert delta["gate_ok"] is True
    assert delta["newly_exact_sites"] == [
        {
            "file": "caller.go",
            "line": 10,
            "method": "Go",
            "classification": "sound",
            "reason": "fanout_0_to_positive",
            "new_implementer_identities": [_identity("Impl", "impl.go", [3, 5])],
        }
    ]

    changed = _delta_site(fanout=1, identities=[_identity("Impl", "impl_linux.go", [3, 5])])
    delta = do.delta_summary([changed], [after])
    assert delta["newly_exact_sites"][0]["reason"] == "new_implementer_identities"
    assert delta["newly_exact_sites"][0]["new_implementer_identities"] == [
        _identity("Impl", "impl_linux.go", [3, 5])
    ]


def test_delta_gate_blocks_a_timed_out_newly_exact_site():
    before = _delta_site(fanout=0, identities=[])
    timeout = _delta_site(
        fanout=1,
        identities=[_identity("Impl", "impl.go", [3, 5])],
        classification="oracle_timeout",
    )
    delta = do.delta_summary([timeout], [before])
    assert delta["gate_ok"] is False
    assert delta["blocking_sites"] == delta["newly_exact_sites"]
    assert delta["blocking_sites"][0]["classification"] == "oracle_timeout"


def test_delta_refuses_mismatched_environment_pins():
    changed_pins = dict(_PINS, GOARCH="amd64")
    with pytest.raises(ValueError, match="environment pins differ"):
        do.validate_environment_pins(_PINS, changed_pins)


def test_environment_pins_use_effective_go_env_and_refuse_unavailable(tmp_path):
    outputs = {
        ("git", "rev-parse", "HEAD"): "abc123",
        ("go", "version"): "go version go1.25.0 darwin/arm64",
        ("gopls", "version"): "golang.org/x/tools/gopls v0.22.0",
        ("go", "env", "GOOS"): "darwin",
        ("go", "env", "GOARCH"): "arm64",
        ("go", "env", "GOFLAGS"): "-tags=effective",
    }
    commands = []
    original = do._command_output
    try:
        def fake_command_output(command, _cwd, _env, **_kwargs):
            commands.append(tuple(command))
            return outputs.get(tuple(command))

        do._command_output = fake_command_output
        pins = do.environment_pins(str(tmp_path), ["gopls", "serve"])
        assert pins["tags"] == "-tags=effective"
        assert ("go", "env", "GOFLAGS") in commands
        with pytest.raises(ValueError, match="required environment pins unavailable"):
            do.validate_environment_pins(
                dict(pins, GOARCH="unavailable"),
                dict(pins, GOARCH="unavailable"),
            )
    finally:
        do._command_output = original
