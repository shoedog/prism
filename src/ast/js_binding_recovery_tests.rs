//! S1b-3 unit rows (SPEC §7 C-42, C-49, carry-forward 9): the recovered top-level import
//! predicate and the names it may bind.
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;

fn parse(src: &str) -> ParsedFile {
    ParsedFile::parse("a.js", src, Language::JavaScript).unwrap()
}

fn root_errors<'a>(p: &'a ParsedFile) -> Vec<tree_sitter::Node<'a>> {
    let root = p.tree.root_node();
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|n| n.is_error())
        .collect()
}

#[test]
fn c173_recovered_import_is_detected() {
    let src = "import { \"\\u{47}\\u{47}\" as h };\nh();\n";
    let p = parse(src);
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
        let p = parse(src);
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
        let p = parse(src);
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
    let p = parse(src);
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
fn recovered_import_names_exclude_keywords_and_digit_led_runs() {
    let src = "import { 9x as h } from a type typeof;\nh();\n";
    let p = parse(src);
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
