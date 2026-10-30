use super::*;
use crate::ast::binding_verdict_memo::{self, Session};
use std::collections::BTreeSet;

fn nodes(p: &ParsedFile) -> Vec<Node<'_>> {
    let mut nodes = Vec::new();
    let mut stack = vec![p.tree.root_node()];
    while let Some(node) = stack.pop() {
        nodes.push(node);
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    nodes
}

fn equivalent(source: &str, language: Language) {
    let p = ParsedFile::parse("synthetic", source, language).unwrap();
    let nodes = nodes(&p);
    let mut names: BTreeSet<String> = ["absent", "", "never_bound", "\\u0061"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for node in &nodes {
        if node.kind().contains("identifier") || node.kind() == "undefined" {
            let text = p.node_text(node);
            names.insert(text.to_owned());
            if let Some(decoded) = ParsedFile::js_ts_decode_identifier_text(text) {
                names.insert(decoded);
            }
        }
    }
    let boundaries = language.callable_boundary_node_types();
    let scopes: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|n| {
            n.kind() == "program"
                || n.kind() == "statement_block"
                || n.parent().is_some_and(|parent| {
                    parent.child_by_field_name("body").map(|b| b.id()) == Some(n.id())
                })
        })
        .collect();
    // Both fallback and session paths, including nonstandard boundaries. The
    // empty list makes nested vars visible, exercising the boundary cache key.
    for cached in [false, true] {
        let _session = cached.then(|| Session::new(&p));
        for boundary_list in [&boundaries[..], &[][..]] {
            for scope in &scopes {
                for name in &names {
                    let expected =
                        p.js_ts_function_scope_binds_reference(*scope, boundary_list, name);
                    assert_eq!(
                        p.js_ts_function_scope_binds(*scope, boundary_list, name),
                        expected,
                        "{language:?} cached={cached} scope={}@{} name={name:?} boundaries={boundary_list:?}",
                        scope.kind(), scope.start_byte()
                    );
                }
            }
        }
    }
}

const FIXTURE: &str = r#"
function outer(p = seed) {
    var a, undefined, \u0062, \u{63}, \u{3b1};
    var {key: renamed = initializer, shorthand, shortDefault = defaultValue,
         nested: [element, ...rest], ...objectRest} = source;
    var [arrayItem = fallback, , ...arrayRest] = input;
    for (var inVar in object) {} for (var ofVar of iterable) {}
    for (var {loopKey: loopBinding} of iterable) {}
    for (var [loopArray] in object) {} for (var counter = 0; counter < 2; counter++) {}
    for (let loopLet of iterable) {} for (const loopConst in object) {}
    let bodyLet; const bodyConst = 1; class BodyClass {}
    function bodyFunction() { var nestedOnly; }
    function* bodyGenerator() { var generatorOnly; }
    labeled: function labeledFunction() { var labelOnly; }
    { var hoisted; let blockLet; const blockConst = 2; class BlockClass {}
      function blockFunction() { var blockFunctionOnly; } }
    switch (p) { case 1: let switchLet; var switchVar; break;
                 default: const switchConst = 3; }
    try {} catch ({catchKey: caught}) { var catchVar; let catchLet; }
    var arrow = () => { var arrowOnly; }, expression = function named() { var exprOnly; };
    var generator = function* () { var genExprOnly; };
    var expressionArrow = () => value;
    var callableBody = () => function() { var returnedOnly; };
    var classBody = () => class { static { var returnedStatic; } };
    class Container { static { var staticOnly; let staticLet; }
      method() { var methodOnly; } field = (() => { var fieldOnly; })(); }
    var C = class { static { var expressionStaticOnly; } method() { var expressionMethodOnly; } };
    use(key, initializer, defaultValue, fallback, source, absent);
}
"#;

#[test]
fn differential_js_ts_tsx() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        equivalent(FIXTURE, language);
    }
    equivalent(
        r#"function typed<T>(p: T) { abstract class Abstract { abstract method(): T; }
            enum E { A, B = 2 } let x: T; { enum Nested { C } }
            const el = <Widget value={p}/>; return el; }"#,
        Language::Tsx,
    );
}

#[test]
fn differential_recovery_and_decoder_refusals() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in [
            r#"function broken() { var {key: value, ...rest} = ; var later; let = ; }"#,
            r#"function broken() { var \u{110000}, \uD800, \u0020, \u0030, \u{zz}, \u0069f; }"#,
            "function broken() { var [missing, ...] = x; if ( { var recovered;",
            "function broken() { var ; for (var in obj) {} class { static { var x; } } }",
        ] {
            let p = ParsedFile::parse("broken", source, language).unwrap();
            assert!(
                p.tree.root_node().has_error(),
                "fixture must exercise recovery"
            );
            equivalent(source, language);
        }
    }
}

#[test]
fn minified_hundreds_of_single_letter_vars() {
    let mut source = String::from("function m(){");
    for index in 0..400 {
        let letter = char::from(b'a' + (index % 26) as u8);
        source.push_str(&format!("var {letter}={index};"));
        if index % 40 == 0 {
            source.push_str("function n(){var nestedOnly;return ()=>{var deeperOnly;};}");
        }
    }
    // Single Unicode letters also exercise hundreds of distinct query names.
    for (index, letter) in (0x100..0x400)
        .filter_map(char::from_u32)
        .filter(|c| c.is_alphabetic())
        .take(400)
        .enumerate()
    {
        source.push_str(&format!("var {letter}={index};"));
    }
    source.push_str("var \\u007a;return a+b+c;}");
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        equivalent(&source, language);
    }
}

