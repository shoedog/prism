use prism::call_graph::{CallGraph, CallSite};
use prism::cpg::CpgContext;
use prism::resolution::{admission_key, iface_key};
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

fn build_cfg(
    sources: &[(&str, &str, prism::languages::Language)],
    cfg: &prism::resolution::ReceiverRecoveryConfig,
) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    let mut files = BTreeMap::new();
    for (path, src, lang) in sources {
        files.insert(
            path.to_string(),
            prism::ast::ParsedFile::parse(path, src, *lang).unwrap(),
        );
    }
    (CallGraph::build_with_receiver_config(&files, cfg), files)
}

// Slice A parity gate: `legacy` reproduces PR-1's P6-lite recovery byte-for-byte,
// and the default `expanded` mode is identical to it (no new forms yet).
#[test]
fn slice_a_legacy_parity_p6_typed_param() {
    use prism::languages::Language::Go;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig};
    let legacy = build_cfg(
        &[("main.go", go_iface_src(), Go)],
        &ReceiverRecoveryConfig::legacy(),
    );
    let expanded = build_cfg(
        &[("main.go", go_iface_src(), Go)],
        &ReceiverRecoveryConfig::default(),
    );
    for (cg, _) in [&legacy, &expanded] {
        let site = site_in(cg, "run", "Go");
        assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::TypedParam));
        let r = cg.resolve_call_site(&site);
        assert_eq!(r.len(), 2);
        assert!(r
            .iter()
            .all(|c| c.kind == ResolutionKind::InterfaceDispatch));
    }
}

// Slice D: the interface-dispatch in-scope manifest (structural; spec §8a).
#[test]
fn callgraph_interface_method_names_populated() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[("main.go", go_iface_src(), Go)]);
    // "Go" is declared on interface Runner → in the interface-method-name set.
    assert!(cg.interface_method_names.contains("Go"));
}

#[test]
fn interface_manifest_includes_inscope_excludes_noninterface_method() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\n\
         func (f Fast) Go() {}\n\
         func (f Fast) Stop() {}\n\
         func use() { _ = Fast{} }\n\
         func run(r Runner) { r.Go(); r.Stop() }\n",
        Go,
    )]);
    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = manifest["sites"].as_array().expect("sites array");
    // r.Go(): typed_param receiver, method "Go" ∈ interface Runner → in scope.
    assert!(
        sites
            .iter()
            .any(|s| s["method"] == "Go" && s["receiver_class"] == "typed_param"),
        "r.Go() must be an in-scope manifest site"
    );
    // r.Stop(): "Stop" is on no interface → excluded by the denominator predicate.
    assert!(
        sites.iter().all(|s| s["method"] != "Stop"),
        "r.Stop() (non-interface method) must be excluded"
    );
    // byte-span identity present on every record.
    assert!(sites.iter().all(|s| {
        s["start_byte"].is_number() && s["end_byte"].is_number() && s["file"].is_string()
    }));
}

// Re-review MAJOR 4: the manifest must only count Go-CALLER sites (real interface dispatch
// is Go-gated in resolution.rs). A non-Go caller that syntactically recovers a same-named
// receiver type must NOT enter the manifest, even when the called method is on a Go interface.
#[test]
fn interface_manifest_excludes_non_go_caller() {
    use prism::languages::Language::{Go, Rust};
    let (cg, _) = build(&[
        (
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             type Fast struct{}\nfunc (f Fast) Go() {}\nfunc use() { _ = Fast{} }\n\
             func run(r Runner) { r.Go() }\n",
            Go,
        ),
        // Rust caller whose typed param `r: Runner` recovers a receiver and calls `Go`
        // (a method name shared with the Go interface). It must be excluded by the Go gate.
        (
            "lib.rs",
            "struct Runner;\nfn run(r: Runner) {\n    r.Go();\n}\n",
            Rust,
        ),
    ]);
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = m["sites"].as_array().expect("sites array");
    // The Go caller site for `Go` is present.
    assert!(
        sites
            .iter()
            .any(|s| s["method"] == "Go" && s["file"] == "main.go"),
        "Go caller site for Go must be in the manifest"
    );
    // The Rust caller site must NOT be present (non-Go caller gate, MAJOR 4).
    assert!(
        sites.iter().all(|s| s["file"] != "lib.rs"),
        "non-Go (Rust) caller site must be excluded from the manifest"
    );
}

