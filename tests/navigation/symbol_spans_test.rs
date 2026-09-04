use prism::navigation::types::QueryError;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    session: NavigationSession,
}

fn fixture(files: &[(&str, &str)]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    for (file, source) in files {
        let path = dir.path().join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, source).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    Fixture {
        _dir: dir,
        session: NavigationSession { repo, index },
    }
}

fn source_slice<'a>(source: &'a str, span: &prism::navigation::types::Location) -> &'a str {
    &source[span.start_byte..span.end_byte]
}

#[test]
fn decorated_python_reports_outer_inner_and_utf8_byte_spans() {
    let source = "# π\r\n@decorator\r\ndef target(x):\r\n\treturn x\r\n";
    let f = fixture(&[("pkg/mod.py", source)]);

    let result =
        queries::symbol_spans(&f.session, Some("target"), Some("pkg/mod.py"), None).unwrap();

    assert_eq!(result.schema_version, "1.0");
    assert_eq!(result.query, "symbol-spans:target@pkg/mod.py");
    assert_eq!(
        source_slice(source, &result.symbol_span),
        "@decorator\r\ndef target(x):\r\n\treturn x"
    );
    assert_eq!(
        source_slice(source, result.name_span.as_ref().unwrap()),
        "target"
    );
    assert_eq!(
        source_slice(source, result.body_span.as_ref().unwrap()),
        "return x"
    );
    assert_eq!(result.symbol_span.start_byte, "# π\r\n".len());
    assert_eq!(result.symbol_span.start_line, 2);
    assert_eq!(result.symbol_span.end_line, 4);
    assert_eq!(result.insert_before.file, "pkg/mod.py");
    assert_eq!(result.insert_before.line, result.symbol_span.start_line);
    assert_eq!(result.insert_before.byte, result.symbol_span.start_byte);
    assert_eq!(result.insert_after.file, "pkg/mod.py");
    assert_eq!(result.insert_after.line, result.symbol_span.end_line);
    assert_eq!(result.insert_after.byte, result.symbol_span.end_byte);
    assert_eq!(result.indentation.symbol.as_deref(), Some(""));
    assert_eq!(result.indentation.body.as_deref(), Some("\t"));
    assert!(result.unavailable.is_empty());
    assert!(result.warnings.is_empty());

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(
        !serialized.contains("return x"),
        "coordinates must not echo source"
    );
    assert!(
        !serialized.contains("@decorator"),
        "coordinates must not echo source"
    );
}

#[test]
fn nested_rust_reports_raw_body_and_exact_indentation() {
    let source = "mod m {\n    fn target() {\n        work();\n    }\n}\n";
    let f = fixture(&[("src/lib.rs", source)]);

    let result =
        queries::symbol_spans(&f.session, Some("target"), Some("src/lib.rs"), None).unwrap();

    assert_eq!(
        source_slice(source, &result.symbol_span),
        "fn target() {\n        work();\n    }"
    );
    assert_eq!(
        source_slice(source, result.name_span.as_ref().unwrap()),
        "target"
    );
    assert_eq!(
        source_slice(source, result.body_span.as_ref().unwrap()),
        "{\n        work();\n    }"
    );
    assert_eq!(result.indentation.symbol.as_deref(), Some("    "));
    assert_eq!(result.indentation.body.as_deref(), Some("        "));
    assert!(result.unavailable.is_empty());
}

#[test]
fn same_line_body_does_not_invent_body_indentation() {
    let source = "fn target() { work(); }\n";
    let f = fixture(&[("src/lib.rs", source)]);

    let result =
        queries::symbol_spans(&f.session, Some("target"), Some("src/lib.rs"), None).unwrap();

    assert!(result.body_span.is_some());
    assert_eq!(result.indentation.body, None);
    assert_eq!(
        result
            .unavailable
            .get("indentation.body")
            .map(String::as_str),
        Some("first named body child is not preceded only by line indentation")
    );
}

#[test]
fn bodyless_java_method_reports_null_body_with_reasons() {
    let source = "abstract class A {\n    abstract void target();\n}\n";
    let f = fixture(&[("A.java", source)]);

    let result = queries::symbol_spans(&f.session, Some("target"), Some("A.java"), None).unwrap();

    assert_eq!(
        source_slice(source, &result.symbol_span),
        "abstract void target();"
    );
    assert_eq!(
        source_slice(source, result.name_span.as_ref().unwrap()),
        "target"
    );
    assert_eq!(result.body_span, None);
    assert_eq!(result.indentation.body, None);
    assert_eq!(
        result.unavailable.get("body_span").map(String::as_str),
        Some("callable grammar node has no body field")
    );
    assert_eq!(
        result
            .unavailable
            .get("indentation.body")
            .map(String::as_str),
        Some("body span unavailable")
    );
}

#[test]
fn same_name_without_file_remains_ambiguous() {
    let f = fixture(&[
        ("a.py", "def target():\n    return 1\n"),
        ("b.py", "def target():\n    return 2\n"),
    ]);

    let error = queries::symbol_spans(&f.session, Some("target"), None, None).unwrap_err();
    assert!(matches!(error, QueryError::AmbiguousSymbol { candidates } if candidates.len() == 2));
}
