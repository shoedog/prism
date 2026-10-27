use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, CallSite},
    languages::Language,
    resolution::{DropReason, ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;

fn graph() -> CallGraph {
    let files: BTreeMap<_, _> = [
        (
            "src/lib.rs",
            include_str!("../fixtures/rust-self-calls/lib.rs"),
        ),
        (
            "src/cases.rs",
            include_str!("../fixtures/rust-self-calls/cases.rs"),
        ),
        (
            "src/other.rs",
            include_str!("../fixtures/rust-self-calls/other.rs"),
        ),
        (
            "src/split.rs",
            include_str!("../fixtures/rust-self-calls/split.rs"),
        ),
    ]
    .into_iter()
    .map(|(path, source)| {
        let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{path}");
        (path.to_string(), parsed)
    })
    .collect();
    let cg = CallGraph::build(&files);
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}

fn site<'a>(cg: &'a CallGraph, caller: &str, callee: &str) -> &'a CallSite {
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == caller && s.callee_name == callee)
        .collect();
    assert_eq!(sites.len(), 1, "{caller}: {callee}");
    sites[0]
}

fn exact(caller: &str, callee: &str, target: &str, file: &str) {
    let cg = graph();
    let outcome = cg.resolve_call_site_full(site(&cg, caller, callee));
    println!("{caller}: {outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{caller}: {outcome:?}");
    let resolved = &outcome.resolved[0];
    assert_eq!(resolved.target.name, target);
    assert_eq!(resolved.target.file, file);
    let line = match target {
        "assoc" => 3,
        "method" => 4,
        "g_assoc" => 19,
        "w_assoc" => 24,
        "b" => 33,
        "new" => 43,
        "same_target" => 45,
        _ => panic!("target line must be pinned"),
    };
    assert_eq!(resolved.target.start_line, line);
    assert_eq!(resolved.confidence, ResolutionConfidence::Exact);
    assert_eq!(resolved.kind, ResolutionKind::SelfReceiver);
}

#[test]
fn associated() {
    exact("assoc_call", "Self::assoc", "assoc", "src/cases.rs");
}
#[test]
fn method() {
    exact("method_call", "Self::method", "method", "src/cases.rs");
}
#[test]
fn lifetime() {
    exact("lifetime_call", "Self::g_assoc", "g_assoc", "src/cases.rs");
}
#[test]
fn generic() {
    exact("generic_call", "Self::w_assoc", "w_assoc", "src/cases.rs");
}
#[test]
fn trait_to_inherent() {
    exact("b", "Self::assoc", "assoc", "src/cases.rs");
}
#[test]
fn trait_to_trait() {
    exact("a", "Self::b", "b", "src/cases.rs");
}
#[test]
fn closure() {
    exact("closure_call", "Self::assoc", "assoc", "src/cases.rs");
}
#[test]
fn split_impl() {
    exact("split_call", "Self::new", "new", "src/cases.rs");
}
#[test]
fn same_named_types() {
    exact(
        "same_call",
        "Self::same_target",
        "same_target",
        "src/cases.rs",
    );
}

fn dropped(caller: &str, callee: &str) {
    let cg = graph();
    let outcome = cg.resolve_call_site_full(site(&cg, caller, callee));
    println!("{caller}: {outcome:?}");
    assert!(outcome.resolved.is_empty(), "{caller}: {outcome:?}");
}
#[test]
fn trait_default() {
    dropped("default_a", "Self::default_b");

    // Isolate the trait-default guard from the ambiguous-pool refusal.
    let path = "src/lib.rs";
    let parsed = ParsedFile::parse(
        path,
        "trait Single { fn single_default() { Self::single_target(); } fn single_target() {} } struct Only; impl Single for Only {}",
        Language::Rust,
    )
    .unwrap();
    let cg = CallGraph::build(&BTreeMap::from([(path.to_string(), parsed)]));
    assert_eq!(
        cg.methods[&("Single".to_string(), "single_target".to_string())].len(),
        1
    );
    let outcome = cg.resolve_call_site_full(site(&cg, "single_default", "Self::single_target"));
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::UnknownName));
}
#[test]
fn nested_fn() {
    dropped("nested", "Self::assoc");
    dropped("nested_call", "Self::assoc");
}
#[test]
fn missing() {
    dropped("missing_call", "Self::missing");

    // The full matrix's first fallback is a cross-impl trait method, now
    // refused independently. Isolate missing-name lookup from applicability.
    let path = "src/lib.rs";
    let parsed = ParsedFile::parse(
        path,
        "struct T;\nimpl T {\nfn aaa() {}\nfn isolated_missing() { Self::missing(); }\nfn direct() { Self::aaa(); }\n}",
        Language::Rust,
    )
    .unwrap();
    let cg = CallGraph::build(&BTreeMap::from([(path.to_string(), parsed)]));
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    let first = cg.methods_by_scope.values().next().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(
        (
            first[0].file.as_str(),
            first[0].start_line,
            first[0].name.as_str()
        ),
        (path, 3, "aaa")
    );
    let direct = cg.resolve_call_site(site(&cg, "direct", "Self::aaa"));
    assert_eq!(direct.len(), 1);
    assert_eq!(
        (
            direct[0].target.file.as_str(),
            direct[0].target.start_line,
            direct[0].target.name.as_str()
        ),
        (path, 3, "aaa")
    );
    assert_eq!(direct[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(direct[0].kind, ResolutionKind::SelfReceiver);
    let outcome = cg.resolve_call_site_full(site(&cg, "isolated_missing", "Self::missing"));
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::UnknownName));
}
#[test]
fn enum_constructor() {
    dropped("enum_call", "Self::Variant");
}
#[test]
fn constant_is_not_a_call() {
    let cg = graph();
    assert!(!cg
        .calls
        .values()
        .flatten()
        .any(|s| s.caller.name == "const_path"));
}
#[test]
fn controls() {
    let cg = graph();
    for (caller, callee) in [("type_control", "T::assoc"), ("dot_control", "method")] {
        let r = cg.resolve_call_site(site(&cg, caller, callee));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    }
}

