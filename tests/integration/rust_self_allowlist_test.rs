use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, CallSite},
    languages::Language,
    resolution::{DropReason, ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;
fn build(files: &[(&str, &str)]) -> CallGraph {
    let files = files
        .iter()
        .map(|(p, s)| {
            (
                (*p).to_string(),
                ParsedFile::parse(p, s, Language::Rust).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let cg = CallGraph::build(&files);
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}
fn call<'a>(cg: &'a CallGraph, caller: &str, line: usize, callee: &str) -> &'a CallSite {
    let sites = cg
        .calls
        .values()
        .flatten()
        .filter(|s| {
            s.caller.name == caller && s.caller.start_line == line && s.callee_name == callee
        })
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 1, "{caller}:{line} {callee}");
    sites[0]
}
fn drop_call(cg: &CallGraph, caller: &str, line: usize, callee: &str) {
    let s = call(cg, caller, line, callee);
    let o = cg.resolve_call_site_full(s);
    assert_eq!(
        o.resolved
            .iter()
            .map(|r| (&r.target.file, r.target.start_line, &r.target.name))
            .collect::<Vec<_>>(),
        vec![],
        "{o:?}"
    );
    assert_eq!(o.drop, Some(DropReason::UnknownName));
}
fn exact_call(
    cg: &CallGraph,
    caller: &str,
    line: usize,
    callee: &str,
    file: &str,
    target_line: usize,
    name: &str,
) {
    let o = cg.resolve_call_site_full(call(cg, caller, line, callee));
    assert_eq!(o.resolved.len(), 1, "{o:?}");
    let r = &o.resolved[0];
    assert_eq!(
        (
            r.target.file.as_str(),
            r.target.start_line,
            r.target.name.as_str()
        ),
        (file, target_line, name)
    );
    assert_eq!(
        (r.confidence, r.kind),
        (ResolutionConfidence::Exact, ResolutionKind::SelfReceiver)
    );
}
macro_rules! fixture {
    ($n:literal) => {
        build(&[(
            "src/lib.rs",
            include_str!(concat!(
                "../fixtures/rust-self-allowlist/",
                $n,
                "/src/lib.rs"
            )),
        )])
    };
}
#[test]
fn opus_p1() {
    let cg = build(&[
        (
            "src/lib.rs",
            include_str!("../fixtures/rust-self-allowlist/p1_default_hidden/src/lib.rs"),
        ),
        (
            "src/a.rs",
            include_str!("../fixtures/rust-self-allowlist/p1_default_hidden/src/a.rs"),
        ),
    ]);
    drop_call(&cg, "a1", 8, "Self::b");
}
#[test]
fn opus_p2() {
    let cg = fixture!("p2_generic_args");
    drop_call(&cg, "g", 12, "Self::f");
}
#[test]
fn opus_p3() {
    let cg = fixture!("p3_generic_trait");
    drop_call(&cg, "use_b", 12, "Self::conv");
}
#[test]
fn opus_p5() {
    let cg = fixture!("p5_local_impl");
    drop_call(&cg, "y", 8, "Self::x");
}
#[test]
fn opus_p6b() {
    let cg = fixture!("p6b_blanket_default");
    drop_call(&cg, "a", 6, "Self::b");
}
#[test]
fn sol_f1_default() {
    let cg = fixture!("generic-default");
    drop_call(&cg, "caller", 6, "Self::b");
}
#[test]
fn sol_f1_inherent() {
    let cg = fixture!("generic-inherent");
    drop_call(&cg, "caller", 4, "Self::b");
}
#[test]
fn sol_f2_visible() {
    let cg = fixture!("default-visible");
    drop_call(&cg, "caller", 6, "Self::b");
}
#[test]
fn sol_f2_ambiguous() {
    let cg = fixture!("default-ambiguity");
    drop_call(&cg, "caller", 6, "Self::b");
}
#[test]
fn sol_f3() {
    let cg = fixture!("local-type");
    drop_call(&cg, "caller", 7, "Self::b");
}
#[test]
fn sol_f4() {
    let cg = fixture!("const-context");
    drop_call(&cg, "caller", 4, "Self::b");
}
#[test]
fn inline_const() {
    let cg = fixture!("const-context");
    exact_call(&cg, "inline", 5, "Self::b", "src/lib.rs", 3, "b");
}
#[test]
fn method_where() {
    let cg = fixture!("where-context");
    exact_call(&cg, "caller", 4, "Self::count", "src/lib.rs", 3, "count");
    exact_call(&cg, "inline", 5, "Self::count", "src/lib.rs", 3, "count");
}
#[test]
fn inherent_trait_collision() {
    let cg = fixture!("inherent-trait");
    drop_call(&cg, "caller", 4, "Self::b");
}
#[test]
fn crate_local_names() {
    let cg = build(&[
        (
            "a/src/lib.rs",
            include_str!("../fixtures/rust-self-allowlist/workspace/a/src/lib.rs"),
        ),
        (
            "b/src/lib.rs",
            include_str!("../fixtures/rust-self-allowlist/workspace/b/src/lib.rs"),
        ),
    ]);
    exact_call(&cg, "caller_a", 2, "Self::b", "a/src/lib.rs", 2, "b");
    exact_call(&cg, "caller_b", 2, "Self::b", "b/src/lib.rs", 2, "b");
}

#[test]
fn opus_p1_inherent() {
    let cg = build(&[
        (
            "src/lib.rs",
            include_str!("../fixtures/rust-self-allowlist/p1_default_hidden/src/lib.rs"),
        ),
        (
            "src/a.rs",
            include_str!("../fixtures/rust-self-allowlist/p1_default_hidden/src/a.rs"),
        ),
    ]);
    drop_call(&cg, "inherent_caller", 11, "Self::b");
}
#[test]
fn opus_p5_shadow() {
    let cg = fixture!("p5_local_impl");
    drop_call(&cg, "y", 21, "Self::x");
}
#[test]
fn module_impl_required() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; impl T { fn b() {} }
fn host() {
    impl T { fn caller() { Self::b(); } }
}",
    )]);
    drop_call(&cg, "caller", 3, "Self::b");
}
#[test]
fn module_target_required() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; impl T { fn caller() { Self::b(); } }
fn host() { impl T { fn b() {} } }",
    )]);
    drop_call(&cg, "caller", 1, "Self::b");
}
#[test]
fn cross_trait_refuses() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; trait Tr { fn b(); } impl Tr for T { fn b() {} }
impl T { fn caller() { Self::b(); } }",
    )]);
    drop_call(&cg, "caller", 2, "Self::b");
}
#[test]
fn generic_headers_refuse() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T<X>(X); impl<X> T<X> { fn b() {} }
impl<X: Clone> T<X> { fn caller() { Self::b(); } }",
    )]);
    drop_call(&cg, "caller", 2, "Self::b");
}
#[test]
fn where_headers_refuse() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T<X>(X); impl<X> T<X> where X: Clone { fn b() {} }
impl<X> T<X> where X: Copy { fn caller() { Self::b(); } }",
    )]);
    drop_call(&cg, "caller", 2, "Self::b");
}
#[test]
fn identical_headers_allow() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T<X>(X); impl<X> T<X> where X: Clone { fn b() {} }
impl < X > T < X > where X : Clone { fn caller() { Self::b(); } }",
    )]);
    exact_call(&cg, "caller", 2, "Self::b", "src/lib.rs", 1, "b");
}
#[test]
fn closure_async_allow() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; impl T { fn b() {}
fn closure() { let _ = || Self::b(); }
fn asynchronous() { let _ = async { Self::b(); }; } }",
    )]);
    exact_call(&cg, "closure", 2, "Self::b", "src/lib.rs", 1, "b");
    exact_call(&cg, "asynchronous", 3, "Self::b", "src/lib.rs", 1, "b");
}
#[test]
fn named_static_refuses() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; impl T { const fn b() -> u8 { 1 }
fn caller() { static C: u8 = Self::b(); } }",
    )]);
    drop_call(&cg, "caller", 2, "Self::b");
}
#[test]
fn nested_module_allow() {
    let cg = build(&[(
        "src/lib.rs",
        "mod inner { struct T; impl T { fn b() {}
fn caller() { Self::b(); } } }",
    )]);
    exact_call(&cg, "caller", 2, "Self::b", "src/lib.rs", 1, "b");
}

