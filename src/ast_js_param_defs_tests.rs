//! js-param-defs PR-A binding-shape occurrences at the AST layer.
use super::*;

fn occurrences(source: &str, language: Language) -> Vec<(String, String)> {
    let parsed = ParsedFile::parse("shapes.x", source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    let function = parsed.all_functions()[0];
    parsed
        .function_parameter_occurrences(&function)
        .into_iter()
        .map(|(name, start, end)| (name, source[start..end].to_string()))
        .collect()
}

fn names(source: &str, language: Language) -> Vec<String> {
    occurrences(source, language)
        .into_iter()
        .map(|(name, text)| {
            assert_eq!(name, text, "occurrence bytes must spell the binding");
            name
        })
        .collect()
}

const JS_TS: [Language; 3] = [Language::JavaScript, Language::TypeScript, Language::Tsx];

#[test]
fn bare_arrow_formal_is_one_occurrence() {
    for language in JS_TS {
        assert_eq!(names("const f = cmd => exec(cmd);", language), ["cmd"]);
        assert_eq!(
            names("const f = async cmd => exec(cmd);", language),
            ["cmd"]
        );
        assert_eq!(names("const f = get => get;", language), ["get"]);
        // Curried: each arrow binds only its own formal.
        assert_eq!(names("const f = x => y => x + y;", language), ["x"]);
        let parsed = ParsedFile::parse("c.x", "const f = x => y => x + y;", language).unwrap();
        let inner = parsed.all_functions()[1];
        assert_eq!(
            parsed
                .function_parameter_occurrences(&inner)
                .into_iter()
                .map(|occurrence| occurrence.0)
                .collect::<Vec<_>>(),
            ["y"]
        );
    }
}

#[test]
fn identifier_rest_formal_is_an_occurrence_in_declaration_order() {
    assert_eq!(
        names(
            "function f(a, ...rest) { use(rest); }",
            Language::JavaScript
        ),
        ["a", "rest"]
    );
    for language in [Language::TypeScript, Language::Tsx] {
        assert_eq!(
            names(
                "function f(a: number, ...rest: string[]) { use(rest); }",
                language
            ),
            ["a", "rest"]
        );
        assert_eq!(
            names(
                "function f(this: Foo, ...rest /* c */: any[]) { use(rest); }",
                language
            ),
            ["rest"]
        );
        assert_eq!(
            names("function f(...rest) { use(rest); }", language),
            ["rest"]
        );
    }
}

#[test]
fn destructured_and_duplicate_rest_stay_refused() {
    for language in JS_TS {
        assert!(names("function f(...[a, b]) { use(a); }", language).is_empty());
        assert!(names("function f(...{ a }) { use(a); }", language).is_empty());
        assert_eq!(names("function f(x, ...[a]) { use(a); }", language), ["x"]);
        // `(...xs, y)` is an early error the grammar accepts; refuse the rest.
        assert_eq!(
            names("function f(...xs, y) { use(xs, y); }", language),
            ["y"]
        );
    }
    // JS: duplicates make a non-simple list an early error; the new rest
    // occurrence is refused, the pre-existing plain occurrence is unchanged.
    assert_eq!(
        names("function f(m, ...m) { use(m); }", Language::JavaScript),
        ["m"]
    );
    // TS: duplicates already refuse the whole occurrence list.
    for language in [Language::TypeScript, Language::Tsx] {
        assert!(names("function f(m, ...m) { use(m); }", language).is_empty());
        assert!(names("const f = \\u0061 => a;", language).is_empty());
    }
}

#[test]
fn positional_slots_still_stop_at_rest_and_bind_the_bare_arrow() {
    for language in JS_TS {
        let source = "function f(a, ...rest) { use(rest); }";
        let parsed = ParsedFile::parse("s.x", source, language).unwrap();
        let function = parsed.all_functions()[0];
        assert_eq!(
            parsed.function_parameter_slots(&function),
            Some(vec!["a".to_string()])
        );
        let parsed = ParsedFile::parse("s.x", "const f = x => x;", language).unwrap();
        let function = parsed.all_functions()[0];
        assert_eq!(
            parsed.function_parameter_slots(&function),
            Some(vec!["x".to_string()])
        );
    }
}

#[test]
fn bare_arrow_formal_is_plain_required_but_rest_is_not() {
    for language in JS_TS {
        for (source, name, plain) in [
            ("const f = cmd => { exec(cmd); };", "cmd", true),
            ("function f(...rest) { use(rest); }", "rest", false),
        ] {
            let parsed = ParsedFile::parse("p.x", source, language).unwrap();
            let function = parsed.all_functions()[0];
            let start = source.find(name).unwrap();
            let span = PathSpan {
                path: AccessPath::simple(name),
                line: 1,
                start_byte: start,
                end_byte: start + name.len(),
            };
            assert_eq!(
                parsed.exact_read_is_plain_required_parameter(&function, &span),
                plain,
                "{language:?}: {source}"
            );
        }
    }
}

#[test]
fn binding_shape_helpers_are_inert_outside_js_ts() {
    // Rust `..` is also a `rest_pattern`; Python lambdas carry `parameters`.
    for (language, source) in [
        (Language::Rust, "fn f([a, ..]: [i32; 2]) { let g = |x| x; }"),
        (Language::Python, "def f(*args):\n    g = lambda x: x\n"),
    ] {
        let parsed = ParsedFile::parse("n.x", source, language).unwrap();
        for function in parsed.all_functions() {
            assert!(parsed.js_ts_bare_arrow_parameter(&function).is_none());
            assert_eq!(
                parsed.parameter_binding_region(&function).map(|n| n.id()),
                parsed.find_parameters_node(&function).map(|n| n.id()),
                "{language:?}"
            );
        }
    }
}
