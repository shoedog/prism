use prism::call_graph::{CallGraph, CallSite};
use prism::cpg::CpgContext;
use prism::navigation::queries;
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

fn build_without_scope_graph(
    sources: &[(&str, &str, prism::languages::Language)],
) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    let mut files = BTreeMap::new();
    for (path, src, lang) in sources {
        files.insert(
            path.to_string(),
            prism::ast::ParsedFile::parse(path, src, *lang).unwrap(),
        );
    }
    (
        CallGraph::build_with_scope_graph_inputs(&files, None),
        files,
    )
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

fn build_rust_complete(
    sources: &[(&str, &str)],
) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    use prism::call_graph::ScopeGraphBuildInputs;
    use prism::languages::Language::Rust;
    use prism::name_resolution::rust_populator::RustCrateConfig;

    let mut files = BTreeMap::new();
    for (path, src) in sources {
        files.insert(
            path.to_string(),
            prism::ast::ParsedFile::parse(path, src, Rust).unwrap(),
        );
    }
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg = RustCrateConfig {
        crate_roots: files.keys().cloned().collect(),
        ..RustCrateConfig::default()
    };
    (
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs)),
        files,
    )
}

fn build_rust_workspace(
    sources: &[(&str, &str)],
    workspace_members: &[&str],
    member_deps: BTreeMap<String, BTreeMap<String, String>>,
) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    use prism::call_graph::ScopeGraphBuildInputs;
    use prism::languages::Language::Rust;
    use prism::name_resolution::rust_populator::RustCrateConfig;

    let mut files = BTreeMap::new();
    for (path, src) in sources {
        files.insert(
            path.to_string(),
            prism::ast::ParsedFile::parse(path, src, Rust).unwrap(),
        );
    }
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: files.keys().cloned().collect(),
        workspace_members: workspace_members.iter().map(|m| m.to_string()).collect(),
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    (
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs)),
        files,
    )
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

// #14 slice 1: target identity is additive. The legacy implementer-name set
// remains the existing consumer contract, while the oracle gets package and
// method-definition evidence needed to distinguish same-named Go types.
#[test]
fn interface_manifest_implementer_identities_are_additive() {
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
    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = manifest["sites"].as_array().expect("sites array");
    let dispatch_site = sites
        .iter()
        .find(|site| site["method"] == "Go" && site["fanout"].as_u64() == Some(2))
        .expect("two-implementer dispatch site");

    // Existing fields retain their exact legacy values for current consumers.
    assert_eq!(
        dispatch_site["implementers"],
        serde_json::json!(["Fast", "Slow"])
    );
    assert_eq!(dispatch_site["fanout"], serde_json::json!(2));

    let identities = dispatch_site["implementer_identities"]
        .as_array()
        .expect("additive identity array");
    assert_eq!(identities.len(), 2);
    for (identity, expected_name) in identities.iter().zip(["Fast", "Slow"]) {
        assert_eq!(identity["name"], expected_name);
        assert_eq!(identity["file"], "main.go");
        assert_eq!(identity["package_dir"], "");
        assert_eq!(identity["package_clause"], "main");
        assert!(
            identity["span"]
                .as_array()
                .is_some_and(|span| span.len() == 2 && span.iter().all(serde_json::Value::is_u64)),
            "method target span must be serialized: {identity:?}"
        );
    }

    let concrete_site = sites
        .iter()
        .find(|site| site["method"] == "Go" && site["fanout"].as_u64() == Some(0))
        .expect("zero-fanout in-scope site");
    assert_eq!(concrete_site["implementers"], serde_json::json!([]));
    assert_eq!(
        concrete_site["implementer_identities"],
        serde_json::json!([]),
        "zero-fanout sites stay emitted and carry an empty identity list"
    );
}

// #14 fix wave 1: serialize the exact pre-#14 site fixture after mechanically
// removing the sole additive field. This pins every legacy key, value, and byte
// ordering to the origin/main emitter semantics rather than sampling two fields.
#[test]
fn interface_manifest_existing_fields_match_origin_main_fixture() {
    use prism::languages::Language::Go;
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
    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let mut legacy_site = manifest["sites"]
        .as_array()
        .expect("sites array")
        .iter()
        .find(|site| site["method"] == "Go" && site["fanout"].as_u64() == Some(2))
        .expect("two-implementer dispatch site")
        .clone();
    let legacy_object = legacy_site.as_object_mut().expect("site object");
    for additive in ["implementer_identities", "dispatch_route"] {
        legacy_object
            .remove(additive)
            .unwrap_or_else(|| panic!("additive field {additive} must be present"));
    }
    let fixture = r#"{"end_byte":202,"fanout":2,"file":"main.go","implementers":["Fast","Slow"],"line":8,"method":"Go","receiver_class":"var_local","start_byte":196}"#;
    assert_eq!(
        serde_json::to_string(&legacy_site).expect("serialize legacy site"),
        fixture,
        "removing the additive field must reproduce origin/main bytes exactly"
    );
}

// The legacy name set dedupes build-tag twins, but the new identity array must
// retain both full targets so the oracle can distinguish their method spans/files.
#[test]
fn interface_manifest_identity_dedup_keeps_build_tag_twins() {
    use prism::languages::Language::Go;
    let (mut cg, _) = build(&[
        (
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             func use() { _ = Impl{} }\n\
             func dispatch(x any) { x.(ExternalRunner).Go() }\n",
            Go,
        ),
        (
            "impl_darwin.go",
            "//go:build darwin\npackage main\n\
             type Impl struct{}\nfunc (Impl) Go() {}\n",
            Go,
        ),
        (
            "impl_linux.go",
            "//go:build linux\npackage main\n\
             type Impl struct{}\nfunc (Impl) Go() {}\n",
            Go,
        ),
    ]);
    // The provider's bare-name satisfaction map intentionally collapses these
    // build partitions upstream (resolution work, out of this oracle slice).
    // Seed the emitter with the two exact FunctionIds it is contracted to retain.
    let twins = cg
        .methods
        .get(&("Impl".to_string(), "Go".to_string()))
        .expect("both parsed build-tag method targets")
        .clone();
    assert_eq!(twins.len(), 2, "CallGraph keeps both file-distinct targets");
    // Use an intentionally unproven receiver key so this remains a serializer
    // test for the unchanged R3 bare lane. A proven `Runner` receiver now
    // correctly applies profile visibility and rejects these conflicting twins.
    cg.interface_impls
        .insert(("ExternalRunner".to_string(), "Go".to_string()), twins);
    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let site = manifest["sites"]
        .as_array()
        .expect("sites array")
        .iter()
        .find(|site| site["method"] == "Go" && site["fanout"].as_u64() == Some(1))
        .expect("dispatch with legacy-name dedup");
    assert_eq!(site["implementers"], serde_json::json!(["Impl"]));
    assert_eq!(site["fanout"], serde_json::json!(1));
    let identities = site["implementer_identities"]
        .as_array()
        .expect("identity array");
    assert_eq!(
        identities.len(),
        2,
        "full tuples must not dedup build-tag twins"
    );
    assert_eq!(
        identities
            .iter()
            .map(|identity| identity["file"].as_str())
            .collect::<Vec<_>>(),
        vec![Some("impl_darwin.go"), Some("impl_linux.go")]
    );
}

// Codex re-review MAJOR: the query/manifest S4 consult must distinguish "no
// S4 route" from "matched S4 route with a missing/empty implementer set" --
// the latter must use the empty implementer slice and must NOT fall through
// to the bare `iface_key(recv_ty)` ladder, mirroring the resolver's own fix
// (resolution.rs's M1 gate-failure drop, ~1669). `holder.Holder` embeds
// `Doer` (a TWO-method interface -- `Do()` + `Extra()` -- so `OtherImpl`
// below does NOT structurally satisfy it; Go interface satisfaction is
// structural, so a single-method `Doer` would spuriously collide with
// `other.Holder`'s single-method shape too, which is a distinct, accepted
// structural-overapprox concern, not the bug under test here). `Doer` has NO
// in-repo implementer; an unrelated `other.Holder` interface (a bare-name
// collision with the STRUCT `holder.Holder`, NOT with the embedded `Doer`)
// declares its own single-method `Do()` with a live implementer `OtherImpl`.
// Pre-fix, the manifest's `.or_else(iface_key(recv_ty))` fallback keys off
// the RECEIVER STRUCT's own bare name ("Holder") and reports `OtherImpl` for
// `h.Do()` -- must report empty instead.
#[test]
fn interface_manifest_s4_matched_route_empty_impls_does_not_fall_through() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "holder/holder.go",
            "package holder\n\
             type Doer interface {\n\tDo()\n\tExtra()\n}\n\
             type Holder struct {\n\tDoer\n}\n\
             func run(h Holder) { h.Do() }\n",
            Go,
        ),
        (
            "other/other.go",
            "package other\n\
             type Holder interface { Do() }\n\
             type OtherImpl struct{}\n\
             func (o OtherImpl) Do() {}\n\
             func use() { _ = OtherImpl{} }\n",
            Go,
        ),
    ]);
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = m["sites"].as_array().expect("sites array");
    let site = sites
        .iter()
        .find(|s| s["method"] == "Do" && s["file"] == "holder/holder.go")
        .expect("holder.Holder's h.Do() dispatch site present");
    assert_eq!(
        site["fanout"].as_u64(),
        Some(0),
        "Doer has no in-repo implementers -- must NOT inherit the unrelated \
         other.Holder interface's OtherImpl via the iface_key fallback: {:?}",
        site
    );
    assert_eq!(
        site["implementers"].as_array().map(|a| a.len()),
        Some(0),
        "matched S4 route with empty impls must report empty, not fall \
         through to an unrelated interface's implementers: {:?}",
        site
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

#[test]
fn rust_receiver_chain_builder_new_cfg_arg_resolves_exact() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Builder;\n\
         impl Builder {\n\
             pub fn new() -> Builder { Builder }\n\
             pub fn cfg(&self, n: u8) -> Builder { Builder }\n\
             pub fn run(&self) {}\n\
         }\n\
         fn drive() { Builder::new().cfg(1).run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "chain receiver should be typed: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_chain_nested_arg_intermediate_resolves_exact() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Builder;\n\
         impl Builder {\n\
             pub fn cfg(&self, n: u8) -> Builder { Builder }\n\
             pub fn tune(&self, a: u8, b: u8) -> Builder { Builder }\n\
             pub fn run(&self) {}\n\
         }\n\
         fn drive(b: Builder) { b.cfg(1).tune(2, 3).run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "nested arg-bearing chain should be typed: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_let_bound_method_init_chain_resolves_exact() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Builder;\n\
         impl Builder { pub fn cfg(&self) -> Builder { Builder } pub fn run(&self) {} }\n\
         fn drive(b: Builder) { let x = b.cfg(); x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "let-bound method-init chain should type: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_destructured_pattern_init_does_not_mistype() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Item; impl Item { pub fn run(&self) {} }\n\
         pub struct Pair(pub Item); impl Pair { pub fn run(&self) {} }\n\
         pub struct Maker; impl Maker { pub fn pair(&self) -> Pair { Pair(Item) } }\n\
         fn drive(m: Maker) { let Pair(x) = m.pair(); x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_none(),
        "destructured pattern binding must not type x as Pair: {site:?}"
    );
    // With same-name decoys (Item::run AND Pair::run), a wrong typed edge would
    // resolve to Pair::run; fail-closed means it does NOT resolve to Pair::run.
    let resolved = cg.resolve_call_site(&site);
    assert!(
        !resolved.iter().any(|r| r.target.name == "run"
            && matches!(&r.confidence, ResolutionConfidence::Exact)),
        "must not Exact-resolve a destructured receiver: {resolved:?}"
    );
}

#[test]
fn rust_receiver_destructured_param_does_not_mistype() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Item; impl Item { pub fn run(&self) {} }\n\
         pub struct Pair(pub Item); impl Pair { pub fn run(&self) {} }\n\
         fn drive(Pair(x): Pair) { x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_none(),
        "destructured param component must not be typed as the param type: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert!(
        !resolved.iter().any(|r| r.target.name == "run"
            && matches!(&r.confidence, ResolutionConfidence::Exact)),
        "must not Exact-resolve a destructured-param receiver: {resolved:?}"
    );
}

