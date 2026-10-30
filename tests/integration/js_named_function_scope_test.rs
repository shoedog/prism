//! A function expression's explicit name is local to its body, not its property.
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
            "no-outer",
            "const o = { f: function f7() {}, n() { return /*site*/f7(); } };",
            None,
        ),
        (
            "different-const",
            "const g = function f7() {}; g(); /*site*/f7();",
            None,
        ),
        (
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
        ("inferred-property", "const o = { f7: function () {}, n() { return /*site*/f7(); } };", None),
    ]);
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
