//! S1 (wrapped-export SPEC §5–§7): declarator counters, source epochs and serde state.
use super::js_wrapped_export_test::WRAP;
use prism::ast::ParsedFile;
use prism::js_exports::JsExportFacts;
use prism::languages::Language;

fn facts(src: &str) -> JsExportFacts {
    ParsedFile::parse("lib.tsx", src, Language::Tsx)
        .unwrap()
        .extract_js_ts_export_facts()
}

#[test]
fn t_o1_admitted_declarator_is_counted_not_skipped() {
    let f = facts(WRAP);
    assert_eq!(
        (
            f.spanned_admitted,
            f.skipped_expr_count,
            f.skipped_decl_reasons.len()
        ),
        (1, 0, 0)
    );
}