#[test]
fn rust_receiver_struct_pattern_param_fails_closed() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Item; impl Item { pub fn run(&self) {} }\n\
         pub struct Point { pub x: Item, pub y: Item } impl Point { pub fn run(&self) {} }\n\
         fn drive(Point { x, .. }: Point) { x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_none(),
        "struct-pattern param component must not be typed as the param type: {site:?}"
    );
}

#[test]
fn rust_receiver_tuple_pattern_param_fails_closed() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct A; impl A { pub fn run(&self) {} }\n\
         pub struct B; impl B { pub fn run(&self) {} }\n\
         fn drive((a, _b): (A, B)) { a.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_none(),
        "tuple-pattern param component must not be typed: {site:?}"
    );
}

#[test]
fn rust_receiver_simple_param_still_types() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Foo; impl Foo { pub fn run(&self) {} }\n\
         pub struct Bar; impl Bar { pub fn run(&self) {} }\n\
         fn drive(x: Foo) { x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "a simple typed param must still type: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_mut_param_still_types() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Foo; impl Foo { pub fn run(&self) {} }\n\
         pub struct Bar; impl Bar { pub fn run(&self) {} }\n\
         fn drive(mut x: Foo) { x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "a `mut x: T` param must still type: {site:?}"
    );
}

#[test]
fn rust_receiver_chain_external_intermediate_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct LocalA; impl LocalA { fn count(&self) {} }\n\
         pub struct LocalB; impl LocalB { fn count(&self) {} }\n\
         fn drive(v: Vec<u8>) { v.iter().count(); }\n",
    )]);
    let site = site_in(&cg, "drive", "count");
    assert!(
        site.receiver_outcome.is_none(),
        "external chain must not type: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_chain_inrepo_then_external_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Foo; impl Foo { pub fn ext(&self) -> String { String::new() } }\n\
         pub struct LocalA; impl LocalA { fn m(&self) {} }\n\
         pub struct LocalB; impl LocalB { fn m(&self) {} }\n\
         fn a() -> Foo { Foo }\n\
         fn drive() { a().ext().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "external return mid-chain must fail closed: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_chain_trait_intermediate_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Foo;\n\
         pub trait T { fn t(&self) -> Foo; }\n\
         impl T for Foo { fn t(&self) -> Foo { Foo } }\n\
         impl Foo { fn m(&self) {} }\n\
         fn drive(f: Foo) { f.t().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "trait intermediate must fail closed: {site:?}"
    );
}

#[test]
fn rust_receiver_chain_wrapper_peel_intermediate_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Foo; pub struct Next; pub struct Other;\n\
         impl Foo { pub fn foo(&self) -> Next { Next } }\n\
         impl Next { fn m(&self) {} }\n\
         impl Other { fn m(&self) {} }\n\
         fn drive(arc: Arc<Foo>) { arc.foo().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "StdWrapperPeel intermediate must fail closed: {site:?}"
    );
    assert!(
        cg.resolve_call_site(&site).is_empty(),
        "wrong typed edge through Arc peel would resolve exactly"
    );
}

#[test]
fn rust_receiver_wrapper_return_chain_fails_closed() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Foo; impl Foo { pub fn step(&self) -> Next { Next } }\n\
         pub struct Next; impl Next { pub fn m(&self) {} }\n\
         pub struct Other; impl Other { pub fn m(&self) {} }\n\
         pub fn make() -> Arc<Foo> { Arc::new(Foo) }\n\
         fn drive() { make().step().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "a chain through an Arc-wrapped return must fail closed: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_alias_wrapper_return_chain_fails_closed() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "type BoxedFoo = Box<Foo>;\n\
         pub struct Foo; impl Foo { pub fn step(&self) -> Next { Next } }\n\
         pub struct Next; impl Next { pub fn m(&self) {} }\n\
         pub struct Other; impl Other { pub fn m(&self) {} }\n\
         pub fn make() -> BoxedFoo { Box::new(Foo) }\n\
         fn drive() { make().step().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "chain through an alias-hidden wrapper return must fail closed: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_alias_wrapper_field_chain_fails_closed() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "type BoxedFoo = Box<Foo>;\n\
         pub struct Foo; impl Foo { pub fn step(&self) -> Next { Next } }\n\
         pub struct Next; impl Next { pub fn m(&self) {} }\n\
         pub struct Other; impl Other { pub fn m(&self) {} }\n\
         pub struct Holder { pub f: BoxedFoo }\n\
         fn drive(h: Holder) { h.f.step().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "chain through an alias-hidden wrapper FIELD must fail closed: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_mut_self_builder_chain_still_types() {
    // &mut Self returns are reference-only peels (NOT std-wrapper) -> buy preserved.
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct B;\n\
         impl B { pub fn new() -> B { B } pub fn opt(&mut self, v: bool) -> &mut Self { self } pub fn run(&self) {} }\n\
         pub struct Other; impl Other { pub fn run(&self) {} }\n\
         fn drive() { let mut b = B::new(); b.opt(true).run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "&mut Self builder chain must still type: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_outcome_cross_module_no_collision() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "mod a { pub struct Foo; impl Foo { pub fn m(&self) {} } }\n\
         mod b { pub struct Foo; impl Foo { pub fn m(&self) {} } }\n\
         fn run(x: crate::b::Foo) { x.m(); }\n",
    )]);
    let site = site_in(&cg, "run", "m");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "b::Foo::m only; a::Foo::m is a decoy");
    assert_eq!(r[0].target.file, "lib.rs");
    assert_eq!(r[0].target.name, "m");
    assert_eq!(r[0].target.start_line, 2);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn rust_receiver_outcome_wins_over_owner_key_collision() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "struct x; impl x { fn m(&self){} }\n\
         struct Real; impl Real { fn m(&self){} }\n\
         fn run(x: Real) { x.m(); }\n",
    )]);
    let site = site_in(&cg, "run", "m");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "Real::m only; x::m is a decoy");
    assert_eq!(r[0].target.file, "lib.rs");
    assert_eq!(r[0].target.name, "m");
    assert_eq!(r[0].target.start_line, 2);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn rust_receiver_outcome_trait_static_dispatch_found_as_nameonly() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "trait Runner { fn go(&self); }\n\
         struct Fast;\n\
         impl Runner for Fast { fn go(&self) {} }\n\
         fn run(f: crate::Fast) { f.go(); }\n",
    )]);
    let site = site_in(&cg, "run", "go");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn rust_receiver_outcome_trait_dyn_dispatch_found_as_nameonly() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "trait Runner { fn go(&self); }\n\
         struct Fast;\n\
         impl Runner for Fast { fn go(&self) {} }\n\
         fn run(r: &dyn crate::Runner) { r.go(); }\n",
    )]);
    let site = site_in(&cg, "run", "go");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn rust_receiver_outcome_external_recv_no_in_repo_method_drops() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "fn get() {}\n\
         fn run(m: std::collections::BTreeMap<u8, u8>) { m.get(&0); }\n",
    )]);
    let site = site_in(&cg, "run", "get");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let out = cg.resolve_call_site_full(&site);
    assert!(
        out.resolved.is_empty(),
        "free fn get must not be a receiver edge"
    );
    assert_eq!(out.drop, Some(DropReason::ExternalReceiver));
}

#[test]
fn rust_receiver_outcome_external_same_bare_decoy_does_not_bind_local_method() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "fn run(s: std::string::String) { s.ext(); }\n\
         mod local {\n\
             pub struct String;\n\
             impl String { pub fn ext(&self) {} }\n\
         }\n",
    )]);
    let site = site_in(&cg, "run", "ext");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let out = cg.resolve_call_site_full(&site);
    assert!(
        out.resolved
            .iter()
            .all(|callee| callee.target.start_line != 4),
        "std String receiver must not bind local::String::ext: {:?}",
        out.resolved
    );
    assert!(out.resolved.is_empty(), "{out:?}");
    assert_eq!(out.drop, Some(DropReason::ExternalReceiver));
}

#[test]
fn rust_receiver_outcome_extension_trait_on_external_resolves() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "trait Ext { fn ext(&self); }\n\
         impl Ext for String { fn ext(&self) {} }\n\
         mod local {\n\
             pub struct String;\n\
             impl String { pub fn ext(&self) {} }\n\
         }\n\
         fn run(s: String) { s.ext(); }\n",
    )]);
    let site = site_in(&cg, "run", "ext");
    assert!(site.receiver_outcome.is_some(), "{site:?}");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "ext");
    assert_eq!(r[0].target.start_line, 2, "must bind the extension impl");
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
}

#[test]
fn rust_receiver_outcome_unrecovered_receiver_hits_residue() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "struct OnlyOwner;\n\
         impl OnlyOwner { fn frobnicate(&self) {} }\n\
         fn mystery() {}\n\
         fn run() { let x = mystery(); x.frobnicate(); }\n",
    )]);
    let site = site_in(&cg, "run", "frobnicate");
    assert!(
        site.receiver_outcome.is_none(),
        "unrecovered receiver stays unrecovered"
    );
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::R6SingleOwner);
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
}

#[test]
fn rust_receiver_outcome_incomplete_identity_bucket_falls_back_to_bare() {
    use prism::resolution::ReceiverRecovery;
    use prism::resolution_identity::{ReceiverOutcome, ReceiverTypeKey};

    let (mut cg, _) = build_rust_complete(&[(
        "lib.rs",
        "mod a { pub struct Foo; impl Foo { pub fn m(&self) {} } }\n\
         fn run(x: crate::a::Foo) { x.m(); }\n",
    )]);
    let mut site = site_in(&cg, "run", "m");
    let (scope, _) = cg
        .methods_by_scope
        .keys()
        .find(|(_, name)| name == "m")
        .cloned()
        .expect("identity-indexed Foo::m");
    cg.methods_by_scope.remove(&(scope, "m".to_string()));
    cg.identity_complete
        .remove(&("Foo".to_string(), "m".to_string()));
    site.receiver_outcome = Some(ReceiverOutcome {
        key: ReceiverTypeKey::InRepo(scope),
        bare: "Foo".to_string(),
        recovery: ReceiverRecovery::TypedParam,
    });

    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "must fall back to bare owner_lookup, not drop");
    assert_eq!(r[0].target.name, "m");
}

