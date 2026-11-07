use super::*;

// P15a-fix1: the full production build path (CallGraph passes + CpgContext
// registry) must perform exactly ONE plain (import-path-free) provider
// construction and ONE import-aware construction, with at most ONE full
// `GoTypeData` alive at any moment. RED on the pre-fix code, which built
// plain providers for promotion AND receiver rematerialization AND the
// registry (3 plain constructions; peak live 2 while dispatch's
// import-aware extraction coexisted with the retained shared one).
// P15a-fix2: generation-keyed drops — a provider built BEFORE the token
// but dropped DURING the measurement must not decrement its live count
// (RED on the pre-fix global ARMED design, which underflowed LIVE).
#[test]
fn p15a_fix2_pre_token_drop_does_not_underflow_live() {
    use crate::languages::Language::Go;
    use crate::type_providers::go::{test_counters::MeasurementToken, GoTypeProvider};

    let src = "package main\n\ntype S struct{ X int }\n";
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", src, Go).unwrap(),
    );

    // Built before any measurement — attributed to generation 0.
    let stale = GoTypeProvider::from_parsed_files(&files);

    let token = MeasurementToken::acquire();
    drop(stale);
    assert_eq!(
        token.live(),
        0,
        "a pre-token extraction's drop must not affect this measurement"
    );
}

#[test]
fn p15a_fix1_single_plain_provider_peak_live_one_through_full_context_path() {
    use crate::cpg::CpgContext;
    use crate::languages::Language::Go;
    use crate::type_providers::go::test_counters::MeasurementToken;

    let src = "package main\n\ntype S struct{ X int }\n\nfunc (s S) Get() int { return s.X }\n\nfunc main() { var s S; _ = s.Get() }\n";
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", src, Go).unwrap(),
    );

    let token = MeasurementToken::acquire();
    let ctx = CpgContext::build(&files, None);
    assert_eq!(
        token.plain_constructions(),
        1,
        "expected exactly 1 plain construction"
    );
    assert_eq!(
        token.import_aware_constructions(),
        1,
        "expected exactly 1 import-aware construction"
    );
    assert_eq!(
        token.peak_live(),
        1,
        "at most 1 full GoTypeData may be alive at a time"
    );
    // P15a-fix2: the provider was TRANSFERRED into the registry — the
    // CPG's CallGraph holds NO provider after context construction, so a
    // long-lived CPG never pins the full Go type data.
    assert!(
        ctx.cpg.call_graph.shared_plain_go_provider.is_none(),
        "CPG must not retain the shared plain Go provider after build"
    );
    drop(ctx);
    assert_eq!(token.live(), 0, "all extractions released");
}

/// P15a-fix3: `build_with_registry` uses a CALLER-SUPPLIED registry, so
/// nothing consumes the plain Go provider stashed by the CPG build — the
/// stash must be dropped, or this (long-lived) context pins a SECOND full
/// Go dataset. RED on pre-fix code (stash was Some).
#[test]
fn p15a_fix3_build_with_registry_drops_stashed_go_provider() {
    use crate::cpg::CpgContext;
    use crate::languages::Language::Go;
    use crate::type_provider::TypeRegistry;

    let src = "package main\n\ntype S struct{ X int }\n\nfunc (s S) Get() int { return s.X }\n\nfunc main() { var s S; _ = s.Get() }\n";
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", src, Go).unwrap(),
    );

    let ctx = CpgContext::build_with_registry(&files, None, TypeRegistry::empty());
    assert!(
        ctx.cpg.call_graph.shared_plain_go_provider.is_none(),
        "build_with_registry must drop the CPG's stashed plain Go provider"
    );
}

