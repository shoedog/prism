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

from tier_a.lsp_client import LspServerError, LspTimeout

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

        def _symbol_at(self, rel, _line, _char=0):
            assert rel == "good/impl.go"
            return {"container": "Impl", "start_line": 4, "end_line": 7}

        def _package_clause(self, rel):
            assert rel == "good/impl.go"
            return "good"

    identity = do.GoplsSatisfiers._identity_at(FakeGoplsAdapter(), "good/impl.go", 5)
    assert identity == _identity("Impl", "good/impl.go", [5, 8], package_clause="good")


def test_symbol_at_disambiguates_same_line_declarations_by_character():
    symbols = [
        {
            "name": "(*Left).M", "kind": 6,
            "range": {"start": {"line": 4, "character": 0},
                      "end": {"line": 4, "character": 12}},
            "selectionRange": {"start": {"line": 4, "character": 8},
                               "end": {"line": 4, "character": 9}},
        },
        {
            "name": "(*Right).M", "kind": 6,
            "range": {"start": {"line": 4, "character": 14},
                      "end": {"line": 4, "character": 27}},
            "selectionRange": {"start": {"line": 4, "character": 23},
                               "end": {"line": 4, "character": 24}},
        },
    ]

    class FakeGoplsAdapter:
        _methods = do.GoplsSatisfiers._methods
        _symbol_at = do.GoplsSatisfiers._symbol_at
        group_timeout = 1

        def __init__(self):
            self._docsym = {}
            self._symbol_details = {}
            self.client = self

        def _did_open(self, _rel):
            return True

        def _uri(self, rel):
            return f"file:///{rel}"

        def request(self, method, _params, timeout):
            assert method == "textDocument/documentSymbol"
            assert timeout == 1
            return symbols

    adapter = FakeGoplsAdapter()
    left = adapter._symbol_at("same.go", 4, 8)
    right = adapter._symbol_at("same.go", 4, 23)
    assert left["container"] == "Left"
    assert right["container"] == "Right"
    assert right["selection_start_character"] == 23


def test_package_clause_ignores_leading_comments(tmp_path):
    (tmp_path / "impl.go").write_text(
        "/*\npackage decoy\n*/\n"
        "// package another_decoy\n"
        "package real\n"
        "var note = \"package string_decoy\"\n"
        "var raw = `package raw_decoy`\n"
    )

    class FakeGoplsAdapter:
        root = str(tmp_path)

    assert do.GoplsSatisfiers._package_clause(FakeGoplsAdapter(), "impl.go") == "real"


def test_package_clause_accepts_unicode_identifier_and_requires_first_token():
    assert do.GoplsSatisfiers._package_clause_from_source(
        "/* package decoy */\npackage π\n"
    ) == "π"
    assert do.GoplsSatisfiers._package_clause_from_source(
        "var note = \"package decoy\"\npackage real\n"
    ) is None


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
    assert summary["overall"]["sound_site_rate"] == pytest.approx(2 / 3)
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
    assert cm["sound_site_rate"] == pytest.approx(1.0)
    h = per[("Handler", "ServeHTTP")]
    assert h["over_approx"] == 1
    assert h["dispatch_precision"] == pytest.approx(0.5)
    assert h["sound_site_rate"] == pytest.approx(0.0)


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
    assert summary["overall"]["sound_site_rate"] is None
    assert summary["groups"] == []
    assert summary["over_approx_sites"] == []
    assert summary["oracle_timeout_groups"] == []


def test_print_summary_labels_site_rate_and_edge_weighted_precision():
    summary = do.summarize([
        _site("good.go", 1, "I", "M", {"A", "B"}, {"A", "B"}),
        _site("bad.go", 2, "I", "M", {"C"}, set()),
    ])
    log = io.StringIO()
    do._print_summary(summary, log)
    assert (
        "overall dispatch_precision (edge-weighted) = 0.6667; "
        "sound_site_rate = 0.5000 (1/2 scored sites)"
        in log.getvalue()
    )


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

def _run_fake_oracle(
    tmp_path, *, definition, satisfiers, prism_identities,
    start_byte=8, end_byte=13, implementation_status=None, legacy=False,
    unresolved_locations=None,
):
    (tmp_path / "caller.go").write_text("adapter.Adapt()\n")
    manifest = tmp_path / "manifest.json"
    site = {
        "file": "caller.go",
        "line": 1,
        "method": "Adapt",
        "fanout": len(prism_identities),
        "implementers": [identity["name"] for identity in prism_identities],
        "start_byte": start_byte,
        "end_byte": end_byte,
    }
    if not legacy:
        site["implementer_identities"] = prism_identities
    manifest.write_text(json.dumps({"sites": [site]}))
    unresolved_locations = list(unresolved_locations or [])

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
            if satisfiers is None:
                if implementation_status is not None:
                    self._last_implementation_status = implementation_status
                return None
            return satisfiers, len(satisfiers) + len(unresolved_locations), unresolved_locations

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    return records[0], FakeGopls.implementation_calls


def _run_shared_interface_decl(tmp_path, *, identity_sets, implementation_results):
    (tmp_path / "caller.go").write_text("a.Go(); b.Go()\n")
    spans = [(0, 6), (8, 14)]
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [
        {
            "file": "caller.go", "line": 1, "method": "Go",
            "fanout": len(identities),
            "implementers": [identity["name"] for identity in identities],
            "implementer_identities": identities,
            "start_byte": spans[index][0], "end_byte": spans[index][1],
        }
        for index, identities in enumerate(identity_sets)
    ]}))

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
            return {"file": "iface.go", "line": 4, "character": 2,
                    "kind": "interface", "identity": None}

        def satisfier_identities(self, _rel, _line, _char):
            call = type(self).implementation_calls
            type(self).implementation_calls += 1
            return implementation_results[min(call, len(implementation_results) - 1)]

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    return records, FakeGopls.implementation_calls