#[test]
fn rust_receiver_outcome_graph_backed_combine_kind_inherent_exact_trait_nameonly() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "struct Fast;\n\
         impl Fast { fn inherent(&self) {} }\n\
         trait Runner { fn go(&self); }\n\
         impl Runner for Fast { fn go(&self) {} }\n\
         fn run(f: crate::Fast) { f.inherent(); f.go(); }\n",
    )]);

    let inherent_site = site_in(&cg, "run", "inherent");
    assert!(
        inherent_site.receiver_outcome.is_some(),
        "{inherent_site:?}"
    );
    let inherent = cg.resolve_call_site(&inherent_site);
    assert_eq!(inherent.len(), 1);
    assert_eq!(inherent[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(inherent[0].kind, ResolutionKind::TypedParam);

    let trait_site = site_in(&cg, "run", "go");
    assert!(trait_site.receiver_outcome.is_some(), "{trait_site:?}");
    let trait_call = cg.resolve_call_site(&trait_site);
    assert_eq!(trait_call.len(), 1);
    assert_eq!(trait_call[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(trait_call[0].kind, ResolutionKind::TypedParam);
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
fn owner_collision_pool_demotes_to_name_only() {
    use prism::languages::Language::Rust;
    // Two distinct `Foo` types, each with an associated `make`. A qualified
    // `Foo::make()` keys the bare index ("Foo","make") to BOTH defs; both share
    // primary owner "Foo", so this is NOT trait-CHA. Build WITHOUT a scope graph
    // so resolution reaches owner_lookup_in_modules directly (a complete scope
    // graph would narrow/drop the call upstream before this rung).
    let (cg, _) = build_without_scope_graph(&[
        (
            "a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n",
            Rust,
        ),
        (
            "b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n",
            Rust,
        ),
        ("c.rs", "fn run() {\n    Foo::make();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "Foo::make");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2, "both Foo::make defs retained (recall)");
    assert!(
        r.iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "collision pool demoted, not Exact"
    );
    assert!(r.iter().all(|c| c.kind == ResolutionKind::QualifiedOwner));
}

#[test]
fn owner_single_candidate_stays_exact() {
    use prism::languages::Language::Rust;
    let (cg, _) = build_without_scope_graph(&[
        (
            "a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n",
            Rust,
        ),
        ("c.rs", "fn run() {\n    Foo::make();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "Foo::make");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn owner_inherent_plus_trait_same_name_demotes() {
    use prism::languages::Language::Rust;
    // ONE type `Foo` with an inherent `m` AND a same-named trait-impl `m`. Both
    // register under ("Foo","m") with primary owner "Foo" -> a non-trait-CHA
    // multi-candidate pool (pool=2, primary_owners=1). Confirms the demote covers
    // the accepted same-owner ambiguity set, not just distinct same-named types.
    let (cg, _) = build_without_scope_graph(&[(
        "a.rs",
        "pub struct Foo;\n\
         impl Foo {\n    pub fn m(&self) {}\n}\n\
         pub trait T {\n    fn m(&self);\n}\n\
         impl T for Foo {\n    fn m(&self) {}\n}\n\
         fn run() {\n    Foo::m();\n}\n",
        Rust,
    )]);
    let site = site_in(&cg, "run", "Foo::m");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2, "inherent + trait-impl m both retained");
    assert!(r
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::QualifiedOwner));
}

#[test]
fn recovered_receiver_collision_demotes_and_keeps_typed_param_relabel() {
    use prism::languages::Language::Rust;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig};
    // Two distinct `Foo` types each with `make`; `run(r: Foo)` calls `r.make()`.
    // P6-lite recovers r:Foo syntactically (no conventional crate root -> no
    // methods_by_scope narrowing preempts); R6 routes to owner_lookup("Foo","make")
    // -> 2-candidate collision -> demote. R6 relabels kind QualifiedOwner ->
    // TypedParam; the NameOnly confidence must ride through.
    let (cg, _) = build_cfg(
        &[
            (
                "a.rs",
                "pub struct Foo;\nimpl Foo {\n    pub fn make(&self) {}\n}\n",
                Rust,
            ),
            (
                "b.rs",
                "pub struct Foo;\nimpl Foo {\n    pub fn make(&self) {}\n}\n",
                Rust,
            ),
            ("c.rs", "fn run(r: Foo) {\n    r.make();\n}\n", Rust),
        ],
        &ReceiverRecoveryConfig::default(),
    );
    let site = site_in(&cg, "run", "make");
    assert_eq!(
        site.receiver_recovery,
        Some(ReceiverRecovery::TypedParam),
        "fixture must recover the typed-param receiver"
    );
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(
        r.iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "demoted confidence rides through the kind relabel"
    );
    assert!(r.iter().all(|c| c.kind == ResolutionKind::TypedParam));
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
fn rust_scope_graph_unqualified_import_narrows_to_single_callable() {
    use prism::languages::Language::Rust;
    let sources = [
        (
            "src/lib.rs",
            "mod engine;\nmod other;\nuse crate::engine::process;\npub fn g() {\n    process();\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn process() {}\n", Rust),
        ("src/other.rs", "pub fn process() {}\n", Rust),
    ];
    let (with_graph, _) = build(&sources);
    let r = with_graph.resolve_call_site(&site_in(&with_graph, "g", "process"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "src/engine.rs");
    assert_ne!(r[0].target.file, "src/other.rs");

    let (without_graph, _) = build_without_scope_graph(&sources);
    let legacy = without_graph.resolve_call_site(&site_in(&without_graph, "g", "process"));
    let legacy_files: std::collections::BTreeSet<&str> =
        legacy.iter().map(|c| c.target.file.as_str()).collect();
    assert_eq!(
        legacy_files,
        std::collections::BTreeSet::from(["src/engine.rs", "src/other.rs"])
    );
}

#[test]
fn rust_scope_graph_alias_import_narrows_to_defining_callable_not_spelling_decoy() {
    use prism::languages::Language::Rust;
    let sources = [
        (
            "src/lib.rs",
            "mod decoy;\nmod engine;\nuse crate::engine::process as run;\npub fn g() {\n    run();\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn process() {}\n", Rust),
        ("src/decoy.rs", "pub fn run() {}\n", Rust),
    ];
    let (cg, _) = build(&sources);
    let r = cg.resolve_call_site(&site_in(&cg, "g", "run"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "process");
    assert_eq!(r[0].target.file, "src/engine.rs");
    assert_ne!(r[0].target.name, "run");
    assert_ne!(r[0].target.file, "src/decoy.rs");
}

#[test]
fn rust_scope_graph_ambiguous_alias_import_falls_through_without_wrong_process() {
    use prism::languages::Language::Rust;
    let sources = [
        (
            "src/lib.rs",
            "mod engine;\nmod other;\nuse crate::engine::process as run;\nuse crate::other::process as run;\npub fn g() {\n    run();\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn process() {}\n", Rust),
        ("src/other.rs", "pub fn process() {}\n", Rust),
    ];
    let (cg, _) = build(&sources);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "run")).is_empty(),
        "ambiguous aliases to two process definitions must fall through"
    );
}

#[test]
fn rust_scope_graph_unqualified_declines_do_not_legacy_guess() {
    use prism::languages::Language::Rust;

    let no_import = [
        (
            "src/lib.rs",
            "mod engine;\nmod other;\npub fn g() {\n    process();\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn process() {}\n", Rust),
        ("src/other.rs", "pub fn process() {}\n", Rust),
    ];
    let (cg, _) = build(&no_import);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "process"))
            .is_empty(),
        "authoritative unresolved bare name must not fall back to same-name fan-out"
    );
    let (legacy_cg, _) = build_without_scope_graph(&no_import);
    assert_eq!(
        legacy_cg
            .resolve_call_site(&site_in(&legacy_cg, "g", "process"))
            .len(),
        2,
        "without an authoritative graph the legacy ladder is unchanged"
    );

    let parent_name = [(
        "src/lib.rs",
        "fn start() {}\nmod child {\n    pub fn g() {\n        start();\n    }\n}\n",
        Rust,
    )];
    let (cg, _) = build(&parent_name);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "start")).is_empty(),
        "bare lookup must stop at the child module boundary"
    );

    let shadowed = [(
        "src/lib.rs",
        "fn process() {}\npub fn g() {\n    let process = || {};\n    process();\n}\n",
        Rust,
    )];
    let (cg, _) = build(&shadowed);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "process"))
            .is_empty(),
        "a local let binding shadows the free fn and must not mint an edge"
    );

    let private_import = [
        (
            "src/lib.rs",
            "mod engine;\nmod other;\nmod child {\n    use crate::engine::process;\n    pub fn g() {\n        process();\n    }\n}\n",
            Rust,
        ),
        ("src/engine.rs", "fn process() {}\n", Rust),
        ("src/other.rs", "pub fn process() {}\n", Rust),
    ];
    let (cg, _) = build(&private_import);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "process"))
            .is_empty(),
        "an inaccessible private import must fall through without a decoy edge"
    );
}

#[test]
fn rust_scope_graph_qualified_paths_resolve_or_disable_legacy_stem() {
    use prism::languages::Language::Rust;
    let positive = [
        (
            "src/lib.rs",
            "mod engine;\npub fn call_crate() {\n    crate::engine::start();\n}\nmod inner {\n    pub fn start() {}\n    pub fn call_self() {\n        self::start();\n    }\n    pub mod child {\n        pub fn call_super() {\n            super::start();\n        }\n    }\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn start() {}\n", Rust),
        ("src/other.rs", "pub fn start() {}\n", Rust),
    ];
    let (cg, _) = build(&positive);
    let crate_edge = cg.resolve_call_site(&site_in(&cg, "call_crate", "crate::engine::start"));
    assert_eq!(crate_edge.len(), 1);
    assert_eq!(crate_edge[0].target.file, "src/engine.rs");

    let self_edge = cg.resolve_call_site(&site_in(&cg, "call_self", "self::start"));
    assert_eq!(self_edge.len(), 1);
    assert_eq!(self_edge[0].target.file, "src/lib.rs");

    let super_edge = cg.resolve_call_site(&site_in(&cg, "call_super", "super::start"));
    assert_eq!(super_edge.len(), 1);
    assert_eq!(super_edge[0].target.file, "src/lib.rs");

    let negative = [
        (
            "src/lib.rs",
            "pub fn g() {\n    crate::missing::target();\n}\n",
            Rust,
        ),
        ("src/missing.rs", "pub fn target() {}\n", Rust),
    ];
    let (cg, _) = build(&negative);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "crate::missing::target"))
            .is_empty(),
        "authoritative unresolved qualified path must not use the legacy stem heuristic"
    );
    let (legacy_cg, _) = build_without_scope_graph(&negative);
    let legacy = legacy_cg.resolve_call_site(&site_in(&legacy_cg, "g", "crate::missing::target"));
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].target.file, "src/missing.rs");
}