/// P15a-fix4: EVERY CpgContext constructor must leave the CPG's stashed
/// plain Go provider as None — transferred into the registry or dropped —
/// so no constructor can pin a full Go dataset on a long-lived context.
/// Parameterized over all real constructors.
#[test]
fn p15a_fix4_all_constructors_leave_no_stashed_provider() {
    use crate::cpg::CpgContext;
    use crate::languages::Language::Go;
    use crate::type_provider::TypeRegistry;

    let src = "package main\n\ntype S struct{ X int }\n\nfunc (s S) Get() int { return s.X }\n\nfunc main() { var s S; _ = s.Get() }\n";
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", src, Go).unwrap(),
    );
    let fresh_cpg = || crate::cpg::CodePropertyGraph::build_enriched(&files, None);

    let cases: Vec<(&str, CpgContext)> = vec![
        ("build", CpgContext::build(&files, None)),
        (
            "build_with_scope_graph_inputs",
            CpgContext::build_with_scope_graph_inputs(&files, None, None),
        ),
        (
            "build_with_fresh_cpg",
            CpgContext::build_with_fresh_cpg(&files, fresh_cpg(), None),
        ),
        // Deserialized-cache-hit constructor with a FRESH CPG input: the
        // stash would otherwise survive alongside the fresh registry.
        (
            "build_with_cached_cpg(fresh cpg)",
            CpgContext::build_with_cached_cpg(&files, fresh_cpg(), None),
        ),
        ("without_cpg", CpgContext::without_cpg(&files, None)),
    ];
    for (name, ctx) in cases {
        assert!(
            ctx.cpg.call_graph.shared_plain_go_provider.is_none(),
            "{name} must not retain the shared plain Go provider"
        );
    }

    // Scoped FILTERED-SUBSET branch (terra final-confirm): a one-file repo
    // falls back to the full build, so exercise a 3-file repo whose diff
    // touches only one file — the filtered branch must also end None.
    {
        // Eight files, only a.go changed and nothing references it:
        // scope = {a.go} = 1/8 ≤ 50%, so the filtered-subset branch runs.
        let mut multi = BTreeMap::new();
        multi.insert(
            "a.go".to_string(),
            ParsedFile::parse(
                "a.go",
                "package main\n\ntype A struct{ X int }\n\nfunc (a A) Get() int { return a.X }\n",
                Go,
            )
            .unwrap(),
        );
        for i in 0..7 {
            let name = format!("b{i}.go");
            let body = format!(
                    "package main\n\ntype B{i} struct{{ Y int }}\n\nfunc (b B{i}) Put{i}() int {{ return b.Y }}\n"
                );
            multi.insert(name.clone(), ParsedFile::parse(&name, &body, Go).unwrap());
        }
        let scoped_subset = CpgContext::build_scoped(
            &multi,
            &crate::diff::DiffInput {
                files: vec![crate::diff::DiffInfo {
                    file_path: "a.go".to_string(),
                    modify_type: crate::diff::ModifyType::Modified,
                    diff_lines: BTreeSet::from([5]),
                }],
            },
            None,
        );
        assert!(
            scoped_subset.scope.is_some(),
            "3-file scoped fixture must take the filtered-subset branch"
        );
        assert!(
            scoped_subset
                .cpg
                .call_graph
                .shared_plain_go_provider
                .is_none(),
            "build_scoped (filtered subset) must not retain the shared plain Go provider"
        );
    }

    // build_scoped takes a DiffInput; a one-file repo falls back to the
    // full build — either way the invariant must hold.
    let scoped = CpgContext::build_scoped(
        &files,
        &crate::diff::DiffInput {
            files: vec![crate::diff::DiffInfo {
                file_path: "main.go".to_string(),
                modify_type: crate::diff::ModifyType::Modified,
                diff_lines: BTreeSet::from([1]),
            }],
        },
        None,
    );
    assert!(
        scoped.cpg.call_graph.shared_plain_go_provider.is_none(),
        "build_scoped must not retain the shared plain Go provider"
    );

    let reg = CpgContext::build_with_registry(&files, None, TypeRegistry::empty());
    assert!(
        reg.cpg.call_graph.shared_plain_go_provider.is_none(),
        "build_with_registry must not retain the shared plain Go provider"
    );
}

/// P15a-fix3/fix4: a FOREIGN thread constructing Go providers while a
/// measurement is live must contribute ZERO attributions to that
/// measurement. RED on the fix2 first-draft global CURRENT_GENERATION
/// (the foreign construction was counted), which flaked only under
/// full-suite parallelism. P15a-fix4: fully deterministic — the foreign
/// thread SIGNALS after its first construction (mpsc) and the assertions
/// run only after that signal, so the test never depends on a sleep.
#[test]
fn p15a_fix3_foreign_thread_constructions_not_attributed() {
    use crate::languages::Language::Go;
    use crate::type_providers::go::test_counters::MeasurementToken;
    use crate::type_providers::go::GoTypeProvider;
    use std::sync::Arc;

    let src = "package main\n\ntype S struct{ X int }\n";
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", src, Go).unwrap(),
    );

    let token = MeasurementToken::acquire();
    let (constructed_tx, constructed_rx) = std::sync::mpsc::channel::<()>();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let files = Arc::new(files);
    let handle = {
        let files = files.clone();
        let stop = stop.clone();
        // The measurement is LIVE for this whole thread's lifetime —
        // exactly the window in which the old global-generation design
        // misattributed.
        std::thread::spawn(move || loop {
            let _p = GoTypeProvider::from_parsed_files(&files);
            let _ = constructed_tx.send(());
            if stop.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        })
    };
    // Block until the foreign thread has PROVEN at least one construction
    // inside the live-measurement window (would have inflated the counts
    // under the pre-fix design).
    constructed_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("foreign thread did not complete a construction");
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    handle.join().unwrap();

    assert_eq!(
        token.plain_constructions(),
        0,
        "foreign-thread constructions must not be attributed to this measurement"
    );
    assert_eq!(token.live(), 0, "nor their drops");
}

#[test]
fn method_class_span_populated_for_python_methods() {
    use crate::languages::Language::Python;
    use std::collections::BTreeMap;

    let mut files = BTreeMap::new();
    files.insert(
        "a.py".to_string(),
        ParsedFile::parse(
            "a.py",
            "class C:\n    def f(self):\n        return 1\n",
            Python,
        )
        .unwrap(),
    );
    let cg = CallGraph::build(&files);
    let fid = cg
        .functions
        .get("f")
        .unwrap()
        .iter()
        .find(|f| f.file == "a.py")
        .unwrap();
    let span = cg.method_class_span.get(fid).expect("span recorded");
    assert_eq!(span.0, 0);
    assert!(span.1 > span.0);
}