def test_method_decl_preserves_external_definition_target(tmp_path):
    class FakeGoplsAdapter:
        group_timeout = 1
        root = str(tmp_path)

        def __init__(self):
            self.client = self

        def _did_open(self, _rel):
            return True

        def _uri(self, rel):
            return f"file:///{rel}"

        def _external_definition_kind(self, _uri, _line, _char):
            return "unknown"

        def request(self, method, _params, timeout):
            assert method == "textDocument/definition"
            assert timeout == 1
            return [{
                "uri": "file:///stdlib/src/net/http/server.go",
                "range": {"start": {"line": 52, "character": 3}},
            }]

    decl = do.GoplsSatisfiers.method_decl(FakeGoplsAdapter(), "caller.go", 1, 2)
    assert decl == {
        "file": "/stdlib/src/net/http/server.go", "line": 52, "character": 3,
        "kind": "external", "external_kind": "unknown", "identity": None,
    }


def test_external_symbol_timeout_is_cached_for_each_site(tmp_path):
    (tmp_path / "caller.go").write_text("a.Go(); b.Go()\n")
    impl = _identity("Impl", "impl.go", [3, 5])
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
         "implementers": ["Impl"], "implementer_identities": [impl],
         "start_byte": 0, "end_byte": 6},
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
         "implementers": ["Impl"], "implementer_identities": [impl],
         "start_byte": 8, "end_byte": 14},
    ]}))

    class FakeGopls:
        group_timeout = 1
        document_symbol_calls = 0
        _definition_kind = staticmethod(do.GoplsSatisfiers._definition_kind)
        method_decl = do.GoplsSatisfiers.method_decl
        _external_definition_kind = do.GoplsSatisfiers._external_definition_kind

        def __init__(self, root, *_args, **_kwargs):
            self.root = root
            self.client = self
            self._settle_s = 0
            self._external_symbol_details = {}
            self._external_symbol_status = {}

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

        def _uri(self, rel):
            return f"file:///{rel}"

        def _open_external_uri(self, _uri):
            return True

        def request(self, method, _params, timeout):
            assert timeout == 1
            if method == "textDocument/definition":
                return [{
                    "uri": "file:///stdlib/src/net/http/server.go",
                    "range": {"start": {"line": 52, "character": 3}},
                }]
            assert method == "textDocument/documentSymbol"
            type(self).document_symbol_calls += 1
            raise LspTimeout("external symbols timed out")

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    assert [record["classification"] for record in records] == [
        "oracle_timeout", "oracle_timeout",
    ]
    assert [record["oracle_status"] for record in records] == ["timeout", "timeout"]
    assert FakeGopls.document_symbol_calls == 1


def test_external_concrete_definition_with_positive_fanout_is_over_approx(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "/stdlib/src/net/http/server.go", "line": 52, "character": 3,
            "kind": "external", "external_kind": "concrete", "identity": None,
        },
        satisfiers=[],
        prism_identities=[impl],
    )
    summary = do.summarize([record])
    assert record["classification"] == "over_approx"
    assert record["definition_kind"] == "external"
    assert implementation_calls == 0
    assert summary["overall"]["external_definition_sites"] == 1


def test_external_interface_definition_queries_in_repo_implementations(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "/stdlib/src/net/http/server.go", "line": 52, "character": 3,
            "kind": "external", "external_kind": "interface", "identity": None,
        },
        satisfiers=[impl],
        prism_identities=[impl],
    )
    assert record["classification"] == "sound"
    assert record["definition_kind"] == "external"
    assert implementation_calls == 1


def test_external_concrete_zero_fanout_is_not_dispatch(tmp_path):
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "/stdlib/src/net/http/server.go", "line": 52, "character": 3,
            "kind": "external", "external_kind": "concrete", "identity": None,
        },
        satisfiers=[],
        prism_identities=[],
    )
    assert record["classification"] == "not_dispatch"
    assert record["definition_kind"] == "external"
    assert implementation_calls == 0


def test_external_definition_with_unprovable_kind_is_oracle_unresolved(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "/stdlib/src/net/http/server.go", "line": 52, "character": 3,
            "kind": "external", "external_kind": "unknown", "identity": None,
        },
        satisfiers=[],
        prism_identities=[impl],
    )
    assert record["classification"] == "oracle_unresolved"
    assert record["definition_kind"] == "external"
    assert record["failure_stage"] == "definition"
    assert implementation_calls == 0


def test_interface_decl_cache_reuses_persistent_empty_per_site(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    records, calls = _run_shared_interface_decl(
        tmp_path,
        identity_sets=[[impl], []],
        implementation_results=[([], 0, []), ([], 0, [])],
    )
    assert [record["classification"] for record in records] == ["over_approx", "sound"]
    assert [record["implementation_outcome"] for record in records] == [
        "persistent_empty", "persistent_empty",
    ]
    assert calls == 2


def test_interface_decl_cache_reuses_timeout_and_partial_mapping_outcomes(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    timeout_records, timeout_calls = _run_shared_interface_decl(
        tmp_path,
        identity_sets=[[impl], [impl]],
        implementation_results=[None],
    )
    assert [record["classification"] for record in timeout_records] == [
        "oracle_timeout", "oracle_timeout",
    ]
    assert [record["implementation_outcome"] for record in timeout_records] == [
        "timeout", "timeout",
    ]
    assert [record["failure_stage"] for record in timeout_records] == [
        "implementation", "implementation",
    ]
    assert timeout_calls == 1

    partial_records, partial_calls = _run_shared_interface_decl(
        tmp_path,
        identity_sets=[[impl], [impl]],
        implementation_results=[([impl], 1, [{
            "file": "generated.go", "line": 1, "reason": "receiver_unknown",
        }])],
    )
    assert [record["classification"] for record in partial_records] == ["sound", "sound"]
    assert [record["implementation_outcome"] for record in partial_records] == [
        "partial_mapping", "partial_mapping",
    ]
    assert partial_calls == 1


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


def test_run_oracle_concrete_zero_fanout_is_not_dispatch(tmp_path):
    adapter = _identity("Adapter", "adapter.go", [32, 64], "caddyfile")
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={
            "file": "adapter.go", "line": 31, "character": 4,
            "kind": "concrete", "identity": adapter,
        },
        satisfiers=[],
        prism_identities=[],
    )
    summary = do.summarize([record])
    assert record["classification"] == "not_dispatch"
    assert implementation_calls == 0
    assert summary["overall"]["in_scope_sites"] == 1
    assert summary["overall"]["not_dispatch_sites"] == 1
    assert summary["overall"]["scored_sites"] == 0
    assert summary["overall"]["dispatch_precision"] is None


def test_definition_timeout_remains_oracle_timeout(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"kind": "unknown", "failure_stage": "definition",
                    "oracle_status": "timeout"},
        satisfiers=[],
        prism_identities=[impl],
    )
    assert record["classification"] == "oracle_timeout"
    assert record["failure_stage"] == "definition"
    assert record["oracle_status"] == "timeout"
    assert implementation_calls == 0