#[test]
fn rust_scope_graph_authority_gate_and_poison_skip_legacy() {
    use prism::languages::Language::Rust;
    let resolving = [
        (
            "src/lib.rs",
            "mod engine;\nmod other;\nuse crate::engine::process;\npub fn g() {\n    process();\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn process() {}\n", Rust),
        ("src/other.rs", "pub fn process() {}\n", Rust),
    ];
    let (with_graph, _) = build(&resolving);
    let narrowed = with_graph.resolve_call_site(&site_in(&with_graph, "g", "process"));
    assert_eq!(narrowed.len(), 1);
    assert_eq!(narrowed[0].target.file, "src/engine.rs");

    let (without_graph, _) = build_without_scope_graph(&resolving);
    let legacy = without_graph.resolve_call_site(&site_in(&without_graph, "g", "process"));
    assert_eq!(legacy.len(), 2);
    let legacy_files: std::collections::BTreeSet<&str> =
        legacy.iter().map(|c| c.target.file.as_str()).collect();
    assert_eq!(
        legacy_files,
        std::collections::BTreeSet::from(["src/engine.rs", "src/other.rs"]),
        "no scope graph keeps the legacy same-name fan-out"
    );
    assert!(legacy.iter().all(|c| c.target.name == "process"));
    assert!(legacy
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(legacy.iter().all(|c| c.kind == ResolutionKind::FreeMulti));

    let poisoned = [
        (
            "src/lib.rs",
            "mod engine;\nmod other;\nuse crate::missing::process;\npub fn g() {\n    process();\n}\n",
            Rust,
        ),
        ("src/engine.rs", "pub fn process() {}\n", Rust),
        ("src/other.rs", "pub fn process() {}\n", Rust),
    ];
    let (cg, _) = build(&poisoned);
    assert!(
        cg.resolve_call_site(&site_in(&cg, "g", "process"))
            .is_empty(),
        "a pending/unresolved import poisons graph lookup and suppresses legacy fan-out"
    );
}

#[test]
fn scope_graph_two_crate_owner_collision_recovers_to_single_exact() {
    use prism::languages::Language::Rust;
    // The ruff CliTest::with_file class in miniature: two crates each define
    // `CliTest::with_file`. With the scope graph present, a call in crate `a`
    // resolves to crate `a`'s definition alone -- single Exact (the headline
    // recovery). The bare `("CliTest","with_file")` key holds BOTH defs.
    let sources = [
        (
            "a/src/lib.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\npub fn drive() {\n    CliTest::with_file();\n}\n",
            Rust,
        ),
        (
            "b/src/lib.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert!(
        cg.scope_graph.is_some(),
        "convention build has a scope graph"
    );
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "with_file".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key collides across both crates"
    );
    let r = cg.resolve_call_site(&site_in(&cg, "drive", "CliTest::with_file"));
    assert_eq!(r.len(), 1, "recovers to a single candidate");
    assert_eq!(r[0].target.file, "a/src/lib.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn call_stats_reports_glob_expand_histogram_shape() {
    use prism::languages::Language::Rust;

    let (cg, _) = build(&[(
        "src/lib.rs",
        "pub use inner::*;\nmod inner { pub struct Widget; impl Widget { pub fn make() {} } }\npub fn drive() {\n    Widget::make();\n}\n",
        Rust,
    )]);
    let stats = queries::call_stats(&cg);
    let ge = stats
        .get("glob_expand")
        .and_then(|v| v.as_object())
        .expect("call_stats must report glob_expand object");
    for key in [
        "resolved_l1",
        "resolved_l2",
        "depth_exceeded",
        "cycle",
        "external",
        "multi_target",
        "vis_unknown",
        "member_multi",
        "member_undecidable",
        "member_hidden_continued",
        "member_hidden_continue_hit",
        "member_hidden_continue_empty",
        "member_hidden_continue_poison",
    ] {
        assert!(
            ge.get(key).and_then(|v| v.as_u64()).is_some(),
            "glob_expand.{key} must be an integer"
        );
    }
}

#[test]
fn cross_crate_glob_facade_collision_dep_crate_recovers_single_exact() {
    let mut consumer_deps = BTreeMap::new();
    consumer_deps.insert("foo".to_string(), "crates/foo".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("crates/consumer".to_string(), consumer_deps);

    let (cg, _) = build_rust_workspace(
        &[
            (
                "crates/consumer/src/lib.rs",
                "use foo::Widget;\npub fn dependent() {\n    Widget::make();\n}\n",
            ),
            (
                "crates/foo/src/lib.rs",
                "pub use inner::*;\nmod inner { pub struct Widget; impl Widget { pub fn make() {} } }\n",
            ),
            (
                "crates/bar/src/lib.rs",
                "pub use inner::*;\nmod inner { pub struct Widget; impl Widget { pub fn make() {} } }\n",
            ),
        ],
        &["crates/bar", "crates/consumer", "crates/foo"],
        member_deps,
    );
    assert!(
        cg.scope_graph.is_some(),
        "workspace build should store a scope graph"
    );
    assert_eq!(
        cg.methods
            .get(&("Widget".to_string(), "make".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across the two facade crates"
    );

    let resolved = cg.resolve_call_site(&site_in(&cg, "dependent", "Widget::make"));
    assert_eq!(resolved.len(), 1, "dependent crate recovers one owner");
    assert_eq!(resolved[0].target.file, "crates/foo/src/lib.rs");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(resolved[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn scope_graph_inherent_plus_trait_owner_demotes_not_drops() {
    use prism::languages::Language::Rust;
    // The resolved type `Widget` owns BOTH an inherent `make` and a trait `make`.
    // The leading segment binds directly + unshadowed, so the predicate runs, but
    // it cannot prune below the inherent/trait pair (both owned by Widget).
    let sources = [(
        "src/lib.rs",
        "pub struct Widget;\npub trait Build { fn make(&self); }\nimpl Widget { pub fn make(&self) {} }\nimpl Build for Widget { fn make(&self) {} }\npub fn drive() {\n    Widget::make();\n}\n",
        Rust,
    )];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Widget::make"));
    assert_eq!(out.drop, None, "must not drop -- recall fix");
    assert_eq!(out.resolved.len(), 2, "inherent + trait both kept");
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "the unprunable owner pair demotes to NameOnly"
    );
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::QualifiedOwner));
}

#[test]
fn scope_graph_unresolved_owner_path_keeps_full_pool_not_drop() {
    use prism::languages::Language::Rust;
    // The owner type path does NOT resolve through the graph (`Missing` is not in
    // scope at the call site), but the bare owner key collides across two files.
    // Keep all candidates and route the `::` site to the #120 demote floor.
    let sources = [
        (
            "src/lib.rs",
            "mod other;\nmod more;\npub fn drive() {\n    Missing::make();\n}\n",
            Rust,
        ),
        (
            "src/other.rs",
            "pub struct Missing;\nimpl Missing {\n    pub fn make(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/more.rs",
            "pub struct Missing;\nimpl Missing {\n    pub fn make(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("Missing".to_string(), "make".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide so the floor is NameOnly, not Exact",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Missing::make"));
    assert_eq!(out.drop, None, "owner-keyed `::` miss demotes, not drops");
    assert_eq!(out.resolved.len(), 2, "both colliding defs are kept");
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "fail-open lands at the #120 NameOnly demote floor"
    );
    assert!(
        out.resolved
            .iter()
            .all(|c| c.kind == ResolutionKind::QualifiedOwner),
        "same-owner collision demotes as QualifiedOwner"
    );
}

#[test]
fn scope_graph_block_local_glob_shadow_keeps_all() {
    use prism::languages::Language::Rust;
    // Module-level `use a::Foo;` plus block-local `use b::*;`. The graph can
    // resolve the module-anchored `Foo`, but the block-local glob may shadow it,
    // so the disproof predicate must keep the full owner pool.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    use crate::b::*;\n    Foo::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None);
    assert!(
        out.resolved.len() >= 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "block-local glob shadow keeps the full pool at NameOnly"
    );
}

#[test]
fn scope_graph_block_local_exact_use_shadow_keeps_all() {
    use prism::languages::Language::Rust;
    // Module-level `use a::Foo;` plus a block-local EXACT `use b::Foo;`. The
    // block-local exact-ident binding may shadow the module-anchored `Foo`, so the
    // disproof predicate's ①C exact-`NS_TYPE` scan must keep the full owner pool
    // (the exact-binding shadow shape, distinct from the glob/macro cases).
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    use crate::b::Foo;\n    Foo::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None);
    assert!(
        out.resolved.len() >= 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "block-local exact `use` shadow keeps the full pool at NameOnly"
    );
}

#[test]
fn scope_graph_macro_wildcard_shadow_keeps_all() {
    use prism::languages::Language::Rust;
    // An item-position macro invocation can introduce a type binding after
    // expansion. Treat the trailing block scope as potentially shadowed.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\nmacro_rules! gen { () => {}; }\npub fn drive() {\n    gen!();\n    Foo::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None);
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "a covering macro wildcard keeps the full pool at NameOnly"
    );
}

#[test]
fn scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact() {
    use prism::languages::Language::Rust;
    // The leading segment `Foo` binds at the call site via a SINGLE named import
    // (`use crate::a::Foo;` -> `BindTarget::Pending`) that resolves unambiguously to
    // one in-repo item (`a::Foo`). The prune-through-`use` slice folds that import
    // through the engine and recovers the colliding pool to the single Exact
    // `a::Foo::m` -- the other `b::Foo::m` is disproved.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    Foo::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::Foo and b::Foo",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None, "the recovered owner path must not drop");
    assert_eq!(
        out.resolved.len(),
        1,
        "a single `use`-imported in-repo owner recovers to one candidate",
    );
    assert_eq!(
        out.resolved[0].target.file, "src/a.rs",
        "the recovered target is the imported `a::Foo`'s method, not `b::Foo`'s",
    );
    assert_eq!(
        out.resolved[0].confidence,
        ResolutionConfidence::Exact,
        "pruning through the single `use` chain mints Exact",
    );
    assert_eq!(out.resolved[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn scope_graph_pending_cross_crate_use_keeps_all() {
    use prism::languages::Language::Rust;
    // A SINGLE visible `Pending` whose multi-segment `use` chain leaves the repo
    // (`use some_external::CliTest as CliTest;`). `some_external` is not an in-repo
    // crate, so the engine fails the non-final prefix segment and returns
    // `ResStatus::Unresolved` -> the helper declines -> keep-all. We must NOT prune
    // to an in-repo `CliTest` we happen to also own. (The realistic cross-crate
    // `use` shape; the `Target::External` candidate branch is pinned separately
    // below by an explicit `extern crate`.)
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse some_external::CliTest as CliTest;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::CliTest and b::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(
        out.drop, None,
        "an unresolved cross-crate import alias must not drop"
    );
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "an unresolved external `use` chain declines -> keep the full colliding pool at NameOnly",
    );
}

#[test]
fn scope_graph_pending_extern_crate_alias_keeps_all() {
    use prism::languages::Language::Rust;
    // A SINGLE visible `Pending` (`use some_external as CliTest;`) whose own path is
    // the single segment `some_external`, bound by `extern crate some_external;` to a
    // `Target::External` candidate at the crate root (not an in-repo crate). The
    // helper re-resolves that path to `ResStatus::Resolved` with one `Target::External`
    // candidate -> not a `Target::Item` -> declines -> keep-all. This is the fixture
    // that genuinely exercises the `Target::External` candidate branch.
    let sources = [
        (
            "src/lib.rs",
            "extern crate some_external;\nmod a;\nmod b;\nuse some_external as CliTest;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::CliTest and b::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(
        out.drop, None,
        "an `extern crate` external alias must not drop"
    );
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "a `Target::External` candidate declines -> keep the full colliding pool at NameOnly",
    );
}

#[test]
fn scope_graph_pending_ambiguous_reexport_keeps_all() {
    use prism::languages::Language::Rust;
    // ONE visible `Pending` (`use crate::facade::CliTest;`) whose target module
    // re-exports `CliTest` from TWO crates ambiguously. The binding's import path
    // resolves `Ambiguous`, so the helper declines -> keep-all. Pins the
    // `Ambiguous` branch via the `Pending` arm (a single visible binding).
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nmod facade;\nuse crate::facade::CliTest;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/facade.rs",
            "pub use crate::a::CliTest;\npub use crate::b::CliTest;\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::CliTest and b::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(out.drop, None, "an ambiguous re-export must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "an ambiguous `use` re-export declines -> keep the full pool at NameOnly",
    );
}

#[test]
fn scope_graph_module_glob_import_keeps_all() {
    use prism::languages::Language::Rust;
    // Module-level `use crate::ru::*;` brings `CliTest` via a glob EDGE, not a
    // `Binding`, so there is no single visible `Pending` for the leading segment.
    // `leading_segment_binds_directly` finds an empty rib and declines -> keep-all.
    let sources = [
        (
            "src/lib.rs",
            "mod ty;\nmod ru;\nuse crate::ru::*;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/ty.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/ru.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across ty::CliTest and ru::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(out.drop, None, "a glob import must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "a module-level glob keeps the full colliding pool at NameOnly",
    );
}

#[test]
fn scope_graph_non_uniform_edition_keeps_all() {
    // Mixed-edition graphs are non-authoritative for disproof. Even when a
    // direct, unshadowed owner path would otherwise prune to one candidate, the
    // edition guard must keep the full colliding pool at the demote floor.
    let sources = [
        (
            "a/src/lib.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\npub fn drive() {\n    Foo::m();\n}\n",
        ),
        (
            "b/src/lib.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
        ),
    ];
    let (mut cg, _) = build_rust_complete(&sources);
    cg.scope_graph
        .as_mut()
        .expect("scope graph")
        .edition_uniform = false;
    assert_eq!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide for the keep-all assertion",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None);
    assert_eq!(out.resolved.len(), 2, "non-uniform edition keeps all");
    assert!(out
        .resolved
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn scope_graph_free_fn_path_not_misrouted_to_colliding_method_pool() {
    use prism::languages::Language::Rust;
    // `crate::m::f()` is a module free-function call. A struct `m` with method
    // `f` in another module creates a colliding bare method bucket, so the
    // free-fn guard must keep this on the graph free-function path.
    let sources = [
        (
            "src/lib.rs",
            "mod m;\nmod other;\npub fn drive() {\n    crate::m::f();\n}\n",
            Rust,
        ),
        ("src/m.rs", "pub fn f() {}\n", Rust),
        (
            "src/other.rs",
            "pub struct m;\nimpl m {\n    pub fn f(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("m".to_string(), "f".to_string()))
            .map(|v| v.len()),
        Some(1),
        "the cross-module struct method must populate the colliding bucket",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "crate::m::f"));
    assert_eq!(out.drop, None, "the free-fn `::` path resolves, not drops");
    assert_eq!(
        out.resolved.len(),
        1,
        "the module free fn is the single resolved target"
    );
    assert_eq!(
        out.resolved[0].target.file, "src/m.rs",
        "must resolve to the module free fn, never the cross-module method pool",
    );
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn non_rust_resolution_is_unchanged_with_scope_graph_present() {
    use prism::languages::Language::Python;
    let sources = [
        ("main.py", "def run():\n    process()\n", Python),
        ("a.py", "def process():\n    pass\n", Python),
        ("b.py", "def process():\n    pass\n", Python),
    ];
    let (with_graph, _) = build(&sources);
    let (without_graph, _) = build_without_scope_graph(&sources);
    assert_eq!(
        format!(
            "{:?}",
            with_graph.resolve_call_site_full(&site_in(&with_graph, "run", "process"))
        ),
        format!(
            "{:?}",
            without_graph.resolve_call_site_full(&site_in(&without_graph, "run", "process"))
        )
    );
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
fn py_receiver_typed_param_recovers_exact_among_collisions() {
    use prism::languages::Language::Python;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\nclass Other:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
        Python,
    )]);
    let site = site_in(&cg, "run", "m");
    assert_eq!(site.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::TypedParam));
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "svc.py");
    assert_eq!(r[0].target.start_line, 2);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn py_inherited_base_typed_param_exact() {
    use prism::languages::Language::Python;
    let (cg, _) = build(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\nclass Other:\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n",
        Python,
    )]);
    let site = site_in(&cg, "run", "go");
    assert_eq!(site.receiver_type.as_deref(), Some("Child"));
    let out = cg.resolve_call_site_full(&site);
    assert_eq!(out.resolved.len(), 1, "{out:?}");
    let callee = &out.resolved[0];
    assert_eq!(callee.target.file, "svc.py");
    assert_eq!(callee.target.start_line, 2);
    assert_eq!(callee.kind, ResolutionKind::TypedParam);
    assert_eq!(callee.confidence, ResolutionConfidence::Exact);
}

#[test]
fn py_imported_receiver_type_skips_recovery_and_no_false_exact() {
    use prism::languages::Language::Python;
    let (cg, _) = build(&[(
        "svc.py",
        "from ext import Foo\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
        Python,
    )]);
    let site = site_in(&cg, "run", "m");
    assert_eq!(site.receiver_type, None);
    let out = cg.resolve_call_site_full(&site);
    assert!(out.resolved.iter().all(|c| {
        c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
    }));
}

#[test]
fn py_wildcard_import_skips_recovery_for_whole_file_both_orders() {
    use prism::languages::Language::Python;
    for (name, src) in [
        (
            "before.py",
            "from ext import *\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
        ),
        (
            "after.py",
            "from ext import *\ndef run(x: Foo):\n    x.m()\nclass Foo:\n    def m(self):\n        pass\n",
        ),
    ] {
        let (cg, _) = build(&[(name, src, Python)]);
        let site = site_in(&cg, "run", "m");
        assert_eq!(site.receiver_type, None, "{name}");
        let out = cg.resolve_call_site_full(&site);
        assert!(out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
        }));
    }
}

#[test]
fn py_recovered_receiver_preempts_r3b_owner_key_collision() {
    use prism::languages::Language::Python;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[(
        "svc.py",
        "class x:\n    def m(self):\n        pass\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
        Python,
    )]);
    let site = site_in(&cg, "run", "m");
    assert_eq!(site.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::TypedParam));
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.start_line, 5, "Foo.m must win over x.m");
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn py_recovered_local_miss_falls_through_to_residue_parity() {
    use prism::languages::Language::Python;
    let (cg, _) = build(&[(
        "svc.py",
        "class Foo:\n    pass\nclass Other:\n    def missing(self):\n        pass\ndef annotated(x: Foo):\n    x.missing()\ndef plain(x):\n    x.missing()\n",
        Python,
    )]);
    let annotated = site_in(&cg, "annotated", "missing");
    let plain = site_in(&cg, "plain", "missing");
    assert_eq!(annotated.receiver_type.as_deref(), Some("Foo"));
    let annotated_out = cg.resolve_call_site_full(&annotated);
    let plain_out = cg.resolve_call_site_full(&plain);
    assert_eq!(annotated_out.drop, plain_out.drop);
    assert_eq!(annotated_out.resolved, plain_out.resolved);
    assert_ne!(annotated_out.drop, Some(DropReason::ExternalReceiver));
}