#[test]
fn python_imported_class_proof_key_ignores_direct_method_body_changes() {
    use crate::languages::Language::Python;

    let build = |method_body: &str| {
        let mut files = BTreeMap::new();
        files.insert(
                "app.py".to_string(),
                ParsedFile::parse(
                    "app.py",
                    "from pkg.models import Client as ImportedClient\ndef run(client: ImportedClient):\n    client.send()\n",
                    Python,
                )
                .unwrap(),
            );
        files.insert(
            "pkg/models.py".to_string(),
            ParsedFile::parse(
                "pkg/models.py",
                &format!("class Client:\n    def send(self):\n        {method_body}\n"),
                Python,
            )
            .unwrap(),
        );
        CallGraph::build(&files)
    };
    let before = build("pass");
    let after = build("return 1");
    let proof_keys = |cg: &CallGraph| {
        python_imported_class_proof_keys(
            &cg.import_bindings,
            &cg.indexed_files,
            &cg.clean_class_spans,
            &cg.python_inert_initializers,
        )
    };

    let expected = BTreeSet::from([(
        "app.py".to_string(),
        "ImportedClient".to_string(),
        "pkg/models.py".to_string(),
        "Client".to_string(),
    )]);
    assert_eq!(proof_keys(&before), expected);
    assert_eq!(proof_keys(&after), expected);
}

#[test]
fn python_qualified_class_proof_key_ignores_direct_method_body_changes() {
    use crate::languages::Language::Python;

    let build = |method_body: &str| {
        let mut files = BTreeMap::new();
        files.insert(
            "app.py".to_string(),
            ParsedFile::parse(
                "app.py",
                "import pkg.models as models\ndef run(client: models.Client):\n    client.send()\n",
                Python,
            )
            .unwrap(),
        );
        files.insert(
            "pkg/models.py".to_string(),
            ParsedFile::parse(
                "pkg/models.py",
                &format!("class Client:\n    def send(self):\n        {method_body}\n"),
                Python,
            )
            .unwrap(),
        );
        CallGraph::build(&files)
    };
    let before = build("pass");
    let after = build("return 1");
    let proof_keys = |cg: &CallGraph| {
        python_imported_class_proof_keys(
            &cg.import_bindings,
            &cg.indexed_files,
            &cg.clean_class_spans,
            &cg.python_inert_initializers,
        )
    };

    let expected = BTreeSet::from([(
        "app.py".to_string(),
        "models.Client".to_string(),
        "pkg/models.py".to_string(),
        "Client".to_string(),
    )]);
    assert_eq!(proof_keys(&before), expected);
    assert_eq!(proof_keys(&after), expected);
}

#[test]
fn python_dotted_class_proof_key_ignores_direct_method_body_changes() {
    use crate::languages::Language::Python;

    let build = |method_body: &str| {
        let mut files = BTreeMap::new();
        files.insert(
            "app.py".to_string(),
            ParsedFile::parse(
                "app.py",
                "import pkg.models\ndef run(client: pkg.models.Client):\n    client.send()\n",
                Python,
            )
            .unwrap(),
        );
        files.insert(
            "pkg/models.py".to_string(),
            ParsedFile::parse(
                "pkg/models.py",
                &format!("class Client:\n    def send(self):\n        {method_body}\n"),
                Python,
            )
            .unwrap(),
        );
        CallGraph::build(&files)
    };
    let before = build("pass");
    let after = build("return 1");
    let proof_keys = |cg: &CallGraph| {
        python_imported_class_proof_keys(
            &cg.import_bindings,
            &cg.indexed_files,
            &cg.clean_class_spans,
            &cg.python_inert_initializers,
        )
    };

    let expected = BTreeSet::from([(
        "app.py".to_string(),
        "pkg.models.Client".to_string(),
        "pkg/models.py".to_string(),
        "Client".to_string(),
    )]);
    assert_eq!(proof_keys(&before), expected);
    assert_eq!(proof_keys(&after), expected);
}

#[test]
fn python_namespace_submodule_class_proof_key_tracks_parent_absence() {
    use crate::languages::Language::Python;

    let build = |method_body: &str, parent: Option<(&str, &str)>| {
        let mut files = BTreeMap::new();
        files.insert(
            "app.py".to_string(),
            ParsedFile::parse(
                "app.py",
                "from pkg import models\ndef run(client: models.Client):\n    client.send()\n",
                Python,
            )
            .unwrap(),
        );
        files.insert(
            "pkg/models.py".to_string(),
            ParsedFile::parse(
                "pkg/models.py",
                &format!("class Client:\n    def send(self):\n        {method_body}\n"),
                Python,
            )
            .unwrap(),
        );
        if let Some((path, source)) = parent {
            files.insert(
                path.to_string(),
                ParsedFile::parse(path, source, Python).unwrap(),
            );
        }
        CallGraph::build(&files)
    };
    let proof_keys = |cg: &CallGraph| {
        python_imported_class_proof_keys(
            &cg.import_bindings,
            &cg.indexed_files,
            &cg.clean_class_spans,
            &cg.python_inert_initializers,
        )
    };
    let expected = BTreeSet::from([(
        "app.py".to_string(),
        "models.Client".to_string(),
        "pkg/models.py".to_string(),
        "Client".to_string(),
    )]);

    assert_eq!(proof_keys(&build("pass", None)), expected);
    assert_eq!(proof_keys(&build("return 1", None)), expected);
    assert!(proof_keys(&build("pass", Some(("pkg.py", "value = 1\n")))).is_empty());
    assert!(proof_keys(&build("pass", Some(("pkg/__init__.py", "value = 1\n")))).is_empty());
}