def test_token_mapping_failure_is_oracle_unresolved(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"kind": "concrete", "identity": impl},
        satisfiers=[],
        prism_identities=[impl],
        start_byte=99,
        end_byte=100,
    )
    assert record["classification"] == "oracle_unresolved"
    assert record["failure_stage"] == "token"
    assert record["oracle_status"] == "unresolved"
    assert implementation_calls == 0


def test_implementation_timeout_remains_oracle_timeout(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"file": "interface.go", "line": 7, "character": 2,
                    "kind": "interface", "identity": None},
        satisfiers=None,
        prism_identities=[impl],
    )
    assert record["classification"] == "oracle_timeout"
    assert record["failure_stage"] == "implementation"
    assert record["oracle_status"] == "timeout"
    assert implementation_calls == 1


def test_implementation_server_error_remains_oracle_unresolved(tmp_path):
    impl = _identity("Impl", "impl.go", [3, 5])
    record, implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"file": "interface.go", "line": 7, "character": 2,
                    "kind": "interface", "identity": None},
        satisfiers=None,
        prism_identities=[impl],
        implementation_status="unresolved",
    )
    assert record["classification"] == "oracle_unresolved"
    assert record["failure_stage"] == "implementation"
    assert record["oracle_status"] == "unresolved"
    assert implementation_calls == 1


@pytest.mark.parametrize(
    ("error_factory", "expected_status"),
    [
        (lambda: LspTimeout("definition timed out"), "timeout"),
        (lambda: LspServerError({"code": -32603, "message": "bad definition"}),
         "unresolved"),
    ],
)
def test_method_decl_preserves_definition_stage_for_lsp_failures(
    tmp_path, error_factory, expected_status
):
    class FakeGoplsAdapter:
        group_timeout = 1
        root = str(tmp_path)

        def __init__(self):
            self.client = self

        def _did_open(self, _rel):
            return True

        def _uri(self, rel):
            return f"file:///{rel}"

        def request(self, method, _params, timeout):
            assert method == "textDocument/definition"
            assert timeout == 1
            raise error_factory()

    decl = do.GoplsSatisfiers.method_decl(FakeGoplsAdapter(), "caller.go", 0, 3)
    assert decl == {
        "kind": "unknown", "failure_stage": "definition",
        "oracle_status": expected_status,
    }


def test_method_decl_marks_malformed_definition_response_unresolved(tmp_path):
    class FakeGoplsAdapter:
        group_timeout = 1
        root = str(tmp_path)

        def __init__(self):
            self.client = self

        def _did_open(self, _rel):
            return True

        def _uri(self, rel):
            return f"file:///{rel}"

        def request(self, _method, _params, timeout):
            assert timeout == 1
            return {"uri": "file:///bad.go", "range": "not-a-range"}

    assert do.GoplsSatisfiers.method_decl(FakeGoplsAdapter(), "caller.go", 0, 3) == {
        "kind": "unknown", "failure_stage": "definition",
        "oracle_status": "unresolved",
    }


@pytest.mark.parametrize(
    ("result_or_error", "expected_status"),
    [
        (lambda: LspTimeout("implementation timed out"), "timeout"),
        (lambda: LspServerError({"code": -32603, "message": "bad implementation"}),
         "unresolved"),
        ({"malformed": True}, "unresolved"),
        ({"uri": "file:///impl.go", "range": "not-a-range"}, "unresolved"),
    ],
)
def test_satisfier_identities_preserves_implementation_stage_for_lsp_failures(
    tmp_path, result_or_error, expected_status
):
    class FakeGoplsAdapter:
        group_timeout = 1
        root = str(tmp_path)

        def __init__(self):
            self.client = self

        def _did_open(self, _rel):
            return True

        def _uri(self, rel):
            return f"file:///{rel}"

        def request(self, method, _params, timeout):
            assert method == "textDocument/implementation"
            assert timeout == 1
            if callable(result_or_error):
                raise result_or_error()
            return result_or_error

    adapter = FakeGoplsAdapter()
    assert do.GoplsSatisfiers.satisfier_identities(adapter, "interface.go", 7, 2) is None
    assert adapter._last_implementation_status == expected_status


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

        def _symbol_at(self, _rel, _line, _char=0):
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
        implementation_raw_result_count=2,
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


def test_legacy_unknown_location_blocks_prism_only_identity(tmp_path):
    hidden = _identity("Hidden", "hidden.go", [3, 5])
    record, _implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"file": "interface.go", "line": 7, "character": 2,
                    "kind": "interface", "identity": None},
        satisfiers=[],
        prism_identities=[hidden],
        legacy=True,
        unresolved_locations=[{
            "file": "generated.go", "line": 1, "reason": "receiver_unknown",
        }],
    )
    assert record["identity_mode"] == "name_only"
    assert record["classification"] == "oracle_unresolved"


def test_legacy_zero_fanout_unknown_location_is_unresolved(tmp_path):
    record, _implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"file": "interface.go", "line": 7, "character": 2,
                    "kind": "interface", "identity": None},
        satisfiers=[],
        prism_identities=[],
        legacy=True,
        unresolved_locations=[{
            "file": "generated.go", "line": 1, "reason": "receiver_unknown",
        }],
    )
    assert record["identity_mode"] == "name_only"
    assert record["classification"] == "oracle_unresolved"


