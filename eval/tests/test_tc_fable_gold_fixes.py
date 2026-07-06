from __future__ import annotations

from pathlib import Path
import tomllib

from tier_c.structural import load_gold, score_structural


GOLD_ROOT = Path("tier_c/gold")


def _real_claims(gold: dict) -> list[dict[str, str]]:
    return [
        {"file": s["file"], "symbol": s["symbol"]}
        for s in gold["sites"]
        if s["adjudication"] == "real"
    ]


def _site_keys(gold: dict, adjudication: str) -> set[tuple[str, str]]:
    return {
        (s["file"], s["symbol"])
        for s in gold["sites"]
        if s["adjudication"] == adjudication
    }


def _site_line_keys(gold: dict, adjudication: str) -> set[tuple[str, str, int]]:
    return {
        (s["file"], s["symbol"], s["line"])
        for s in gold["sites"]
        if s["adjudication"] == adjudication
    }


def test_prometheus_matchstring_fable_bait_entries_are_scored_phantom():
    gold = load_gold(GOLD_ROOT / "prometheus-matchstring/gold.json")
    added_bait = [
        {"file": "cmd/promtool/unittest.go", "symbol": "matchesRun"},
        {"file": "model/relabel/relabel.go", "symbol": "relabel"},
        {"file": "promql/promqltest/test.go", "symbol": "CheckMatch"},
        {"file": "promql/promqltest/test_migrate.go", "symbol": "processTestFileLines"},
        {"file": "storage/remote/azuread/azuread.go", "symbol": "Validate"},
        {"file": "template/template.go", "symbol": "NewTemplateExpander"},
        {"file": "util/httputil/cors.go", "symbol": "SetCORS"},
        {"file": "util/logging/dedupe.go", "symbol": "HandleWarningHeaderWithContext"},
    ]

    excluded = _site_keys(gold, "excluded")
    assert {(s["file"], s["symbol"]) for s in added_bait} <= excluded

    bait_report = score_structural(added_bait, gold)
    assert bait_report.phantom == len(added_bait)
    assert bait_report.unmatched_extra == 0

    perfect = score_structural(_real_claims(gold), gold)
    assert (perfect.file_f1, perfect.d_recall, perfect.gold_size, perfect.d_gold_size, perfect.phantom) == (
        1.0,
        1.0,
        12,
        9,
        0,
    )


def test_prometheus_promql_walk_driver_is_neutral_not_phantom_bait():
    gold = load_gold(GOLD_ROOT / "prometheus-promql-walk/gold.json")
    excluded = _site_keys(gold, "excluded")
    assert ("promql/parser/ast.go", "Walk") not in excluded

    report = score_structural([{"file": "promql/parser/ast.go", "symbol": "Walk"}], gold)
    assert report.phantom == 0
    assert report.unmatched_extra == 1

    perfect = score_structural(_real_claims(gold), gold)
    assert (perfect.file_f1, perfect.d_recall, perfect.gold_size, perfect.d_gold_size, perfect.phantom) == (
        1.0,
        1.0,
        16,
        6,
        0,
    )


def test_caddy_requestmatcher_decl_real_and_anymatch_neutral():
    gold = load_gold(GOLD_ROOT / "caddy-requestmatcher-migration/gold.json")
    real = _site_keys(gold, "real")
    excluded = _site_keys(gold, "excluded")

    assert ("modules/caddyhttp/caddyhttp.go", "RequestMatcher") in real
    assert ("modules/caddyhttp/routes.go", "MatcherSets.AnyMatch") not in excluded

    anymatch_report = score_structural(
        [{"file": "modules/caddyhttp/routes.go", "symbol": "MatcherSets.AnyMatch"}],
        gold,
    )
    assert anymatch_report.phantom == 0
    assert anymatch_report.unmatched_extra == 1

    perfect = score_structural(_real_claims(gold), gold)
    assert (perfect.file_f1, perfect.d_recall, perfect.gold_size, perfect.d_gold_size, perfect.phantom) == (
        1.0,
        1.0,
        26,
        3,
        0,
    )


def test_caddy_scope_documents_zero_legacy_guards_and_anymatch_out():
    data = tomllib.loads(Path("tier_c/issues/structural.toml").read_text())
    caddy = next(t for t in data["task"] if t["id"] == "caddy-requestmatcher-migration")

    assert "interface guards (none remain legacy at this SHA)" in caddy["scope"]
    assert "already-migrated MatcherSets.AnyMatch" in caddy["scope"]


