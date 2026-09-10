//! Compiler-backed boundaries around the locally repaired generic call grammar.
use prism::{ast::ParsedFile, languages::Language};
use std::collections::BTreeSet;

fn parse(source: &str, language: Language) -> ParsedFile {
    let parsed = ParsedFile::parse("boundary.ts", source, language).unwrap();
    assert_eq!(parsed.source, source);
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    parsed
}

#[test]
fn await_generic_callee_and_span_exclude_await() {
    for language in [Language::TypeScript, Language::Tsx] {
        for call in [
            "get<number>()",
            "get<typeof import(\"m\")>()",
            "get<Array<typeof import(\"m\")>>()",
            "get<number, typeof import(\"m\")>()",
            "client.get<typeof import(\"m\")>()",
            "get?.<typeof import(\"m\")>()",
        ] {
            let source = format!("async function owner() {{ await {call}; }}");
            let parsed = parse(&source, language);
            let root = parsed.tree.root_node();
            let sites = parsed.function_calls_with_spans_on_lines(
                &root,
                &BTreeSet::from([1]),
                &BTreeSet::new(),
            );
            assert_eq!(sites.len(), 1, "{language:?}: {source}");
            assert_eq!(sites[0].callee_name, "get", "{source}");
            assert_eq!(&source[sites[0].start_byte..sites[0].end_byte], call);
            let node = root
                .descendant_for_byte_range(sites[0].start_byte, sites[0].end_byte)
                .unwrap();
            assert_eq!(node.kind(), "call_expression");
            assert_eq!(
                node.parent().unwrap().kind(),
                "await_expression",
                "{source}"
            );
        }
    }
}

#[test]
fn parenthesized_await_is_a_distinct_generic_callee() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "async function owner() { (await get)<number>(); }";
        let parsed = parse(source, language);
        let root = parsed.tree.root_node();
        let call = root
            .named_child(0)
            .unwrap()
            .child_by_field_name("body")
            .unwrap()
            .named_child(0)
            .unwrap()
            .named_child(0)
            .unwrap();
        assert_eq!(call.kind(), "call_expression");
        let callee = call.child_by_field_name("function").unwrap();
        assert_eq!(callee.kind(), "parenthesized_expression");
        assert_eq!(&source[callee.byte_range()], "(await get)");
        assert_eq!(callee.named_child(0).unwrap().kind(), "await_expression");
    }
}

#[test]
fn generic_constructor_is_not_a_call_on_a_bare_new_expression() {
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "new Factory<number>();",
            "new Factory<typeof import(\"m\")>();",
        ] {
            let parsed = parse(source, language);
            let node = parsed
                .tree
                .root_node()
                .named_child(0)
                .unwrap()
                .named_child(0)
                .unwrap();
            assert_eq!(node.kind(), "new_expression", "{source}");
            assert!(node.child_by_field_name("type_arguments").is_some());
            assert!(node.child_by_field_name("arguments").is_some());
        }
        let parsed = parse("new Factory()<number>();", language);
        let node = parsed
            .tree
            .root_node()
            .named_child(0)
            .unwrap()
            .named_child(0)
            .unwrap();
        assert_eq!(node.kind(), "call_expression");
        assert_eq!(
            node.child_by_field_name("function").unwrap().kind(),
            "new_expression"
        );
    }
}

#[test]
fn generic_shared_lookahead_keeps_instantiation_heritage_and_comparisons() {
    for language in [Language::TypeScript, Language::Tsx] {
        for (source, expected) in [
            ("makeBox<string>;", "instantiation_expression"),
            ("a < b.c;", "binary_expression"),
        ] {
            let parsed = parse(source, language);
            assert_eq!(
                parsed
                    .tree
                    .root_node()
                    .named_child(0)
                    .unwrap()
                    .named_child(0)
                    .unwrap()
                    .kind(),
                expected
            );
        }
        for source in [
            "class C extends Base<string> {}",
            "class C extends mixin<T>(Base)<string> implements I {}",
            "class C extends Factory()<string> {}",
        ] {
            let parsed = parse(source, language);
            assert!(parsed.tree.root_node().to_sexp().contains("extends_clause"));
            assert!(parsed.tree.root_node().to_sexp().contains("type_arguments"));
        }
    }
}

#[test]
fn asserted_non_null_and_parenthesized_generic_callees_remain_supported() {
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "(get as Fn)<number>();",
            "(get satisfies Fn)<number>();",
            "(get<T>)<number>();",
            "get!<number>();",
        ] {
            let parsed = parse(source, language);
            assert_eq!(
                parsed
                    .tree
                    .root_node()
                    .named_child(0)
                    .unwrap()
                    .named_child(0)
                    .unwrap()
                    .kind(),
                "call_expression"
            );
        }
    }
    let parsed = parse("<Fn>get<number>();", Language::TypeScript);
    let assertion = parsed
        .tree
        .root_node()
        .named_child(0)
        .unwrap()
        .named_child(0)
        .unwrap();
    assert_eq!(assertion.kind(), "type_assertion");
    assert_eq!(assertion.named_child(1).unwrap().kind(), "call_expression");
    parse("(<Fn>get)<number>();", Language::TypeScript);
}

#[test]
fn tsx_generic_jsx_and_arrow_ambiguity_remains_supported() {
    for source in [
        "const x = <Component<Props> />;",
        "const x = <Component<Props>>child</Component>;",
        "const f = <T,>(x: T) => x;",
        "const x = <Component value={get<typeof import(\"m\")>()} />;",
    ] {
        parse(source, Language::Tsx);
    }
}

#[test]
fn await_continuations_keep_full_operand_and_nested_awaits() {
    for language in [Language::TypeScript, Language::Tsx] {
        for (expression, kind, operand) in [
            ("await get()()", "await_expression", "call_expression"),
            (
                "await get<number>()()",
                "await_expression",
                "call_expression",
            ),
            ("await get().value", "await_expression", "member_expression"),
            (
                "await get<number>().value",
                "await_expression",
                "member_expression",
            ),
            (
                "await await get<number>()",
                "await_expression",
                "await_expression",
            ),
            (
                "await new Factory<number>()",
                "await_expression",
                "new_expression",
            ),
            (
                "await new Factory()<number>()",
                "await_expression",
                "call_expression",
            ),
            (
                "await get<number>() < other",
                "binary_expression",
                "await_expression",
            ),
            (
                "await get<number>() * other",
                "binary_expression",
                "await_expression",
            ),
            ("await get < other", "binary_expression", "await_expression"),
        ] {
            let source = format!("async function owner() {{ {expression}; }}");
            let parsed = parse(&source, language);
            let node = parsed
                .tree
                .root_node()
                .named_child(0)
                .unwrap()
                .child_by_field_name("body")
                .unwrap()
                .named_child(0)
                .unwrap()
                .named_child(0)
                .unwrap();
            assert_eq!(node.kind(), kind, "{language:?}: {source}");
            assert_eq!(
                node.named_child(0).unwrap().kind(),
                operand,
                "{language:?}: {source}"
            );
            if expression == "await await get<number>()" {
                assert_eq!(
                    node.named_child(0).unwrap().named_child(0).unwrap().kind(),
                    "call_expression"
                );
            }
        }
    }
}
