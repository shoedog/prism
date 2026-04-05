#[path = "../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_error_rate_clean_file() {
    let source = r#"
void foo(int x) {
    if (x > 0) {
        return;
    }
}
"#;
    let pf = ParsedFile::parse("clean.c", source, Language::C).unwrap();
    assert_eq!(pf.error_rate(), 0.0, "valid C should have 0 error rate");
}

#[test]
fn test_error_rate_with_macros() {
    // Undefined macros produce ERROR nodes in tree-sitter
    let source = r#"
void foo(void) {
    FOREACH_ITEM(list, item) {
        process(item);
    }
    LOG_DEBUG("done");
}
"#;
    let pf = ParsedFile::parse("macros.c", source, Language::C).unwrap();
    assert!(
        pf.parse_error_count > 0,
        "macro-heavy C should have parse errors, got error_count={}",
        pf.parse_error_count
    );
    assert!(
        pf.error_rate() > 0.0,
        "macro-heavy C should have non-zero error rate"
    );
}

#[test]
fn test_collect_error_lines() {
    // Use syntax that tree-sitter can't parse: a compound macro with braces
    let source = r#"
void foo(void) {
    FOREACH_ITEM(list, item) {
        process(item);
    }
    int a = 1;
}
"#;
    let pf = ParsedFile::parse("errors.c", source, Language::C).unwrap();
    if pf.parse_error_count > 0 {
        let error_lines = prism::ast::collect_error_lines(&pf.tree, 20);
        assert!(
            !error_lines.is_empty(),
            "when errors exist, collect_error_lines should find them"
        );
        for &line in &error_lines {
            assert!(line >= 1 && line <= 8, "error line {} out of range", line);
        }
    }

    // Guaranteed parse error: invalid syntax
    let bad_source = "void foo(void) { int = ; }\n";
    let pf2 = ParsedFile::parse("bad.c", bad_source, Language::C).unwrap();
    assert!(
        pf2.parse_error_count > 0,
        "invalid syntax should produce errors"
    );
    let error_lines2 = prism::ast::collect_error_lines(&pf2.tree, 20);
    assert!(
        !error_lines2.is_empty(),
        "should find error lines for invalid syntax"
    );
}

#[test]
fn test_check_parse_quality_structured() {
    // Create a file with parse errors
    let source = r#"
void foo(void) {
    FOREACH_ITEM(list, item) {
        process(item);
    }
    ANOTHER_MACRO(a, b, c);
    THIRD_MACRO {
        x = 1;
    };
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "macros.c".to_string(),
        ParsedFile::parse("macros.c", source, Language::C).unwrap(),
    );

    let (warnings, quality) = algorithms::check_parse_quality(&files);

    // Should have quality entry for the file with errors
    if let Some(fq) = quality.get("macros.c") {
        assert!(fq.error_count > 0, "should report error count");
        assert!(fq.node_count > 0, "should report node count");
        assert!(fq.error_rate > 0.0, "should report non-zero error rate");
        assert!(
            ["degraded", "poor", "unparseable"].contains(&fq.quality.as_str()),
            "quality should be degraded/poor/unparseable, got: {}",
            fq.quality
        );
        // error_lines should be populated
        assert!(
            !fq.error_lines.is_empty(),
            "should report error line numbers"
        );
    }

    // Clean file should not appear in quality map
    let clean_source = "void bar(int x) { return; }\n";
    let mut files2 = BTreeMap::new();
    files2.insert(
        "clean.c".to_string(),
        ParsedFile::parse("clean.c", clean_source, Language::C).unwrap(),
    );
    let (_, quality2) = algorithms::check_parse_quality(&files2);
    assert!(
        !quality2.contains_key("clean.c"),
        "clean file should not appear in parse_quality"
    );
}

#[test]
fn test_finding_parse_quality_annotation() {
    // Verify SliceFinding can carry parse_quality
    let finding = SliceFinding {
        algorithm: "test".to_string(),
        file: "test.c".to_string(),
        line: 1,
        severity: "info".to_string(),
        description: "test".to_string(),
        function_name: None,
        related_lines: vec![],
        related_files: vec![],
        category: None,
        parse_quality: Some("poor".to_string()),
    };
    assert_eq!(finding.parse_quality.as_deref(), Some("poor"));

    // Verify it serializes in JSON
    let json = serde_json::to_string(&finding).unwrap();
    assert!(
        json.contains("parse_quality"),
        "parse_quality should appear in JSON output"
    );

    // Verify None doesn't serialize
    let finding_clean = SliceFinding {
        parse_quality: None,
        ..finding
    };
    let json_clean = serde_json::to_string(&finding_clean).unwrap();
    assert!(
        !json_clean.contains("parse_quality"),
        "None parse_quality should be omitted from JSON"
    );
}

#[test]
fn test_cpg_file_parse_quality() {
    let clean_source = "void foo(int x) { return; }\n";
    let mut files = BTreeMap::new();
    files.insert(
        "clean.c".to_string(),
        ParsedFile::parse("clean.c", clean_source, Language::C).unwrap(),
    );

    let ctx = CpgContext::without_cpg(&files, None);
    assert_eq!(
        ctx.file_parse_quality("clean.c"),
        Some("clean"),
        "valid file should have 'clean' quality"
    );
    assert_eq!(
        ctx.file_parse_quality("nonexistent.c"),
        None,
        "missing file should return None"
    );
}
