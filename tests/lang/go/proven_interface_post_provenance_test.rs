use prism::call_graph::{CallGraph, CallSite};
use prism::resolution::{
    DropReason, GoOwnerIdentity, ReceiverRecovery, ResolutionConfidence, ResolutionKind,
};

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    super::test_support::build_go(sources)
}

fn func_field_collision_fixture(registered: bool) -> CallGraph {
    let app_types = if registered {
        "package app\n\
         type S struct{ Run func() }\n\
         func worker() {}\n\
         func New() S { return S{Run: worker} }\n"
    } else {
        "package app\n\
         type S struct{ Run func() }\n\
         func New() S { return S{} }\n"
    };
    build_go(&[
        ("app/types.go", app_types),
        (
            "app/use.go",
            "package app\n\
             func invoke() {\n\
               type S struct{}\n\
               c := New()\n\
               c.Run()\n\
             }\n",
        ),
        (
            "decoy/types.go",
            "package decoy\n\
             type S interface{ Run() }\n\
             type Wrong struct{}\n\
             func (Wrong) Run() {}\n\
             func retain() { var _ S = Wrong{} }\n",
        ),
    ])
}

fn site(cg: &CallGraph) -> &CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Run")
        .expect("invoke->Run site")
}

fn assert_source_reachable_legacy_seam(call: &CallSite) {
    assert_eq!(call.receiver_type.as_deref(), Some("S"), "{call:?}");
    assert_eq!(call.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&GoOwnerIdentity {
            package_dir: "app".to_string(),
            package_clause: "app".to_string(),
            name: "S".to_string(),
        }),
        "{call:?}"
    );
    assert!(call.receiver_local_type_shadowed, "{call:?}");
}

#[test]
fn go_proven_interface_owner_beats_bare_collision_with_func_field() {
    let cg = func_field_collision_fixture(true);
    let call = site(&cg);
    assert_source_reachable_legacy_seam(call);

    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    let resolved = &outcome.resolved[0];
    assert_eq!(resolved.target.file, "app/types.go", "{outcome:?}");
    assert_eq!(resolved.target.name, "worker", "{outcome:?}");
    assert_eq!(resolved.kind, ResolutionKind::FuncValueField);
    assert_eq!(resolved.confidence, ResolutionConfidence::NameOnly);
    assert!(
        outcome
            .resolved
            .iter()
            .all(|resolved| resolved.target.file != "decoy/types.go"),
        "bare collision minted decoy.Wrong.Run: {outcome:?}"
    );
}

#[test]
fn go_proven_interface_invalid_owner_terminal_drop_without_bare_retry() {
    let cg = func_field_collision_fixture(false);
    let call = site(&cg);
    assert_source_reachable_legacy_seam(call);

    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
}

#[test]
fn interface_manifest_go_proven_interface_parity() {
    let cg = func_field_collision_fixture(true);
    let call = site(&cg);
    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    let target = &outcome.resolved[0].target;

    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let row = manifest["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|row| row["file"] == "app/use.go" && row["method"] == "Run")
        .unwrap_or_else(|| panic!("missing invoke Run row: {manifest:#}"));
    assert_eq!(row["dispatch_route"], "func_value_field", "{manifest:#}");
    assert_eq!(row["fanout"], 1, "{manifest:#}");
    let identities = row["implementer_identities"]
        .as_array()
        .expect("implementer identities");
    assert_eq!(identities.len(), 1, "{manifest:#}");
    assert_eq!(identities[0]["file"], target.file, "{manifest:#}");
    assert_eq!(
        identities[0]["span"],
        serde_json::json!([target.start_line, target.end_line]),
        "{manifest:#}"
    );
}

#[test]
fn go_proven_interface_terminal_negative_matrix() {
    let cg = build_go(&[
        (
            "app/use.go",
            "package app\n\
             func invoke() {\n\
               type S struct{ Run func() }\n\
               var c S\n\
               c.Run()\n\
             }\n",
        ),
        (
            "decoy/types.go",
            "package decoy\n\
             type S interface{ Run() }\n\
             type Wrong struct{}\n\
             func (Wrong) Run() {}\n",
        ),
    ]);
    let call = site(&cg);
    assert!(call.receiver_owner_identity.is_none(), "{call:?}");
    assert!(call.receiver_materialized, "{call:?}");
    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        0
    );
}
