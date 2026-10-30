//! A function expression's explicit name is local to its parameters and body.
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::ResolutionConfidence::Exact;
use std::collections::BTreeMap;

fn files(source: &str, ext: &str) -> BTreeMap<String, ParsedFile> {
    [
        (format!("a.{ext}"), source),
        (format!("dep.{ext}"), "export function f7() {}"),
    ]
    .into_iter()
    .filter(|(path, _)| path.starts_with("a.") || source.contains("import "))
    .map(|(path, source)| {
        let parsed = ParsedFile::parse(&path, source, Language::from_path(&path).unwrap()).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{ext}: {source}");
        (path, parsed)
    })
    .collect()
}

// Mark the callee with a comment to select the site by bytes, even on one line.
fn exact_targets(graph: &CallGraph, source: &str) -> Vec<(String, String, usize)> {
    let start = source.find("/*site*/").unwrap() + "/*site*/".len();
    let site = graph
        .calls
        .values()
        .flatten()
        .find(|s| s.start_byte == start && s.caller.file.starts_with("a."))
        .unwrap();
    let outcome = graph.resolve_call_site_full(site);
    outcome
        .resolved
        .iter()
        .filter(|r| r.confidence == Exact)
        .map(|r| {
            (
                r.target.file.clone(),
                r.target.name.clone(),
                r.target.start_line,
            )
        })
        .collect()
}

type Case<'a> = (&'a str, &'a str, Option<(&'a str, &'a str, usize)>);