fn build_rust_call_graph(source: &str) -> CallGraph {
    use crate::ast::ParsedFile;
    use crate::languages::Language::Rust;
    use crate::name_resolution::rust_populator::RustCrateConfig;

    let mut files = std::collections::BTreeMap::new();
    files.insert(
        "a.rs".into(),
        ParsedFile::parse("a.rs", source, Rust).unwrap(),
    );
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg = RustCrateConfig {
        crate_roots: files.keys().cloned().collect(),
        ..RustCrateConfig::default()
    };
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn build_complete(srcs: &[(&str, &str)]) -> CallGraph {
    use crate::ast::ParsedFile;
    use crate::languages::Language::Rust;
    use crate::name_resolution::rust_populator::RustCrateConfig;

    let mut files = std::collections::BTreeMap::new();
    for (path, source) in srcs {
        files.insert(
            path.to_string(),
            ParsedFile::parse(path, source, Rust).unwrap(),
        );
    }
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg = RustCrateConfig {
        crate_roots: files.keys().cloned().collect(),
        ..RustCrateConfig::default()
    };
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

#[test]
fn rust_receiver_updates_parallel_matches_serial_reference() {
    // Real-dir load -> populated scope_graph (in-memory ad-hoc Rust fixtures have no
    // crate root, so RustReceiverTyper::new would panic). src/navigation: 397 updates.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/navigation");
    let repo = crate::repo_loader::load_repo(&dir).unwrap();
    let ctx = crate::cpg::CpgContext::build(&repo.files, None);
    let cg = &ctx.cpg.call_graph;
    let par = cg.compute_rust_receiver_updates(&repo.files);
    let serial = cg.compute_rust_receiver_updates_reference(&repo.files);
    assert_eq!(par, serial, "receiver update sequence diverged");
    assert!(!par.is_empty(), "fixture produced no receiver updates");
}

#[test]
fn rematerialize_corpus_has_rust_receiver_updates() {
    // Floors the cache-byte gate's corpus (src/cpg) so it actually exercises remat.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
    let repo = crate::repo_loader::load_repo(&dir).unwrap();
    let ctx = crate::cpg::CpgContext::build(&repo.files, None);
    let n = ctx
        .cpg
        .call_graph
        .compute_rust_receiver_updates(&repo.files)
        .len();
    assert!(
        n > 50,
        "too few receiver updates to surface a remat divergence: {n}"
    ); // measured 1251
}

fn call_graph_type_scope(cg: &CallGraph, path: &str, name: &str) -> ScopeId {
    let graph = cg.scope_graph.as_ref().expect("scope graph");
    let file = graph.file_paths.get(path).copied().expect("file id");
    let scope = CallGraph::module_scope_for_byte(graph, file, 0).expect("module scope");
    match resolve_type_path_to_type_scope(graph, scope, name) {
        Some(TypeKey::InRepo(scope)) => scope,
        other => panic!("expected in-repo type scope for {name}, got {other:?}"),
    }
}

#[test]
fn field_and_return_indices_resolved_self_and_def_scope() {
    let cg = build_complete(&[(
        "a.rs",
        "pub struct Inner; impl Inner { fn poke(&self){} pub fn new()->Self{Inner} }\n\
             pub struct Outer { pub inner: Inner }\npub fn make()->Inner{Inner}\n",
    )]);
    let inner = call_graph_type_scope(&cg, "a.rs", "Inner");
    let outer = call_graph_type_scope(&cg, "a.rs", "Outer");
    assert_eq!(
        cg.field_types
            .get(&(outer, "inner".into()))
            .and_then(|v| v.first()),
        Some(&(None, TypeKey::InRepo(inner)))
    );
    let fid = |n: &str| {
        cg.functions
            .values()
            .flatten()
            .find(|f| f.name == n)
            .cloned()
            .unwrap()
    };
    assert_eq!(
        cg.return_types.get(&fid("make")).and_then(|v| v.first()),
        Some(&(None, TypeKey::InRepo(inner)))
    );
    assert_eq!(
        cg.return_types.get(&fid("new")).and_then(|v| v.first()),
        Some(&(None, TypeKey::InRepo(inner)))
    );
}

#[test]
fn field_and_return_indices_alias_cfg_and_omit_fallthrough_types() {
    let cg = build_complete(&[(
        "a.rs",
        "pub struct Inner;\ntype Alias = Inner;\n\
             pub struct Outer {\n\
                 #[cfg(feature = \"a\")]\n\
                 pub aliased: Alias,\n\
                 pub missing: Missing,\n\
                 pub text: String,\n\
             }\n\
             #[cfg(feature = \"a\")]\n\
             pub fn make_alias()->Alias{Inner}\n\
             pub fn make_text()->String{String::new()}\n",
    )]);
    let inner = call_graph_type_scope(&cg, "a.rs", "Inner");
    let outer = call_graph_type_scope(&cg, "a.rs", "Outer");
    assert_eq!(
        cg.field_types
            .get(&(outer, "aliased".into()))
            .and_then(|v| v.first()),
        Some(&(
            Some("#[cfg(feature = \"a\")]".to_string()),
            TypeKey::InRepo(inner)
        ))
    );
    assert!(!cg.field_types.contains_key(&(outer, "missing".into())));
    assert!(!cg.field_types.contains_key(&(outer, "text".into())));

    let fid = |n: &str| {
        cg.functions
            .values()
            .flatten()
            .find(|f| f.name == n)
            .cloned()
            .unwrap()
    };
    assert_eq!(
        cg.return_types
            .get(&fid("make_alias"))
            .and_then(|v| v.first()),
        Some(&(
            Some("#[cfg(feature = \"a\")]".to_string()),
            TypeKey::InRepo(inner)
        ))
    );
    assert!(!cg.return_types.contains_key(&fid("make_text")));
}

#[test]
fn cfg_gated_alias_does_not_collapse_to_one_unconditional_target() {
    let cg = build_complete(&[(
        "a.rs",
        "pub struct A;\npub struct B;\n\
             #[cfg(feat_a)]\n\
             pub type Alias = A;\n\
             #[cfg(feat_b)]\n\
             pub type Alias = B;\n\
             pub struct Holder { pub f: Alias }\n",
    )]);
    let holder = call_graph_type_scope(&cg, "a.rs", "Holder");
    let entries = cg
        .field_types
        .get(&(holder, "f".into()))
        .cloned()
        .unwrap_or_default();

    assert!(
        entries.iter().all(|(cfg, _)| cfg.is_some()) || entries.is_empty(),
        "cfg-gated alias collapsed to unconditional: {entries:?}"
    );
    assert!(entries.len() != 1 || entries[0].0.is_some());
}

#[test]
fn callsite_derived_receiver_fields_serde_default_and_excluded_from_cmp_key() {
    let cg = build_rust_call_graph(
        "struct Engine;\nimpl Engine { fn go(&self) {} }\nfn run(e: Engine) { e.go(); }\n",
    );
    let mut a = cg
        .calls
        .values()
        .flat_map(|sites| sites.iter())
        .find(|site| site.callee_name == "go")
        .cloned()
        .expect("go call site");
    let b = a.clone();
    a.receiver_outcome = Some(crate::resolution_identity::ReceiverOutcome {
        key: crate::resolution_identity::ReceiverTypeKey::Bare("Engine".to_string()),
        bare: "Engine".to_string(),
        recovery: crate::resolution::ReceiverRecovery::TypedParam,
    });
    a.receiver_materialized = true;
    a.receiver_newly_recovered = true;
    a.receiver_lexically_bound = true;
    a.origin = CallSiteOrigin::IndirectResolution;

    assert_eq!(a.cmp_key(), b.cmp_key());
    let mut legacy_json = serde_json::to_value(&b).unwrap();
    legacy_json
        .as_object_mut()
        .unwrap()
        .remove("receiver_outcome");
    legacy_json
        .as_object_mut()
        .unwrap()
        .remove("receiver_materialized");
    legacy_json
        .as_object_mut()
        .unwrap()
        .remove("receiver_newly_recovered");
    legacy_json
        .as_object_mut()
        .unwrap()
        .remove("receiver_lexically_bound");
    legacy_json.as_object_mut().unwrap().remove("origin");
    let defaulted: CallSite = serde_json::from_value(legacy_json).unwrap();
    assert_eq!(defaulted.receiver_outcome, None);
    assert!(!defaulted.receiver_materialized);
    assert!(!defaulted.receiver_newly_recovered);
    assert!(!defaulted.receiver_lexically_bound);
    assert_eq!(defaulted.origin, CallSiteOrigin::Source);

    let back: CallSite = bincode::deserialize(&bincode::serialize(&a).unwrap()).unwrap();
    assert_eq!(a, back);
}

/// P8: a macro-arg-minted call site carries `kind: Call` / `origin:
/// MacroArg` -- NOT `call_kind_at`'s ancestor-walk classification, which
/// would otherwise tag any span nested under a `macro_invocation` as
/// `MacroInvocation` (routing it to the wrong NS_MACRO namespace).
#[test]
fn macro_arg_call_site_carries_call_kind_and_macro_arg_origin() {
    let cg = build_rust_call_graph(
        "fn check(x: i32) -> bool { x > 0 }\nfn host() { assert!(check(1)); }\n",
    );
    let site = site_in(&cg, "host", "check");
    assert_eq!(site.kind, CallKind::Call);
    assert_eq!(site.origin, CallSiteOrigin::MacroArg);
    assert_eq!(site.arg_count, None);
    assert!(!site.arg_spread);
}

/// Companion pin: an ordinary (non-macro) call site's `kind`/`origin`
/// derivation is byte-for-byte unchanged by the P8 plumbing -- still
/// `Call`/`Source` via the pre-existing `call_kind_at` path.
#[test]
fn ordinary_call_site_kind_and_origin_unchanged() {
    let cg = build_rust_call_graph("fn check(x: i32) -> bool { x > 0 }\nfn host() { check(1); }\n");
    let site = site_in(&cg, "host", "check");
    assert_eq!(site.kind, CallKind::Call);
    assert_eq!(site.origin, CallSiteOrigin::Source);
}

/// A macro invocation that mints nothing (non-allowlisted, or allowlisted
/// but with no call-shaped args) still produces zero `CallSite`s for the
/// macro name itself -- `call_kind_at`'s classification of the macro's
/// OWN span is never exercised by a real CallSite because no site is ever
/// minted at that span (pre-existing PR-2 behavior, unchanged).
#[test]
fn nonallowlisted_macro_still_mints_nothing_for_its_own_name() {
    let cg = build_rust_call_graph("fn host() { my_undefined_macro!(check(1)); }\n");
    assert!(
        !cg.calls
            .values()
            .flat_map(|s| s.iter())
            .any(|s| s.callee_name == "my_undefined_macro" || s.callee_name == "check"),
        "non-allowlisted macro must not mint any call site"
    );
}

fn site_in(cg: &CallGraph, caller: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|site| site.callee_name == callee))
        .cloned()
        .unwrap_or_else(|| panic!("missing call site {caller}->{callee}"))
}

