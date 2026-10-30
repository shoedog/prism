use super::*;
use crate::ast::binding_verdict_memo::{self, probe, Session};

#[test]
fn one_collection_for_distinct_names() {
    let p = ParsedFile::parse(
        "synthetic.js",
        "function f(){let a; const b=1; class C{} function g(){} }",
        Language::JavaScript,
    )
    .unwrap();
    let boundaries = p.language.callable_boundary_node_types();
    let body = p.all_functions()[0].child_by_field_name("body").unwrap();
    let _session = Session::new(&p);
    assert_eq!(
        probe::count_scope_walks(|| {
            for name in ["a", "b", "C", "g", "absent", ""] {
                let expected = binding_verdict_memo::direct(|| {
                    p.js_ts_scope_binds(body, &boundaries, name, body.start_byte())
                });
                assert_eq!(
                    p.js_ts_scope_binds(body, &boundaries, name, body.start_byte()),
                    expected
                );
            }
        }),
        1,
        "distinct names must share one scope collection"
    );
}

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

fn equivalent(source: &str, language: Language, every_byte: bool) {
    use std::collections::{BTreeSet, HashMap};
    let p = ParsedFile::parse("synthetic", source, language).unwrap();
    let nodes = nodes(&p);
    let boundaries = language.callable_boundary_node_types();
    let mut names = BTreeSet::from([String::new(), "absent".into(), "never_bound".into()]);
    let mut bytes = BTreeSet::from([0, source.len()]);
    for node in &nodes {
        bytes.extend([node.start_byte(), node.end_byte()]);
        if node.kind().contains("identifier") || node.kind() == "undefined" {
            names.insert(p.node_text(node).to_owned());
            if let Some(decoded) = ParsedFile::js_ts_decode_identifier_text(p.node_text(node)) {
                names.insert(decoded);
            }
        }
        if node.kind() == "string" {
            if let Some(decoded) = p.js_ts_module_export_name(*node) {
                names.insert(decoded);
            }
        }
    }
    if every_byte {
        bytes.extend(0..=source.len());
    }
    let scopes: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|node| {
            node.kind() == "program" || binding_verdict_memo::cacheable(*node, &boundaries)
        })
        .collect();
    // No session: exercise fallback at each scope's start, plus all byte
    // regions below with the production session. Expected verdicts are reused
    // only for byte-equivalent regions of the unchanged reference predicate.
    for scope in &scopes {
        for name in &names {
            let byte = scope.start_byte();
            let expected = binding_verdict_memo::direct(|| {
                p.js_ts_scope_binds(*scope, &boundaries, name, byte)
            });
            assert_eq!(
                p.js_ts_scope_binds(*scope, &boundaries, name, byte),
                expected
            );
        }
    }
    let _session = Session::new(&p);
    for scope in scopes {
        let mut regions = BTreeSet::new();
        let scope_bytes: Vec<_> = bytes
            .iter()
            .copied()
            .filter(|byte| {
                every_byte || regions.insert(p.binding_verdict_region(scope, &boundaries, *byte))
            })
            .collect();
        for name in &names {
            let mut expected = HashMap::new();
            for byte in &scope_bytes {
                let region = p.binding_verdict_region(scope, &boundaries, *byte);
                let expected = *expected.entry(region).or_insert_with(|| {
                    binding_verdict_memo::direct(|| {
                        p.js_ts_scope_binds(scope, &boundaries, name, *byte)
                    })
                });
                assert_eq!(
                    p.js_ts_scope_binds(scope, &boundaries, name, *byte),
                    expected,
                    "{language:?} scope={}@{} name={name:?} byte={byte}",
                    scope.kind(),
                    scope.start_byte()
                );
            }
        }
    }
}

const FIXTURE: &str = r#"
function outer({key: p = init}, ...rest) {
 let \u0061, {key: local = value, short, ...tail} = source;
 const [first, ...others] = source; class C {} function g() {} function* gen() {}
 label: again: function labelled() {} label2: let labelledLet;
 { let inner; const k = 1; class D {} label: function blockFn() {} }
 switch(p) { case 1: let caseLet; function caseFn() {} break; default: const d = 2; }
 for(let i=0,j=1;i<2;i++) {} for(var v=0;v<1;v++) {}
 for(const {key: ofName} of source) {} for(let [inName] in source) {}
 for(var hoisted in source) {} for (plain of source) {}
 try {} catch({key: caught = fallback, ...catchRest}) { let c; }
 try {} catch {} const f = function named(q = defaultValue) { var bodyVar; };
 class Static { static { let staticLet; var staticVar; } method(x) {} }
}
"#;