// Re-review MINOR 8: pin the manifest `fanout` VALUE. A dispatch receiver (interface type
// with 2 live in-repo implementers) carries `fanout == 2`; a concrete-receiver site (the
// receiver type is a struct, not an interface key) carries `fanout == 0`.
#[test]
fn interface_manifest_fanout_value() {
    use prism::languages::Language::Go;
    // Two implementers Fast + Slow of Runner, both constructed (live). `var r Runner`
    // is an interface receiver -> iface_key("Runner") -> (Runner, Go) -> 2 impls.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         func use() { _ = Fast{}; _ = Slow{} }\n\
         func dispatch() { var r Runner; r.Go() }\n",
        Go,
    )]);
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = m["sites"].as_array().expect("sites array");
    let dispatch_site = sites
        .iter()
        .find(|s| s["method"] == "Go")
        .expect("dispatch site present");
    assert_eq!(
        dispatch_site["fanout"].as_u64(),
        Some(2),
        "Runner has 2 live in-repo implementers (Fast + Slow)"
    );

    // Concrete receiver: `var r Fast` is a concrete struct type, so iface_key("Fast")
    // misses (Fast, Go) in interface_impls -> fanout 0, even though `Go` is an interface
    // method name (so the site is still in scope).
    let (cg2, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func concrete() { var r Fast; r.Go() }\n",
        Go,
    )]);
    let m2 = prism::navigation::queries::interface_dispatch_manifest(&cg2);
    let sites2 = m2["sites"].as_array().expect("sites array");
    let concrete_site = sites2
        .iter()
        .find(|s| s["method"] == "Go")
        .expect("concrete site present (Go is an interface method name)");
    assert_eq!(
        concrete_site["fanout"].as_u64(),
        Some(0),
        "concrete struct receiver Fast is not an interface key -> fanout 0"
    );
}

// Slice E: a dispatch site emits the minted implementer SET (owner type names), sorted +
// deduped, alongside `fanout` (= its cardinality). The 2-implementer Runner fixture (Fast +
// Slow both live) must yield implementers == ["Fast", "Slow"] and fanout == 2. A concrete
// (fanout == 0) receiver yields implementers == [].
#[test]
fn interface_manifest_implementers_set() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         func use() { _ = Fast{}; _ = Slow{} }\n\
         func dispatch() { var r Runner; r.Go() }\n\
         func concrete() { var c Fast; c.Go() }\n",
        Go,
    )]);
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = m["sites"].as_array().expect("sites array");

    // `var r Runner; r.Go()` — interface receiver -> 2 live implementers, sorted + deduped.
    let dispatch_site = sites
        .iter()
        .find(|s| s["method"] == "Go" && s["fanout"].as_u64() == Some(2))
        .expect("2-implementer dispatch site present");
    let implementers: Vec<&str> = dispatch_site["implementers"]
        .as_array()
        .expect("implementers array")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        implementers,
        vec!["Fast", "Slow"],
        "implementers must be the sorted owner type names of the live impls"
    );
    assert_eq!(
        dispatch_site["fanout"].as_u64(),
        Some(implementers.len() as u64),
        "fanout must equal implementers cardinality"
    );

    // `var c Fast; c.Go()` — concrete struct receiver -> empty implementer set, fanout 0.
    let concrete_site = sites
        .iter()
        .find(|s| s["method"] == "Go" && s["fanout"].as_u64() == Some(0))
        .expect("concrete site present (Go is an interface method name)");
    assert_eq!(
        concrete_site["implementers"]
            .as_array()
            .expect("implementers array")
            .len(),
        0,
        "a concrete (fanout 0) receiver mints no implementers"
    );
}

// Slice F (sketch only): the reserved variant exists; the classifier returns None for it.
#[test]
#[ignore = "SliceElem is reserved (spec §5/§10); classifier returns None until a future slice"]
fn slice_elem_variant_reserved() {
    // Compiles iff the variant exists; no recovery behavior is wired yet.
    let _ = prism::resolution::ReceiverRecovery::SliceElem;
}

// Whole-branch review #2: untyped multi-name `var` must NOT mis-recover the first
// initializer's type for every bound name (a wrong resolution edge).
#[test]
fn var_local_untyped_multiname_does_not_mis_recover() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func run() { var slow, fast = Slow{}, Fast{}; _ = slow; fast.Go() }\n",
        Go,
    )]);
    // `fast` is Fast, not Slow — untyped multi-name is ambiguous, so recovery must bail.
    assert_eq!(site_in(&cg, "run", "Go").receiver_type, None);
}

#[test]
fn var_local_typed_multiname_recovers_shared_type() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    // `var a, b Runner` — the declared type is shared across names, so it is safe.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\nfunc use() { _ = Fast{} }\n\
         func run() { var a, b Runner; _ = a; b.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::VarDecl));
}

