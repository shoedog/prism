use prism::call_graph::{CallGraph, CallSite};
use prism::resolution::{DropReason, GoOwnerIdentity, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeSet;

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    super::test_support::build_go(sources)
}

fn site<'a>(cg: &'a CallGraph, caller: &str) -> &'a CallSite {
    let matches: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|site| site.caller.name == caller && site.callee_name == "Next")
        .collect();
    assert_eq!(matches.len(), 1, "expected one {caller}->Next site");
    matches[0]
}

fn expected_owner(name: &str) -> GoOwnerIdentity {
    GoOwnerIdentity {
        package_dir: "p".to_string(),
        package_clause: "p".to_string(),
        name: name.to_string(),
    }
}

fn resolved_files(cg: &CallGraph, caller: &str) -> BTreeSet<String> {
    cg.resolve_call_site_full(site(cg, caller))
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

fn manifest_site<'a>(manifest: &'a serde_json::Value, caller_file: &str) -> &'a serde_json::Value {
    manifest["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["file"] == caller_file && site["method"] == "Next")
        .unwrap_or_else(|| panic!("missing manifest site {caller_file}: {manifest:#}"))
}

fn colliding_interfaces() -> CallGraph {
    build_go(&[
        (
            "p/types.go",
            "package p\n\
             type Iterator interface { Next() bool }\n\
             type PImpl struct{}\n\
             func (PImpl) Next() bool { return true }\n\
             func NewIterator() Iterator { return PImpl{} }\n\
             func retain() { _ = PImpl{} }\n",
        ),
        (
            "q/types.go",
            "package q\n\
             type Iterator interface { Next(int) bool }\n\
             type QImpl struct{}\n\
             func (QImpl) Next(int) bool { return true }\n\
             func retain() { _ = QImpl{} }\n",
        ),
        (
            "app/ondemand.go",
            "package app\n\
             import p \"example/p\"\n\
             func onDemand() { var it p.Iterator; it.Next() }\n",
        ),
        (
            "app/carried.go",
            "package app\n\
             import p \"example/p\"\n\
             func carried() { it := p.NewIterator(); it.Next() }\n",
        ),
        (
            "app/rebound.go",
            "package app\n\
             import p \"example/p\"\n\
             func reset(it p.Iterator) (p.Iterator, error) { return it, nil }\n\
             func rebound() {\n\
                 var it p.Iterator\n\
                 it, err := reset(it)\n\
                 _ = err\n\
                 it.Next()\n\
             }\n",
        ),
    ])
}

#[test]
fn eager_local_owner_bypasses_the_bare_name_collision_guard() {
    let cg = colliding_interfaces();
    let call = site(&cg, "onDemand");
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&expected_owner("Iterator")),
        "{call:?}"
    );

    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(
        resolved_files(&cg, "onDemand"),
        BTreeSet::from(["p/types.go".into()])
    );
    assert!(outcome.resolved.iter().all(|resolved| {
        resolved.confidence == ResolutionConfidence::Exact
            && resolved.kind == ResolutionKind::InterfaceDispatch
    }));
    assert_eq!(outcome.telemetry.go_r2_on_demand_name_collision_bail, 0);

    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let record = manifest_site(&manifest, "app/ondemand.go");
    assert_eq!(record["dispatch_route"], "interface_dispatch");
    assert_eq!(record["fanout"], 1);

    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_r2_on_demand_name_collision_bail"], 1);
}

#[test]
fn shadowed_on_demand_interface_collision_still_takes_the_terminal_bail() {
    let cg = colliding_interfaces();
    let call = site(&cg, "rebound");
    assert!(call.receiver_owner_identity.is_none(), "{call:?}");
    assert!(call.receiver_local_type_shadowed, "{call:?}");

    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(outcome.telemetry.go_r2_on_demand_name_collision_bail, 1);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        0
    );

    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let record = manifest_site(&manifest, "app/rebound.go");
    assert_eq!(record["dispatch_route"], "unproven_drop");
    assert_eq!(record["fanout"], 0);
}

#[test]
fn unique_on_demand_interface_name_keeps_s4_dispatch() {
    let cg = build_go(&[
        (
            "p/types.go",
            "package p\n\
             type UniqueIterator interface { Next() bool }\n\
             type Good struct{}\n\
             func (Good) Next() bool { return true }\n\
             func retain() { _ = Good{} }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import p \"example/p\"\n\
             func unique() { var it p.UniqueIterator; it.Next() }\n",
        ),
    ]);
    let call = site(&cg, "unique");
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&expected_owner("UniqueIterator")),
        "{call:?}"
    );

    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(
        resolved_files(&cg, "unique"),
        BTreeSet::from(["p/types.go".into()])
    );
    assert!(outcome.resolved.iter().all(|resolved| {
        resolved.confidence == ResolutionConfidence::Exact
            && resolved.kind == ResolutionKind::InterfaceDispatch
    }));
    assert_eq!(outcome.telemetry.go_r2_on_demand_name_collision_bail, 0);

    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let record = manifest_site(&manifest, "app/use.go");
    assert_eq!(record["dispatch_route"], "interface_dispatch");
    assert_eq!(record["fanout"], 1);
    assert_eq!(
        prism::navigation::queries::call_stats(&cg)["go_r2_on_demand_name_collision_bail"],
        0
    );
}

#[test]
fn carried_interface_identity_ignores_the_on_demand_collision_guard() {
    let cg = colliding_interfaces();
    let call = site(&cg, "carried");
    assert!(call.receiver_owner_identity.is_some(), "{call:?}");

    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert!(!outcome.resolved.is_empty(), "{outcome:?}");
    assert!(outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_eq!(outcome.telemetry.go_r2_on_demand_name_collision_bail, 0);

    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let record = manifest_site(&manifest, "app/carried.go");
    assert_eq!(record["dispatch_route"], "interface_dispatch");
    assert!(record["fanout"].as_u64().is_some_and(|fanout| fanout > 0));
}
