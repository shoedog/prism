//! JS/TS/TSX default-identifier parameter occurrences (the inert subset).
//!
//! Newly-admitted default occurrences are purely additive: they never change
//! required or optional-without-default occurrences computed by the existing
//! paths. Admission is a whole-signature guard — every runtime parameter must
//! be a supported ordinary simple identifier, either required/optional
//! without a default, or required with an INERT default (`number`, `string`,
//! `true`, `false`, `null`, or an empty `object`/`array`). A single
//! disqualifying parameter refuses default occurrences for the whole list.

use super::*;

fn occurrences(source: &str, language: Language) -> (Vec<ParameterOccurrence>, Vec<String>, usize) {
    let parsed = ParsedFile::parse("default.ts", source, language).unwrap();
    let function = parsed.all_functions()[0];
    let occ = parsed.function_parameter_occurrences(&function);
    let names = parsed.function_parameter_names(&function);
    (occ, names, parsed.parse_error_count)
}

#[test]
fn default_grammar_shape_matches_pinned_fields_across_dialects() {
    let checks: [(Language, &str, &str); 5] = [
        (
            Language::JavaScript,
            "function f(a = 1) {}",
            "(formal_parameters (assignment_pattern left: (identifier) right: (number)))",
        ),
        (
            Language::TypeScript,
            "function f(a = 1) {}",
            "(formal_parameters (required_parameter pattern: (identifier) value: (number)))",
        ),
        (
            Language::TypeScript,
            "function f(a: number = 1) {}",
            "(formal_parameters (required_parameter pattern: (identifier) type: (type_annotation (predefined_type)) value: (number)))",
        ),
        (
            Language::Tsx,
            "function f(a = 1) {}",
            "(formal_parameters (required_parameter pattern: (identifier) value: (number)))",
        ),
        (
            Language::TypeScript,
            "function f(a?: number = 1) {}",
            "(formal_parameters (optional_parameter pattern: (identifier) type: (type_annotation (predefined_type)) value: (number)))",
        ),
    ];
    for (language, source, expected) in checks {
        let parsed = ParsedFile::parse("shape.ts", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
        let function = parsed.all_functions()[0];
        let params = function.child_by_field_name("parameters").unwrap();
        assert_eq!(params.to_sexp(), expected, "{language:?}: {source}");
    }
}

#[test]
fn default_inert_literal_and_empty_container_identifiers_gain_exact_token_occurrences() {
    let cases = [
        ("number", "1"),
        ("string", "\"clean\""),
        ("true", "true"),
        ("false", "false"),
        ("null", "null"),
        ("empty_object", "{}"),
        ("empty_object_with_comment", "{ /* ok */ }"),
        ("empty_array", "[]"),
        ("empty_array_with_comment", "[ // ok\n ]"),
    ];
    for (case, value) in cases {
        let js_source = format!("function take(a = {value}) {{ sink(a); }}");
        let a = js_source.find("(a").unwrap() + 1;
        let (occ, _, errors) = occurrences(&js_source, Language::JavaScript);
        assert_eq!(errors, 0, "js/{case}: {js_source}");
        assert!(
            occ.contains(&("a".to_string(), a, a + 1)),
            "js/{case}: {occ:?}"
        );

        for (language, parameter) in [
            (Language::TypeScript, format!("a = {value}")),
            (Language::TypeScript, format!("a: any = {value}")),
            (Language::Tsx, format!("a = {value}")),
        ] {
            let source = format!("function take({parameter}) {{ sink(a); }}");
            let a = source.find("(a").unwrap() + 1;
            let (occ, _, errors) = occurrences(&source, language);
            assert_eq!(errors, 0, "{language:?}/{case}: {source}");
            assert!(
                occ.contains(&("a".to_string(), a, a + 1)),
                "{language:?}/{case}: {occ:?}"
            );
        }
    }
}

#[test]
fn default_inert_occurrences_support_unicode_and_multiline_signatures() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let source = "function take(\n  café = 1,\n  b = \"clean\"\n) {\n  sink(café, b);\n}";
        let (occ, _, errors) = occurrences(source, language);
        assert_eq!(errors, 0, "{language:?}: {source}");
        let cafe = source.find("café").unwrap();
        let cafe_len = "café".len();
        let b = source.find("b = \"clean\"").unwrap();
        assert!(
            occ.contains(&("café".to_string(), cafe, cafe + cafe_len)),
            "{language:?}: {occ:?}"
        );
        assert!(
            occ.contains(&("b".to_string(), b, b + 1)),
            "{language:?}: {occ:?}"
        );
    }
}

