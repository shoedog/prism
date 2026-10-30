use prism::navigation::types::{QueryError, SymbolRef};
use prism::navigation::{queries, seed, NavigationIndex, NavigationSession};
use prism::reasoning::seeds::{self, SeedRole, SeedSpec};
use prism::repo_loader::load_repo;
use serde_json::json;
use std::sync::Arc;

fn session(file: &str, source: &str) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(file), source).unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

const FIXTURES: &[(&str, &str, usize)] = &[
    ("a.py", "class Item:\n    pass\n# comment\nvalue = 1\ndef run():\n    return value\n", 6),
    ("a.rs", "struct Item;\n// comment\nconst VALUE: i32 = 1;\nfn run() -> i32 {\n    VALUE\n}\n", 5),
    ("a.ts", "class Item {}\n// comment\nconst value = 1;\nfunction run() {\n    return value;\n}\n", 5),
    ("a.go", "package sample\ntype Item struct {}\n// comment\nvar value = 1\nfunc run() int {\n    return value\n}\n", 6),
];

fn assert_error(error: QueryError, kind: &str, file: &str, line: usize) {
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({kind: {"file": file, "line": line}})
    );
}

fn no_callable(fixture: usize) {
    let (file, source, body_line) = FIXTURES[fixture];
    let s = session(file, source);
    for line in 1..body_line - 1 {
        assert_error(
            seed::resolve_fn(&s, None, None, Some(&format!("{file}:{line}"))).unwrap_err(),
            "NoEnclosingCallable",
            file,
            line,
        );
    }
}

#[test]
fn location_seed_python_no_callable() {
    no_callable(0);
}

#[test]
fn location_seed_rust_no_callable() {
    no_callable(1);
}

#[test]
fn location_seed_typescript_no_callable() {
    no_callable(2);
}

#[test]
fn location_seed_go_no_callable() {
    no_callable(3);
}

#[test]
fn location_seed_bounds_and_callable_spans() {
    for &(file, source, body_line) in FIXTURES {
        let s = session(file, source);
        for line in [0, source.lines().count() + 1, 99999] {
            assert_error(
                seed::resolve_fn(&s, None, None, Some(&format!("{file}:{line}"))).unwrap_err(),
                "LocationOutOfRange",
                file,
                line,
            );
        }
        let spans =
            queries::symbol_spans(&s, None, None, Some(&format!("{file}:{body_line}"))).unwrap();
        assert!(
            spans.symbol_span.start_line <= body_line && body_line <= spans.symbol_span.end_line
        );
        assert!(
            matches!(seed::resolve_fn(&s, None, None, Some(&format!("{file}:{body_line}"))).unwrap().symbol, SymbolRef::Function { ref name, .. } if name == "run")
        );
    }
}

#[test]
fn location_seed_ego_fallback_distinguishes_bounds_from_no_callable() {
    let s = session("a.py", "# comment\ndef run():\n    return 1\n");
    for (line, kind) in [
        (1, "NoEnclosingCallable"),
        (0, "LocationOutOfRange"),
        (4, "LocationOutOfRange"),
    ] {
        assert_error(
            queries::ego_graph(&s, None, None, Some(&format!("a.py:{line}")), 1, &[]).unwrap_err(),
            kind,
            "a.py",
            line,
        );
    }
    assert!(queries::ego_graph(&s, None, None, Some("a.py:3"), 1, &[]).is_ok());
}

#[test]
fn location_seed_empty_and_unterminated_files() {
    for (source, line, kind) in [
        ("", 1, "LocationOutOfRange"),
        ("# comment", 1, "NoEnclosingCallable"),
        ("# comment", 2, "LocationOutOfRange"),
    ] {
        let s = session("a.py", source);
        assert_error(
            seed::resolve_fn(&s, None, None, Some(&format!("a.py:{line}"))).unwrap_err(),
            kind,
            "a.py",
            line,
        );
    }
}

#[test]
fn location_seed_reasoning_reports_no_callable_for_unusable_module_lines() {
    let s = session(
        "a.py",
        "class Item:\n    pass\n# comment\ndef run():\n    # inside\n    return 1\n",
    );
    for role in [SeedRole::Source, SeedRole::Sink] {
        for line in [1, 3] {
            let spec = SeedSpec::Loc {
                file: "a.py".into(),
                line,
            };
            assert_error(
                seeds::resolve(&s, &[spec], role).unwrap_err(),
                "NoEnclosingCallable",
                "a.py",
                line,
            );
        }
        let spec = SeedSpec::Loc {
            file: "a.py".into(),
            line: 7,
        };
        assert_error(
            seeds::resolve(&s, &[spec], role).unwrap_err(),
            "LocationOutOfRange",
            "a.py",
            7,
        );
        let spec = SeedSpec::Loc {
            file: "a.py".into(),
            line: 5,
        };
        assert!(matches!(
            seeds::resolve(&s, &[spec], role),
            Err(QueryError::SymbolNotFound { .. })
        ));
    }
}

#[test]
fn location_seed_reasoning_anonymous_go_is_not_outside_a_callable() {
    let s = session(
        "a.go",
        "package sample\nvar h = func() int {\n  // inside\n\n  return 1\n}\n",
    );
    for role in [SeedRole::Source, SeedRole::Sink] {
        for line in [3, 4, 5] {
            let spec = SeedSpec::Loc {
                file: "a.go".into(),
                line,
            };
            assert!(matches!(
                seeds::resolve(&s, &[spec], role),
                Err(QueryError::SymbolNotFound { .. })
            ));
        }
    }
}

#[test]
fn location_seed_reasoning_js_ts_module_and_callback_are_taint_regions() {
    for file in ["a.js", "a.ts", "a.tsx"] {
        let s = session(
            file,
            "// module\nsetTimeout(function () {\n  // inside\n\n  return 1;\n}, 10);\n",
        );
        for role in [SeedRole::Source, SeedRole::Sink] {
            for line in [1, 3, 4, 5] {
                let spec = SeedSpec::Loc {
                    file: file.into(),
                    line,
                };
                assert!(matches!(
                    seeds::resolve(&s, &[spec], role),
                    Err(QueryError::SymbolNotFound { .. })
                ));
            }
        }
    }
}

#[test]
fn location_seed_parse_failure_and_named_outer_controls() {
    let s = session("a.py", "# module\nvalue = 1\ndef broken(:\n    ???\n");
    for line in [3, 4] {
        assert_error(
            seed::resolve_fn(&s, None, None, Some(&format!("a.py:{line}"))).unwrap_err(),
            "NoEnclosingCallable",
            "a.py",
            line,
        );
    }
    // Tree-sitter can recover the same malformed signature when the body is valid.
    let recovered = session("a.py", "# module\nvalue = 1\ndef broken(:\n    pass\n");
    assert!(
        matches!(seed::resolve_fn(&recovered, None, None, Some("a.py:4")).unwrap().symbol,
        SymbolRef::Function { ref name, .. } if name == "broken")
    );
    let s = session(
        "a.js",
        "function outer() {\n  setTimeout(() => {\n    // inside\n\n    return 1;\n  }, 10);\n}\n",
    );
    for line in [3, 4, 5] {
        assert!(
            matches!(seed::resolve_fn(&s, None, None, Some(&format!("a.js:{line}"))).unwrap().symbol,
            SymbolRef::Function { ref name, .. } if name == "outer")
        );
    }
}