#[test]
fn leaf_and_pattern_predicates_are_exact() {
    let source = format!("{FIXTURE}\nfunction t<T>(p: T, q?: U) {{ var x; use(undefined); }}");
    let mut seen = BTreeSet::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let p = ParsedFile::parse("patterns.ts", &source, language).unwrap();
        let mut names = BTreeSet::from(["absent".to_owned(), String::new()]);
        for node in nodes(&p) {
            names.insert(p.node_text(&node).to_owned());
            if let Some(name) = ParsedFile::js_ts_decode_identifier_text(p.node_text(&node)) {
                names.insert(name);
            }
        }
        for node in nodes(&p) {
            seen.insert(node.kind());
            for (collect, predicate) in [
                (
                    pattern as fn(&ParsedFile, Node<'_>, &mut HashSet<String>),
                    ParsedFile::js_ts_fence_pattern_binds
                        as fn(&ParsedFile, Node<'_>, &str) -> bool,
                ),
                (declaration, ParsedFile::js_ts_declaration_binds),
                (block_statement, ParsedFile::js_ts_block_statement_binds),
            ] {
                let mut collected = HashSet::new();
                collect(&p, node, &mut collected);
                for name in &names {
                    assert_eq!(
                        collected.contains(name),
                        predicate(&p, node, name),
                        "node={} text={:?} name={name:?}",
                        node.kind(),
                        p.node_text(&node)
                    );
                }
            }
        }
    }
    for kind in [
        "undefined",
        "type_identifier",
        "pair_pattern",
        "assignment_pattern",
        "object_assignment_pattern",
        "required_parameter",
        "optional_parameter",
        "formal_parameters",
        "object_pattern",
        "array_pattern",
        "rest_pattern",
    ] {
        assert!(seen.contains(kind), "missing pattern coverage: {kind}");
    }
}

#[test]
fn one_walk_per_body_and_boundaries_with_epoch_isolation() {
    let mut p = ParsedFile::parse("one.js", FIXTURE, Language::JavaScript).unwrap();
    let boundaries = p.language.callable_boundary_node_types();
    let body = p.all_functions()[0].child_by_field_name("body").unwrap();
    let queries = || {
        for name in ["a", "b", "c", "nestedOnly", "absent", "", "rest", "hoisted"] {
            p.js_ts_function_scope_binds(body, &boundaries, name);
        }
    };
    assert_eq!(
        binding_verdict_memo::probe::count_body_walks(queries),
        8,
        "no session retains no set"
    );
    {
        let first = Session::new(&p);
        assert_eq!(
            binding_verdict_memo::probe::count_body_walks(queries),
            1,
            "distinct names must share one body walk"
        );
        assert_eq!(
            binding_verdict_memo::probe::count_body_walks(queries),
            0,
            "positive and negative membership cached"
        );
        assert!(!p.js_ts_function_scope_binds(body, &boundaries, "nestedOnly"));
        assert!(p.js_ts_function_scope_binds(body, &[], "nestedOnly"));
        let second = Session::new(&p);
        assert_eq!(
            binding_verdict_memo::probe::count_body_walks(queries),
            1,
            "overlapping sessions remain independent"
        );
        drop(first);
        assert_eq!(
            binding_verdict_memo::probe::count_body_walks(queries),
            0,
            "out-of-order drop preserves active cache"
        );
        drop(second);
    }
    assert_eq!(
        binding_verdict_memo::probe::count_body_walks(queries),
        8,
        "drop removes cache"
    );
    {
        let _session = Session::new(&p);
        assert_eq!(
            binding_verdict_memo::probe::count_body_walks(|| binding_verdict_memo::direct(queries)),
            8,
            "the existing direct seam must use the original per-name walk"
        );
    }
    p = ParsedFile::parse(
        "one.js",
        "function f(){var replaced;}",
        Language::JavaScript,
    )
    .unwrap();
    let body = p.all_functions()[0].child_by_field_name("body").unwrap();
    let _session = Session::new(&p);
    assert!(p.js_ts_function_scope_binds(body, &boundaries, "replaced"));
    assert!(!p.js_ts_function_scope_binds(body, &boundaries, "a"));
}

#[test]
fn cache_file_body_and_boundary_keys() {
    let p = ParsedFile::parse(
        "p.js",
        "function a(){var x;} function b(){var y;}",
        Language::JavaScript,
    )
    .unwrap();
    let q = ParsedFile::parse("q.js", "function a(){var z;}", Language::JavaScript).unwrap();
    let _p = Session::new(&p);
    let _q = Session::new(&q);
    let bodies: Vec<_> = p
        .all_functions()
        .iter()
        .map(|n| n.child_by_field_name("body").unwrap())
        .collect();
    let boundaries = p.language.callable_boundary_node_types();
    assert!(p.js_ts_function_scope_binds(bodies[0], &boundaries, "x"));
    assert!(!p.js_ts_function_scope_binds(bodies[1], &boundaries, "x"));
    assert!(p.js_ts_function_scope_binds(bodies[1], &boundaries, "y"));
    let qb = q.all_functions()[0].child_by_field_name("body").unwrap();
    assert!(q.js_ts_function_scope_binds(qb, &boundaries, "z"));
    assert!(!q.js_ts_function_scope_binds(qb, &boundaries, "x"));
    // Cache equality must compare the contents of boundary lists, not their addresses.
    let copied = boundaries.clone();
    assert_eq!(
        binding_verdict_memo::probe::count_body_walks(|| {
            assert!(p.js_ts_function_scope_binds(bodies[0], &copied, "x"));
        }),
        0
    );
}