fn check(cases: &[Case<'_>]) {
    let mut failures = Vec::new();
    for ext in ["js", "ts", "tsx"] {
        for (label, source, expected) in cases {
            let parsed = files(source, ext);
            for (builder, graph) in [
                CallGraph::build(&parsed),
                CallGraph::build_skeleton(&parsed),
                CallGraph::build_direct_subset(&parsed, &parsed.keys().cloned().collect()),
            ]
            .into_iter()
            .enumerate()
            {
                // These constructors deliberately omit cross-file import proofs.
                if *label == "import" && builder != 0 {
                    continue;
                }
                let got = exact_targets(&graph, source);
                let want: Vec<_> = expected
                    .iter()
                    .map(|(file, name, line)| (format!("{file}.{ext}"), name.to_string(), *line))
                    .collect();
                if got != want {
                    failures.push(format!(
                        "{label}/{ext}/builder{builder}: got {got:?}, want {want:?}"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn outside_body_never_binds_expression_name() {
    check(&[
        (
            "issue-450",
            "var f7 = make();\nconst o = { f7: function f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
        (
            "let",
            "let f7 = make();\nconst o = { f7: function f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
        (
            "const-unknown",
            "const f7 = make();\nconst o = { f7: function f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
        (
            // Guard: already refused on main.
            "no-outer",
            "const o = { f: function f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
        (
            // Guard: already refused on main.
            "different-const",
            "const g = function f7() {}; g(); /*site*/f7();",
            None,
        ),
        (
            // Guard: already refused on main.
            "same-line-before",
            "function run() { /*site*/f7(); const g = function f7() {}; }",
            None,
        ),
        (
            "same-line-var",
            "var f7 = make(); const o = { f: function f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
        (
            // Future-proofing guard: named generators are not currently indexed.
            "generator-expression",
            "var f7 = make(); const o = { f: function* f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
    ]);
}

#[test]
fn recursion_and_nested_callables_keep_expression_binding() {
    check(&[
        (
            "recursion",
            "const o = { f: function f7(n) { return n && /*site*/f7(n - 1); } };",
            Some(("a", "f7", 1)),
        ),
        (
            "body-fallback-control",
            "const o = { f: function f7() { f7 = unknown(); return /*site*/f7(); } };",
            Some(("a", "f7", 1)),
        ),
        (
            "nested-function",
            "const o = { f: function f7() { function nested() { return /*site*/f7(); } } };",
            Some(("a", "f7", 1)),
        ),
        (
            "nested-arrow",
            "const o = { f: function f7() { return () => /*site*/f7(); } };",
            Some(("a", "f7", 1)),
        ),
        (
            "nested-method",
            "const o = { f: function f7() { return { n() { return /*site*/f7(); } }; } };",
            Some(("a", "f7", 1)),
        ),
        (
            "outer-collision",
            "function f7() {}\nconst g = function f7() { return /*site*/f7(); };",
            Some(("a", "f7", 2)),
        ),
        (
            "inner-shadow",
            "const g = function f7() { function nested(f7) { return /*site*/f7(); } };",
            None,
        ),
    ]);
}

#[test]
fn distinct_outer_bindings_and_inferred_names_keep_their_targets() {
    check(&[
        ("declaration", "function f7() {}\nconst o = { f: function f7() {}, n() { return /*site*/f7(); } };", Some(("a", "f7", 1))),
        ("const", "function actual() {}\nconst f7 = actual;\nconst o = { f: function f7() {}, n() { return /*site*/f7(); } };", Some(("a", "actual", 1))),
        ("var", "var f7 = function actual() {};\nconst o = { f: function f7() {}, n() { return /*site*/f7(); } };", Some(("a", "actual", 1))),
        ("let", "let f7 = function actual() {};\nconst o = { f: function f7() {}, n() { return /*site*/f7(); } };", Some(("a", "actual", 1))),
        ("import", "import { f7 } from './dep';\nconst o = { f: function f7() {}, n() { return /*site*/f7(); } };", Some(("dep", "f7", 1))),
        ("different-const-control", "const g = function f7() {}; /*site*/g(); f7();", Some(("a", "f7", 1))),
        ("inferred-declarator", "const g = function () {}; /*site*/g();", Some(("a", "g", 1))),
        ("inferred-assignment-fallback-guard", "let s7; s7 = function () {}; function getS() { return /*site*/s7(); }", Some(("a", "s7", 1))),
        ("named-assignment-refusal-guard", "let t7; t7 = function t7() {}; function getT() { return /*site*/t7(); }", None),
        ("inferred-property", "const o = { f7: function () {}, n() { return /*site*/f7(); } };", None),
    ]);
}

#[test]
fn parameter_defaults_see_self_but_not_shadowed_or_enclosing_names() {
    check(&[
        (
            "default-fallback",
            "const g = function f(a = /*site*/f(0)) { f = 1; };",
            Some(("a", "f", 1)),
        ),
        // Guards: positive binding proof and lexical shadowing already work.
        (
            "default-proven-guard",
            "const g = function f(a = /*site*/f(0)) {};",
            Some(("a", "f", 1)),
        ),
        (
            "parameter-shadow-guard",
            "const g = function f(f, a = /*site*/f(0)) { f = 1; };",
            None,
        ),
        (
            "enclosing-default-guard",
            "function outer(a = /*site*/f(0)) { const g = function f() { f = 1; }; }",
            None,
        ),
    ]);
}

#[test]
fn imports_do_not_expose_private_expression_names() {
    use prism::resolution::ResolutionKind::{ImportMember, ImportQualified};
    let mut failures = Vec::new();
    for ext in ["js", "ts", "tsx"] {
        for (label, caller, barrel, kind) in [
            (
                "direct",
                "import { f7 } from './dep'; function run() { /*site*/f7(); }",
                None,
                ImportMember,
            ),
            (
                "barrel",
                "import { f7 } from './barrel'; function run() { /*site*/f7(); }",
                Some("export { f7 } from './dep';"),
                ImportMember,
            ),
            (
                "star-barrel",
                "import { f7 } from './barrel'; function run() { /*site*/f7(); }",
                Some("export * from './dep';"),
                ImportMember,
            ),
            (
                "namespace",
                "import * as A from './dep'; function run() { /*site*/A.f7(); }",
                None,
                ImportQualified,
            ),
            (
                "cjs-member",
                "const { f7 } = require('./dep'); function run() { /*site*/f7(); }",
                None,
                ImportMember,
            ),
        ] {
            for (control, dep, want) in [
                (
                    "private",
                    "let f7; f7 = make(); const g = function f7() {}; export { f7 };",
                    false,
                ),
                // Guards: a callable span proves the exported binding, even when
                // its local name happens to equal the expression's private name.
                ("verified", "export const f7 = function f7() {};", true),
                ("declaration", "function f7() {} export { f7 };", true),
            ] {
                let mut parsed = files(caller, ext);
                for (stem, source) in [("dep", Some(dep)), ("barrel", barrel)] {
                    if let Some(source) = source {
                        let path = format!("{stem}.{ext}");
                        let file =
                            ParsedFile::parse(&path, source, Language::from_path(&path).unwrap())
                                .unwrap();
                        assert!(!file.tree.root_node().has_error());
                        parsed.insert(path, file);
                    }
                }
                let graph = CallGraph::build(&parsed);
                let got = exact_targets(&graph, caller);
                let expected = if want {
                    vec![(format!("dep.{ext}"), "f7".into(), 1)]
                } else {
                    vec![]
                };
                if got != expected {
                    failures.push(format!(
                        "{label}/{control}/{ext}: got {got:?}, want {expected:?}"
                    ));
                }
                if want && !got.is_empty() {
                    let start = caller.find("/*site*/").unwrap() + "/*site*/".len();
                    let site = graph
                        .calls
                        .values()
                        .flatten()
                        .find(|s| s.caller.file == format!("a.{ext}") && s.start_byte == start)
                        .unwrap();
                    assert!(graph
                        .resolve_call_site_full(site)
                        .resolved
                        .iter()
                        .all(|r| r.kind == kind));
                }
            }
        }
        // Guard against indiscriminately excluding verified renamed exports.
        for (caller, barrel) in [
            (
                "import { g } from './dep'; function run() { /*site*/g(); }",
                None,
            ),
            (
                "import { g } from './barrel'; function run() { /*site*/g(); }",
                Some("export { g } from './dep';"),
            ),
        ] {
            let mut parsed = files(caller, ext);
            for (stem, source) in [
                ("dep", Some("export const g = function f7() {};")),
                ("barrel", barrel),
            ] {
                if let Some(source) = source {
                    let path = format!("{stem}.{ext}");
                    parsed.insert(
                        path.clone(),
                        ParsedFile::parse(&path, source, Language::from_path(&path).unwrap())
                            .unwrap(),
                    );
                }
            }
            assert_eq!(
                exact_targets(&CallGraph::build(&parsed), caller),
                vec![(format!("dep.{ext}"), "f7".into(), 1)]
            );
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn expression_name_does_not_escape_to_other_files() {
    for ext in ["js", "ts", "tsx"] {
        let source = "const g = function f7() {};";
        let mut parsed = files(source, ext);
        let caller = "function run() { /*site*/f7(); }";
        let path = format!("b.{ext}");
        parsed.insert(
            path.clone(),
            ParsedFile::parse(&path, caller, Language::from_path(&path).unwrap()).unwrap(),
        );
        let graph = CallGraph::build(&parsed);
        let site = graph
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.file == path)
            .unwrap();
        assert!(
            graph
                .resolve_call_site_full(site)
                .resolved
                .iter()
                .all(|r| r.confidence != Exact),
            "{ext}: expression name escaped across files"
        );
    }
}

#[test]
fn expression_scope_survives_full_and_partial_cache_hits() {
    use prism::cpg::CodePropertyGraph;
    use prism::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    for ext in ["js", "ts", "tsx"] {
        let source =
            "var f7 = make();\nconst o = { f: function f7() {}, n() { return /*site*/f7(); } };";
        let mut parsed = files(source, ext);
        let other = format!("other.{ext}");
        parsed.insert(
            other.clone(),
            ParsedFile::parse(
                &other,
                "function other() {}",
                Language::from_path(&other).unwrap(),
            )
            .unwrap(),
        );
        let sources: BTreeMap<_, _> = parsed
            .iter()
            .map(|(p, f)| (p.clone(), f.source.clone()))
            .collect();
        let cache = tempfile::tempdir().unwrap();
        let cold = CodePropertyGraph::build(&parsed);
        assert!(exact_targets(&cold.call_graph, source).is_empty());
        save_cache(&cold, &compute_file_hashes(&sources), false, cache.path()).unwrap();
        let CacheResult::Hit(warm) =
            load_cache(&compute_file_hashes(&sources), false, cache.path())
        else {
            panic!("full hit required")
        };
        assert!(exact_targets(&warm.call_graph, source).is_empty());
        let bytes = bincode::serialize(&warm.call_graph).unwrap();
        let mut restored: CallGraph = bincode::deserialize(&bytes).unwrap();
        // MB-15: assert before incremental rebuilding can reconstruct all proofs.
        restored.remove_files(&[other.clone()].into_iter().collect());
        assert!(exact_targets(&restored, source).is_empty());
        for (path, replacement) in [
            (other, "function changed() {}"),
            (
                format!("a.{ext}"),
                "function f7() {}\nfunction run() { /*site*/f7(); }",
            ),
        ] {
            let mut after = sources.clone();
            after.insert(path.clone(), replacement.into());
            let mut after_parsed = files(after.get(&format!("a.{ext}")).unwrap(), ext);
            for (p, s) in &after {
                after_parsed.insert(
                    p.clone(),
                    ParsedFile::parse(p, s, Language::from_path(p).unwrap()).unwrap(),
                );
            }
            let CacheResult::PartialHit {
                cached_call_graph,
                cached_dfg,
                changed_files,
            } = load_cache(&compute_file_hashes(&after), false, cache.path())
            else {
                panic!("partial hit required")
            };
            let partial = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &after_parsed,
                None,
            );
            if path.starts_with("a.") {
                assert_eq!(
                    exact_targets(&partial.call_graph, replacement),
                    vec![(format!("a.{ext}"), "f7".into(), 1)]
                );
            } else {
                assert!(exact_targets(&partial.call_graph, source).is_empty());
            }
        }
    }
}

// Each positive has a private-name twin with the same export syntax. Namespace
// importers also exercise the legacy stem fallback when terminal proof refuses.
fn cjs_expression_values(want: bool) {
    let mut failures = Vec::new();
    for ext in ["js", "ts", "tsx"] {
        for (label, positive, negative) in [
            (
                "const-member",
                "const origin = function origin(input) { return input; }; exports.origin = origin;",
                "const value = function origin() {}; exports.origin = value;",
            ),
            (
                "const-object",
                "const origin = function origin() {}; module.exports = { origin: origin };",
                "const value = function origin() {}; module.exports = { origin: value };",
            ),
            (
                "var-module-member",
                "var origin = function origin() {}; module.exports.origin = origin;",
                "var value = function origin() {}; module.exports.origin = value;",
            ),
            (
                "direct-member",
                "exports.origin = function origin() {};",
                "exports.value = function origin() {};",
            ),
            (
                "let-member",
                "let origin = function origin() {}; exports.origin = origin;",
                "let value = function origin() {}; exports.origin = value;",
            ),
            (
                "direct-module-member",
                "module.exports.origin = function origin() {};",
                "module.exports.value = function origin() {};",
            ),
        ] {
            for caller in [
                "const { origin: invoke } = require('./dep'); function run() { /*site*/invoke(); }",
                "import * as ns from './dep'; function run() { /*site*/ns.origin(); }",
            ] {
                let source = if want { positive } else { negative };
                let path = format!("dep.{ext}");
                let mut parsed = files(caller, ext);
                let file =
                    ParsedFile::parse(&path, source, Language::from_path(&path).unwrap()).unwrap();
                assert!(!file.tree.root_node().has_error());
                parsed.insert(path.clone(), file);
                let got = exact_targets(&CallGraph::build(&parsed), caller);
                let expected = if want {
                    vec![(path, "origin".into(), 1)]
                } else {
                    vec![]
                };
                if got != expected {
                    failures.push(format!(
                        "{label}/{ext}/{caller}: got {got:?}, want {expected:?}"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn cjs_direct_expression_values_keep_exact() {
    cjs_expression_values(true);
}

#[test]
fn cjs_different_values_do_not_expose_private_names() {
    cjs_expression_values(false);
}