#[test]
fn default_inert_allowlist_excludes_non_inert_forms() {
    let js_cases = [
        ("unary_negative_number", "a = -1"),
        ("template_no_substitution", "a = `x`"),
        ("undefined", "a = undefined"),
        ("identifier", "a = other"),
        ("call", "a = seed()"),
        ("property_read", "a = obj.field"),
        ("arrow_function", "a = () => 1"),
        ("parenthesized", "a = (1)"),
        ("filled_object", "a = {x: 1}"),
        ("filled_array", "a = [1]"),
        ("spread_array", "a = [...rest]"),
    ];
    for (case, parameter) in js_cases {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let source = format!("function take({parameter}) {{ sink(a); }}");
            let (occ, _, errors) = occurrences(&source, language);
            assert_eq!(errors, 0, "{language:?}/{case}: {source}");
            assert!(
                !occ.iter().any(|(name, _, _)| name == "a"),
                "{language:?}/{case}: non-inert default gained an occurrence: {occ:?}"
            );
        }
    }
    let ts_cases = [
        ("asserted", "a: any = x as any"),
        ("satisfies", "a: any = x satisfies unknown"),
        ("parenthesized_typed", "a: any = (1 as any)"),
    ];
    for (case, parameter) in ts_cases {
        for language in [Language::TypeScript, Language::Tsx] {
            let source = format!("function take({parameter}) {{ sink(a); }}");
            let (occ, _, errors) = occurrences(&source, language);
            assert_eq!(errors, 0, "{language:?}/{case}: {source}");
            assert!(
                !occ.iter().any(|(name, _, _)| name == "a"),
                "{language:?}/{case}: {occ:?}"
            );
        }
    }
}

#[test]
fn default_admission_requires_every_parameter_in_the_signature_to_qualify() {
    let function_cases = [
        ("sibling_non_inert_default", "a: any = 1, b: any = seed()"),
        ("sibling_destructured", "a: any = 1, {x}: any"),
        ("sibling_rest", "a: any = 1, ...rest: any[]"),
        ("sibling_this_first", "this: any, a: any = 1"),
    ];
    for (case, parameters) in function_cases {
        for language in [Language::TypeScript, Language::Tsx] {
            let source = format!("function take({parameters}) {{ sink(a); }}");
            let (occ, _, errors) = occurrences(&source, language);
            assert_eq!(errors, 0, "{language:?}/{case}: {source}");
            assert!(
                !occ.iter().any(|(name, _, _)| name == "a"),
                "{language:?}/{case}: sibling disqualification failed: {occ:?}"
            );
        }
    }
    let constructor_cases = [
        ("sibling_property", "a: any = 1, public b: any"),
        ("sibling_decorator", "a: any = 1, @inject b: any"),
    ];
    for (case, parameters) in constructor_cases {
        for language in [Language::TypeScript, Language::Tsx] {
            let source = format!("class C {{ constructor({parameters}) {{ sink(a); }} }}");
            let parsed = ParsedFile::parse("ctor.ts", &source, language).unwrap();
            assert_eq!(parsed.parse_error_count, 0, "{language:?}/{case}: {source}");
            let function = parsed
                .all_functions()
                .into_iter()
                .find(|f| language.function_name(f).is_some())
                .unwrap();
            let occ = parsed.function_parameter_occurrences(&function);
            assert!(
                !occ.iter().any(|(name, _, _)| name == "a"),
                "{language:?}/{case}: sibling disqualification failed: {occ:?}"
            );
        }
    }
}

#[test]
fn optional_with_default_is_refused_as_both_optional_and_default() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function take(a?: number = 1) { sink(a); }";
        let (occ, _, errors) = occurrences(source, language);
        assert_eq!(errors, 0, "{language:?}: {source}");
        assert!(
            !occ.iter().any(|(name, _, _)| name == "a"),
            "{language:?}: {occ:?}"
        );
    }
}

