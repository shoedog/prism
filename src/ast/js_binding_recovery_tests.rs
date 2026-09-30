//! S1b-3 unit rows (SPEC §7 C-42, C-49, carry-forward 9): the recovered top-level import
//! predicate and the names it may bind.
use crate::ast::js_binding_helper_tests::{parse, root_errors};
use std::collections::BTreeSet;

#[test]
fn c173_recovered_import_is_detected() {
    let src = "import { \"\\u{47}\\u{47}\" as h };\nh();\n";
    let p = parse("a.js", src);
    assert!(p.tree.root_node().has_error(), "fixture should recover");
    let errors = root_errors(&p);
    assert!(
        errors.iter().any(|e| p.js_ts_recovered_import(*e)),
        "a top-level ERROR starting `import` is recognized"
    );
}

#[test]
fn c179_import_meta_and_dynamic_import_are_not_recovered_imports() {
    for src in ["import.meta load)\n", "import('./x') load2)\n"] {
        let p = parse("a.js", src);
        assert!(
            p.tree.root_node().has_error(),
            "{src:?}: fixture should recover"
        );
        let errors = root_errors(&p);
        assert!(!errors.is_empty(), "{src:?}: expected a top-level ERROR");
        for e in errors {
            assert!(
                !p.js_ts_recovered_import(e),
                "{src:?}: import.meta/import() must not poison"
            );
        }
    }
}

#[test]
fn c189_comment_trivia_does_not_defeat_the_check() {
    let cases = ["import /* c */ .meta load)\n", "import // c\n.meta load)\n"];
    for src in cases {
        let p = parse("a.js", src);
        assert!(
            p.tree.root_node().has_error(),
            "{src:?}: fixture should recover"
        );
        let errors = root_errors(&p);
        assert!(!errors.is_empty(), "{src:?}: expected a top-level ERROR");
        for e in errors {
            assert!(
                !p.js_ts_recovered_import(e),
                "{src:?}: a comment must not turn `.meta` into an import"
            );
        }
    }
}

#[test]
fn c181_unicode_and_zwnj_names_stay_whole() {
    // `é` (e + U+0301) and a ZWNJ-joined name must not be split at the combining mark.
    let src = "import { \u{e9}\u{301} as h };\nh();\n";
    let p = parse("a.js", src);
    let errors = root_errors(&p);
    let e = *errors
        .iter()
        .find(|e| p.js_ts_recovered_import(**e))
        .expect("recovered import");
    let names = p.js_ts_recovered_import_names(e);
    assert!(
        names
            .iter()
            .any(|n| n.chars().count() > 1 || n.contains('\u{301}')),
        "the combining mark must stay attached to its base letter: {names:?}"
    );
}

#[test]
fn d_fold_collector_answer_for_names_spelled_in_a_broken_import() {
    // Fold D (gpt-6.1-sol r1 W3b): a collector-answer (not just predicate/name-extraction)
    // row for a name spelled inside a recovered top-level import, through
    // `js_ts_module_binding` — the entry point the marker actually feeds. Disabling marker
    // insertion (`js_binding_site.rs`'s program-scope arm) passed every existing test; these
    // rows exercise the classify() answer directly.
    let ascii = "import { \"\\u{47}\\u{47}\" as h };\nh();\n";
    let combining_mark = "import { \u{e9}\u{301} as h };\nh();\n";
    let zwnj = "import { a\u{200c}b as h };\nh();\n";
    let clean_import_control = "import { h } from './x';\nh();\n";
    let trivia_control = "import /* c */ { \"\\u{47}\\u{47}\" as h };\nh();\n";
    for (id, src, want_recovery) in [
        ("D-1 ASCII", ascii, true),
        ("D-2 combining mark", combining_mark, true),
        ("D-3 ZWNJ", zwnj, true),
        ("D-4 clean import control", clean_import_control, false),
        ("D-5 trivia control", trivia_control, true),
    ] {
        let p = parse("a.js", src);
        let mut cache = crate::ast::js_binding::JsBindingCache::default();
        let got = p.js_ts_module_binding("h", p.tree.root_node(), &mut cache);
        let want = if want_recovery {
            crate::ast::js_binding::JsBinding::Refused("import_parse_recovery")
        } else {
            crate::ast::js_binding::JsBinding::Import
        };
        assert_eq!(got, want, "{id}");
    }
}

#[test]
fn recovered_import_names_exclude_keywords_and_digit_led_runs() {
    let src = "import { 9x as h } from a type typeof;\nh();\n";
    let p = parse("a.js", src);
    let e = root_errors(&p)
        .into_iter()
        .find(|e| p.js_ts_recovered_import(*e))
        .expect("recovered import");
    let names = p.js_ts_recovered_import_names(e);
    for kw in ["import", "from", "as", "type", "typeof"] {
        assert!(
            !names.contains(kw),
            "{kw} must not be treated as a name: {names:?}"
        );
    }
    let digit_led: BTreeSet<_> = names
        .iter()
        .filter(|n| n.starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    assert!(digit_led.is_empty(), "digit-led runs excluded: {names:?}");
}