// Whole-branch review MAJOR 5 + MINOR 6: pin the receiver_class wire strings + the
// interface_dispatch_computed signal. The reserved "slice_elem" and the deferred
// "slice_candidate" never appear on real sites.
#[test]
fn interface_manifest_receiver_class_strings() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\nfunc use() { _ = Fast{} }\n\
         func tp(r Runner) { r.Go() }\n\
         func ta(x any) { x.(Runner).Go() }\n\
         func vd() { var r Runner; r.Go() }\n",
        Go,
    )]);
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let classes: std::collections::BTreeSet<&str> = m["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["receiver_class"].as_str().unwrap())
        .collect();
    assert!(classes.contains("typed_param"));
    assert!(classes.contains("type_assertion"));
    assert!(classes.contains("var_local"));
    assert!(!classes.contains("slice_elem"));
    assert!(!classes.contains("slice_candidate"));
    assert!(m["interface_dispatch_computed"].as_bool().unwrap());
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
fn interface_dispatch_kind_as_str() {
    assert_eq!(
        prism::resolution::ResolutionKind::InterfaceDispatch.as_str(),
        "interface_dispatch"
    );
}

#[test]
fn iface_key_strips_pkg_and_pointer() {
    assert_eq!(iface_key("Runner").as_deref(), Some("Runner"));
    assert_eq!(iface_key("io.Reader").as_deref(), Some("Reader"));
    assert_eq!(iface_key("*Runner").as_deref(), Some("Runner"));
}

#[test]
fn iface_key_gaps_on_generic_instantiation() {
    assert_eq!(iface_key("Container[T]"), None);
    assert_eq!(iface_key("pkg.Map[string,int]"), None);
}

#[test]
fn admission_key_distinguishes_pointer() {
    assert_eq!(admission_key("Fast", false), "Fast");
    assert_eq!(admission_key("Fast", true), "*Fast");
}

#[test]
fn callgraph_exposes_interface_impls() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    )]);
    // Fast is constructed -> live -> interface_impls has (Runner, Go) -> [Fast.Go].
    let ids = cg
        .interface_impls
        .get(&("Runner".to_string(), "Go".to_string()))
        .expect("interface_impls populated");
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].name, "Go");
}

#[test]
fn removing_implementer_drops_interface_edge_no_phantom() {
    use prism::languages::Language::Go;

    let key = ("Runner".to_string(), "Go".to_string());
    let mut files = BTreeMap::new();
    files.insert(
        "iface.go".to_string(),
        prism::ast::ParsedFile::parse(
            "iface.go",
            "package main\n\
             type Runner interface { Go() }\n\
             func run(r Runner) { r.Go() }\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "fast.go".to_string(),
        prism::ast::ParsedFile::parse(
            "fast.go",
            "package main\n\
             type Fast struct{}\n\
             func (f Fast) Go() {}\n\
             func use() { _ = Fast{} }\n",
            Go,
        )
        .unwrap(),
    );

    let cg = CallGraph::build(&files);
    assert!(
        cg.interface_impls.contains_key(&key),
        "constructed Fast should populate Runner.Go"
    );

    files.remove("fast.go");
    let cg = CallGraph::build(&files);
    assert!(
        !cg.interface_impls.contains_key(&key),
        "removed implementer must not leave a phantom Runner.Go edge"
    );
    let _ctx = CpgContext::build(&files, None);
}

#[test]
fn non_go_repo_has_empty_interface_impls() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[(
        "a.rs",
        "pub struct A;\nimpl A { pub fn go(&self){} }\n",
        Rust,
    )]);
    assert!(cg.interface_impls.is_empty());
}

fn go_iface_src() -> &'static str {
    "package main\n\
     type Runner interface { Go() }\n\
     type Fast struct{}\nfunc (f Fast) Go() {}\n\
     type Slow struct{}\nfunc (s Slow) Go() {}\n\
     func use() { _ = Fast{}; _ = Slow{} }\n\
     func run(r Runner) { r.Go() }\n"
}

#[test]
fn interface_dispatch_resolves_multi_implementer_exact() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[("main.go", go_iface_src(), Go)]);
    let site = site_in(&cg, "run", "Go");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2, "Fast + Slow (both live)");
    assert!(r
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::Exact));
    assert!(r
        .iter()
        .all(|c| c.kind == ResolutionKind::InterfaceDispatch));
}

#[test]
fn interface_fallback_no_construction_full_set_exact() {
    use prism::languages::Language::Go;
    // constructs nothing -> empty live -> fallback -> full satisfier set, Exact.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::Exact));
}

#[test]
fn interface_rta_prunes_uninstantiated() {
    use prism::languages::Language::Go;
    // only Fast constructed -> Slow pruned.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Go");
}

