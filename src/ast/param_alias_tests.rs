//! #761: classification and alias target of a parameter-list default (`src/ast/js_ts/param_alias.rs`).
use super::*;

fn parse(language: Language, source: &str) -> ParsedFile {
    let name = match language {
        Language::JavaScript => "p.js",
        Language::TypeScript => "p.ts",
        _ => "p.tsx",
    };
    let parsed = ParsedFile::parse(name, source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{source}");
    parsed
}

fn use_of(language: Language, source: &str, name: &str) -> ParameterUse {
    let parsed = parse(language, source);
    let function = parsed.all_functions()[0];
    parsed.classify_parameter_use(&function, name)
}

#[test]
fn a_bare_read_in_a_default_makes_the_formal_mixed_not_member_only() {
    for (language, source) in [
        (Language::JavaScript, "function f(o, u = o) { return o.x; }"),
        (
            Language::JavaScript,
            "function f(o, u = g(o)) { return o.x; }",
        ),
        (
            Language::JavaScript,
            "function f(o, u = () => o) { return o.x; }",
        ),
        (
            Language::JavaScript,
            "function f(o, { k = o } = {}) { return o.x; }",
        ),
        (
            Language::JavaScript,
            "function f(o, { [o]: k } = {}) { return o.x; }",
        ),
        (
            Language::JavaScript,
            "function f(o, [a = o] = []) { return o.x; }",
        ),
        (Language::JavaScript, "const f = (o, u = o) => o.x;"),
        (
            Language::TypeScript,
            "function f(o: any, u: any = o) { return o.x; }",
        ),
        (
            Language::Tsx,
            "function f(o: any, u?: any = o) { return o.x; }",
        ),
        (
            Language::JavaScript,
            "class C { m(o, u = o) { return o.x; } }",
        ),
    ] {
        assert_eq!(
            use_of(language, source, "o"),
            ParameterUse::Bare,
            "{source}"
        );
    }
}

#[test]
fn reads_that_are_not_bare_keep_the_formal_member_only() {
    for (language, source) in [
        // a member read of the formal, a literal, another formal's alias
        (
            Language::JavaScript,
            "function f(o, u = o.x) { return o.x; }",
        ),
        (Language::JavaScript, "function f(o, u = 1) { return o.x; }"),
        (
            Language::JavaScript,
            "function f(o, p, u = p) { return o.x; }",
        ),
        // the binding name of a default is not a read, nor is a TS type position
        (
            Language::JavaScript,
            "function f(o, o2 = 1) { return o.x; }",
        ),
        (
            Language::TypeScript,
            "function f(o: any, u: typeof o = 1) { return o.x; }",
        ),
    ] {
        assert_eq!(
            use_of(language, source, "o"),
            ParameterUse::MemberOnly,
            "{source}"
        );
    }
}

#[test]
fn only_the_own_parameter_list_counts() {
    // Another callable's default is that callable's own business: here `g`'s `u = o` reads a global `o`.
    let source = "function f(o) { return o.x; }\nfunction g(p, u = o) { return p.x; }";
    assert_eq!(
        use_of(Language::JavaScript, source, "o"),
        ParameterUse::MemberOnly
    );
}

fn alias_targets(language: Language, source: &str) -> Vec<(String, Option<String>)> {
    let parsed = parse(language, source);
    let function = parsed.all_functions()[0];
    let lines = (1..=source.lines().count()).collect();
    let mut spans = Vec::new();
    parsed.js_ts_param_default_alias_spans(&function, &lines, &mut spans);
    spans
        .iter()
        .map(|s| {
            (
                s.path.base.clone(),
                parsed.js_ts_own_alias_target(s.start_byte, s.end_byte),
            )
        })
        .collect()
}

#[test]
fn a_plain_identifier_default_is_an_alias_of_its_target() {
    let some = |a: &str, t: &str| (a.to_string(), Some(t.to_string()));
    assert_eq!(
        alias_targets(Language::JavaScript, "function f(o, u = o, w = u) {}"),
        vec![some("u", "o"), some("w", "u")]
    );
    assert_eq!(
        alias_targets(Language::TypeScript, "function f(o: any, u: any = o) {}"),
        vec![some("u", "o")]
    );
    assert_eq!(
        alias_targets(Language::JavaScript, "const f = (o, u = o) => u;"),
        vec![some("u", "o")]
    );
}

#[test]
fn forms_the_body_does_not_alias_are_not_aliases_in_the_list() {
    for source in [
        "function f(o, u = o.x) {}",
        "function f(o, u = g(o)) {}",
        "function f(o, { a: u } = { a: o }) {}",
        "function f(o, [u] = [o]) {}",
        "function f(o, { u = o } = {}) {}",
        "function f(o, ...u) {}",
        // a self or forward reference is a TDZ error: no value flows, so no alias
        "function f(u = u) {}",
        "function f(a = b, b = 1) {}",
        "function f(a = b, { b } = {}) {}",
        "function f(o, u) {}",
    ] {
        assert_eq!(
            alias_targets(Language::JavaScript, source),
            vec![],
            "{source}"
        );
    }
    // Only the callable's own list: the nested callable's default and a destructuring default in the body
    // are not the owner's aliases.
    for source in [
        "function f(o) { const g = (u = o) => u; }",
        "function f(o) { let u, a; [a = u] = o; }",
    ] {
        assert_eq!(
            alias_targets(Language::JavaScript, source),
            vec![],
            "{source}"
        );
    }
}

#[test]
fn a_pattern_default_with_an_identifier_value_is_the_body_destructuring_alias() {
    let some = |a: &str, t: &str| (a.to_string(), Some(t.to_string()));
    // Object patterns alias the field of the target, anonymous owners included (the body's `js_ts_own_alias_target`).
    assert_eq!(
        alias_targets(Language::JavaScript, "function f(o, { a: u } = o) {}"),
        vec![some("u", "o.a")]
    );
    assert_eq!(
        alias_targets(Language::JavaScript, "function f(o, { a } = o) {}"),
        vec![some("a", "o.a")]
    );
    assert_eq!(
        alias_targets(
            Language::TypeScript,
            "function f(o: any, { a: u }: any = o) {}"
        ),
        vec![some("u", "o.a")]
    );
    // An array pattern has its def, but no anonymous-owner alias (as for `const [u] = o`); the named-owner alias
    // map gets it from the body's pattern extractor.
    assert_eq!(
        alias_targets(Language::JavaScript, "function f(o, [u] = o) {}"),
        vec![("u".to_string(), None)]
    );
    assert_eq!(
        triples(
            Language::JavaScript,
            "function f(o, [u] = o, { a: w } = o) {}"
        ),
        vec![
            ("u".to_string(), "o".to_string(), 1),
            ("w".to_string(), "o.a".to_string(), 1)
        ]
    );
    // A defaulted entry is no alias (the body's extractor skips it), nor is a TDZ self reference.
    for source in [
        "function f(o, { a: u = 1 } = o) {}",
        "function f(o, { a = 1 } = o) {}",
        "function f([o] = o) {}",
        "function f([u] = o, o) {}",
    ] {
        assert_eq!(triples(Language::JavaScript, source), vec![], "{source}");
    }
}

fn triples(language: Language, source: &str) -> Vec<(String, String, usize)> {
    let parsed = parse(language, source);
    let function = parsed.all_functions()[0];
    let lines = (1..=source.lines().count()).collect();
    parsed.js_ts_param_default_alias_triples(&function, &lines)
}

#[test]
fn every_alias_def_and_triple_sits_on_the_owners_first_line() {
    // Main puts every formal's def on the first line; a row of the signature that holds no CFG node would make the
    // twin neither kill nor collapse with the formal's def.
    for source in [
        "function f(\n  o,\n  u = o,\n) {}",
        "function f(a,\n  o,\n  [u] = o,\n  { k: w } = o,\n) {}",
    ] {
        let parsed = parse(Language::JavaScript, source);
        let function = parsed.all_functions()[0];
        let lines = (1..=source.lines().count()).collect();
        let mut spans = Vec::new();
        parsed.js_ts_param_default_alias_spans(&function, &lines, &mut spans);
        assert!(!spans.is_empty(), "{source}");
        assert!(spans.iter().all(|s| s.line == 1), "{source}: {spans:?}");
        // the bytes stay on the identifier
        for s in &spans {
            assert_eq!(&source[s.start_byte..s.end_byte], s.path.base, "{source}");
        }
        let triples = parsed.js_ts_param_default_alias_triples(&function, &lines);
        assert!(
            !triples.is_empty() && triples.iter().all(|t| t.2 == 1),
            "{triples:?}"
        );
    }
}

#[test]
fn entries_that_mint_a_def_but_no_triple_are_still_alias_entries() {
    // The census trace keys `param-alias` on the minted defs, so these owners get a line (#782 round 2).
    for source in [
        "function f(o, { a: u = 1 } = o) {}",
        "function f(o, { [k]: u } = o) {}",
        "function f(o, { 'a': u } = o) {}",
    ] {
        let parsed = parse(Language::JavaScript, source);
        let function = parsed.all_functions()[0];
        let lines = (1..=source.lines().count()).collect();
        let mut spans = Vec::new();
        parsed.js_ts_param_default_alias_spans(&function, &lines, &mut spans);
        assert_eq!(spans.len(), 1, "{source}");
        assert!(
            parsed
                .js_ts_param_default_alias_triples(&function, &lines)
                .is_empty(),
            "{source}"
        );
    }
}

#[test]
fn only_the_binding_of_an_alias_entry_is_a_param_alias_binding() {
    let source = "function f(o, u = o, { a: w } = o, p = 1, q = g(u)) { return u + w; }";
    let parsed = parse(Language::JavaScript, source);
    let at = |anchor: &str, offset: usize| {
        let start = source.find(anchor).unwrap() + offset;
        parsed.js_ts_is_param_alias_binding_at(start, start + 1)
    };
    // `u` of `u = o`, `w` of `{ a: w } = o`; not the read `o`, a non-alias default's name, or reads of `u`.
    assert!(at("u = o", 0) && at("w }", 0));
    assert!(!at("= o,", 2) && !at("p = 1", 0));
    assert!(!at("g(u)", 2) && !at("u + w", 0));
}