#[test]
fn default_name_global_refuses() {
    let cg = build(&[(
        "src/lib.rs",
        "trait Tr { fn b() {} } struct T;
impl T { fn b() {} fn caller() { Self::b(); } }",
    )]);
    drop_call(&cg, "caller", 2, "Self::b");
}

#[test]
fn same_impl_trait_allow() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; trait Tr { fn b(); fn caller(); }
impl Tr for T { fn b() {} fn caller() { Self::b(); } }",
    )]);
    exact_call(&cg, "caller", 2, "Self::b", "src/lib.rs", 2, "b");
}

#[test]
fn macro_tokens_refuse() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T; impl T { fn b() -> u8 { 1 }
fn caller() { let _ = vec![Self::b()]; } }",
    )]);
    assert_eq!(
        format!("{:?}", call(&cg, "caller", 2, "Self::b").origin),
        "MacroArg"
    );
    drop_call(&cg, "caller", 2, "Self::b");
}
#[test]
fn duplicate_impl_identity_refuses() {
    let cg = build(&[(
        "src/lib.rs",
        "struct T;
impl T { fn b() {} } impl T { fn b() {} }
impl T { fn caller() { Self::b(); } }",
    )]);
    let pool = &cg.methods[&("T".to_string(), "b".to_string())];
    assert_eq!(
        pool.iter()
            .map(|id| (&id.file, id.start_line, &id.name))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        1
    );
    drop_call(&cg, "caller", 3, "Self::b");
}

#[test]
fn token_boundary_refuses() {
    let cg = fixture!("token-boundary");
    drop_call(&cg, "caller", 5, "Self::f");
}
#[test]
fn literal_whitespace_refuses() {
    let cg = fixture!("literal-whitespace");
    drop_call(&cg, "caller", 4, "Self::f");
}
#[test]
fn generic_module_blocks_refuse() {
    let cg = fixture!("generic-module-blocks");
    drop_call(&cg, "caller", 8, "Self::f");
}
#[test]
fn generic_module_files_refuse() {
    let cg = build(&[
        (
            "src/lib.rs",
            include_str!("../fixtures/rust-self-allowlist/generic-lexical-owner/src/lib.rs"),
        ),
        (
            "src/a.rs",
            include_str!("../fixtures/rust-self-allowlist/generic-lexical-owner/src/a.rs"),
        ),
        (
            "src/b.rs",
            include_str!("../fixtures/rust-self-allowlist/generic-lexical-owner/src/b.rs"),
        ),
    ]);
    drop_call(&cg, "caller", 3, "Self::f");
}