#[test]
fn default_duplicate_recovery_and_escaped_bindings_fail_closed() {
    for language in [Language::TypeScript, Language::Tsx] {
        for (case, parameters) in [
            ("duplicate", "a: any = 1, a: any = 2"),
            ("escaped", "\\u0061: any = 1, b: any"),
        ] {
            let source = format!("function take({parameters}) {{ sink(b); }}");
            let (occ, _, errors) = occurrences(&source, language);
            assert_eq!(errors, 0, "{language:?}/{case}: {source}");
            assert!(
                !occ.iter().any(|(n, _, _)| n == "a"),
                "{language:?}/{case}: {occ:?}"
            );
        }
        let source = "function take(a: any = 1, b: ) { sink(a); }";
        let parsed = ParsedFile::parse("r.ts", source, language).unwrap();
        assert!(parsed.parse_error_count > 0);
        let function = parsed.all_functions()[0];
        assert!(parsed.function_parameter_occurrences(&function).is_empty());
    }
    for (case, parameters) in [
        ("duplicate", "a = 1, a = 2, b"),
        ("escaped", "\\u0061 = 1, b"),
    ] {
        let source = format!("function take({parameters}) {{ sink(b); }}");
        let (occ, _, errors) = occurrences(&source, Language::JavaScript);
        assert_eq!(errors, 0, "js/{case}: {source}");
        assert!(!occ.iter().any(|(n, _, _)| n == "a"), "js/{case}: {occ:?}");
    }
}

#[test]
fn default_admission_never_removes_existing_required_or_optional_occurrences() {
    for language in [Language::TypeScript, Language::Tsx] {
        // A destructured/non-inert sibling refuses the whole default class
        // but must never clobber `b`'s pre-existing, independent required
        // occurrence.
        let (occ, _, errors) = occurrences(
            "function take({x}: any, b: any, c: any = compute()) { sink(b); }",
            language,
        );
        assert_eq!(errors, 0, "{language:?}");
        assert_eq!(
            occ.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
            ["b"],
            "{language:?}: {occ:?}"
        );

        // When every parameter qualifies, `c` gains a NEW default occurrence
        // (in declaration order), while `b`'s optional occurrence
        // stays refused by the pre-existing, unrelated whole-signature
        // initializer barrier for optional parameters.
        let (occ2, _, errors2) = occurrences(
            "function take(a: any, b?: any, c: any = 1) { sink(a, b, c); }",
            language,
        );
        assert_eq!(errors2, 0, "{language:?}");
        assert_eq!(
            occ2.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
            ["a", "c"],
            "{language:?}: {occ2:?}"
        );
    }
}

#[test]
fn javascript_function_parameter_names_legacy_behavior_diverges_from_occurrences() {
    let source = "function take(a, b = 1) { sink(a, b); }";
    let (occ, names, errors) = occurrences(source, Language::JavaScript);
    assert_eq!(errors, 0);
    assert!(
        occ.iter().any(|(n, _, _)| n == "b"),
        "js: new default occurrence missing: {occ:?}"
    );
    assert_eq!(
        names,
        vec!["a".to_string()],
        "js: function_parameter_names is not derived from occurrences and must keep its \
         pre-existing (no-default) contract: {names:?}"
    );

    let ts_source = "function take(a: any, b: any = 1) { sink(a, b); }";
    let (ts_occ, ts_names, ts_errors) = occurrences(ts_source, Language::TypeScript);
    assert_eq!(ts_errors, 0);
    assert_eq!(
        ts_names,
        vec!["a".to_string(), "b".to_string()],
        "ts: function_parameter_names routes through occurrences and must include the \
         new default: {ts_names:?}"
    );
    assert_eq!(
        ts_names,
        ts_occ.into_iter().map(|p| p.0).collect::<Vec<_>>()
    );
}

#[test]
fn non_js_ts_default_parameter_handling_is_unaffected_by_this_change() {
    let source = "def take(a, b=1):\n    sink(a, b)\n";
    let parsed = ParsedFile::parse("take.py", source, Language::Python).unwrap();
    let function = parsed.all_functions()[0];
    let occ = parsed.function_parameter_occurrences(&function);
    assert_eq!(
        occ.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
        ["a", "b"],
        "Python default-parameter occurrence support predates and is untouched by this \
         JS/TS-only slice: {occ:?}"
    );
}

#[test]
fn default_inert_occurrences_preserve_declaration_order() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (occ, names, errors) =
            occurrences("function take(a = 1, b, c = 2) { sink(a,b,c); }", language);
        assert_eq!(errors, 0);
        assert_eq!(
            occ.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        if language != Language::JavaScript {
            assert_eq!(names, ["a", "b", "c"]);
        }
    }
}

#[test]
fn default_inert_array_holes_are_not_empty_containers() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for value in ["[,]", "[,,]", "[/* hole */ ,]"] {
            let (occ, _, errors) = occurrences(
                &format!("function take(a = {value}) {{ sink(a); }}"),
                language,
            );
            assert_eq!(errors, 0);
            assert!(occ.is_empty(), "{language:?}: {value}: {occ:?}");
        }
    }
}