#[test]
fn missing_on_same_named_type() {
    dropped("absent_in_this_type", "Self::foreign_only");
}

#[test]
fn multiple_candidates_drop() {
    let cg = graph();
    let outcome = cg.resolve_call_site_full(site(&cg, "configured_call", "Self::choice"));
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::UnknownName));
}

#[test]
fn distinct_from_impls_drop() {
    let cg = graph();
    let pool = &cg.methods[&("T".to_string(), "from".to_string())];
    assert_eq!(pool.len(), 2, "both From impls must be indexed");
    let outcome = cg.resolve_call_site_full(site(&cg, "from_x", "Self::from"));
    // x is statically X: in particular the From<Y> edge must never escape.
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::UnknownName));
}

#[test]
fn absent_owner_evidence_refuses() {
    let mut cg = graph();
    let call = site(&cg, "assoc_call", "Self::assoc").clone();
    cg.method_owners.remove(&call.caller);
    assert!(cg.resolve_call_site(&call).is_empty());
}

#[test]
fn absent_facts_refuses() {
    let mut cg = graph();
    let call = site(&cg, "assoc_call", "Self::assoc").clone();
    cg.method_facts.remove(&call.caller);
    assert!(cg.resolve_call_site(&call).is_empty());
}

#[test]
fn absent_scope_evidence_refuses() {
    let mut cg = graph();
    let call = site(&cg, "assoc_call", "Self::assoc").clone();
    cg.methods_by_scope
        .values_mut()
        .for_each(|ids| ids.retain(|id| id != &call.caller));
    assert!(cg.resolve_call_site(&call).is_empty());
}

#[test]
fn ambiguous_scope_evidence_refuses() {
    let mut cg = graph();
    let call = site(&cg, "same_call", "Self::same_target").clone();
    let other = cg
        .methods_by_scope
        .iter()
        .find(|((_, name), ids)| {
            name == "same_target" && ids.iter().any(|id| id.file == "src/other.rs")
        })
        .unwrap()
        .0
         .0;
    cg.methods_by_scope
        .entry((other, call.caller.name.clone()))
        .or_default()
        .push(call.caller.clone());
    assert!(cg.resolve_call_site(&call).is_empty());
}

#[test]
fn orphan_preserves_legacy() {
    let parsed = ParsedFile::parse(
        "src/orphan.rs",
        "struct T; impl T { fn helper() {} fn caller() { Self::helper(); } }",
        Language::Rust,
    )
    .unwrap();
    let root = ParsedFile::parse("src/lib.rs", "pub fn root() {}", Language::Rust).unwrap();
    let cg = CallGraph::build(&BTreeMap::from([
        ("src/lib.rs".to_string(), root),
        ("src/orphan.rs".to_string(), parsed),
    ]));
    let call = site(&cg, "caller", "Self::helper");
    let graph = cg.scope_graph.as_ref().unwrap();
    let file = graph.file_paths["src/orphan.rs"];
    assert!(
        prism::name_resolution::rust_populator::enclosing_scope(graph, file, call.start_byte)
            .is_none()
    );
    let r = cg.resolve_call_site(call);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::SelfReceiver);
}