def test_legacy_zero_fanout_raw_empty_result_is_sound(tmp_path):
    record, _implementation_calls = _run_fake_oracle(
        tmp_path,
        definition={"file": "interface.go", "line": 7, "character": 2,
                    "kind": "interface", "identity": None},
        satisfiers=[],
        prism_identities=[],
        legacy=True,
    )
    assert record["identity_mode"] == "name_only"
    assert record["classification"] == "sound"
    assert record["oracle_reason"] == "empty_satisfier_set"


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


def test_promoted_interface_method_location_is_unresolved_not_over_approx(tmp_path):
    (tmp_path / "caller.go").write_text("receiver.Go()\n")
    promoted = _identity("W", "wrapper.go", [3, 5])
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [{
        "file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
        "implementers": ["W"], "implementer_identities": [promoted],
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
            return [], 1, [{
                "file": "iface.go", "line": 4, "reason": "interface_location",
            }]

    records, _summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    assert records[0]["classification"] == "oracle_unresolved"
    assert records[0]["oracle_reason"] == "external_interface_promoted_ambiguous"
    assert records[0]["implementation_raw_result_count"] == 1
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


def test_run_oracle_stdout_scored_count_excludes_not_dispatch(tmp_path):
    (tmp_path / "caller.go").write_text("a.Go(); b.Go(); c.Go()\n")
    good = _identity("Good", "good.go", [3, 5])
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"sites": [
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
         "implementers": ["Good"], "implementer_identities": [good],
         "start_byte": 0, "end_byte": 6},
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 0,
         "implementers": [], "implementer_identities": [],
         "start_byte": 8, "end_byte": 14},
        {"file": "caller.go", "line": 1, "method": "Go", "fanout": 1,
         "implementers": ["Good"], "implementer_identities": [good],
         "start_byte": 16, "end_byte": 22},
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
            if char == 18:
                return {"kind": "unknown", "failure_stage": "definition",
                        "oracle_status": "timeout"}
            return {"file": "concrete.go", "line": 2, "character": char,
                    "kind": "concrete", "identity": good}

    log = io.StringIO()
    records, summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=log, oracle_factory=FakeGopls,
    )
    assert [record["classification"] for record in records] == [
        "sound", "not_dispatch", "oracle_timeout",
    ]
    assert summary["overall"]["scored_sites"] == 1
    assert (
        "scored 1/3 sites; excluded not_dispatch=1 oracle_timeout=1 "
        "oracle_unresolved=0; 0 unique interface-method declarations;"
        in log.getvalue()
    )


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
            "fully_resolved": True,
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


def test_summary_reports_fanout_positive_coverage_and_status_rates():
    first = _identity("First", "first.go", [3, 5])
    second = _identity("Second", "second.go", [3, 5])
    sound = _delta_site(fanout=2, identities=[first, second])
    observed_over_approx = _delta_site(
        fanout=1, identities=[_identity("Observed", "observed.go", [3, 5])],
        classification="over_approx",
    )
    unresolved = _delta_site(
        fanout=1, identities=[_identity("Unknown", "unknown.go", [3, 5])],
        classification="oracle_unresolved",
    )
    unresolved["failure_stage"] = "mapping"
    timeout = _delta_site(
        fanout=1, identities=[_identity("Slow", "slow.go", [3, 5])],
        classification="oracle_timeout",
    )
    for record in (sound, observed_over_approx, unresolved, timeout):
        record["definition_kind"] = "interface"
    timeout["failure_stage"] = "implementation"
    interface_zero = _delta_site(fanout=0, identities=[])
    interface_zero["definition_kind"] = "interface"
    concrete_zero = _delta_site(fanout=0, identities=[], classification="not_dispatch")
    concrete_zero["definition_kind"] = "concrete"
    unknown_definition = _delta_site(fanout=0, identities=[])
    unknown_definition["definition_kind"] = "unknown"

    summary = do.summarize([
        sound, observed_over_approx, unresolved, timeout, interface_zero,
        concrete_zero, unknown_definition,
    ])
    coverage = summary["fanout_positive_coverage"]
    assert coverage["site_coverage"] == pytest.approx(0.5)
    assert coverage["edge_coverage"] == pytest.approx(0.6)
    assert coverage["resolved_sites"] == 2
    assert coverage["total_sites"] == 4
    assert coverage["scored_identity_occurrences"] == 3
    assert coverage["identity_occurrences"] == 5
    assert summary["overall"]["not_dispatch_sites"] == 1
    assert summary["overall"]["interface_zero_fanout_sites"] == 1
    assert summary["overall"]["unknown_definition_sites"] == 1
    rates = {entry["failure_stage"]: entry for entry in summary["failure_stage_rates"]}
    assert rates["mapping"]["oracle_unresolved"] == 1
    assert rates["mapping"]["oracle_unresolved_rate"] == pytest.approx(1.0)
    assert rates["implementation"]["oracle_timeout"] == 1
    assert rates["implementation"]["oracle_timeout_rate"] == pytest.approx(1.0)


def test_delta_gate_requires_coverage_floors_even_without_delta_blocker():
    before = _delta_site(fanout=0, identities=[])
    after = _delta_site(fanout=1, identities=[_identity("Impl", "impl.go", [3, 5])])
    held = _delta_site(fanout=1, identities=[_identity("Held", "held.go", [3, 5])])
    held["file"] = "held_caller.go"
    held["start_byte"] = 200
    held["end_byte"] = 210
    held_before = dict(held)
    held["classification"] = "oracle_unresolved"
    held["failure_stage"] = "mapping"

    delta = do.delta_summary([after, held], [before, held_before])
    assert delta["blocking_sites"] == []
    assert delta["site_coverage"] == pytest.approx(0.5)
    assert delta["edge_coverage"] == pytest.approx(0.5)
    assert delta["coverage_ok"] is False
    assert delta["gate_ok"] is False

    relaxed = do.delta_summary(
        [after, held], [before, held_before],
        site_coverage_floor=0.5, edge_coverage_floor=0.5,
    )
    assert relaxed["coverage_ok"] is True
    assert relaxed["gate_ok"] is True