#[test]
fn every_scope_name_and_byte_js_ts_tsx() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        equivalent(FIXTURE, language, true);
    }
    equivalent(
        r#"
function typed<T>(p: T, q?: T): p is T {
 abstract class Abstract {} enum E { A, "b" = 2, \u0063 = 3 }
 enum E { D = A, "\u0065" = 4 } enum Other { Foreign }
 { enum E { Nested } } const el = <Widget value={p}/>;
 return true;
}
const method = { [computed](p = seed): typeof p { let bodyOnly; } };
const single = p => p; const named = function self(p) { return self; };
const generator = function* gen(p) { yield p; };
"#,
        Language::Tsx,
        true,
    );
}

#[test]
fn big_declarations_and_recovery() {
    let mut source = String::from("function outer(){");
    for i in 0..64 {
        source.push_str(&format!(
            "let l{i}; const c{i}=1; class C{i}{{}} function f{i}(){{}} enum E{i}{{M{i}}}"
        ));
    }
    source.push('}');
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        equivalent(&source, language, false);
        for broken in [
            r#"function broken({key: p = }) { let {key: x,...rest} = ; for(let i=;i<;){} catch(){} }"#,
            r#"function broken() { let \u{110000}, \uD800, \u0020, \u0030, \u{zz}, \u0069f; }"#,
            "function broken() { let [missing,...] = x; if ( { const recovered=;",
            "enum E { A=, 'x'=1 } enum E { B = A } enum E { C",
        ] {
            let p = ParsedFile::parse("broken", broken, language).unwrap();
            assert!(p.tree.root_node().has_error());
            equivalent(broken, language, true);
        }
    }
}

#[test]
fn callable_exception_labels_and_enum_merge() {
    let p = ParsedFile::parse(
        "exact.ts",
        r#"
function f() { top: nested: function hidden() {} let local;
 { top: nested: function visible() {} } }
enum E { A, "\u0062"=1 } enum \u0045 { C=A } enum Other { Foreign }
{ enum E { Nested } }
"#,
        Language::TypeScript,
    )
    .unwrap();
    let boundaries = p.language.callable_boundary_node_types();
    let all = nodes(&p);
    let bodies: Vec<_> = all
        .iter()
        .copied()
        .filter(|n| n.kind() == "statement_block")
        .collect();
    let body = bodies
        .iter()
        .copied()
        .find(|b| p.node_text(b).contains("let local"))
        .unwrap();
    let block = bodies
        .iter()
        .copied()
        .find(|b| {
            p.node_text(b)
                .starts_with("{ top: nested: function visible")
        })
        .unwrap();
    let enums: Vec<_> = all
        .iter()
        .copied()
        .filter(|n| n.kind() == "enum_body")
        .collect();
    let e = enums
        .iter()
        .copied()
        .find(|e| p.node_text(e).contains("\\u0062"))
        .unwrap();
    let _session = Session::new(&p);
    assert!(!p.js_ts_scope_binds(body, &boundaries, "hidden", 0));
    assert!(p.js_ts_scope_binds(block, &boundaries, "visible", 0));
    assert!(!p.js_ts_scope_binds(block, &boundaries, "hidden", 0));
    for name in ["A", "b", "C"] {
        assert!(p.js_ts_scope_binds(e, &boundaries, name, 0), "{name}");
    }
    for name in ["Foreign", "Nested", "absent", "\\u0062"] {
        assert!(!p.js_ts_scope_binds(e, &boundaries, name, 0), "{name}");
    }
    // Callable-body status must be recomputed from boundary contents.
    assert!(p.js_ts_scope_binds_uncached(body, &[], "hidden", 0));
    assert!(!p.js_ts_scope_binds_uncached(body, &boundaries, "hidden", 0));
}

