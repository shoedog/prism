//! Item B (#7): JS/TS sanitizer advisory-tier language gate.
//!
//! `function_body_cleansed_for` / `cleansed_categories_for_source` feed
//! `apply_cleansers`, which marks `FlowPath.cleansed_for` for EVERY active recognizer category
//! matched anywhere in the source function body — including, pre-fix, a recognizer registered for
//! a DIFFERENT language whose `call_path` text happens to match. These tests exercise the
//! consequence at the structured `SlicingAlgorithm::Taint` sink-suppression layer: a false
//! cross-language cleanse hides a real XSS sink finding (the unsafe direction per engineering
//! doctrine 7), while a genuine same-language JS/TS cleanser must keep suppressing correctly.
//!
//! `bleach.clean` (rather than `markupsafe.escape` / bare `escape`) is used as the cross-language
//! probe: its tail (`clean`) does not collide with any `JS_TS_RECOGNIZERS` bare-name entry via
//! `call_path_matches`'s language-scoped tail-match branch (which legitimately treats any
//! `*.escape(...)` call as the JS/TS bare `escape` recognizer) — isolating the cross-table
//! exact-match bug this fix closes from that unrelated, intentional same-language behavior.

use crate::common::*;

fn run_taint_tsx(source: &str, diff_lines: BTreeSet<usize>) -> prism::slice::SliceResult {
    let path = "page.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn has_taint_sink(result: &prism::slice::SliceResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink"))
}

#[test]
fn test_js_cross_language_bleach_clean_does_not_suppress_xss_sink() {
    // `bleach.clean` is a Python-only recognizer (`languages: &[Language::Python]`); calling it by
    // name in a TSX file is nonsensical, which is exactly the wrong-language-table collision this
    // predicate must close. Pre-fix, the advisory tier ignored `languages` and marked
    // `FlowPath.cleansed_for` for XSS anyway, suppressing the real `dangerouslySetInnerHTML`
    // finding below (a false CWE sink suppression / hidden vulnerability).
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    const safe = bleach.clean(html);
    return <div dangerouslySetInnerHTML={{ __html: safe }} />;
  }
}
"#;
    let result = run_taint_tsx(source, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "a Python-only recognizer called by name in TSX must not suppress the XSS sink (got: {:#?})",
        result.findings
    );
}

#[test]
fn test_js_dompurify_sanitize_still_suppresses_xss_sink() {
    // Same-language positive pole: `DOMPurify.sanitize` is registered for JS/TS/Tsx and must keep
    // suppressing — guards against an inverted predicate silently disabling the whole
    // `JS_TS_RECOGNIZERS` table.
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    const safe = DOMPurify.sanitize(html);
    return <div dangerouslySetInnerHTML={{ __html: safe }} />;
  }
}
"#;
    let result = run_taint_tsx(source, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "DOMPurify.sanitize should still suppress the XSS sink in its own language (got: {:#?})",
        result.findings
    );
}