def test_delta_gate_requires_each_newly_exact_site_to_be_fully_resolved():
    before = _delta_site(fanout=0, identities=[])
    partial = _delta_site(
        fanout=1, identities=[_identity("Impl", "impl.go", [3, 5])],
    )
    partial["unresolved_locations"] = [{
        "file": "generated.go", "line": 1, "reason": "receiver_unknown",
    }]
    partial["implementation_outcome"] = "partial_mapping"
    delta = do.delta_summary([partial], [before])
    assert delta["gate_ok"] is False
    assert delta["blocking_sites"] == delta["newly_exact_sites"]
    assert delta["blocking_sites"][0]["classification"] == "sound"
    assert delta["blocking_sites"][0]["fully_resolved"] is False


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


# ---------------------------------------------------------------------------
# Build-constraint awareness (roadmap increment 5)
#
# gopls only type-checks the files the current build configuration selects, so a
# `//go:build extended` file (hugo scss/tocss.go) is unadjudicable under the
# default empty tag set. These pin the pure logic that derives the tag set such a
# file needs, and the fail-closed rules around adjudicating under one.
# ---------------------------------------------------------------------------

def _darwin_env():
    return do.build_env("darwin", "arm64", cgo=True, release_max=26)


def test_parse_build_expression_precedence_and_negation():
    env = _darwin_env()
    # && binds tighter than ||: pin the tree shape, not just the truth value.
    node = do.parse_build_expression("windows || linux && arm64")
    assert node == ("or", [("tag", "windows"),
                           ("and", [("tag", "linux"), ("tag", "arm64")])])
    assert do.evaluate_build_constraint(node, (), env) is False
    assert do.evaluate_build_constraint(
        do.parse_build_expression("!windows && arm64"), (), env
    ) is True
    assert do.evaluate_build_constraint(
        do.parse_build_expression("(a || b) && arm64"), ("b",), env
    ) is True


def test_parse_build_expression_rejects_malformed_input():
    for expr in ("&&", "a &&", "(a", "a)", "a b"):
        with pytest.raises(do.BuildConstraintSyntaxError):
            do.parse_build_expression(expr)


def test_evaluate_build_constraint_knows_derived_go_tags():
    env = _darwin_env()

    def holds(expr, tags=()):
        return do.evaluate_build_constraint(do.parse_build_expression(expr), tags, env)

    assert holds("unix") is True          # darwin is a unix GOOS
    assert holds("cgo") is True
    assert holds("gc") is True
    assert holds("go1.13") is True        # release tag <= toolchain
    assert holds("go1.99") is False
    assert holds("darwin && arm64") is True
    assert holds("extended") is False
    assert holds("extended", ("extended",)) is True


def test_filename_os_arch_ignores_everything_before_the_first_underscore():
    # go/build's rule: `js.go` is an ordinary file, `sync_darwin.go` is pinned.
    assert do.filename_os_arch("tpl/js.go") == (None, None)
    assert do.filename_os_arch("tpl/js_test.go") == (None, None)
    assert do.filename_os_arch("fileutil/sync_darwin.go") == ("darwin", None)
    assert do.filename_os_arch("fileutil/sync_linux_test.go") == ("linux", None)
    assert do.filename_os_arch("a/foo_linux_amd64.go") == ("linux", "amd64")
    assert do.filename_os_arch("a/foo_amd64.go") == (None, "amd64")
    assert do.filename_os_arch("a/plan.go") == (None, None)


def test_source_build_constraint_prefers_go_build_over_legacy_plus_build():
    node, raw, syntax = do.source_build_constraint(
        "//go:build extended\n// +build ignored\n\npackage scss\n"
    )
    assert (node, syntax) == (("tag", "extended"), "go:build")
    assert raw == "//go:build extended"


def test_source_build_constraint_reads_legacy_plus_build_lines():
    node, _raw, syntax = do.source_build_constraint(
        "// +build linux,amd64 darwin\n// +build !race\n\npackage a\n"
    )
    assert syntax == "+build"
    assert node == ("and", [
        ("or", [("and", [("tag", "linux"), ("tag", "amd64")]), ("tag", "darwin")]),
        ("not", ("tag", "race")),
    ])


def test_source_build_constraint_stops_at_the_package_clause():
    # A `//go:build` line after the package clause is an ordinary comment, and
    # `//go:buildx` is not a constraint directive at all.
    assert do.source_build_constraint("package a\n//go:build extended\n")[0] is None
    assert do.source_build_constraint("//go:buildextended\n\npackage a\n")[0] is None


def test_file_build_requirement_derives_the_tag_set_a_file_needs():
    extended = do.file_build_requirement(
        "resources/tocss/scss/tocss.go", "//go:build extended\n\npackage scss\n",
        _darwin_env(),
    )
    assert extended["status"] == "tags_required"
    assert extended["tags"] == ["extended"]
    assert extended["constraint"] == "//go:build extended"


def test_file_build_requirement_marks_a_satisfied_constraint_as_needing_nothing():
    env = _darwin_env()
    for source in ("//go:build !windows\n\npackage a\n",
                   "//go:build go1.13\n\npackage a\n",
                   "//go:build !slicelabels && !dedupelabels\n\npackage a\n"):
        requirement = do.file_build_requirement("a/b.go", source, env)
        assert requirement["status"] == "satisfied"
        assert requirement["tags"] == []


def test_file_build_requirement_reports_constraints_no_tag_set_can_satisfy():
    env = _darwin_env()
    # GOOS/GOARCH/cgo are decided by the pinned environment: `-tags=linux` would
    # satisfy go/build's matcher but type-check the package for the wrong platform,
    # so these are reported unadjudicable instead of adjudicated under a lie.
    assert do.file_build_requirement(
        "a/b.go", "//go:build linux\n\npackage a\n", env
    ) == {"status": "unsatisfiable", "tags": [], "reason": "no_settable_tags",
          "constraint": "//go:build linux", "syntax": "go:build"}
    assert do.file_build_requirement(
        "a/b.go", "//go:build cgo && amd64\n\npackage a\n", env
    )["reason"] == "no_settable_tags"
    filename = do.file_build_requirement("a/sync_linux.go", "package a\n", env)
    assert filename["status"] == "unsatisfiable"
    assert filename["reason"] == "filename_os_arch"