def test_ruff_typechecker_fable_missed_dict_hop_and_parser_bait():
    gold = load_gold(GOLD_ROOT / "ruff-typechecker-match-annotation/gold.json")
    real = _site_keys(gold, "real")
    excluded = _site_line_keys(gold, "excluded")

    assert gold["closure_summary"]["gold_size"] == 32
    assert gold["closure_summary"]["d1_count"] == 28
    assert {
        ("crates/ruff_python_semantic/src/analyze/typing.rs", "is_known_to_be_of_type_dict"),
        (
            "crates/ruff_linter/src/rules/flake8_simplify/rules/if_else_block_instead_of_dict_get.rs",
            "if_else_block_instead_of_dict_get",
        ),
        ("crates/ruff_linter/src/rules/ruff/rules/falsy_dict_get_fallback.rs", "falsy_dict_get_fallback"),
        ("crates/ruff_linter/src/rules/ruff/rules/if_key_in_dict_del.rs", "if_key_in_dict_del"),
    } <= real
    assert {
        (
            "crates/ruff_python_parser/src/parser/mod.rs",
            "SequenceMatchPatternParentheses::is_list",
            949,
        ),
        (
            "crates/ruff_python_parser/src/parser/pattern.rs",
            "Parser::parse_parenthesized_or_sequence_pattern",
            349,
        ),
        (
            "crates/ruff_python_parser/src/parser/pattern.rs",
            "Parser::parse_parenthesized_or_sequence_pattern",
            365,
        ),
    } <= excluded

    parser_bait_report = score_structural(
        [
            {
                "file": "crates/ruff_python_parser/src/parser/mod.rs",
                "symbol": "SequenceMatchPatternParentheses::is_list",
            },
            {
                "file": "crates/ruff_python_parser/src/parser/pattern.rs",
                "symbol": "Parser::parse_parenthesized_or_sequence_pattern",
            },
        ],
        gold,
    )
    assert parser_bait_report.phantom == 2
    assert parser_bait_report.unmatched_extra == 0

    perfect = score_structural(_real_claims(gold), gold)
    assert (perfect.file_f1, perfect.d_recall, perfect.gold_size, perfect.d_gold_size, perfect.phantom) == (
        1.0,
        1.0,
        32,
        28,
        0,
    )


def test_ruff_imported_qualified_name_fable_bait_entries_have_real_symbols():
    gold = load_gold(GOLD_ROOT / "ruff-imported-qualified-name/gold.json")
    excluded = _site_line_keys(gold, "excluded")

    assert gold["closure_summary"]["gold_size"] == 17
    assert gold["closure_summary"]["d2_count"] == 17
    assert not [
        site["symbol"]
        for site in gold["sites"]
        if site["adjudication"] == "excluded" and site["symbol"].startswith("n/a")
    ]
    assert {
        ("crates/ruff_linter/src/checkers/ast/analyze/statement.rs", "statement", 633),
        ("crates/ruff_linter/src/checkers/ast/analyze/statement.rs", "statement", 777),
        ("crates/ruff_linter/src/checkers/ast/analyze/statement.rs", "statement", 835),
        ("crates/ruff_linter/src/checkers/ast/analyze/statement.rs", "statement", 921),
        (
            "crates/ruff_linter/src/rules/flake8_tidy_imports/rules/banned_module_level_imports.rs",
            "BannedModuleImportPolicies::new",
            99,
        ),
        ("crates/ruff_python_semantic/src/model.rs", "resolve_qualified_name", 1073),
        ("crates/ruff_python_semantic/src/model.rs", "resolve_qualified_name", 1103),
        (
            "crates/ruff_linter/src/rules/pyupgrade/rules/unnecessary_future_import.rs",
            "is_import_required_by_isort",
            112,
        ),
        (
            "crates/ruff_linter/src/rules/pyupgrade/rules/unnecessary_future_import.rs",
            "is_import_required_by_isort",
            125,
        ),
        ("crates/ruff_workspace/src/configuration.rs", "conflicting_import_settings", 1696),
        ("crates/ruff_workspace/src/configuration.rs", "conflicting_required_import_pyi025", 1735),
        ("crates/ty_python_semantic/src/types/class.rs", "ClassType::qualified_name", 1013),
        ("crates/ty_python_semantic/src/types/diagnostic.rs", "report_invalid_method_override", 3687),
        ("crates/ty_python_semantic/src/types/diagnostic.rs", "report_overridden_final_method", 3872),
        ("crates/ty_python_semantic/src/types/diagnostic.rs", "report_overridden_final_variable", 4040),
        ("crates/ty_python_semantic/src/types/display.rs", "NamedItem::qualified_name_components", 75),
        ("crates/ty_python_semantic/src/types/display.rs", "NamedItem::qualified_name_components", 77),
        ("crates/ty_python_semantic/src/types/display.rs", "ClassDisplay::fmt_detailed", 778),
        ("crates/ty_python_semantic/src/types/display.rs", "TypeAliasDisplay::fmt_detailed", 842),
    } <= excluded

    bait_report = score_structural(
        [
            {"file": "crates/ruff_linter/src/checkers/ast/analyze/statement.rs", "symbol": "statement"},
            {"file": "crates/ruff_python_semantic/src/model.rs", "symbol": "resolve_qualified_name"},
            {"file": "crates/ruff_workspace/src/configuration.rs", "symbol": "conflicting_import_settings"},
            {"file": "crates/ty_python_semantic/src/types/display.rs", "symbol": "ClassDisplay::fmt_detailed"},
        ],
        gold,
    )
    assert bait_report.phantom == 4
    assert bait_report.unmatched_extra == 0

    perfect = score_structural(_real_claims(gold), gold)
    assert (perfect.file_f1, perfect.d_recall, perfect.gold_size, perfect.d_gold_size, perfect.phantom) == (
        1.0,
        1.0,
        17,
        16,
        0,
    )
