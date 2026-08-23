use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionKind};
use std::collections::BTreeMap;

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

fn fixture() -> CallGraph {
    build_go(&[
        (
            "decoy/closer.go",
            "package decoy\n\
             type Closer interface { Close() error }\n\
             type HTTPResourceClient struct{}\n\
             func (*HTTPResourceClient) Close() error { return nil }\n\
             func retain() { _ = &HTTPResourceClient{} }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import \"io\"\n\
             type Local interface { M() }\n\
             type Good struct{}\n\
             func (Good) M() {}\n\
             func retain() { _ = Good{} }\n\
             func newlyRecoveredExternal() {\n\
               _ = func(cl io.Closer) error { return cl.Close() }(nil)\n\
             }\n\
             func newlyRecoveredInRepo() {\n\
               func(value Local) { value.M() }(Good{})\n\
             }\n\
             func legacyExternal() {\n\
               _ = func() error {\n\
                 var cl io.Closer\n\
                 return cl.Close()\n\
               }()\n\
             }\n",
        ),
    ])
}

fn site<'a>(cg: &'a CallGraph, caller: &str, method: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .unwrap_or_else(|| panic!("missing {caller}->{method}"))
}

fn manifest_site(cg: &CallGraph, call: &CallSite) -> serde_json::Value {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|entry| {
            entry["file"] == call.caller.file
                && entry["start_byte"] == call.start_byte
                && entry["end_byte"] == call.end_byte
        })
        .unwrap_or_else(|| panic!("missing manifest site for {call:?}"))
        .clone()
}

#[test]
fn newly_recovered_qualified_external_receiver_drops_before_bare_ladder() {
    let cg = fixture();
    let call = site(&cg, "newlyRecoveredExternal", "Close");
    assert_eq!(call.receiver_type.as_deref(), Some("io.Closer"), "{call:?}");
    assert!(call.receiver_newly_recovered, "{call:?}");

    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(outcome.telemetry.go_external_receiver_new_recovery_drop, 1);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        0
    );

    let manifest = manifest_site(&cg, call);
    assert_eq!(manifest["dispatch_route"], "unproven_drop");
    assert_eq!(manifest["fanout"], 0);
    assert_eq!(manifest["implementer_identities"], serde_json::json!([]));
    assert_eq!(
        prism::navigation::queries::call_stats(&cg)["go_external_receiver_new_recovery_drop"],
        1
    );
}

#[test]
fn newly_recovered_in_repo_interface_keeps_interface_dispatch() {
    let cg = fixture();
    let call = site(&cg, "newlyRecoveredInRepo", "M");
    assert_eq!(call.receiver_type.as_deref(), Some("Local"), "{call:?}");
    assert!(call.receiver_newly_recovered, "{call:?}");

    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.file, "app/use.go");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::InterfaceDispatch);
    assert_eq!(outcome.telemetry.go_external_receiver_new_recovery_drop, 0);

    let manifest = manifest_site(&cg, call);
    assert_eq!(manifest["dispatch_route"], "interface_dispatch");
    assert_eq!(manifest["fanout"], 1);
}

#[test]
fn preexisting_recovery_external_receiver_keeps_legacy_bare_output() {
    let cg = fixture();
    let call = site(&cg, "legacyExternal", "Close");
    assert_eq!(call.receiver_type.as_deref(), Some("io.Closer"), "{call:?}");
    assert!(!call.receiver_newly_recovered, "{call:?}");

    let outcome = cg.resolve_call_site_full(call);
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.file, "decoy/closer.go");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::InterfaceDispatch);
    assert_eq!(outcome.telemetry.go_external_receiver_new_recovery_drop, 0);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        1
    );
    assert_eq!(outcome.telemetry.go_unproven_receiver_bare_fallback_hits, 1);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_edges,
        1
    );

    let manifest = manifest_site(&cg, call);
    assert_eq!(manifest["dispatch_route"], "interface_dispatch");
    assert_eq!(manifest["fanout"], 1);
}