#[test]
fn interface_dispatch_does_not_cross_language() {
    use prism::languages::Language::{Go, Rust};
    let (cg, _) = build(&[
        ("main.go", go_iface_src(), Go),
        (
            "lib.rs",
            "struct Runner;\nfn run_rust(x: Runner) {\n    x.Go();\n}\n",
            Rust,
        ),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "run_rust", "Go"));
    assert!(out.resolved.is_empty());
    assert_eq!(out.drop, Some(DropReason::ExternalReceiver));
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

#[test]
fn r6_multi_owner_unknown_receiver_drops() {
    use prism::languages::Language::Rust;
    // The tokio `poll` class: x.poll() with poll on 2+ owner types => unresolved.
    // Receiver `x` is bound from a plain call whose return type prism cannot
    // infer — genuinely unrecoverable by P6-lite, so this exercises the R6
    // residue policy (a *typed* param would be recovered and dropped as
    // ExternalReceiver instead — see r6_typed_param_to_undefined_type_is_external).
    let (cg, _) = build(&[
        ("a.rs", "impl A {\n    fn poll(&self) {}\n}\n", Rust),
        ("b.rs", "impl B {\n    fn poll(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn drive() {\n    let x = mystery();\n    x.poll();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "drive", "poll");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::MultiOwnerCollision)
    );
}

#[test]
fn r6_single_owner_unknown_receiver_kept_demoted() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "a.rs",
            "impl OnlyOwner {\n    fn frobnicate(&self) {}\n}\n",
            Rust,
        ),
        (
            "m.rs",
            "fn run() {\n    let x = mystery();\n    x.frobnicate();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "frobnicate");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(r[0].kind, ResolutionKind::R6SingleOwner);
}

#[test]
fn r6_caller_file_single_owner_preferred_over_repo_multi() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "m.rs",
            "impl Local {\n    fn step(&self) {}\n}\nfn run() {\n    let x = mystery();\n    x.step();\n}\n",
            Rust,
        ),
        ("far.rs", "impl Far {\n    fn step(&self) {}\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "step");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "m.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
}

#[test]
fn r6_typed_param_to_undefined_type_is_external() {
    use prism::languages::Language::Rust;
    // A *typed* param whose type is not in the owner index is provably
    // external (the Vec::truncate class generalized): P6-lite recovers the
    // type, finds no owner, and drops as ExternalReceiver — NOT as a
    // multi-owner collision, even though `poll` has multiple owners.
    let (cg, _) = build(&[
        ("a.rs", "impl A {\n    fn poll(&self) {}\n}\n", Rust),
        ("b.rs", "impl B {\n    fn poll(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn drive(x: UnknownToIndex) {\n    x.poll();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "drive", "poll");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::ExternalReceiver)
    );
}

#[test]
fn p6_rebinding_after_call_does_not_cancel_recovery() {
    use prism::languages::Language::Rust;
    // M6 (plan review): only bindings AT OR BEFORE the call line count toward
    // the shadow bail. A rebinding that occurs *after* the call must not
    // retroactively cancel a valid typed-param recovery.
    let (cg, _) = build(&[
        ("a.rs", "impl Sender {\n    fn send(&self) {}\n}\n", Rust),
        ("b.rs", "impl Pipe {\n    fn send(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn run(x: &Sender) {\n    x.send();\n    let x = other();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "send");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "after-call rebind must not cancel recovery");
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn r6_never_binds_receiver_call_to_free_function() {
    use prism::languages::Language::Go;
    // The caddy `t.Error` class: receiver call must not hit free `Error`.
    let (cg, _) = build(&[
        (
            "notify/n.go",
            "package notify\nfunc Error(e error) error { return e }\n",
            Go,
        ),
        (
            "x.go",
            "package x\nfunc run(t Untyped) {\n    t.Error(nil)\n}\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "run", "Error");
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn r6_never_binds_receiver_call_to_local_static_function() {
    use prism::languages::Language::Cpp;
    let (cg, _) = build(&[(
        "m.cpp",
        "static void poll() {}\nvoid drive(Unknown x) {\n    x.poll();\n}\n",
        Cpp,
    )]);
    let site = site_in(&cg, "drive", "poll");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::UnknownName)
    );
}

#[test]
fn p6_typed_param_recovers_exact_among_collisions() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Sender {\n    fn send(&self) {}\n}\n", Rust),
        ("b.rs", "impl Pipe {\n    fn send(&self) {}\n}\n", Rust),
        ("m.rs", "fn run(tx: &Sender) {\n    tx.send();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "send");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "typed param defeats the multi-owner drop");
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn p6_peel_list_handles_pin_mut_self_shape() {
    assert_eq!(prism::resolution::peel_type("Pin<&mut Self>"), "Self");
    assert_eq!(prism::resolution::peel_type("Arc<Mutex<Conn>>"), "Mutex"); // outermost wrapper peeled, Mutex<Conn> → Mutex
    assert_eq!(prism::resolution::peel_type("&mut Foo"), "Foo");
    assert_eq!(prism::resolution::peel_type("Box<dyn Render>"), "Render");
    assert_eq!(prism::resolution::peel_type("*T"), "T");
}

#[test]
fn p6_constructor_local_recovers() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "a.rs",
            "impl Engine {\n    pub fn new() -> Engine { Engine }\n    pub fn start(&self) {}\n}\n",
            Rust,
        ),
        ("b.rs", "impl Other {\n    fn start(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn main() {\n    let e = Engine::new();\n    e.start();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "main", "start");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].kind, ResolutionKind::ConstructorLocal);
}

#[test]
fn p6_shadowing_bails_to_residue() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl A {\n    fn go(&self) {}\n}\n", Rust),
        ("b.rs", "impl B {\n    fn go(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn main() {\n    let x = A::new();\n    let x = mystery();\n    x.go();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "main", "go");
    assert!(
        cg.resolve_call_site(&site).is_empty(),
        "rebinding ⇒ bail ⇒ multi-owner drop"
    );
}

#[test]
fn p6_external_recovered_type_drops_stdlib_binding() {
    use prism::languages::Language::Rust;
    // The Vec::truncate→AccessPath::truncate class: receiver provably Vec ⇒ drop,
    // even though `truncate` has exactly one in-repo owner.
    let (cg, _) = build(&[
        (
            "ap.rs",
            "impl AccessPath {\n    fn truncate(&mut self) {}\n}\n",
            Rust,
        ),
        (
            "m.rs",
            "fn run(items: &mut Vec<String>) {\n    items.truncate(5);\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "truncate");
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn p6_rust_associated_non_constructor_does_not_recover() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "a.rs",
            "impl A {\n    fn make_b() -> B { B }\n    fn go(&self) {}\n}\n",
            Rust,
        ),
        ("b.rs", "impl B {\n    fn go(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn run() {\n    let x = A::make_b();\n    x.go();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "go");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::MultiOwnerCollision)
    );
}

#[test]
fn p6_rust_qualified_typed_param_normalizes_to_owner_key() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Sender {\n    fn send(&self) {}\n}\n", Rust),
        ("b.rs", "impl Pipe {\n    fn send(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn run(tx: &crate::Sender) {\n    tx.send();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "send");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn p6_compound_rust_param_pattern_bails_to_residue() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Sender {\n    fn send(&self) {}\n}\n", Rust),
        ("b.rs", "impl Pipe {\n    fn send(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn run((tx, _): (&Sender, &Pipe)) {\n    tx.send();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "send");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::MultiOwnerCollision)
    );
}

#[test]
fn p6_go_typed_param_recovers_exact_among_collisions() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package p\n\ntype Sender struct{}\nfunc (s *Sender) Send() {}\ntype Pipe struct{}\nfunc (p *Pipe) Send() {}\nfunc run(tx *Sender) {\n    tx.Send()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Send");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Send");
    assert_eq!(r[0].target.file, "main.go");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn p6_go_constructor_local_recovers_new_type_convention() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package p\n\ntype Engine struct{}\nfunc NewEngine() *Engine { return &Engine{} }\nfunc (e *Engine) Start() {}\ntype Other struct{}\nfunc (o *Other) Start() {}\nfunc run() {\n    e := NewEngine()\n    e.Start()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Start");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "main.go");
    assert_eq!(r[0].kind, ResolutionKind::ConstructorLocal);
}

#[test]
fn p6_go_wrong_constructor_guess_drops_after_owner_lookup_miss() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package p\n\ntype AccessPath struct{}\nfunc (a *AccessPath) Truncate() {}\nfunc run() {\n    items := NewVec()\n    items.Truncate()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Truncate");
    assert!(cg.resolve_call_site(&site).is_empty());
    assert_eq!(
        cg.resolve_call_site_full(&site).drop,
        Some(DropReason::ExternalReceiver)
    );
}

#[test]
fn java_receiver_method_call_resolves_via_qualifier() {
    use prism::languages::Language::Java;
    // Java method_invocation `svc.readData()` must carry qualifier `svc` so the
    // ladder routes it to R6 (single-owner method) rather than treating it as an
    // unqualified call that R5 drops. (Merge-gate fix: Task 10 surfaced that Java
    // method_invocation qualifiers were never extracted.)
    let (cg, _) = build(&[
        (
            "svc.java",
            "class FileService {\n    public String readData(String p) { return p; }\n}\n",
            Java,
        ),
        (
            "h.java",
            "class Handler {\n    void handle() {\n        FileService svc = new FileService();\n        svc.readData(\"/tmp\");\n    }\n}\n",
            Java,
        ),
    ]);
    let site = site_in(&cg, "handle", "readData");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "java receiver call must resolve: {r:?}");
    assert_eq!(r[0].target.file, "svc.java");
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
}

#[test]
fn c_function_pointer_field_call_binds_single_free_fn_demoted() {
    use prism::languages::Language::C;
    // C has no methods: `ops->process()` is a function-pointer field access, so
    // R6 binds it to a single same-named free function, demoted (the struct-
    // callback heuristic membrane relies on). Method-languages keep method-only.
    let (cg, _) = build(&[
        (
            "api.c",
            "int process(int *d, int len) {\n    return 0;\n}\n",
            C,
        ),
        (
            "driver.c",
            "int run(struct ops *o, int *d, int len) {\n    return o->process(d, len);\n}\n",
            C,
        ),
    ]);
    let site = site_in(&cg, "run", "process");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "C fptr field call must bind: {r:?}");
    assert_eq!(r[0].target.file, "api.c");
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
}

// ---- full-branch review fixes (codex 2026-06-13) ----

#[test]
fn r1_module_prefix_disambiguates_same_named_type() {
    use prism::languages::Language::Rust;
    // foo::Engine::start() must NOT also resolve bar::Engine::start() (same bare
    // owner key, different module) — module prefix narrows to foo's.
    let (cg, _) = build(&[
        (
            "foo.rs",
            "pub struct Engine;\nimpl Engine {\n    pub fn start() {}\n}\n",
            Rust,
        ),
        (
            "bar.rs",
            "pub struct Engine;\nimpl Engine {\n    pub fn start() {}\n}\n",
            Rust,
        ),
        (
            "main.rs",
            "fn run() {\n    foo::Engine::start();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "foo::Engine::start");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "module prefix must narrow: {r:?}");
    assert_eq!(r[0].target.file, "foo.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn r3_import_qualified_does_not_bind_to_method() {
    use prism::languages::Language::Python;
    // import util; util.f() must NOT bind to a METHOD f in util.py (only a
    // module-level free function counts).
    let (cg, _) = build(&[
        (
            "util.py",
            "class C:\n    def f(self):\n        pass\n",
            Python,
        ),
        ("main.py", "import util\ndef run():\n    util.f()\n", Python),
    ]);
    let site = site_in(&cg, "run", "f");
    assert!(
        cg.resolve_call_site(&site).is_empty(),
        "must not bind to the class method"
    );
}

#[test]
fn p6_peel_strips_lifetimes_and_raw_pointers() {
    assert_eq!(prism::resolution::peel_type("&'a Sender"), "Sender");
    assert_eq!(prism::resolution::peel_type("&'a mut Sender"), "Sender");
    assert_eq!(prism::resolution::peel_type("*const Foo"), "Foo");
    assert_eq!(prism::resolution::peel_type("*mut Foo"), "Foo");
}

#[test]
fn p6_lifetime_typed_param_recovers_among_collisions() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        (
            "a.rs",
            "struct Sender;\nimpl Sender {\n    fn send(&self) {}\n}\n",
            Rust,
        ),
        (
            "b.rs",
            "struct Pipe;\nimpl Pipe {\n    fn send(&self) {}\n}\n",
            Rust,
        ),
        (
            "m.rs",
            "fn run<'a>(tx: &'a Sender) {\n    tx.send();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "send");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "lifetime ref param must still recover: {r:?}");
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn go_embedded_method_resolves_exact() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Ping");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Ping");
    assert_eq!(r[0].target.file, "main.go");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_embedded_transitive_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype C struct{}\nfunc (c C) M() {}\ntype B struct{ C }\ntype A struct{ B }\nfunc run(a A) {\n\ta.M()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "run", "M"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_embedded_pointer_receiver_addressable_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b *Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "run", "Ping"));
    assert_eq!(
        r.len(),
        1,
        "addressable value receiver can call a pointer-receiver promoted method"
    );
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_embedded_method_labeled_on_receiver_var_path() {
    use prism::languages::Language::Go;
    // The call is via the method receiver `w` (self/receiver-var path, not P6-lite param).
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc (w Wrap) Run() {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "Run", "Ping"));
    assert_eq!(r.len(), 1);
    assert_eq!(
        r[0].kind,
        ResolutionKind::EmbeddedPromotion,
        "relabel must apply on the receiver-var path too"
    );
}

#[test]
fn go_direct_method_wins_over_promoted() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc (w Wrap) Ping() {}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "run", "Ping"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.start_line, 7, "direct Wrap.Ping (line 7) wins");
    assert_ne!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_equal_depth_embedding_ambiguity_drops() {
    use prism::languages::Language::Go;
    use prism::resolution::DropReason;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype X struct{}\nfunc (x X) M() {}\ntype Y struct{}\nfunc (y Y) M() {}\ntype A struct {\n\tX\n\tY\n}\nfunc run(a A) {\n\ta.M()\n}\n",
        Go,
    )]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "run", "M"));
    assert!(
        out.resolved.is_empty(),
        "equal-depth M is ambiguous -> not promoted"
    );
    assert!(matches!(
        out.drop,
        Some(DropReason::ExternalReceiver) | Some(DropReason::MultiOwnerCollision)
    ));
}

#[test]
fn go_embedded_interface_field_not_promoted() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype R interface { Read() }\ntype S struct {\n\tR\n}\nfunc run(s S) {\n\ts.Read()\n}\n",
        Go,
    )]);
    assert!(cg
        .resolve_call_site_full(&site_in(&cg, "run", "Read"))
        .resolved
        .is_empty());
}

#[test]
fn go_embedding_dropped_on_incremental_when_embedding_file_changes() {
    use prism::cpg::CodePropertyGraph;
    use prism::data_flow::DataFlowGraph;
    use prism::languages::Language::Go;
    use std::collections::{BTreeMap, BTreeSet};

    // Base.Ping in base.go (UNCHANGED); Wrap embeds Base in wrap.go.
    let parse = |p: &str, s: &str| {
        (
            p.to_string(),
            prism::ast::ParsedFile::parse(p, s, Go).unwrap(),
        )
    };
    let mut v1 = BTreeMap::new();
    v1.extend([
        parse(
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
        ),
        parse("wrap.go", "package p\ntype Wrap struct {\n\tBase\n}\n"),
    ]);
    let cg_v1 = prism::call_graph::CallGraph::build(&v1);
    assert!(cg_v1
        .promoted_aliases
        .contains_key(&("Wrap".to_string(), "Ping".to_string())));

    // v2: wrap.go removes the embedding (base.go's fid file is UNCHANGED -> remove_files won't prune it).
    let mut v2 = BTreeMap::new();
    v2.extend([
        parse(
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
        ),
        parse("wrap.go", "package p\ntype Wrap struct{}\n"),
    ]);
    let changed: BTreeSet<String> = ["wrap.go".to_string()].into_iter().collect();
    let dfg = DataFlowGraph::build(&v2);
    let cpg = CodePropertyGraph::build_incremental(cg_v1, dfg, &changed, &v2, None);
    assert!(
        !cpg.call_graph
            .promoted_aliases
            .contains_key(&("Wrap".to_string(), "Ping".to_string())),
        "stale promoted alias must be cleared even though Base.Ping's file is unchanged"
    );
}

#[test]
fn go_remove_files_clears_promoted_aliases_cross_file() {
    // Hardening: remove_files alone (no re-apply) must not leave a stale promoted alias
    // whose target fid lives in an UNCHANGED file (by-fid.file pruning can't catch it).
    use prism::languages::Language::Go;
    use std::collections::{BTreeMap, BTreeSet};
    let parse = |p: &str, s: &str| {
        (
            p.to_string(),
            prism::ast::ParsedFile::parse(p, s, Go).unwrap(),
        )
    };
    let mut files = BTreeMap::new();
    files.extend([
        parse(
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
        ),
        parse("wrap.go", "package p\ntype Wrap struct {\n\tBase\n}\n"),
    ]);
    let mut cg = prism::call_graph::CallGraph::build(&files);
    let key = ("Wrap".to_string(), "Ping".to_string());
    assert!(cg.promoted_aliases.contains_key(&key));
    // Remove ONLY wrap.go; base.go (the alias target's file) is untouched.
    let excl: BTreeSet<String> = ["wrap.go".to_string()].into_iter().collect();
    cg.remove_files(&excl);
    assert!(
        cg.promoted_aliases.is_empty(),
        "remove_files clears promoted aliases"
    );
    assert!(
        !cg.methods.contains_key(&key),
        "stale promoted alias dropped from methods despite base.go unchanged"
    );
}

// ---------------------------------------------------------------------------
// Slice B — TypeAssertion receiver recovery
// ---------------------------------------------------------------------------

fn go_assert_src() -> &'static str {
    "package main\n\
     type Runner interface { Go() }\n\
     type Fast struct{}\nfunc (f Fast) Go() {}\n\
     func use() { _ = Fast{} }\n\
     func run(x any) { x.(Runner).Go() }\n"
}