def test_file_build_requirement_reports_an_unparseable_constraint():
    requirement = do.file_build_requirement(
        "a/b.go", "//go:build a &&\n\npackage a\n", _darwin_env()
    )
    assert requirement["status"] == "unparseable"


def test_resolve_build_tags_picks_the_smallest_then_lexicographic_tag_set():
    env = _darwin_env()
    assert do.resolve_build_tags(
        do.parse_build_expression("(a && b) || c"), env
    )["tags"] == ["c"]
    assert do.resolve_build_tags(
        do.parse_build_expression("zebra || alpha"), env
    )["tags"] == ["alpha"]
    assert do.resolve_build_tags(
        do.parse_build_expression("a && b"), env
    )["tags"] == ["a", "b"]


def test_resolve_build_tags_bounds_the_search():
    env = _darwin_env()
    wide = " && ".join(f"t{index}" for index in range(do.MAX_CANDIDATE_TAGS + 1))
    resolved = do.resolve_build_tags(do.parse_build_expression(wide), env)
    assert resolved == {"status": "unsatisfiable", "tags": [],
                        "reason": "candidate_search_bounded"}


# --- planning: which unadjudicated sites a tag set can repair -----------------

def test_build_constraint_plan_groups_unadjudicated_sites_by_required_tags():
    def site(file):
        return {"file": file, "line": 1, "method": "M"}

    def record(file, classification):
        return {"file": file, "line": 1, "method": "M",
                "classification": classification}

    dispatch = [site("tagged.go"), site("plain.go"), site("tagged.go"),
                site("other.go"), site("blocked.go")]
    records = [
        record("tagged.go", "oracle_unresolved"),
        record("plain.go", "oracle_unresolved"),   # unconstrained: tags cannot help
        record("tagged.go", "sound"),              # already adjudicated: left alone
        record("other.go", "oracle_timeout"),
        record("blocked.go", "oracle_unresolved"),
    ]

    class Index:
        def requirement(self, rel):
            return {
                "tagged.go": {"status": "tags_required", "tags": ["extended"]},
                "plain.go": {"status": "unconstrained", "tags": []},
                "other.go": {"status": "tags_required", "tags": ["withdeploy"]},
                "blocked.go": {"status": "unsatisfiable", "tags": [],
                               "reason": "no_settable_tags"},
            }[rel]

    plan, blocked = do.build_constraint_plan(dispatch, records, Index())
    assert plan == {("extended",): [1], ("withdeploy",): [4]}
    assert blocked == [5]


def test_build_constraint_index_reads_real_constraints_and_tag_exclusion(tmp_path):
    for rel, source in {
        "scss/tocss.go": "//go:build extended\n\npackage scss\n",
        "scss/client_notavailable.go": "//go:build !extended\n\npackage scss\n",
        "identity/identity.go": "package identity\n",
    }.items():
        path = tmp_path / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
    index = do._BuildConstraintIndex(str(tmp_path), _darwin_env())
    assert index.requirement("scss/tocss.go")["tags"] == ["extended"]
    # `!extended` is satisfied by DEFAULT yet excluded once -tags=extended is set:
    # exclusion has to be re-evaluated per tag set, not read off the default status.
    assert index.requirement("scss/client_notavailable.go")["status"] == "satisfied"
    assert index.excluded_under("scss/client_notavailable.go", ("extended",)) is True
    assert index.excluded_under("scss/client_notavailable.go", ()) is False
    assert index.excluded_under("identity/identity.go", ("extended",)) is False
    # An unreadable header is never CLAIMED as excluded: suppressing a real
    # over_approx is worse than surfacing one.
    assert index.excluded_under("missing.go", ("extended",)) is False


# --- fail-closed comparison under an explicit tag set ------------------------

def test_compare_site_fails_closed_when_a_prism_identity_is_excluded_by_tags():
    record = do.compare_site(
        file="scss/tocss.go", line=122, interface="Manager", method="AddIdentity",
        prism_identities=[_identity("Client", "scss/client_notavailable.go", [10, 20],
                                    "scss")],
        gopls_identities=[],
        excluded_identity_files={"scss/client_notavailable.go"},
    )
    # gopls cannot see a type whose file this tag set excludes, so "prism minted a
    # non-satisfier" is not a claim this session is entitled to make.
    assert record["classification"] == "oracle_unresolved"
    assert record["unresolved_locations"] == [{
        "file": "scss/client_notavailable.go", "line": 10,
        "reason": "implementer_excluded_by_tags",
    }]
    assert record["oracle_reason"] == "implementer_excluded_by_tags"


def test_compare_site_keeps_sound_when_the_excluded_file_is_not_prism_only():
    identity = _identity("Manager", "identity/identity.go", [382, 383], "identity")
    record = do.compare_site(
        file="scss/tocss.go", line=122, interface="Manager", method="AddIdentity",
        prism_identities=[identity], gopls_identities=[identity],
        excluded_identity_files={"scss/client_notavailable.go"},
    )
    assert record["classification"] == "sound"
    assert record["unresolved_locations"] == []


# --- end to end: the second, tag-pinned pass --------------------------------

