//! Property method names never introduce lexical bindings. Parse fixtures only.
use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSiteOrigin};
use prism::languages::Language;
use prism::resolution::ResolutionConfidence::Exact;
use std::collections::BTreeMap;

fn graphs(source: &str, ext: &str) -> Vec<CallGraph> {
    let files = BTreeMap::from([
        (format!("a.{ext}"), source.to_string()),
        (
            format!("dep.{ext}"),
            "export function resolveName() {}".into(),
        ),
    ])
    .into_iter()
    .filter(|(p, _)| p.starts_with("a.") || source.contains("import "))
    .map(|(p, s)| {
        let parsed = ParsedFile::parse(&p, &s, Language::from_path(&p).unwrap()).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{p}: {s}");
        (p, parsed)
    })
    .collect();
    vec![
        CallGraph::build(&files),
        CallGraph::build_skeleton(&files),
        CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
    ]
}

type Case<'a> = (&'a str, String, Option<(&'a str, usize)>);
// Collect every failing shape before failing, so the fail-first receipt is exhaustive.
fn check(cases: &[Case<'_>]) {
    check_named(cases, "resolveName");
}

fn check_named(cases: &[Case<'_>], name: &str) {
    check_languages(cases, name, &["js", "ts", "tsx"]);
}

fn check_languages(cases: &[Case<'_>], name: &str, languages: &[&str]) {
    let mut failures = Vec::new();
    for ext in languages {
        for (label, source, expected) in cases {
            for (builder, graph) in graphs(source, ext).into_iter().enumerate() {
                // Skeletons deliberately omit returned-value/export proof passes.
                if expected.is_some() && builder != 0 {
                    continue;
                }
                let sites: Vec<_> = graph
                    .calls
                    .values()
                    .flatten()
                    .filter(|s| {
                        s.origin == CallSiteOrigin::Source
                            && s.callee_name == name
                            && s.qualifier.is_none()
                    })
                    .collect();
                assert_eq!(sites.len(), 1, "{label}/{ext}/{builder}: {sites:?}");
                let site = sites[0];
                let out = graph.resolve_call_site_full(site);
                let exact: Vec<_> = out
                    .resolved
                    .iter()
                    .filter(|r| r.confidence == Exact)
                    .map(|r| (r.target.name.as_str(), r.target.start_line))
                    .collect();
                let wanted: Vec<_> = expected.iter().copied().collect();
                if exact != wanted {
                    failures.push(format!("{label}/{ext}/{builder}: binding={:?}, owner={:?}, method_owner={:?}, got={out:?}, want={wanted:?}",
                        site.local_binding, site.caller, graph.method_owners.get(&site.caller)));
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
fn unresolved_values_never_bind_property_method_names() {
    let mut cases = Vec::new();
    for binding in ["var", "let"] {
        for (shape, owner) in [
            (
                "object",
                "const host = { resolveName() { return resolveName(); } };",
            ),
            (
                "getter",
                "const host = { get resolveName() { return resolveName(); } };",
            ),
            (
                "setter",
                "const host = { set resolveName(v) { resolveName(); } };",
            ),
            (
                "class",
                "class Host { resolveName() { return resolveName(); } }",
            ),
            (
                "class-getter",
                "class Host { get resolveName() { return resolveName(); } }",
            ),
            (
                "class-setter",
                "class Host { set resolveName(v) { resolveName(); } }",
            ),
            (
                "function-property",
                "const host = { resolveName: function() { return resolveName(); } };",
            ),
        ] {
            cases.push((shape, format!("function make() {{ function actualResolver() {{}} return actualResolver; }}\n{binding} resolveName = make();\n{owner}\n"), None));
        }
    }
    cases.push((
        "parameter",
        "function host(resolveName) { return { resolveName() { return resolveName(); } }; }".into(),
        None,
    ));
    cases.push((
        "no-binding",
        "const host = { resolveName() { return resolveName(); } };".into(),
        None,
    ));
    check(&cases);
}

#[test]
fn proven_outer_bindings_and_named_function_expressions_keep_exact() {
    check(&[
        ("const-control", "function make() { function actualResolver() {} return actualResolver; }\nconst resolveName = make();\nconst host = { resolveName() { return resolveName(); } };".into(), Some(("actualResolver", 1))),
        ("outer-function", "function resolveName() {}\nconst host = { resolveName() { return resolveName(); } };".into(), Some(("resolveName", 1))),
        ("class-outer-function", "function resolveName() {}\nclass Host { resolveName() { return resolveName(); } }".into(), Some(("resolveName", 1))),
        ("import", "import { resolveName } from './dep';\nconst host = { resolveName() { return resolveName(); } };".into(), Some(("resolveName", 1))),
        ("named-expression", "const host = { m: function resolveName() { return resolveName(); } };".into(), Some(("resolveName", 1))),
        ("function-property-const", "function actualResolver() {}\nconst resolveName = actualResolver;\nconst host = { resolveName: function() { return resolveName(); } };".into(), Some(("actualResolver", 1))),
    ]);
}

#[test]
fn supplied_identity_fixtures() {
    check(&[
        ("identity-var", "function createNameResolver() {\n    return function actualResolver() { return 17; };\n}\nexport function makeChecker() {\n    var resolveName = createNameResolver();\n    return {\n        resolveName() {\n            return resolveName();\n        },\n    };\n}\n".into(), None),
        ("identity", "function createNameResolver() {\n    return function actualResolver() { return 17; };\n}\nexport function makeChecker() {\n    const resolveName = createNameResolver();\n    return {\n        resolveName() {\n            return resolveName();\n        },\n    };\n}\n".into(), Some(("actualResolver", 2))),
    ]);
}

#[test]
fn property_exclusion_survives_cache_hits_and_unrelated_partial_rebuilds() {
    use prism::cpg::CodePropertyGraph;
    use prism::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    let assert_no_exact = |graph: &CallGraph| {
        let site = graph
            .calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "resolveName" && s.qualifier.is_none())
            .unwrap();
        assert!(graph
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .all(|r| r.confidence != Exact));
    };
    for ext in ["js", "ts", "tsx"] {
        let source = "var resolveName = external();\nconst host = { resolveName() { return resolveName(); } };";
        let original = graphs(source, ext).remove(0);
        let bytes = bincode::serialize(&original).unwrap();
        let mut restored: CallGraph = bincode::deserialize(&bytes).unwrap();
        assert_no_exact(&restored);
        // Clearing cross-file proof authority must retain unrelated negative syntax facts.
        restored.remove_files(&[format!("other.{ext}")].into_iter().collect());
        assert_no_exact(&restored);

        let sources = BTreeMap::from([
            (format!("a.{ext}"), source.to_string()),
            (format!("other.{ext}"), "function other() {}".into()),
        ]);
        let parse = |sources: &BTreeMap<String, String>| {
            sources
                .iter()
                .map(|(p, s)| {
                    (
                        p.clone(),
                        ParsedFile::parse(p, s, Language::from_path(p).unwrap()).unwrap(),
                    )
                })
                .collect()
        };
        let cold = CodePropertyGraph::build(&parse(&sources));
        let cache = tempfile::tempdir().unwrap();
        save_cache(&cold, &compute_file_hashes(&sources), false, cache.path()).unwrap();
        let CacheResult::Hit(warm) =
            load_cache(&compute_file_hashes(&sources), false, cache.path())
        else {
            panic!("full cache hit required");
        };
        assert_no_exact(&warm.call_graph);
        for (file, replacement) in [
            (format!("other.{ext}"), "function changed() {}"),
            (
                format!("a.{ext}"),
                "function resolveName() {}\nfunction run() { resolveName(); }",
            ),
        ] {
            let mut after = sources.clone();
            after.insert(file.clone(), replacement.into());
            let CacheResult::PartialHit {
                cached_call_graph,
                cached_dfg,
                changed_files,
            } = load_cache(&compute_file_hashes(&after), false, cache.path())
            else {
                panic!("partial cache hit required");
            };
            let partial = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &parse(&after),
                None,
            );
            if file.starts_with("other.") {
                assert_no_exact(&partial.call_graph);
            } else {
                let site = partial
                    .call_graph
                    .calls
                    .values()
                    .flatten()
                    .find(|s| s.callee_name == "resolveName")
                    .unwrap();
                let out = partial.call_graph.resolve_call_site_full(site);
                assert_eq!(out.resolved.len(), 1);
                assert_eq!(out.resolved[0].confidence, Exact);
                assert_eq!(out.resolved[0].target.start_line, 1);
            }
        }
    }
}

fn property_shapes() -> Vec<String> {
    let mut shapes = Vec::new();
    for value in ["() => resolveName()", "function() { resolveName(); }"] {
        for object in ["obj", "exports", "this", "Foo.prototype", "module.exports"] {
            shapes.push(format!("{object}.resolveName = {value};"));
        }
        for class in ["const C = class", "export default class", "class Named"] {
            for modifier in ["", "static "] {
                shapes.push(format!("{class} {{ {modifier}resolveName = {value}; }};"));
            }
        }
    }
    shapes
}

#[test]
fn inferred_members_and_fields_are_not_lexical_bindings() {
    let shapes = property_shapes();
    let mut cases = Vec::new();
    for shape in &shapes {
        cases.push((shape.as_str(), format!("function make() {{ function actual() {{}} return actual; }}\nvar resolveName = make();\n{shape}"), None));
        cases.push((shape.as_str(), shape.clone(), None));
    }
    cases.push((
        "different-declarator",
        "var different = (host.resolveName = () => resolveName());".into(),
        None,
    ));
    cases.push((
        "wrapped-call-not-value",
        "var resolveName = wrap(host.resolveName = () => resolveName());".into(),
        None,
    ));
    cases.push((
        "sequence-not-value",
        "var resolveName = (host.resolveName = () => resolveName(), other);".into(),
        None,
    ));
    check(&cases);
    check_named(
        &[(
            "readFile-wrapper",
            "const readFile = host.readFile; host.readFile = (n) => readFile(n);".into(),
            None,
        )],
        "readFile",
    );
}

#[test]
fn property_shapes_keep_proven_outer_bindings() {
    let shapes = property_shapes();
    let mut cases = Vec::new();
    for shape in &shapes {
        for (binding, expected) in [
            ("function resolveName() {}", "resolveName"),
            ("function actual() {} const resolveName = actual;", "actual"),
            ("import { resolveName } from './dep';", "resolveName"),
        ] {
            cases.push((
                shape.as_str(),
                format!("{binding}\n{shape}"),
                Some((expected, 1)),
            ));
        }
    }
    check(&cases);
}

#[test]
fn member_assignment_value_of_same_named_declarator_keeps_exact() {
    let mut cases = Vec::new();
    for binding in ["var", "let", "const"] {
        for object in ["obj", "exports", "this", "Foo.prototype", "module.exports"] {
            for value in ["() => 0", "function() {}"] {
                for (open, close) in [("", ""), ("(", ")"), ("((", "))")] {
                    cases.push(("same-named-value", format!("{binding} resolveName = {open}{object}.resolveName = {value}{close};\nfunction run() {{ resolveName(); }}"), Some(("resolveName", 1))));
                }
            }
        }
    }
    cases.push((
        "identifier-assignment",
        "var resolveName; resolveName = () => 0;\nfunction run() { resolveName(); }".into(),
        Some(("resolveName", 1)),
    ));
    cases.push((
        "explicit-member-name",
        "obj.other = function resolveName() { resolveName(); };".into(),
        Some(("resolveName", 1)),
    ));
    cases.push((
        "explicit-field-name",
        "const C = class { other = function resolveName() { resolveName(); }; };".into(),
        Some(("resolveName", 1)),
    ));
    check(&cases);
    check_named(
        &[(
            "woff2-exemption",
            "var _free = (Module._free = function() {});\nfunction run() { _free(); }".into(),
            Some(("_free", 1)),
        )],
        "_free",
    );
}

#[test]
fn singleton_r5_rejects_property_names_across_files() {
    use prism::resolution::DropReason;
    let mut failures = Vec::new();
    for ext in ["js", "ts", "tsx"] {
        let mut shapes = property_shapes();
        shapes.push("const o = { resolveName() {} };".into());
        shapes.push("const o = { resolveName: () => 0 };".into());
        if ext != "js" {
            shapes.push("abstract class C { resolveName = () => 0; }".into());
            shapes.push("abstract class C { static resolveName = function() {}; }".into());
        }
        for shape in shapes {
            let target = shape.replace("resolveName", "helper");
            // Bodies are irrelevant here; avoid introducing extra helper() sites.
            let target = target
                .replace("=> helper()", "=> 0")
                .replace("{ helper(); }", "{}");
            let files = BTreeMap::from([
                (
                    format!("a.{ext}"),
                    ParsedFile::parse(
                        &format!("a.{ext}"),
                        &target,
                        Language::from_extension(ext).unwrap(),
                    )
                    .unwrap(),
                ),
                (
                    format!("b.{ext}"),
                    ParsedFile::parse(
                        &format!("b.{ext}"),
                        "function g() { helper(); }",
                        Language::from_extension(ext).unwrap(),
                    )
                    .unwrap(),
                ),
            ]);
            assert!(files.values().all(|p| !p.tree.root_node().has_error()));
            for graph in [
                CallGraph::build(&files),
                CallGraph::build_skeleton(&files),
                CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
            ] {
                let site = graph
                    .calls
                    .values()
                    .flatten()
                    .find(|s| s.callee_name == "helper")
                    .unwrap();
                let out = graph.resolve_call_site_full(site);
                if !out.resolved.is_empty()
                    || !matches!(
                        out.drop,
                        Some(DropReason::LocalBindingUnproven | DropReason::UnknownName)
                    )
                {
                    failures.push(format!("{ext}: {target}: {out:?}"));
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
fn abstract_class_fields_share_property_exclusion() {
    let shapes: Vec<_> = ["", "static "]
        .into_iter()
        .flat_map(|modifier| {
            ["() => resolveName()", "function() { resolveName(); }"]
                .map(|value| format!("abstract class C {{ {modifier}resolveName = {value}; }}"))
        })
        .collect();
    let mut cases = Vec::new();
    for shape in &shapes {
        cases.push((
            shape.as_str(),
            format!("var resolveName = unknown();\n{shape}"),
            None,
        ));
        cases.push((
            shape.as_str(),
            format!("function resolveName() {{}}\n{shape}"),
            Some(("resolveName", 1)),
        ));
    }
    check_languages(&cases, "resolveName", &["ts", "tsx"]);
}