#[test]
fn type_assertion_interface_receiver_dispatches_exact() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[("main.go", go_assert_src(), Go)]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    assert_eq!(
        site.receiver_recovery,
        Some(ReceiverRecovery::TypeAssertion)
    );
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1); // assert BEFORE .all() (no vacuous pass)
    assert!(r
        .iter()
        .all(|c| c.kind == ResolutionKind::InterfaceDispatch));
}

#[test]
fn type_assertion_concrete_pointer_receiver_owner_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func run(x any) { x.(*Fast).Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Fast")); // owner_key peels the '*'
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Go");
}

#[test]
fn type_assertion_comma_ok_is_not_a_call_receiver() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run(x any) { v, ok := x.(Runner); _ = ok; _ = v }\n",
        Go,
    )]);
    if let Some((_, sites)) = cg.calls.iter().find(|(fid, _)| fid.name == "run") {
        assert!(sites
            .iter()
            .all(|c| c.receiver_recovery != Some(ReceiverRecovery::TypeAssertion)));
    }
}

#[test]
fn type_assertion_grammar_pin_normalization() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let cases = [
        ("x.(Runner).Go()", "Runner"),
        ("x.(pkg.Runner).Go()", "pkg.Runner"), // owner_key keeps pkg.; iface_key strips at route time
        ("x.(*Fast).Go()", "Fast"),
        ("x.((Runner)).Go()", "Runner"), // parenthesized_type unwrapped
    ];
    for (call, want) in cases {
        let src = format!("package main\nfunc run(x any) {{ {call} }}\n");
        let (cg, _) = build(&[("main.go", Box::leak(src.into_boxed_str()), Go)]);
        let site = site_in(&cg, "run", "Go");
        assert_eq!(site.receiver_type.as_deref(), Some(want), "call {call}");
        assert_eq!(
            site.receiver_recovery,
            Some(ReceiverRecovery::TypeAssertion),
            "call {call}"
        );
    }
}

