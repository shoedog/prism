use prism::call_graph::{CallGraph, CallSite};
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn build(
    sources: &[(&str, &str, prism::languages::Language)],
) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    let mut files = BTreeMap::new();
    for (path, src, lang) in sources {
        files.insert(
            path.to_string(),
            prism::ast::ParsedFile::parse(path, src, *lang).unwrap(),
        );
    }
    (CallGraph::build(&files), files)
}

fn site_in(cg: &CallGraph, caller_name: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller_name)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("no site {caller_name}->{callee}"))
        .clone()
}

#[test]
fn r1_type_qualified_call_resolves_to_owner_method_exact() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "engine.rs",
            "pub struct Engine;\nimpl Engine {\n    pub fn start() {}\n}\n",
            Rust,
        ),
        ("main.rs", "fn main() {\n    Engine::start();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "main", "Engine::start");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "engine.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn r1_trait_qualified_multi_impl_demotes_to_name_only() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "a.rs",
            "impl Render for A {\n    fn draw(&self) {}\n}\n",
            Rust,
        ),
        (
            "b.rs",
            "impl Render for B {\n    fn draw(&self) {}\n}\n",
            Rust,
        ),
        (
            "main.rs",
            "fn go(x: &dyn Render) {\n    Render::draw(x);\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "go", "Render::draw");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::TraitCha));
}

#[test]
fn r2_self_method_call_resolves_via_enclosing_owner_cross_file() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "a.rs",
            "impl Foo {\n    fn entry(&self) {\n        self.helper();\n    }\n}\n",
            Rust,
        ),
        (
            "b.rs",
            "impl Foo {\n    fn helper(&self) {}\n}\nimpl Bar {\n    fn helper(&self) {}\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "entry", "helper");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "must hit Foo::helper only, not Bar::helper");
    assert_eq!(r[0].kind, ResolutionKind::SelfReceiver);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn r2_go_receiver_var_call() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "a.go",
        "package p\ntype T struct{}\nfunc (t *T) A() {\n    t.B()\n}\nfunc (t *T) B() {}\ntype U struct{}\nfunc (u *U) B() {}\n",
        Go,
    )]);
    let site = site_in(&cg, "A", "B");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::SelfReceiver);
}

#[test]
fn r7_stem_fallback_excludes_cross_file_statics() {
    use prism::languages::Language::Cpp;
    // `static` free fn in eng.cpp (internal linkage): `eng::start()` from
    // another file must NOT resolve to it; a non-static sibling stem-match would.
    let (cg, _) = build(&[
        ("eng.cpp", "static void start() {}\n", Cpp),
        ("main.cpp", "void run() {\n    eng::start();\n}\n", Cpp),
    ]);
    let site = site_in(&cg, "run", "eng::start");
    assert!(
        cg.resolve_call_site(&site).is_empty(),
        "cross-file static must not resolve via stem fallback"
    );
}