#[test]
fn scope_cache_keys_lifetime_and_bypass() {
    let mut p = ParsedFile::parse(
        "same.js",
        "function f(p){let x;} function g(q){let y;}",
        Language::JavaScript,
    )
    .unwrap();
    let q = ParsedFile::parse("other.js", "function f(p){let z;}", Language::JavaScript).unwrap();
    let boundaries = p.language.callable_boundary_node_types();
    let owners = p.all_functions();
    let a = owners[0].child_by_field_name("body").unwrap();
    let b = owners[1].child_by_field_name("body").unwrap();
    let queries = || {
        for name in ["x", "y", "p", "absent"] {
            p.js_ts_scope_binds_uncached(a, &boundaries, name, 0);
        }
    };
    assert_eq!(probe::count_scope_walks(queries), 4);
    {
        let first = Session::new(&p);
        let _q = Session::new(&q);
        assert_eq!(probe::count_scope_walks(queries), 1);
        assert_eq!(probe::count_scope_walks(queries), 0);
        assert!(!p.js_ts_scope_binds(b, &boundaries, "x", 0));
        assert!(p.js_ts_scope_binds(b, &boundaries, "y", 0));
        let qb = q.all_functions()[0].child_by_field_name("body").unwrap();
        assert!(q.js_ts_scope_binds(qb, &boundaries, "z", 0));
        assert!(!q.js_ts_scope_binds(qb, &boundaries, "x", 0));
        assert_eq!(
            probe::count_scope_walks(|| {
                p.js_ts_scope_binds_uncached(a, &boundaries.clone(), "x", 0);
            }),
            0
        );
        let second = Session::new(&p);
        assert_eq!(probe::count_scope_walks(queries), 1);
        drop(first);
        assert_eq!(probe::count_scope_walks(queries), 0);
        let _fresh = Session::new(&p);
        assert_eq!(
            probe::count_scope_walks(|| binding_verdict_memo::direct(queries)),
            0,
            "bypass must use the per-name reference, never the collector"
        );
        drop(_fresh);
        drop(second);
    }
    assert_eq!(probe::count_scope_walks(queries), 4);
    p = ParsedFile::parse(
        "same.js",
        "function f(){let replaced;}",
        Language::JavaScript,
    )
    .unwrap();
    let _session = Session::new(&p);
    let body = p.all_functions()[0].child_by_field_name("body").unwrap();
    assert!(p.js_ts_scope_binds(body, &boundaries, "replaced", 0));
    assert!(!p.js_ts_scope_binds(body, &boundaries, "x", 0));
}

#[test]
fn every_scanning_branch_collects_once() {
    let source = format!("{FIXTURE}\nenum E {{ A }} enum E {{ B }} const single = x => x;");
    let p = ParsedFile::parse("synthetic.ts", &source, Language::TypeScript).unwrap();
    let boundaries = p.language.callable_boundary_node_types();
    let mut seen = std::collections::BTreeSet::new();
    for scope in nodes(&p).into_iter().filter(|n| {
        boundaries.contains(&n.kind())
            || matches!(
                n.kind(),
                "statement_block"
                    | "switch_body"
                    | "for_statement"
                    | "for_in_statement"
                    | "catch_clause"
                    | "enum_body"
            )
    }) {
        seen.insert(scope.kind());
        let byte = scope
            .child_by_field_name("body")
            .map_or(scope.start_byte(), |b| b.start_byte());
        let _session = Session::new(&p);
        assert_eq!(
            probe::count_scope_walks(|| {
                for name in [
                    "p", "a", "local", "caught", "i", "ofName", "A", "B", "absent", "",
                ] {
                    p.js_ts_scope_binds_uncached(scope, &boundaries, name, byte);
                }
            }),
            1,
            "one collection for {}@{}",
            scope.kind(),
            scope.start_byte()
        );
    }
    for kind in [
        "function_declaration",
        "function_expression",
        "arrow_function",
        "statement_block",
        "switch_body",
        "for_statement",
        "for_in_statement",
        "catch_clause",
        "enum_body",
    ] {
        assert!(seen.contains(kind), "uncovered collector branch: {kind}");
    }
}