// ---------------------------------------------------------------------------
// Slice C — VarDecl receiver recovery (`var r Runner`)
// ---------------------------------------------------------------------------

#[test]
fn var_local_interface_receiver_dispatches_exact() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run() { var r Runner; r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::VarDecl));
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1); // assert BEFORE .all()
    assert!(r
        .iter()
        .all(|c| c.kind == ResolutionKind::InterfaceDispatch));
}

#[test]
fn var_local_concrete_receiver_owner_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func run() { var r Fast; r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Fast"));
    assert_eq!(cg.resolve_call_site(&site).len(), 1);
}

#[test]
fn var_local_shadowed_binding_bails() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run(x Runner) { var r Runner; r = x; r.Go() }\n",
        Go,
    )]);
    assert_eq!(site_in(&cg, "run", "Go").receiver_type, None); // >1 binding → bail
}

#[test]
fn var_local_false_name_in_initializer_not_recovered() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run() { var r Runner = f(); f.Go() }\n",
        Go,
    )]);
    // `f.Go()` must NOT recover `f` as Runner (f appears only in the initializer).
    if let Some((_, sites)) = cg.calls.iter().find(|(fid, _)| fid.name == "run") {
        for s in sites.iter().filter(|s| s.qualifier.as_deref() == Some("f")) {
            assert_eq!(s.receiver_type, None);
        }
    }
}