#[test]
fn py_recovered_multi_owner_hit_preserves_nameonly_confidence() {
    use prism::languages::Language::Python;
    let (cg, _) = build(&[
        (
            "a.py",
            "class Foo:\n    def m(self):\n        pass\n",
            Python,
        ),
        (
            "b.py",
            "class Foo:\n    def m(self):\n        pass\n",
            Python,
        ),
        ("run.py", "def run(x: Foo):\n    x.m()\n", Python),
    ]);
    let site = site_in(&cg, "run", "m");
    assert_eq!(site.receiver_type.as_deref(), Some("Foo"));
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|c| c.kind == ResolutionKind::TypedParam));
    assert!(r
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn js_new_constructor_and_bare_call_do_not_recover() {
    // P3: `m` must stay OVER the R6 fanout cap (4 owners: Foo/Other/Other2/
    // Other3) so this residue keeps testing what its name says — constructor-
    // local recovery does not engage for JS `new`/bare calls — rather than
    // the P3 candidate path a 2-owner pool would now hit instead. See
    // r6_candidate_test for the <=3-owner candidate case.
    use prism::languages::Language::JavaScript;
    let (cg, _) = build(&[(
        "svc.js",
        "class Foo { m() {} }\nclass Other { m() {} }\nclass Other2 { m() {} }\nclass Other3 { m() {} }\nfunction made() { const x = new Foo(); x.m(); }\nfunction factory() { const x = Foo(); x.m(); }\n",
        JavaScript,
    )]);
    let made = site_in(&cg, "made", "m");
    assert_eq!(made.receiver_type, None);
    assert!(!made.receiver_materialized);
    assert!(cg.resolve_call_site(&made).is_empty());

    let factory = site_in(&cg, "factory", "m");
    assert_eq!(factory.receiver_type, None);
    assert!(!factory.receiver_materialized);
    assert!(cg.resolve_call_site(&factory).is_empty());
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
fn go_embedded_concrete_method_keeps_existing_promotion() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Ping");
    let outcome = cg.resolve_call_site_full(&site);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.file, "main.go");
    assert_eq!(outcome.resolved[0].target.start_line, 3);
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_existing, 1);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_deferred, 0);
}

#[test]
fn go_embedded_transitive_concrete_method_keeps_existing_promotion() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype C struct{}\nfunc (c C) M() {}\ntype B struct{ C }\ntype A struct{ B }\nfunc run(a A) {\n\ta.M()\n}\n",
        Go,
    )]);
    let outcome = cg.resolve_call_site_full(&site_in(&cg, "run", "M"));
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.start_line, 3);
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_existing, 1);
}

#[test]
fn go_embedded_pointer_receiver_addressable_keeps_existing_promotion() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b *Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let outcome = cg.resolve_call_site_full(&site_in(&cg, "run", "Ping"));
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.start_line, 3);
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_existing, 1);
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
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype X struct{}\nfunc (x X) M() {}\ntype Y struct{}\nfunc (y Y) M() {}\ntype A struct {\n\tX\n\tY\n}\nfunc run(a A) {\n\ta.M()\n}\n",
        Go,
    )]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "run", "M"));
    assert!(
        out.resolved.is_empty(),
        "equal-depth M is ambiguous -> no promoted edge"
    );
    assert_eq!(out.drop, Some(DropReason::ConcreteReceiverNoSelector));
    assert_eq!(out.telemetry.go_concrete_receiver_no_selector_drop, 1);
    assert_eq!(out.telemetry.go_concrete_receiver_promoted_deferred, 0);
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

#[test]
fn method_arity_records_param_count_excluding_receiver_and_variadic() {
    use prism::call_graph::MethodArity;
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type H struct{}\nfunc (h H) Do(a int, b int) {}\n\
         type V struct{}\nfunc (v V) Do(xs ...int) {}\n",
        Go,
    )]);
    // Collect all arities for "Do".
    let do_arities: Vec<&MethodArity> = cg
        .method_arity
        .iter()
        .filter(|(f, _)| f.name == "Do")
        .map(|(_, a)| a)
        .collect();
    assert_eq!(do_arities.len(), 2, "two Do methods recorded");
    // H.Do: 2 params (a int, b int), not variadic, receiver excluded.
    let non_variadic: Vec<&&MethodArity> = do_arities.iter().filter(|a| !a.variadic).collect();
    assert_eq!(non_variadic.len(), 1, "exactly one non-variadic Do");
    assert_eq!(non_variadic[0].params, 2, "two params, receiver excluded");
    // V.Do: variadic, 1 param name.
    let variadic: Vec<&&MethodArity> = do_arities.iter().filter(|a| a.variadic).collect();
    assert_eq!(variadic.len(), 1, "exactly one variadic Do");
    assert_eq!(variadic[0].params, 1, "one variadic param name");
}