fn c_files(srcs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    srcs.iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, crate::languages::Language::C).unwrap(),
            )
        })
        .collect()
}

fn indirect_call_dump(cg: &CallGraph) -> Vec<String> {
    let mut out = Vec::new();
    for (caller, sites) in &cg.calls {
        for site in sites {
            if site.origin == CallSiteOrigin::IndirectResolution {
                out.push(format!(
                    "{}:{}:{}:{}:{}-{}",
                    caller.file,
                    caller.name,
                    site.callee_name,
                    site.line,
                    site.start_byte,
                    site.end_byte
                ));
            }
        }
    }
    out.sort();
    out
}

fn indirect_caller_dump(cg: &CallGraph) -> Vec<String> {
    let mut out = Vec::new();
    for (callee, sites) in &cg.callers {
        for site in sites {
            if site.origin == CallSiteOrigin::IndirectResolution {
                out.push(format!(
                    "{}:{}:{}:{}:{}-{}",
                    callee,
                    site.caller.file,
                    site.caller.name,
                    site.line,
                    site.start_byte,
                    site.end_byte
                ));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn clear_indirect_calls_removes_only_synthetic_entries_from_calls_and_callers() {
    let mut cg = CallGraph::build(&c_files(&[(
        "callbacks.c",
        "void target() {}\nvoid run() { target(); }\n",
    )]));
    let source = site_in(&cg, "run", "target");
    assert_eq!(source.origin, CallSiteOrigin::Source);

    let mut synthetic = source.clone();
    synthetic.origin = CallSiteOrigin::IndirectResolution;
    let original_call_count = cg.calls.get(&source.caller).unwrap().len();
    let inserted = cg
        .calls
        .get_mut(&source.caller)
        .unwrap()
        .insert(synthetic.clone());
    assert!(
        !inserted,
        "origin is excluded from CallSite identity so source wins in calls"
    );
    assert_eq!(
        cg.calls.get(&source.caller).unwrap().len(),
        original_call_count
    );
    cg.callers
        .entry("target".to_string())
        .or_default()
        .push(synthetic);

    assert!(cg
        .callers
        .get("target")
        .unwrap()
        .iter()
        .any(|site| site.origin == CallSiteOrigin::IndirectResolution));

    cg.clear_indirect_calls();

    let calls = cg.calls.get(&source.caller).unwrap();
    assert!(calls
        .iter()
        .any(|site| site.origin == CallSiteOrigin::Source));
    assert!(!calls
        .iter()
        .any(|site| site.origin == CallSiteOrigin::IndirectResolution));
    assert!(cg
        .callers
        .get("target")
        .unwrap()
        .iter()
        .all(|site| site.origin == CallSiteOrigin::Source));
}

#[test]
fn recompute_indirect_calls_is_idempotent_for_unsupported_c_callbacks() {
    let files_v1 = c_files(&[(
            "callbacks.c",
            "void old_handler() {}\nvoid execute(void (*cb)()) { cb(); }\nvoid outer() { execute(old_handler); }\n",
        )]);
    let files_v2 = c_files(&[(
            "callbacks.c",
            "void new_handler() {}\nvoid execute(void (*cb)()) { cb(); }\nvoid outer() { execute(new_handler); }\n",
        )]);

    let mut merged = CallGraph::build(&files_v1);
    let changed = BTreeSet::from(["callbacks.c".to_string()]);
    merged.remove_files(&changed);
    merged.merge(CallGraph::build_direct_subset(&files_v2, &changed));
    merged.recompute_indirect_calls(&files_v2);
    let once = indirect_call_dump(&merged);
    let callers_once = indirect_caller_dump(&merged);
    assert!(!once.iter().any(|entry| entry.contains(":new_handler:")));
    assert!(!once.iter().any(|entry| entry.contains(":old_handler:")));

    merged.recompute_indirect_calls(&files_v2);
    assert_eq!(once, indirect_call_dump(&merged));
    assert_eq!(callers_once, indirect_caller_dump(&merged));

    let full = CallGraph::build(&files_v2);
    assert_eq!(once, indirect_call_dump(&full));
    assert_eq!(callers_once, indirect_caller_dump(&full));
}

/// P8: at the CallGraph-internal mechanics level, `remove_files`
/// retaining `macro_arg_facts` by file and `merge` extending the map
/// (the same `js_ts_exports` pattern) is exactly correct in isolation --
/// this test pins that low-level plumbing. It does NOT by itself prove
/// the extracted facts stay valid: `macro_arg_facts`' CONTENT depends on
/// the repo-wide macro shadow set (F1 fix, codex re-review BLOCKER), so
/// the higher-level `build_incremental_with_scope_graph_inputs` caller
/// must additionally guard against that set drifting underneath an
/// incremental rebuild and fall back to a full rebuild when it does --
/// see `macro_shadow_intersection` and the
/// `incremental_from_previous_falls_back_to_full_rebuild_on_*_macro_shadow`
/// tests in `src/navigation/mod.rs`.
#[test]
fn macro_arg_facts_remove_files_retains_by_file_and_merge_extends() {
    let files = build_complete(&[
        (
            "a.rs",
            "fn check(x: i32) -> bool { x > 0 }\nfn host() { assert!(check(1)); }\n",
        ),
        ("b.rs", "fn other() {}\n"),
    ]);
    let mut files_map: std::collections::BTreeMap<String, ParsedFile> = Default::default();
    for (path, source) in [
        (
            "a.rs",
            "fn check(x: i32) -> bool { x > 0 }\nfn host() { assert!(check(1)); }\n",
        ),
        ("b.rs", "fn other() {}\n"),
    ] {
        files_map.insert(
            path.to_string(),
            ParsedFile::parse(path, source, crate::languages::Language::Rust).unwrap(),
        );
    }

    assert!(files.macro_arg_facts.contains_key("a.rs"));
    assert!(!files.macro_arg_facts.contains_key("b.rs"));
    assert_eq!(files.macro_arg_facts["a.rs"].calls_recorded, 1);

    let mut merged = files;
    let changed = BTreeSet::from(["a.rs".to_string()]);
    merged.remove_files(&changed);
    assert!(
        !merged.macro_arg_facts.contains_key("a.rs"),
        "remove_files must retain by file, dropping the removed file's facts"
    );

    merged.merge(CallGraph::build_direct_subset(&files_map, &changed));
    assert!(
        merged.macro_arg_facts.contains_key("a.rs"),
        "merge must extend the map from the rebuilt subset"
    );
    assert_eq!(merged.macro_arg_facts["a.rs"].calls_recorded, 1);
}

#[test]
fn rematerialize_sets_receiver_outcome_and_keeps_calls_callers_in_sync() {
    let cg = build_complete(&[(
        "a.rs",
        "struct Inner; impl Inner { fn poke(&self){} }\n\
             struct Outer { inner: Inner }\n\
             fn run(o: Outer) { let x = o.inner; x.poke(); }\n",
    )]);

    let site = site_in(&cg, "run", "poke");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    let outcome = site.receiver_outcome.expect("materialized outcome");
    assert!(matches!(
        outcome.key,
        crate::resolution_identity::ReceiverTypeKey::InRepo(_)
    ));
    assert_eq!(
        outcome.recovery,
        crate::resolution::ReceiverRecovery::FieldTyped
    );

    let from_callers = cg
        .callers
        .get("poke")
        .unwrap()
        .iter()
        .find(|site| site.caller.name == "run")
        .unwrap()
        .receiver_outcome
        .clone();
    assert_eq!(Some(outcome), from_callers);
}

#[test]
fn rematerialize_unresolved_receiver_falls_back_to_bare_outcome() {
    let cg = build_complete(&[("a.rs", "fn f(x: Unresolvable) { x.m(); }\n")]);
    let site = site_in(&cg, "f", "m");

    assert_eq!(site.receiver_type.as_deref(), Some("Unresolvable"));
    let outcome = site.receiver_outcome.expect("materialized outcome");
    assert_eq!(
        outcome.key,
        crate::resolution_identity::ReceiverTypeKey::Bare("Unresolvable".to_string())
    );
    assert_eq!(outcome.bare, "Unresolvable");
    assert_eq!(
        outcome.recovery,
        crate::resolution::ReceiverRecovery::TypedParam
    );
}

#[test]
fn rematerialize_generic_receiver_keeps_none_outcome() {
    let cg = build_complete(&[("a.rs", "fn g<U>(x: U) { x.m(); }\n")]);
    let site = site_in(&cg, "g", "m");

    assert_eq!(site.receiver_type.as_deref(), Some("U"));
    assert_eq!(site.receiver_outcome, None);
}

#[test]
fn rematerialize_typed_annotations_use_qualified_identity_not_same_name_decoy() {
    let cg = build_complete(&[(
        "a.rs",
        "mod a { pub struct Inner; impl Inner { pub fn m(&self){} } }\n\
             mod b { pub struct Inner; impl Inner { pub fn m(&self){} } }\n\
             fn f(x: crate::a::Inner) { x.m(); }\n\
             fn l() { let x: crate::a::Inner = crate::a::Inner; x.m(); }\n",
    )]);
    let a_inner = call_graph_type_scope(&cg, "a.rs", "crate::a::Inner");
    let b_inner = call_graph_type_scope(&cg, "a.rs", "crate::b::Inner");
    assert_ne!(a_inner, b_inner);

    let param_outcome = site_in(&cg, "f", "m")
        .receiver_outcome
        .expect("typed-param materialized outcome");
    assert_eq!(
        param_outcome.key,
        crate::resolution_identity::ReceiverTypeKey::InRepo(a_inner)
    );
    assert_ne!(
        param_outcome.key,
        crate::resolution_identity::ReceiverTypeKey::InRepo(b_inner)
    );
    assert_eq!(param_outcome.bare, "Inner");
    assert_eq!(
        param_outcome.recovery,
        crate::resolution::ReceiverRecovery::TypedParam
    );

    let let_outcome = site_in(&cg, "l", "m")
        .receiver_outcome
        .expect("typed-let materialized outcome");
    assert_eq!(
        let_outcome.key,
        crate::resolution_identity::ReceiverTypeKey::InRepo(a_inner)
    );
    assert_ne!(
        let_outcome.key,
        crate::resolution_identity::ReceiverTypeKey::InRepo(b_inner)
    );
    assert_eq!(let_outcome.bare, "Inner");
    assert_eq!(
        let_outcome.recovery,
        crate::resolution::ReceiverRecovery::TypedLet
    );
}

#[test]
fn rematerialize_preserves_call_iteration_order() {
    use crate::ast::ParsedFile;
    use crate::languages::Language::Rust;
    use crate::name_resolution::rust_populator::RustCrateConfig;

    let mut files = std::collections::BTreeMap::new();
    files.insert(
        "a.rs".into(),
        ParsedFile::parse(
            "a.rs",
            "struct Inner; impl Inner { fn poke(&self){} fn tap(&self){} }\n\
                 struct Outer { inner: Inner }\n\
                 fn run(o: Outer) { let x = o.inner; x.poke(); x.tap(); }\n",
            Rust,
        )
        .unwrap(),
    );
    fn order_key(
        site: &CallSite,
    ) -> (
        String,
        String,
        usize,
        CallKind,
        usize,
        usize,
        Option<String>,
        Option<String>,
    ) {
        (
            site.caller.name.clone(),
            site.callee_name.clone(),
            site.line,
            site.kind,
            site.start_byte,
            site.end_byte,
            site.qualifier.clone(),
            site.receiver_type.clone(),
        )
    }
    let only = files.keys().cloned().collect();
    let mut cg = CallGraph::build_direct_subset(&files, &only);
    let before: Vec<_> = cg
        .calls
        .iter()
        .find(|(fid, _)| fid.name == "run")
        .unwrap()
        .1
        .iter()
        .map(order_key)
        .collect();

    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg = RustCrateConfig {
        crate_roots: files.keys().cloned().collect(),
        ..RustCrateConfig::default()
    };
    cg.rebuild_scope_graph(&files, Some(&inputs));

    let sites: Vec<_> = cg
        .calls
        .iter()
        .find(|(fid, _)| fid.name == "run")
        .unwrap()
        .1
        .iter()
        .collect();
    let after: Vec<_> = sites.iter().map(|site| order_key(site)).collect();
    assert_eq!(before, after);

    let poke = sites
        .iter()
        .position(|site| site.callee_name == "poke")
        .unwrap();
    let tap = sites
        .iter()
        .position(|site| site.callee_name == "tap")
        .unwrap();
    assert!(poke < tap);
    assert!(sites[poke].receiver_outcome.is_some());
    assert!(sites[tap].receiver_outcome.is_some());
}

#[test]
fn level4_index_matches_legacy_oracle_over_full_universe() {
    // Corpus: quirk fixtures + prism's own top-level src/*.rs sources.
    let mut sources: Vec<(String, String)> = vec![(
            "quirks.c".into(),
            "s.cb = f; t->cb = g;\ns.cb = f; t->cbx = g;\nstatic struct ops o = { .open = do_open, .close = do_close };\na.cb == nope;\nb.cb = &handler;\n".into(),
        )];
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&src_dir).unwrap().flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            sources.push((
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            ));
        }
    }
    let known: std::collections::BTreeSet<String> = [
        "f", "g", "do_open", "do_close", "handler", "build", "new", "slice", "run",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    // Universe: ALL post-accessor identifiers across the corpus (index-independent)
    // + explicit negatives.
    let mut universe: std::collections::BTreeSet<String> =
        ["no_such_field", "cb", "cbx", "open", "close"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    for (_, src) in &sources {
        for line in src.lines() {
            universe.extend(crate::ast::candidate_fields_on_line(line.trim()));
        }
    }

    // Build the index exactly as CallGraph::build does.
    let inputs: Vec<_> = sources
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect();
    let index = super::level4::build_sources(&inputs, &known);
    assert_eq!(index, super::level4::reference(&inputs, &known));

    // Half 1 — EXCESS: every (field, file) the index claims must equal the legacy scan.
    for (field, by_file) in &index {
        for (path, targets) in by_file {
            let src = &sources.iter().find(|(p, _)| p == path).unwrap().1;
            let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
            let got: Vec<String> = targets.iter().cloned().collect();
            assert_eq!(got, legacy, "excess: field={field} file={path}");
        }
    }
    // Half 2 — MISSES: per universe field, legacy-scan ONLY files containing the
    // ->field / .field substring (the legacy has_field check hoisted to file level —
    // provably outcome-preserving), and assert the index agrees (absent key == empty).
    for field in &universe {
        let arrow = format!("->{field}");
        let dot = format!(".{field}");
        for (path, src) in &sources {
            if !(src.contains(&arrow) || src.contains(&dot)) {
                continue; // legacy provably returns empty; index has no entry by construction
            }
            let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
            let from_index: Vec<String> = index
                .get(field)
                .and_then(|m| m.get(path))
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(from_index, legacy, "miss: field={field} file={path}");
        }
    }
}