#[test]
fn var_local_off_in_legacy_mode() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecoveryConfig;
    let (cg, _) = build_cfg(
        &[(
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             func run() { var r Runner; r.Go() }\n",
            Go,
        )],
        &ReceiverRecoveryConfig::legacy(),
    );
    assert_eq!(site_in(&cg, "run", "Go").receiver_type, None);
}

#[test]
fn config_var_local_only_gates_type_assertion_off() {
    use prism::languages::Language::Go;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig, ReceiverRecoveryMode};
    let cfg = ReceiverRecoveryConfig {
        mode: ReceiverRecoveryMode::Expanded,
        type_assertion: false,
        var_local: true,
    };
    let (cg, _) = build_cfg(
        &[(
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             func a(x any) { x.(Runner).Go() }\n\
             func b() { var r Runner; r.Go() }\n",
            Go,
        )],
        &cfg,
    );
    assert_eq!(site_in(&cg, "a", "Go").receiver_type, None); // type-assertion OFF
    assert_eq!(
        site_in(&cg, "b", "Go").receiver_recovery,
        Some(ReceiverRecovery::VarDecl)
    ); // var ON
}

#[test]
fn config_type_assertion_only_gates_var_local_off() {
    use prism::languages::Language::Go;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig, ReceiverRecoveryMode};
    let cfg = ReceiverRecoveryConfig {
        mode: ReceiverRecoveryMode::Expanded,
        type_assertion: true,
        var_local: false,
    };
    let (cg, _) = build_cfg(
        &[(
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             func a(x any) { x.(Runner).Go() }\n\
             func b() { var r Runner; r.Go() }\n",
            Go,
        )],
        &cfg,
    );
    assert_eq!(
        site_in(&cg, "a", "Go").receiver_recovery,
        Some(ReceiverRecovery::TypeAssertion)
    ); // assert ON
    assert_eq!(site_in(&cg, "b", "Go").receiver_type, None); // var OFF
}
