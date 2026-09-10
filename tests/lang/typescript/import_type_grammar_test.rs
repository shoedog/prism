//! Source-preserving regression contract from PR304's compiler-backed matrix.
//! These tests deliberately require the repaired behavior, not the recorded
//! `current_native_errors` baseline. TypeScript compiler validity was measured
//! separately; a native zero-error tree alone is not compiler authority.
use prism::{ast::ParsedFile, languages::Language};
use serde_json::Value;

fn fixture(id: &str) -> (String, Value) {
    let matrix: Value = serde_json::from_str(include_str!(
        "../../../docs/eval/receiver-closure/native-parser-classification-fixtures.json"
    ))
    .unwrap();
    let case = matrix["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone();
    let source = format!(
        "{}{}{}",
        matrix["prefix"].as_str().unwrap(),
        case["body"].as_str().unwrap(),
        matrix["suffix"].as_str().unwrap()
    );
    (source, case)
}

fn check(id: &str, path: &str, language: Language) {
    let (source, case) = fixture(id);
    {
        let parsed = ParsedFile::parse(path, &source, language).unwrap();
        assert_eq!(parsed.source, source, "original source bytes must survive");
        if id == "malformed" {
            assert!(
                parsed.parse_error_count > 0,
                "{path}: malformed syntax must remain an error"
            );
            return;
        }
        assert_eq!(case["compiler_diagnostic_codes"], serde_json::json!([]));
        assert_eq!(
            parsed.parse_error_count, 0,
            "{id}.{path}: valid source must parse"
        );
        let mut pending = vec![parsed.tree.root_node()];
        let mut ordinary_calls = vec![];
        while let Some(node) = pending.pop() {
            if node.kind() == "call_expression" {
                if let Some(callee) = node.child_by_field_name("function") {
                    if &source[callee.byte_range()] == "get" {
                        ordinary_calls.push(node);
                    }
                }
            }
            let mut cursor = node.walk();
            pending.extend(node.children(&mut cursor));
        }
        if id == "dynamic_import" {
            assert!(ordinary_calls.is_empty());
        } else {
            assert_eq!(
                ordinary_calls.len(),
                1,
                "{id}.{path}: retain actual get call: {}",
                parsed.tree.root_node().to_sexp()
            );
        }
        if matches!(id, "await_import" | "plain_import" | "parenthesized") {
            let call = ordinary_calls[0];
            assert_eq!(
                &source[call.byte_range()],
                "get<typeof import(\"./module\")>()"
            );
            let args = call
                .child_by_field_name("type_arguments")
                .expect("generic arguments");
            assert_eq!(args.named_child_count(), 1);
            let query = args.named_child(0).unwrap();
            assert_eq!(query.kind(), "type_query");
            assert_eq!(&source[query.byte_range()], "typeof import(\"./module\")");
            assert_eq!(
                &source[call.child_by_field_name("arguments").unwrap().byte_range()],
                "()"
            );
        }
    }
}

macro_rules! grammar_case {
    ($name:ident) => {
        mod $name {
            use super::*;
            #[test]
            fn ts() {
                check(stringify!($name), "fixture.ts", Language::TypeScript);
            }
            #[test]
            fn tsx() {
                check(stringify!($name), "fixture.tsx", Language::Tsx);
            }
        }
    };
}
grammar_case!(await_import);
grammar_case!(plain_import);
grammar_case!(parenthesized);
grammar_case!(await_number);
grammar_case!(await_typeof);
grammar_case!(await_alias);
grammar_case!(type_annotation);
grammar_case!(dynamic_import);
grammar_case!(malformed);
