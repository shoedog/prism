use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn site<'a>(cg: &'a CallGraph, caller: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == "Next")
        .unwrap_or_else(|| panic!("missing {caller}->Next call site"))
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
             type Iterator interface { Next() bool }\n\
             type QImpl struct{}\n\
             func (QImpl) Next() bool { return true }\n\
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
    ])
}

#[test]
fn on_demand_r2_name_collision_bails_with_zero_fanout_and_telemetry() {
    let cg = colliding_interfaces();
    let call = site(&cg, "onDemand");
    assert!(call.receiver_owner_identity.is_none(), "{call:?}");

    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        0
    );
    assert_eq!(outcome.telemetry.go_r2_on_demand_name_collision_bail, 1);

    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    let record = manifest_site(&manifest, "app/ondemand.go");
    assert_eq!(record["dispatch_route"], "unproven_drop");
    assert_eq!(record["fanout"], 0);
    assert_eq!(record["implementer_identities"], serde_json::json!([]));

    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_r2_on_demand_name_collision_bail"], 1);
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
    assert!(call.receiver_owner_identity.is_none(), "{call:?}");

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