def _tagged_run(tmp_path, *, source, tagged_answer, legacy=False):
    """One site in `caller.go`; the default session never resolves it.

    `legacy=True` emits a pre-identity manifest (names only, no
    `implementer_identities`) — the compatibility path.
    """
    (tmp_path / "caller.go").write_text(source)
    manifest = tmp_path / "manifest.json"
    site = {
        "file": "caller.go",
        "line": source[:source.index("ctx")].count("\n") + 1,
        "method": "AddIdentity", "fanout": 1,
        "implementers": ["Client"] if legacy else ["nopManager"],
        "start_byte": source.index("ctx"), "end_byte": len(source.rstrip()),
    }
    if not legacy:
        site["implementer_identities"] = [
            _identity("nopManager", "identity.go", [382, 383], "identity")
        ]
    manifest.write_text(json.dumps({"sites": [site]}))

    class FakeGopls:
        sessions = []

        def __init__(self, *_args, build_tags=(), **_kwargs):
            self.build_tags = tuple(build_tags)
            self._settle_s = 0
            type(self).sessions.append(self.build_tags)

        def start(self):
            pass

        def stop(self):
            pass

        def _did_open(self, _rel):
            return True

        def _methods(self, _rel):
            return []

        def _type_at(self, _rel, _line):
            return "Manager"

        def resettle(self, **_kwargs):
            pass

        def method_decl(self, _rel, _line, _char):
            if not self.build_tags:
                return {"kind": "unknown", "failure_stage": "definition",
                        "oracle_status": "unresolved"}
            return {"file": "identity.go", "line": 280, "character": 1,
                    "kind": "interface", "identity": None}

        def satisfier_identities(self, _rel, _line, _char):
            if tagged_answer is None:
                self._last_implementation_status = "timeout"
                return None
            return tagged_answer, len(tagged_answer), []

    records, summary = do.run_oracle(
        str(manifest), str(tmp_path), ["fake-gopls"], 1,
        log=io.StringIO(), oracle_factory=FakeGopls,
    )
    return records, summary, FakeGopls.sessions


def test_run_oracle_readjudicates_a_go_build_constrained_site_under_its_tags(
    tmp_path, monkeypatch
):
    monkeypatch.setattr(do, "repo_build_env", lambda _repo: _darwin_env())
    satisfier = _identity("nopManager", "identity.go", [382, 383], "identity")
    records, summary, sessions = _tagged_run(
        tmp_path,
        source="//go:build extended\n\npackage scss\n\nctx.AddIdentity(x)\n",
        tagged_answer=[satisfier],
    )
    # The default session left it unadjudicated; the -tags=extended session settled it.
    assert sessions == [(), ("extended",)]
    assert records[0]["classification"] == "sound"
    assert records[0]["build_tags"] == ["extended"]
    assert records[0]["build_constraint"] == "//go:build extended"
    assert records[0]["build_tag_status"] == "adjudicated_under_tags"
    constraints = summary["build_constraints"]
    assert constraints["tag_sets"] == [{
        "tags": ["extended"], "files": ["caller.go"],
        "sites": 1, "adjudicated": 1, "still_unadjudicated": 0,
    }]
    assert constraints["unadjudicated_sites"] == []
    # The site is adjudicated, so it now COUNTS as covered rather than waived.
    assert summary["fanout_positive_coverage"]["site_coverage"] == 1.0
    assert summary["overall"]["oracle_unresolved"] == 0


def test_run_oracle_keeps_an_unrepaired_constrained_site_counted_and_named(
    tmp_path, monkeypatch
):
    monkeypatch.setattr(do, "repo_build_env", lambda _repo: _darwin_env())
    records, summary, sessions = _tagged_run(
        tmp_path,
        source="//go:build extended\n\npackage scss\n\nctx.AddIdentity(x)\n",
        tagged_answer=None,
    )
    assert sessions == [(), ("extended",)]
    # Still unadjudicated => still in the coverage denominator, and named.
    assert records[0]["classification"] in do.UNADJUDICATED
    assert records[0]["build_tag_status"] == "unadjudicated_under_tags"
    assert summary["fanout_positive_coverage"]["total_sites"] == 1
    assert summary["fanout_positive_coverage"]["site_coverage"] == 0.0
    assert summary["build_constraints"]["tag_sets"][0]["still_unadjudicated"] == 1
    assert summary["build_constraints"]["unadjudicated_sites"] == [{
        "file": "caller.go", "line": 5, "method": "AddIdentity", "fanout": 1,
        "classification": records[0]["classification"],
        "constraint": "//go:build extended", "constraint_status": "tags_required",
        "reason": None, "tags": ["extended"],
    }]


def test_run_oracle_names_a_site_no_tag_set_can_adjudicate(tmp_path, monkeypatch):
    monkeypatch.setattr(do, "repo_build_env", lambda _repo: _darwin_env())
    records, summary, sessions = _tagged_run(
        tmp_path,
        source="//go:build linux\n\npackage scss\n\nctx.AddIdentity(x)\n",
        tagged_answer=None,
    )
    # No second session: `linux` on darwin is not something -tags can supply.
    assert sessions == [()]
    assert records[0]["classification"] == "oracle_unresolved"
    assert records[0]["build_tag_status"] == "unsatisfiable_under_pins"
    assert summary["build_constraints"]["tag_sets"] == []
    assert summary["build_constraints"]["unadjudicated_sites"][0]["reason"] == \
        "no_settable_tags"
    assert summary["build_constraints"]["constrained_files"] == [{
        "file": "caller.go", "constraint": "//go:build linux",
        "status": "unsatisfiable", "reason": "no_settable_tags", "tags": [],
    }]


def test_run_oracle_omits_the_build_constraint_report_without_constrained_sites(
    tmp_path, monkeypatch
):
    monkeypatch.setattr(do, "repo_build_env", lambda _repo: _darwin_env())
    satisfier = _identity("nopManager", "identity.go", [382, 383], "identity")
    records, summary, sessions = _tagged_run(
        tmp_path,
        source="package scss\n\nctx.AddIdentity(x)\n",
        tagged_answer=[satisfier],
    )
    # Unconstrained file: exactly one session, no report key, byte-stable output.
    assert sessions == [()]
    assert "build_constraints" not in summary
    assert "build_tags" not in records[0]


# --- fix wave (terra r1): three constructible fail-closed gaps -----------------

def test_compare_site_fails_closed_for_a_legacy_manifest_under_build_tags():
    """WRONG 1: a name-only manifest has no file evidence, so a tag-pinned session
    cannot prove a prism-only NAME is not a satisfier — the type may live in a file
    THIS tag set excludes. Without the guard the site upgrades to over_approx."""
    tagged = do.compare_site(
        file="scss/tocss.go", line=122, interface="Manager", method="AddIdentity",
        prism_set={"Client"}, gopls_set={"Other"},
        tagged_adjudication=True,
    )
    assert tagged["identity_mode"] == "name_only"
    assert tagged["classification"] == "oracle_unresolved"
    assert tagged["oracle_reason"] == "legacy_identity_evidence_unavailable"
    assert tagged["unresolved_locations"] == [{
        "file": "scss/tocss.go", "line": 122,
        "reason": "legacy_identity_evidence_unavailable",
    }]
    # Control: the DEFAULT session compares the whole build universe, so the same
    # evidence there is a real over_approx and must stay one.
    default = do.compare_site(
        file="scss/tocss.go", line=122, interface="Manager", method="AddIdentity",
        prism_set={"Client"}, gopls_set={"Other"},
    )
    assert default["classification"] == "over_approx"
    assert default["unresolved_locations"] == []


