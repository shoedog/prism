use prism::call_graph::{CallGraph, CallSite};
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
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

#[test]
fn r3_import_qualified_with_no_repo_candidate_is_unresolved() {
    use prism::languages::Language::Go;
    // `zap` is imported but resolves to no in-repo file => provably external => NO edge
    // (the ~148-FP caddy class, spec 2.2 R3).
    let (cg, _) = build(&[
        (
            "notify/notify.go",
            "package notify\nfunc Error(err error) error { return err }\n",
            Go,
        ),
        (
            "main.go",
            "package main\nimport \"go.uber.org/zap\"\nfunc main() {\n    zap.Error(nil)\n}\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "main", "Error");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::ImportExternal)
    );
}

#[test]
fn r3_import_qualified_absent_name_is_unknown_name() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\nimport \"go.uber.org/zap\"\nfunc main() {\n    zap.Warn(nil)\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "main", "Warn");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::UnknownName)
    );
}

#[test]
fn r3_go_import_matches_package_directory_not_file_stem() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "modules/caddyhttp/errors.go",
            "package caddyhttp\nfunc Error(c int) error { return nil }\n",
            Go,
        ),
        (
            "main.go",
            "package main\nimport \"github.com/x/y/modules/caddyhttp\"\nfunc main() {\n    caddyhttp.Error(1)\n}\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "main", "Error");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "modules/caddyhttp/errors.go");
    assert_eq!(r[0].kind, ResolutionKind::ImportQualified);
}

#[test]
fn r3b_qualifier_as_owner_resolves_statics() {
    use prism::languages::Language::Python;
    let (cg, _) = build(&[
        (
            "c.py",
            "class Config:\n    def load(self):\n        pass\n",
            Python,
        ),
        ("m.py", "def main():\n    Config.load(cfg)\n", Python),
    ]);
    let site = site_in(&cg, "main", "load");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::QualifierOwner);
}

#[test]
fn r4_local_free_definition_wins_alone() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "fn slice() {}\nfn run() {\n    slice();\n}\n", Rust),
        ("b.rs", "fn slice() {}\n", Rust),
        ("c.rs", "fn slice() {}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "slice");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "local-def preference: a.rs only");
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].kind, ResolutionKind::LocalDef);
}

#[test]
fn r4b_java_sibling_call_survives() {
    use prism::languages::Language::Java;
    // Java has NO free functions - unqualified f() is implicit-this (spec B1 fix).
    let (cg, _) = build(&[(
        "App.java",
        "class App {\n    void run() {\n        helper();\n    }\n    void helper() {}\n}\n",
        Java,
    )]);
    let site = site_in(&cg, "run", "helper");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::ImplicitThis);
}

#[test]
fn r5_unqualified_never_binds_to_methods() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "fn run() {\n    process();\n}\n", Rust),
        ("b.rs", "impl Worker {\n    fn process(&self) {}\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "process");
    assert!(
        cg.resolve_call_site(&site).is_empty(),
        "method requires a receiver"
    );
}

#[test]
fn r5_cross_file_free_multi_kept_demoted() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "fn run() {\n    helper();\n}\n", Rust),
        ("b.rs", "fn helper() {}\n", Rust),
        ("c.rs", "fn helper() {}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "helper");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::FreeMulti));
}