#[test]
fn callsite_records_go_argument_count_and_spread() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         func f(a int, b int, c int) {}\n\
         func variad(xs ...int) {}\n\
         func g() { f(1, 2, 3); s := []int{1,2}; variad(s...) }\n",
        Go,
    )]);
    let f = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "f")
        .expect("call f");
    assert_eq!(f.arg_count, Some(3));
    assert!(!f.arg_spread);
    let v = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "variad")
        .expect("call variad");
    assert!(v.arg_spread, "variad(s...) is a spread call");
}

// ---------------------------------------------------------------------------
// Arity-disambiguation of same-named interface-dispatch candidates (Task 3)
// ---------------------------------------------------------------------------

// The headline FP (caddy `MiddlewareHandler.ServeHTTP` shape): the recovered
// receiver interface `Handler` declares a 2-param `ServeHTTP` (impl: HandlerFunc),
// but a sibling site calls it with 3 args. Go satisfaction is signature-strict, so
// the `interface_impls[(Handler, ServeHTTP)]` set holds ONLY the 2-param HandlerFunc
// — but the 3-arg site still minted it pre-fix (name-keyed, arity-ignored). The
// filter must DROP the 2-param candidate at the 3-arg site (confident exact
// mismatch) while KEEPING it at the 2-arg site.
#[test]
fn interface_dispatch_filters_candidates_by_call_arity() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Handler interface { ServeHTTP(w int, r int) }\n\
         type HandlerFunc struct{}\nfunc (h HandlerFunc) ServeHTTP(w int, r int) {}\n\
         func use() { _ = HandlerFunc{} }\n\
         func twoarg(x Handler) { x.ServeHTTP(1, 2) }\n\
         func threearg(x Handler) { x.ServeHTTP(1, 2, 3) }\n",
        Go,
    )]);
    // Sanity: the impl set is the single 2-param HandlerFunc (satisfaction excludes
    // any other arity), and the lookup is wired so the mint fires.
    let pre = cg
        .interface_impls
        .get(&("Handler".to_string(), "ServeHTTP".to_string()))
        .expect("interface_impls has (Handler, ServeHTTP)");
    assert_eq!(
        pre.len(),
        1,
        "only HandlerFunc.ServeHTTP (2 params) satisfies"
    );

    // 2-arg site: arity matches → candidate KEPT (mint fires, owner HandlerFunc).
    let two = site_in(&cg, "twoarg", "ServeHTTP");
    assert_eq!(two.arg_count, Some(2));
    let r2 = cg.resolve_call_site(&two);
    let owners2: std::collections::BTreeSet<&str> = r2
        .iter()
        .map(|c| cg.method_owners.get(c.target).map(|s| s.as_str()).unwrap())
        .collect();
    assert_eq!(
        owners2,
        std::collections::BTreeSet::from(["HandlerFunc"]),
        "2-arg call must KEEP the 2-param candidate"
    );

    // 3-arg site: confident exact mismatch (3 != 2) → candidate DROPPED; the set
    // empties, so the outcome is the no-impl drop, NOT a fall-through to another rung.
    let three = site_in(&cg, "threearg", "ServeHTTP");
    assert_eq!(three.arg_count, Some(3));
    assert!(!three.arg_spread);
    let full = cg.resolve_call_site_full(&three);
    assert!(
        full.resolved.is_empty(),
        "3-arg call must DROP the 2-param HandlerFunc candidate (was minted pre-fix); got {:?}",
        full.resolved
            .iter()
            .map(|c| cg.method_owners.get(c.target).cloned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        full.drop,
        Some(DropReason::ExternalReceiver),
        "emptied set takes the existing no-impl drop path"
    );
}

// Manifest-level proof (the resolver test does NOT cover the oracle path: the
// dispatch oracle reads `interface_dispatch_manifest`, which consults
// `interface_impls` directly). The SAME shared filter must run here so `fanout`
// reflects the arity-filtered set: the 3-arg site empties to fanout 0 /
// implementers [], while the sibling 2-arg site keeps HandlerFunc.
#[test]
fn interface_manifest_arity_filters_fanout() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Handler interface { ServeHTTP(w int, r int) }\n\
         type HandlerFunc struct{}\nfunc (h HandlerFunc) ServeHTTP(w int, r int) {}\n\
         func use() { _ = HandlerFunc{} }\n\
         func twoarg(x Handler) { x.ServeHTTP(1, 2) }\n\
         func threearg(x Handler) { x.ServeHTTP(1, 2, 3) }\n",
        Go,
    )]);
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let sites = m["sites"].as_array().expect("sites array");
    let by_line = |line: u64| {
        sites
            .iter()
            .find(|s| s["method"] == "ServeHTTP" && s["line"].as_u64() == Some(line))
            .unwrap_or_else(|| panic!("ServeHTTP site at line {line}"))
    };
    // 2-arg site keeps the matching candidate.
    let two = by_line(6); // `func twoarg ... x.ServeHTTP(1, 2)`
    assert_eq!(two["fanout"].as_u64(), Some(1));
    let two_impls: Vec<&str> = two["implementers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(two_impls, vec!["HandlerFunc"]);
    // 3-arg site: arity-filtered to empty -> fanout 0, implementers [].
    let three = by_line(7); // `func threearg ... x.ServeHTTP(1, 2, 3)`
    assert_eq!(
        three["fanout"].as_u64(),
        Some(0),
        "3-arg site's 2-param candidate is filtered out -> fanout 0"
    );
    assert_eq!(
        three["implementers"].as_array().unwrap().len(),
        0,
        "filtered-empty set mints no implementers"
    );
}

// Recall-guard (a): a VARIADIC candidate is never dropped, even when the call's
// fixed arg count differs from the declared param count. `T.Do(xs ...int)` has
// MethodArity{params:1, variadic:true}; `x.Do(1, 2)` (2 args) must KEEP T.
#[test]
fn interface_dispatch_keeps_variadic_candidate_against_mismatched_arity() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type V interface { Do(xs ...int) }\n\
         type T struct{}\nfunc (t T) Do(xs ...int) {}\n\
         func use() { _ = T{} }\n\
         func call(x V) { x.Do(1, 2) }\n",
        Go,
    )]);
    let site = site_in(&cg, "call", "Do");
    assert_eq!(site.arg_count, Some(2));
    assert!(!site.arg_spread);
    let r = cg.resolve_call_site(&site);
    let owners: std::collections::BTreeSet<&str> = r
        .iter()
        .map(|c| cg.method_owners.get(c.target).map(|s| s.as_str()).unwrap())
        .collect();
    assert_eq!(
        owners,
        std::collections::BTreeSet::from(["T"]),
        "a variadic candidate must never be dropped on an arity mismatch"
    );
    // Manifest mirrors: variadic candidate kept -> fanout 1.
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let s = m["sites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["method"] == "Do")
        .expect("Do site");
    assert_eq!(s["fanout"].as_u64(), Some(1));
}

// Recall-guard (b): a SPREAD call (`x.ServeHTTP(s...)`) drops nothing — the
// effective arg count is unknown, so every candidate is kept even when the
// recorded fixed arg count (1, the spread slice) differs from the params (2).
#[test]
fn interface_dispatch_spread_call_keeps_all_candidates() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Handler interface { ServeHTTP(w int, r int) }\n\
         type HandlerFunc struct{}\nfunc (h HandlerFunc) ServeHTTP(w int, r int) {}\n\
         func use() { _ = HandlerFunc{} }\n\
         func call(x Handler) { s := []int{1, 2, 3}; x.ServeHTTP(s...) }\n",
        Go,
    )]);
    let site = site_in(&cg, "call", "ServeHTTP");
    assert!(site.arg_spread, "x.ServeHTTP(s...) is a spread call");
    let r = cg.resolve_call_site(&site);
    let owners: std::collections::BTreeSet<&str> = r
        .iter()
        .map(|c| cg.method_owners.get(c.target).map(|s| s.as_str()).unwrap())
        .collect();
    assert_eq!(
        owners,
        std::collections::BTreeSet::from(["HandlerFunc"]),
        "a spread call must keep all candidates regardless of fixed arg count"
    );
    // Manifest mirrors: spread keeps the candidate -> fanout 1.
    let m = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let s = m["sites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["method"] == "ServeHTTP")
        .expect("ServeHTTP site");
    assert_eq!(s["fanout"].as_u64(), Some(1));
}

// Direct unit coverage of the conservative `arity_admits`/`arity_filter` contract
// (codex code-review do-now): every "unknown" keeps the candidate; only a known,
// non-spread, non-variadic, exact mismatch drops.
#[test]
fn arity_admits_and_filter_keep_on_every_unknown() {
    use prism::call_graph::{FunctionId, MethodArity};
    use prism::resolution::{arity_admits, arity_filter};
    use std::collections::BTreeMap;

    let two = MethodArity {
        params: 2,
        variadic: false,
    };
    let variad = MethodArity {
        params: 1,
        variadic: true,
    };

    // Drops ONLY on a confident exact mismatch:
    assert!(
        !arity_admits(Some(3), false, Some(&two)),
        "known 3 != 2, non-spread, non-variadic -> drop"
    );
    assert!(
        arity_admits(Some(2), false, Some(&two)),
        "exact match -> keep"
    );
    // Every unknown keeps:
    assert!(
        arity_admits(None, false, Some(&two)),
        "unknown call arity -> keep"
    );
    assert!(
        arity_admits(Some(3), false, None),
        "missing method arity -> keep"
    );
    assert!(
        arity_admits(Some(3), true, Some(&two)),
        "spread call -> keep"
    );
    assert!(
        arity_admits(Some(5), false, Some(&variad)),
        "variadic candidate -> keep"
    );

    // arity_filter: a candidate with NO recorded MethodArity is kept (unknown -> keep),
    // even against a confidently-known call arity.
    let fid = FunctionId {
        file: "x.go".into(),
        name: "M".into(),
        start_line: 1,
        end_line: 1,
    };
    let impls = vec![fid.clone()];
    let no_arity: BTreeMap<FunctionId, MethodArity> = BTreeMap::new();
    assert_eq!(
        arity_filter(&impls, Some(3), false, &no_arity).len(),
        1,
        "missing entry -> kept"
    );
    // And it DOES drop when the entry is present and mismatches.
    let with_arity = BTreeMap::from([(fid.clone(), two.clone())]);
    assert_eq!(
        arity_filter(&impls, Some(3), false, &with_arity).len(),
        0,
        "known 3 != 2 -> dropped"
    );
    assert_eq!(
        arity_filter(&impls, Some(2), false, &with_arity).len(),
        1,
        "known 2 == 2 -> kept"
    );
}