def test_compare_site_legacy_under_tags_still_scores_a_subset_soundly():
    # Fail-closed applies only to unprovable ABSENCE; a name-only subset is still sound.
    record = do.compare_site(
        file="scss/tocss.go", line=122, interface="Manager", method="AddIdentity",
        prism_set={"Client"}, gopls_set={"Client", "Other"},
        tagged_adjudication=True,
    )
    assert record["classification"] == "sound"
    assert record["unresolved_locations"] == []


def test_run_oracle_never_upgrades_a_legacy_site_under_an_excluding_tag_set(
    tmp_path, monkeypatch
):
    """WRONG 1 end to end: old manifest mints `Client`; the site needs -tags=extended;
    the tagged session does not return `Client`. The site must stay unadjudicated
    (counted + named), never become a scored over_approx."""
    monkeypatch.setattr(do, "repo_build_env", lambda _repo: _darwin_env())
    records, summary, sessions = _tagged_run(
        tmp_path,
        source="//go:build extended\n\npackage scss\n\nctx.AddIdentity(x)\n",
        tagged_answer=[_identity("Other", "identity.go", [10, 12], "identity")],
        legacy=True,
    )
    assert sessions == [(), ("extended",)]
    assert records[0]["identity_mode"] == "name_only"
    assert records[0]["classification"] == "oracle_unresolved"
    assert summary["overall"]["over_approx"] == 0
    assert summary["build_constraints"]["tag_sets"][0] == {
        "tags": ["extended"], "files": ["caller.go"],
        "sites": 1, "adjudicated": 0, "still_unadjudicated": 1,
    }
    assert summary["build_constraints"]["unadjudicated_sites"][0]["file"] == "caller.go"


def test_source_build_constraint_requires_the_go_plus_build_separator():
    """WRONG 2: go/build demands the first comment field be exactly `+build`, so
    `// +buildextended` is an ordinary comment — not an `extended` constraint."""
    assert do.source_build_constraint("// +buildextended\n\npackage a\n")[0] is None
    assert do.source_build_constraint("// +build-extended\n\npackage a\n")[0] is None
    # The forms the go command DOES accept, including a missing space after `//`.
    assert do.source_build_constraint("// +build extended\n\npackage a\n")[0] == \
        ("tag", "extended")
    assert do.source_build_constraint("//+build extended\n\npackage a\n")[0] == \
        ("tag", "extended")
    assert do.source_build_constraint("//   +build   extended\n\npackage a\n")[0] == \
        ("tag", "extended")
    # An option-less `+build` is go/build's tag("ignore"), not "unconstrained".
    assert do.source_build_constraint("// +build\n\npackage a\n")[0] == ("tag", "ignore")


def test_file_build_requirement_ignores_a_plus_build_without_a_separator():
    requirement = do.file_build_requirement(
        "a/b.go", "// +buildextended\n\npackage a\n", _darwin_env()
    )
    assert requirement["status"] == "unconstrained"
    assert requirement["tags"] == []


def test_filename_os_arch_selection_follows_go_platform_aliases():
    """WRONG 3: go/build routes a filename's GOOS through the tag matcher, so the
    aliases apply — `foo_linux.go` IS built for GOOS=android. Literal equality
    falsely withheld coverage from files the go command selects."""
    android = do.build_env("android", "arm64", release_max=26)
    ios = do.build_env("ios", "arm64", release_max=26)
    illumos = do.build_env("illumos", "amd64", release_max=26)
    darwin = _darwin_env()

    def status(rel, env):
        return do.file_build_requirement(rel, "package a\n", env)["status"]

    assert do.filename_selects_file("a/foo_linux.go", android) is True
    assert status("a/foo_linux.go", android) == "unconstrained"
    assert do.filename_selects_file("a/foo_darwin.go", ios) is True
    assert status("a/foo_darwin.go", ios) == "unconstrained"
    assert do.filename_selects_file("a/foo_solaris.go", illumos) is True
    # The aliases are one-way and platform-specific: darwin does not select linux,
    # android does not select windows, and a wrong GOARCH still excludes.
    assert do.filename_selects_file("a/foo_linux.go", darwin) is False
    assert status("a/foo_linux.go", darwin) == "unsatisfiable"
    assert do.filename_selects_file("a/foo_windows.go", android) is False
    assert do.filename_selects_file("a/foo_linux_amd64.go", android) is False


def test_build_constraint_index_excludes_by_alias_aware_filename(tmp_path):
    (tmp_path / "foo_linux.go").write_text("package a\n")
    android = do._BuildConstraintIndex(str(tmp_path), do.build_env("android", "arm64"))
    darwin = do._BuildConstraintIndex(str(tmp_path), _darwin_env())
    assert android.excluded_under("foo_linux.go", ()) is False
    assert darwin.excluded_under("foo_linux.go", ()) is True


def test_gopls_session_scopes_build_tags_to_goflags(tmp_path, monkeypatch):
    session = do.GoplsSatisfiers(str(tmp_path), ["gopls", "serve"], 1,
                                 build_tags=("extended", "withdeploy"))
    monkeypatch.delenv("GOFLAGS", raising=False)
    assert session._goflags() == "-tags=extended,withdeploy"
    # An ambient GOFLAGS is preserved, with our tag set appended so it wins.
    monkeypatch.setenv("GOFLAGS", "-mod=mod")
    assert session._goflags() == "-mod=mod -tags=extended,withdeploy"
    assert do.GoplsSatisfiers(str(tmp_path), ["gopls", "serve"], 1).build_tags == ()
