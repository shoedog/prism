use super::*;
use crate::data_flow::DataFlowGraph;
use std::collections::BTreeMap;

fn check(source: &str, expected: &[&str]) {
    for language in [Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("params.ts", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{source}");
        let function = parsed.all_functions()[0];
        let occurrences = parsed.function_parameter_occurrences(&function);
        assert_eq!(
            occurrences.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
            expected,
            "{language:?}: {source}: {}",
            function.to_sexp()
        );
        assert_eq!(parsed.function_parameter_names(&function), expected);
        for (name, start, end) in occurrences {
            assert_eq!(&source[start..end], name);
            assert!(start >= function.start_byte() && end <= function.end_byte());
        }
    }
}

#[test]
fn required_parameter_identifiers_keep_exact_tokens_across_function_forms() {
    for source in [
        "function take(a: any, b: any) { sink(a, b); }",
        "function take(a, b) { sink(a, b); }",
        "const take = (a: any, b: any) => { sink(a, b); };",
        "const take = function(a: any, b: any) { sink(a, b); };",
        "class C { take(a: any, b: any) { sink(a, b); } }",
        "function take(\n/* Ω */ a /* comment */: Other,\nb: { a: Other }) { sink(a, b); }",
    ] {
        check(source, &["a", "b"]);
    }
    check(
        "function take($a: any, café: any) { sink($a, café); }",
        &["$a", "café"],
    );
}

#[test]
fn required_parameter_unsupported_forms_do_not_supply_definitions() {
    for parameter in [
        "a?: any",
        "a: any = value",
        "...a: any[]",
        "{a}: any",
        "[a]: any",
        "this: any",
    ] {
        check(&format!("function take({parameter}) {{ sink(a); }}"), &[]);
    }
    for parameter in [
        "public a: any",
        "private a: any",
        "protected a: any",
        "readonly a: any",
        "public override a: any",
        "@inject a: any",
    ] {
        check(
            &format!("class C {{ constructor({parameter}) {{ sink(a); }} }}"),
            &[],
        );
    }
    check(
        "function take({x}: any, b: any, c: any = value) { sink(b); }",
        &["b"],
    );
}

#[test]
fn required_parameter_duplicate_and_escaped_binding_lists_fail_closed() {
    for parameters in [
        "a: any, a: any",
        "a: any, {a}: any",
        "{a = 0}: any, a: any",
        "a: any, ...a: any[]",
        "a: any, \\u0061: any",
        "\\u0061: any, b: any",
    ] {
        check(
            &format!("function take({parameters}) {{ sink(a, b); }}"),
            &[],
        );
    }
}

#[test]
fn required_parameter_recovery_lists_fail_closed() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function take(a: any, b: ) { sink(a); }";
        let parsed = ParsedFile::parse("params.ts", source, language).unwrap();
        assert!(parsed.parse_error_count > 0);
        let function = parsed.all_functions()[0];
        assert!(parsed.function_parameter_occurrences(&function).is_empty());
        assert!(parsed.function_parameter_names(&function).is_empty());
    }
}

#[test]
fn required_parameter_dfg_definitions_keep_signature_bytes_and_subset_parity() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function take(\n a: any,\n b: any\n) {\n sink(a, b);\n}";
        let parsed = ParsedFile::parse("params.ts", source, language).unwrap();
        let files = BTreeMap::from([("params.ts".to_string(), parsed)]);
        let full = DataFlowGraph::build(&files);
        let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["params.ts".into()]));
        let defs = full.defs.values().flatten().collect::<Vec<_>>();
        assert_eq!(defs.len(), 2, "{language:?}: {defs:?}");
        for def in defs {
            assert_eq!(def.line, 1);
            assert_eq!(&source[def.start_byte..def.end_byte], def.path.to_string());
        }
        assert_eq!(
            serde_json::to_value(full.defs.values().collect::<Vec<_>>()).unwrap(),
            serde_json::to_value(subset.defs.values().collect::<Vec<_>>()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&full.edges).unwrap(),
            serde_json::to_value(&subset.edges).unwrap()
        );
        assert_eq!(full.labels, subset.labels);
    }
}

#[test]
fn required_parameter_write_shadow_and_field_isolation_match_javascript() {
    for body in [
        "sink(a);\na = other;\nsink(a);",
        "sink(a);\n{ let a = other;\nsink(a); }\nsink(a);",
        "a = other; sink(a);\nsink(a);",
        "sink(a.field);\nsink(a.other);",
    ] {
        let build = |language, parameter| {
            let source = format!("function take({parameter}) {{\n{body}\n}}");
            let parsed = ParsedFile::parse("params", &source, language).unwrap();
            DataFlowGraph::build(&BTreeMap::from([("params".into(), parsed)]))
        };
        let control = build(Language::JavaScript, "a");
        let labels = |graph: &DataFlowGraph| {
            graph
                .labels
                .iter()
                .map(|((from, to), confidence)| {
                    (
                        from.path.to_string(),
                        from.line,
                        to.path.to_string(),
                        to.line,
                        *confidence,
                    )
                })
                .collect::<Vec<_>>()
        };
        for language in [Language::TypeScript, Language::Tsx] {
            let candidate = build(language, "a: any");
            assert_eq!(labels(&candidate), labels(&control), "{language:?}: {body}");
            let defs = |graph: &DataFlowGraph| {
                graph
                    .defs
                    .values()
                    .flatten()
                    .map(|def| (def.path.to_string(), def.line))
                    .collect::<Vec<_>>()
            };
            assert_eq!(defs(&candidate), defs(&control), "{language:?}: {body}");
            if body.starts_with("sink(a);") {
                assert!(labels(&candidate)
                    .iter()
                    .any(|(path, from, _, to, _)| path == "a" && *from == 1 && *to == 2));
            }
            if body.starts_with("sink(a.field)") {
                assert!(!defs(&candidate).iter().any(|(path, _)| path == "a"));
            }
        }
    }
}