#[test]
fn go_unqualified_freefn_resolves_same_package_not_cross_package() {
    use prism::languages::Language::Go;
    // Two packages (= directories) each define a free fn `NewThing`. An
    // unqualified call in package `a`, in a *different file* than the def (so the
    // same-FILE rung R4 misses), must resolve to `a`'s NewThing ONLY — Go
    // semantics: an unqualified call resolves within its own package, never
    // cross-package (a real cross-package call is qualified `b.NewThing()`).
    // Pre-fix the repo-wide free-fn rung (R5) over-attributes to both a's and b's
    // as a demoted FreeMulti; R4.5 must narrow to the same-package def.
    let (cg, _) = build(&[
        (
            "a/def.go",
            "package a\nfunc NewThing() int { return 1 }\n",
            Go,
        ),
        (
            "a/use.go",
            "package a\nfunc useNew() int { return NewThing() }\n",
            Go,
        ),
        (
            "b/def.go",
            "package b\nfunc NewThing() int { return 2 }\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "useNew", "NewThing");
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(
        resolved.len(),
        1,
        "exactly the same-package target, not cross-package FreeMulti: {resolved:?}"
    );
    assert_eq!(resolved[0].target.file, "a/def.go");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(resolved[0].kind, ResolutionKind::SamePackage);
}

#[test]
fn go_same_package_freefn_multi_def_demotes_not_exact() {
    use prism::languages::Language::Go;
    // A directory holding two same-name free funcs — e.g. mutually-exclusive
    // build-tag files (or a black-box `_test` package) the build-agnostic
    // whole-text scan can't separate — must DEMOTE rather than over-claim Exact.
    // R4.5 still prefers the same dir over R5's repo-wide set, but the targets
    // are NameOnly because we can't pick one without package-clause/build context.
    let (cg, _) = build(&[
        ("a/x.go", "package a\nfunc F() int { return 1 }\n", Go),
        (
            "a/y.go",
            "//go:build other\npackage a\nfunc F() int { return 2 }\n",
            Go,
        ),
        (
            "a/use.go",
            "package a\nfunc useF() int { return F() }\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "useF", "F");
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 2, "both same-dir defs kept: {resolved:?}");
    assert!(resolved
        .iter()
        .all(|r| r.confidence == ResolutionConfidence::NameOnly));
    assert!(resolved
        .iter()
        .all(|r| r.kind == ResolutionKind::SamePackage));
}

#[test]
fn go_package_clause_partition_exact_for_whitebox_test_caller() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/common_test.go", "package a\nfunc withLogger() {}\n", Go),
        (
            "a/blackbox_test.go",
            "package a_test\nfunc withLogger() {}\n",
            Go,
        ),
        (
            "a/global_test.go",
            "package a\nfunc TestIt() { withLogger() }\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "TestIt", "withLogger");
    let out = cg.resolve_call_site_full(&site);
    assert_eq!(
        out.resolved.len(),
        1,
        "partition should leave one target: {out:?}"
    );
    assert_eq!(out.resolved[0].target.file, "a/common_test.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SamePackage);
    assert_eq!(out.telemetry.go_pkg_clause_partition_exact, 1);
}

#[test]
fn go_package_clause_only_candidates_drop_not_free_single() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "a/blackbox_test.go",
            "package a_test\nfunc helper() {}\n",
            Go,
        ),
        (
            "a/use_test.go",
            "package a\nfunc TestIt() { helper() }\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "TestIt", "helper");
    let out = cg.resolve_call_site_full(&site);
    assert_eq!(out.drop, Some(DropReason::GoSamePkgAllFiltered));
    assert!(
        out.resolved.is_empty(),
        "must not fall through to FreeSingle: {out:?}"
    );
    assert_eq!(out.telemetry.go_same_pkg_all_filtered_drop, 1);
}

#[test]
fn go_filename_suffix_partition_exact_for_suffixed_caller() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/lock_linux.go", "package a\nfunc TryLockFile() {}\n", Go),
        (
            "a/lock_windows.go",
            "package a\nfunc TryLockFile() {}\n",
            Go,
        ),
        (
            "a/use_linux.go",
            "package a\nfunc use() { TryLockFile() }\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "use", "TryLockFile");
    let out = cg.resolve_call_site_full(&site);
    assert_eq!(
        out.resolved.len(),
        1,
        "linux caller should see linux target: {out:?}"
    );
    assert_eq!(out.resolved[0].target.file, "a/lock_linux.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.telemetry.go_build_partition_exact, 1);
}

#[test]
fn go_build_expr_complement_partition_exact() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "a/fast.go",
            "//go:build fast\n\npackage a\nfunc hash() {}\n",
            Go,
        ),
        (
            "a/slow.go",
            "//go:build !fast\n\npackage a\nfunc hash() {}\n",
            Go,
        ),
        (
            "a/use.go",
            "//go:build fast\n\npackage a\nfunc use() { hash() }\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "use", "hash");
    let out = cg.resolve_call_site_full(&site);
    assert_eq!(
        out.resolved.len(),
        1,
        "fast caller should see fast target: {out:?}"
    );
    assert_eq!(out.resolved[0].target.file, "a/fast.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.telemetry.go_build_partition_exact, 1);
}

#[test]
fn go_sat_bound_uncertain_unique_survivor_demotes_not_exact() {
    use prism::languages::Language::Go;
    let uncertain = "//go:build !t0 && t1 && t2 && t3 && t4 && t5 && t6 && t7 && t8

package a
func f() {}
";
    let filtered = "//go:build !t0

package a
func f() {}
";
    let caller = "//go:build t0

package a
func use() { f() }
";
    let (cg, _) = build(&[
        ("a/uncertain.go", uncertain, Go),
        ("a/filtered.go", filtered, Go),
        ("a/use.go", caller, Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "f"));
    assert_eq!(out.resolved.len(), 1, "one fail-open survivor: {out:?}");
    assert_eq!(out.resolved[0].target.file, "a/uncertain.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(out.telemetry.go_build_partition_exact, 0);
    assert_eq!(out.telemetry.go_build_expr_unparsed, 1);
}

#[test]
fn go_negation_only_build_expr_demotes_unconstrained_caller() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "a/b.go",
            "//go:build !windows && !plan9 && !solaris\n\npackage a\nfunc f() {}\n",
            Go,
        ),
        ("a/c_windows.go", "package a\nfunc f() {}\n", Go),
        ("a/use.go", "package a\nfunc use() { f() }\n", Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "f"));
    assert_eq!(
        out.resolved.len(),
        2,
        "unconstrained caller sees both: {out:?}"
    );
    assert!(out
        .resolved
        .iter()
        .all(|r| r.confidence == ResolutionConfidence::NameOnly));
    assert_eq!(out.telemetry.go_build_partition_exact, 0);
}

#[test]
fn goarch_negation_only_build_expr_demotes_unconstrained_caller() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "a/b.go",
            "//go:build !amd64\n\npackage a\nfunc f() {}\n",
            Go,
        ),
        ("a/c_amd64.go", "package a\nfunc f() {}\n", Go),
        ("a/use.go", "package a\nfunc use() { f() }\n", Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "f"));
    assert_eq!(
        out.resolved.len(),
        2,
        "unconstrained caller sees both: {out:?}"
    );
    assert!(out
        .resolved
        .iter()
        .all(|r| r.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn go_syslist_suffix_zos_is_incompatible_with_linux() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/x_zos.go", "package a\nfunc f() {}\n", Go),
        ("a/x_linux.go", "package a\nfunc f() {}\n", Go),
        ("a/use_linux.go", "package a\nfunc use() { f() }\n", Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "f"));
    assert_eq!(
        out.resolved.len(),
        1,
        "linux caller should exclude zos: {out:?}"
    );
    assert_eq!(out.resolved[0].target.file, "a/x_linux.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn go_malformed_build_unique_survivor_is_demoted() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "a/bad.go",
            "//go:build linux &&\n\npackage a\nfunc f() {}\n",
            Go,
        ),
        ("a/other_windows.go", "package a\nfunc f() {}\n", Go),
        ("a/use_linux.go", "package a\nfunc use() { f() }\n", Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "f"));
    assert_eq!(
        out.resolved.len(),
        1,
        "malformed candidate stays visible: {out:?}"
    );
    assert_eq!(out.resolved[0].target.file, "a/bad.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(out.telemetry.go_build_partition_exact, 0);
    assert_eq!(out.telemetry.go_build_expr_unparsed, 0);
}

#[test]
fn go_empty_package_clause_candidate_survives_but_demotes() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/invalid.go", "func f() {}\n", Go),
        ("a/use.go", "package a\nfunc use() { f() }\n", Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "f"));
    assert_eq!(
        out.resolved.len(),
        1,
        "unknown package clause is compatible: {out:?}"
    );
    assert_eq!(out.resolved[0].target.file, "a/invalid.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::NameOnly);
}

#[test]
fn go_test_visibility_partition_counts_pkg_clause_exact() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/helper.go", "package a\nfunc helper() {}\n", Go),
        ("a/helper_test.go", "package a\nfunc helper() {}\n", Go),
        ("a/use.go", "package a\nfunc use() { helper() }\n", Go),
    ]);
    let out = cg.resolve_call_site_full(&site_in(&cg, "use", "helper"));
    assert_eq!(out.resolved.len(), 1, "{out:?}");
    assert_eq!(out.resolved[0].target.file, "a/helper.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.telemetry.go_pkg_clause_partition_exact, 1);
    assert_eq!(out.telemetry.go_build_partition_exact, 0);
}

#[test]
fn go_return_typed_fact_filters_by_build_profile() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/t_linux.go", "package a\ntype LinuxT struct{}\nfunc (t *LinuxT) M() {}\nfunc newT() *LinuxT { return &LinuxT{} }\n", Go),
        ("a/t_windows.go", "package a\ntype WindowsT struct{}\nfunc (t *WindowsT) M() {}\nfunc newT() *WindowsT { return &WindowsT{} }\n", Go),
        ("a/use_linux.go", "package a\nfunc runLinux() { x := newT(); x.M() }\n", Go),
        ("a/use.go", "package a\nfunc runAny() { x := newT(); x.M() }\n", Go),
    ]);
    let linux_site = site_in(&cg, "runLinux", "M");
    assert_eq!(linux_site.receiver_type.as_deref(), Some("LinuxT"));
    let any_site = site_in(&cg, "runAny", "M");
    assert_eq!(
        any_site.receiver_type, None,
        "unsuffixed caller is compatible with both facts"
    );
}

#[test]
fn go_return_typed_fact_filters_by_package_clause() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        (
            "a/t_test.go",
            "package a_test\ntype T struct{}\nfunc newT() *T { return &T{} }\n",
            Go,
        ),
        (
            "a/use_test.go",
            "package a\nfunc TestIt() { x := newT(); x.M() }\n",
            Go,
        ),
        (
            "a/method.go",
            "package a\ntype T struct{}\nfunc (t *T) M() {}\n",
            Go,
        ),
    ]);
    let site = site_in(&cg, "TestIt", "M");
    assert_eq!(
        site.receiver_type, None,
        "foo caller must not consume foo_test newT fact"
    );
}

#[test]
fn go_return_typed_fact_negation_only_demotes_to_ambiguous() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/t_unix.go", "//go:build !windows && !plan9 && !solaris\n\npackage a\ntype UnixT struct{}\nfunc (t *UnixT) M() {}\nfunc newT() *UnixT { return &UnixT{} }\n", Go),
        ("a/t_windows.go", "package a\ntype WindowsT struct{}\nfunc (t *WindowsT) M() {}\nfunc newT() *WindowsT { return &WindowsT{} }\n", Go),
        ("a/use.go", "package a\nfunc run() { x := newT(); x.M() }\n", Go),
    ]);
    assert_eq!(site_in(&cg, "run", "M").receiver_type, None);
}

#[test]
fn go_return_typed_fact_sat_bound_uncertain_survivor_bails() {
    use prism::languages::Language::Go;
    let uncertain = "//go:build !t0 && t1 && t2 && t3 && t4 && t5 && t6 && t7 && t8

package a
type BadT struct{}
func (t *BadT) M() {}
func newT() *BadT { return &BadT{} }
";
    let filtered = "//go:build !t0

package a
type OtherT struct{}
func (t *OtherT) M() {}
func newT() *OtherT { return &OtherT{} }
";
    let caller = "//go:build t0

package a
func run() { x := newT(); x.M() }
";
    let (cg, _) = build(&[
        ("a/t_uncertain.go", uncertain, Go),
        ("a/t_filtered.go", filtered, Go),
        ("a/use.go", caller, Go),
    ]);
    assert_eq!(site_in(&cg, "run", "M").receiver_type, None);
}

#[test]
fn go_return_typed_fact_unparsed_survivor_bails() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("a/t_bad.go", "//go:build linux &&\n\npackage a\ntype BadT struct{}\nfunc (t *BadT) M() {}\nfunc newT() *BadT { return &BadT{} }\n", Go),
        ("a/t_windows.go", "package a\ntype WindowsT struct{}\nfunc (t *WindowsT) M() {}\nfunc newT() *WindowsT { return &WindowsT{} }\n", Go),
        ("a/use_linux.go", "package a\nfunc run() { x := newT(); x.M() }\n", Go),
    ]);
    assert_eq!(site_in(&cg, "run", "M").receiver_type, None);
}

#[test]
fn go_build_profile_incremental_parity_edit_and_revert() {
    use prism::cpg::CodePropertyGraph;
    use prism::data_flow::DataFlowGraph;
    use prism::languages::Language::Go;
    use std::collections::{BTreeMap, BTreeSet};

    let parse_map = |defs: &[(&str, &str)]| -> BTreeMap<String, prism::ast::ParsedFile> {
        defs.iter()
            .map(|(p, src)| {
                (
                    p.to_string(),
                    prism::ast::ParsedFile::parse(p, src, Go).unwrap(),
                )
            })
            .collect()
    };
    let linux_def = "package a\ntype LinuxT struct{}\nfunc (t *LinuxT) M() {}\nfunc newT() *LinuxT { return &LinuxT{} }\n";
    let linux_def_windows_tag = "//go:build windows\n\npackage a\ntype LinuxT struct{}\nfunc (t *LinuxT) M() {}\nfunc newT() *LinuxT { return &LinuxT{} }\n";
    let windows_def = "package a\ntype WindowsT struct{}\nfunc (t *WindowsT) M() {}\nfunc newT() *WindowsT { return &WindowsT{} }\n";
    let use_linux = "package a\nfunc run() { x := newT(); x.M() }\n";

    let v1 = parse_map(&[
        ("a/t_linux.go", linux_def),
        ("a/t_windows.go", windows_def),
        ("a/use_linux.go", use_linux),
    ]);
    let full_v1 = CallGraph::build(&v1);
    assert_eq!(
        site_in(&full_v1, "run", "M").receiver_type.as_deref(),
        Some("LinuxT")
    );

    let v2 = parse_map(&[
        ("a/t_linux.go", linux_def_windows_tag),
        ("a/t_windows.go", windows_def),
        ("a/use_linux.go", use_linux),
    ]);
    let changed: BTreeSet<String> = ["a/t_linux.go".to_string()].into_iter().collect();
    let cpg_v2 = CodePropertyGraph::build_incremental(
        full_v1,
        DataFlowGraph::build(&v2),
        &changed,
        &v2,
        None,
    );
    let full_v2 = CallGraph::build(&v2);
    assert_eq!(
        site_in(&cpg_v2.call_graph, "run", "M").receiver_type,
        site_in(&full_v2, "run", "M").receiver_type
    );
    assert_eq!(site_in(&full_v2, "run", "M").receiver_type, None);

    let v3 = v1;
    let cpg_v3 = CodePropertyGraph::build_incremental(
        cpg_v2.call_graph,
        DataFlowGraph::build(&v3),
        &changed,
        &v3,
        None,
    );
    let full_v3 = CallGraph::build(&v3);
    assert_eq!(
        site_in(&cpg_v3.call_graph, "run", "M").receiver_type,
        site_in(&full_v3, "run", "M").receiver_type
    );
    assert_eq!(
        site_in(&full_v3, "run", "M").receiver_type.as_deref(),
        Some("LinuxT")
    );
}

#[test]
fn go_bare_value_ref_filters_and_counts_same_pkg_ambiguity() {
    use prism::languages::Language::Go;
    use prism::resolution::resolve_go_bare_value_ref;
    let (cg, _) = build(&[
        ("a/cb_test.go", "package a\nfunc cb() {}\n", Go),
        ("a/cb_black_test.go", "package a_test\nfunc cb() {}\n", Go),
        ("a/use_test.go", "package a\nfunc TestIt() { _ = cb }\n", Go),
    ]);
    let mut ambiguous = 0usize;
    let target = resolve_go_bare_value_ref(
        &cg.functions,
        &cg.method_owners,
        &cg.go_file_profiles,
        &mut ambiguous,
        "a/use_test.go",
        "cb",
    )
    .expect("package profile should rescue one value target");
    assert_eq!(target.file, "a/cb_test.go");
    assert_eq!(ambiguous, 1);
}

#[test]
fn go_bare_value_ref_negation_only_remains_ambiguous() {
    use prism::languages::Language::Go;
    use prism::resolution::resolve_go_bare_value_ref;
    let (cg, _) = build(&[
        (
            "a/b.go",
            "//go:build !windows && !plan9 && !solaris\n\npackage a\nfunc cb() {}\n",
            Go,
        ),
        ("a/c_windows.go", "package a\nfunc cb() {}\n", Go),
        ("a/use.go", "package a\nfunc use() { _ = cb }\n", Go),
    ]);
    let mut ambiguous = 0usize;
    let target = resolve_go_bare_value_ref(
        &cg.functions,
        &cg.method_owners,
        &cg.go_file_profiles,
        &mut ambiguous,
        "a/use.go",
        "cb",
    );
    assert!(target.is_none());
    assert_eq!(ambiguous, 1);
}

#[test]
fn go_bare_value_ref_sat_bound_uncertain_unique_survivor_returns_none() {
    use prism::languages::Language::Go;
    use prism::resolution::resolve_go_bare_value_ref;
    let uncertain = "//go:build !t0 && t1 && t2 && t3 && t4 && t5 && t6 && t7 && t8

package a
func cb() {}
";
    let filtered = "//go:build !t0

package a
func cb() {}
";
    let caller = "//go:build t0

package a
func use() { _ = cb }
";
    let (cg, _) = build(&[
        ("a/uncertain.go", uncertain, Go),
        ("a/filtered.go", filtered, Go),
        ("a/use.go", caller, Go),
    ]);
    let mut ambiguous = 0usize;
    let target = resolve_go_bare_value_ref(
        &cg.functions,
        &cg.method_owners,
        &cg.go_file_profiles,
        &mut ambiguous,
        "a/use.go",
        "cb",
    );
    assert!(target.is_none());
    assert_eq!(ambiguous, 1);
}

#[test]
fn go_bare_value_ref_unparsed_unique_survivor_returns_none() {
    use prism::languages::Language::Go;
    use prism::resolution::resolve_go_bare_value_ref;
    let (cg, _) = build(&[
        (
            "a/bad.go",
            "//go:build linux &&\n\npackage a\nfunc cb() {}\n",
            Go,
        ),
        ("a/c_windows.go", "package a\nfunc cb() {}\n", Go),
        ("a/use_linux.go", "package a\nfunc use() { _ = cb }\n", Go),
    ]);
    let mut ambiguous = 0usize;
    let target = resolve_go_bare_value_ref(
        &cg.functions,
        &cg.method_owners,
        &cg.go_file_profiles,
        &mut ambiguous,
        "a/use_linux.go",
        "cb",
    );
    assert!(target.is_none());
    assert_eq!(ambiguous, 1);
}

#[test]
fn mixed_edition_workspace_recovers_intra_crate_collision() {
    use prism::repo_loader::load_repo;
    // A pure-2018+ MIXED-edition workspace driven end-to-end through the real loader:
    // crate `a` (2021) holds an intra-crate same-name owner collision (m1::Foo,
    // m2::Foo) and a `use`-imported call site that pins m1::Foo; crate `b` inherits
    // `{ workspace = true }` -> 2024, making the workspace mixed. Pre-fix:
    // edition_uniform == false (a:2021 + b mis-parsed 2015) -> disproof bails ->
    // keep-all (2 NameOnly). Post-fix: {2021,2024} anchoring-uniform -> disproof runs
    // -> single Exact (m1::Foo).
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("a/src")).unwrap();
    std::fs::create_dir_all(p.join("b/src")).unwrap();
    std::fs::write(
        p.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\n[workspace.package]\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nedition = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/src/lib.rs"),
        "mod m1;\nmod m2;\nuse crate::m1::Foo;\npub fn drive() {\n    Foo::m();\n}\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/src/m1.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/src/m2.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();
    std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
    let repo = load_repo(p).unwrap();
    let inputs = repo
        .scope_graph_inputs
        .as_ref()
        .expect("scope graph inputs");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, Some(inputs));
    assert_eq!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across m1::Foo and m2::Foo"
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None, "the recovered owner path must not drop");
    assert_eq!(
        out.resolved.len(),
        1,
        "a pure-2018+ mixed-edition workspace now recovers the collision to one Exact"
    );
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn cross_crate_use_collision_recovers_to_single_exact() {
    use prism::call_graph::CallGraph;
    use prism::repo_loader::load_repo;
    // A pure-2018+ workspace driven end-to-end through the real loader. Crate `a`
    // (2021) declares a PATH dep on crate `b` and pins a same-name `Foo` collision
    // with `use b_crate::Foo;` (b's crate is the in-source name `b_crate`). Both
    // crate `a` and crate `b` define a `Foo` with method `m`, so the bare owner key
    // ("Foo","m") collides. Pre-fix: the leading `b_crate` segment is Unresolved ->
    // the (A) disproof's pending re-resolve declines -> keep-all (2 NameOnly).
    // Post-fix: `b_crate` resolves via crate_deps_by_root to b's lib Root -> Foo to
    // b's Foo (one in-repo item) -> the disproof prunes to one Exact.
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("a/src")).unwrap();
    std::fs::create_dir_all(p.join("b/src")).unwrap();
    std::fs::write(
        p.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb_crate = { path = \"../b\" }\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nedition = \"2021\"\n",
    )
    .unwrap();
    // Crate a: import b's Foo and call Foo::m. Crate a ALSO defines its own Foo so
    // the bare ("Foo","m") owner key collides across the two crates.
    std::fs::write(
        p.join("a/src/lib.rs"),
        "use b_crate::Foo;\npub struct LocalFoo;\nimpl LocalFoo { pub fn m(&self) {} }\npub fn drive() {\n    Foo::m();\n}\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/src/lib.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();
    // A second crate `c` whose Foo::m collides on the bare owner key with b's, so
    // the ("Foo","m") key holds >=2 defs and the floor is NameOnly (not Exact)
    // until the disproof prunes.
    std::fs::create_dir_all(p.join("c/src")).unwrap();
    std::fs::write(
        p.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\n",
    )
    .unwrap();
    std::fs::write(
        p.join("c/Cargo.toml"),
        "[package]\nname = \"c\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("c/src/lib.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();

    let repo = load_repo(p).unwrap();
    let inputs = repo
        .scope_graph_inputs
        .as_ref()
        .expect("scope graph inputs");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, Some(inputs));
    assert!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len())
            .unwrap_or(0)
            >= 2,
        "the bare owner key must collide across b::Foo and c::Foo (NameOnly floor)"
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(
        out.drop, None,
        "the recovered cross-crate owner path must not drop"
    );
    assert_eq!(
        out.resolved.len(),
        1,
        "the cross-crate `use b_crate::Foo` now recovers the collision to one Exact"
    );
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].target.file, "b/src/lib.rs");
}
