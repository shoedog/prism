use crate::call_graph::{CallGraph, FunctionId};
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
pub use crate::navigation::call_stats::call_stats;
use crate::navigation::dfg_edge_endpoint::dfg_endpoint;
pub use crate::navigation::dfg_edge_endpoint::DfgEdgeEndpoint;
use crate::navigation::seed;
use crate::navigation::types::*;
use crate::navigation::NavigationSession;
use crate::resolution::go::concrete_receiver::GoConcreteReceiverRoute;
use crate::resolution::go::promoted_snapshot::{
    consult_promoted_snapshot, GoPromotedSnapshotConsult,
};
use crate::resolution::ResolutionConfidence;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, serde::Serialize)]
pub struct DfgEdgeRecord {
    pub from: DfgEdgeEndpoint,
    pub to: DfgEdgeEndpoint,
    pub confidence: &'static str,
    pub doubt: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kill_line: Option<u32>,
}

pub fn dfg_edge_dump(cpg: &CodePropertyGraph) -> Vec<DfgEdgeRecord> {
    crate::cpg::s1_trace::report_final(cpg);
    let mut records: Vec<_> = cpg
        .graph
        .edge_references()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(confidence) = *edge.weight() else {
                return None;
            };
            let from = dfg_endpoint(cpg.node(edge.source()))?;
            let to = dfg_endpoint(cpg.node(edge.target()))?;
            Some(DfgEdgeRecord {
                from,
                to,
                confidence: confidence.level(),
                doubt: confidence.doubt_spelling(),
                kill_line: confidence.kill_line(),
            })
        })
        .collect();
    records.sort_by(|left, right| (&left.from, &left.to).cmp(&(&right.from, &right.to)));
    records
}

/// Deterministic JSONL-ready custody records for every raw call-graph site.
pub fn call_site_dump(cg: &CallGraph) -> Vec<serde_json::Value> {
    call_site_dump_iter(cg).collect()
}

/// Build one custody record at a time, keeping large CLI dumps bounded.
pub fn call_site_dump_iter(cg: &CallGraph) -> impl Iterator<Item = serde_json::Value> + '_ {
    use crate::resolution::ResolutionConfidence;

    cg.calls
        .values()
        .flat_map(|sites| sites.iter())
        .map(move |site| {
            let outcome = cg.resolve_call_site_full(site);
            let resolved_targets: Vec<_> = outcome
                .resolved
                .iter()
                .map(|resolved| {
                    serde_json::json!({
                        "function_id": resolved.target,
                        "kind": resolved.kind.as_str(),
                        "confidence": match resolved.confidence {
                            ResolutionConfidence::Exact => "exact",
                            ResolutionConfidence::NameOnly => "name_only",
                        },
                    })
                })
                .collect();
            serde_json::json!({
                "record_kind": "call_site",
                "caller": site.caller,
                "source_span": {
                    "file": site.caller.file,
                    "line": site.line,
                    "start_byte": site.start_byte,
                    "end_byte": site.end_byte,
                },
                "callee_text": site.callee_name,
                "source_callee_text": site.effective_source_callee_name(),
                "call_kind": site.kind,
                "origin": site.origin,
                "exact_target": site.pre_resolved_target,
                "resolved_targets": resolved_targets,
                "drop": outcome.drop.map(|drop| format!("{drop:?}")),
            })
        })
        .chain(cg.go_level3_b1_telemetry.sites.iter().map(|site| {
            serde_json::json!({
                "record_kind": "go_level3_b1_candidate",
                "inbound_caller": site.inbound_caller,
                "inbound_span": {
                    "file": site.inbound_caller.file,
                    "line": site.inbound_line,
                    "start_byte": site.inbound_start_byte,
                    "end_byte": site.inbound_end_byte,
                },
                "hof_name": site.hof_name,
                "slot": site.slot,
                "argument": site.argument,
                "hof": site.hof,
                "exact_target": site.target,
                "callback_parameter": site.callback_parameter,
                "invocation_spans": site.invocation_spans,
                "decision": if site.accepted { "accepted" } else { "dropped" },
                "drop_reason": site.drop_reason,
            })
        }))
}

type InterfaceManifestTargetIdentity =
    (String, Option<String>, String, String, usize, usize, String);

fn interface_manifest_target_identities<'a>(
    cg: &CallGraph,
    targets: impl IntoIterator<Item = &'a FunctionId>,
) -> BTreeSet<InterfaceManifestTargetIdentity> {
    targets
        .into_iter()
        .map(|fid| {
            let name = cg
                .method_owners
                .get(fid)
                .cloned()
                .unwrap_or_else(|| crate::resolution::file_stem(&fid.file).to_string());
            let package_dir = crate::resolution::dir_of(&fid.file).to_string();
            let package_clause = cg
                .go_file_profiles
                .get(&fid.file)
                .map(|profile| profile.package_clause.clone());
            (
                package_dir,
                package_clause,
                name,
                fid.file.clone(),
                fid.start_line,
                fid.end_line,
                fid.name.clone(),
            )
        })
        .collect()
}

fn interface_manifest_target_identity_values(
    identities: &BTreeSet<InterfaceManifestTargetIdentity>,
) -> Vec<serde_json::Value> {
    identities
        .iter()
        .map(
            |(package_dir, package_clause, name, file, start_line, end_line, _)| {
                serde_json::json!({
                    "name": name,
                    "file": file,
                    "span": [start_line, end_line],
                    "package_dir": package_dir,
                    "package_clause": package_clause,
                })
            },
        )
        .collect()
}

fn p16_target_identity_values(
    identities: &BTreeSet<InterfaceManifestTargetIdentity>,
) -> Vec<serde_json::Value> {
    identities
        .iter()
        .map(
            |(
                package_dir,
                package_clause,
                owner_name,
                file,
                start_line,
                end_line,
                function_name,
            )| {
                serde_json::json!({
                    "owner_name": owner_name,
                    "function_name": function_name,
                    "file": file,
                    "span": [start_line, end_line],
                    "package_dir": package_dir,
                    "package_clause": package_clause,
                })
            },
        )
        .collect()
}

fn p16_terminal_outcome(outcome: &crate::resolution::ResolutionOutcome<'_>) -> serde_json::Value {
    if outcome.resolved.is_empty() {
        serde_json::json!({
            "kind": "drop",
            "drop_reason": outcome.drop.map(|reason| format!("{reason:?}")),
        })
    } else {
        let resolution_kinds: BTreeSet<_> = outcome
            .resolved
            .iter()
            .map(|resolved| resolved.kind.as_str())
            .collect();
        serde_json::json!({
            "kind": "hit",
            "resolution_kinds": resolution_kinds,
        })
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod p16_census_tests {
    use super::{
        interface_dispatch_manifest_inner, interface_manifest_target_identities,
        interface_manifest_target_identity_values, p16_target_identity_values,
    };
    use crate::ast::ParsedFile;
    use crate::call_graph::CallGraph;
    use crate::languages::Language;
    use crate::resolution::{DropReason, GoOwnerIdentity, GoProvenInterfaceRoute};
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

    #[test]
    fn p16_census_route_partition_wire_strings_are_exhaustive() {
        let routes = [
            GoProvenInterfaceRoute::LiveHit,
            GoProvenInterfaceRoute::FallbackHit,
            GoProvenInterfaceRoute::InvalidDrop,
            GoProvenInterfaceRoute::WalkDrop,
            GoProvenInterfaceRoute::ArityDrop,
        ];
        assert_eq!(
            routes.map(GoProvenInterfaceRoute::as_str),
            [
                "live_hit",
                "fallback_hit",
                "invalid_drop",
                "walk_drop",
                "arity_drop",
            ]
        );
    }

    #[test]
    fn p16_census_consult_reaches_each_route_partition() {
        let mut cg = build_go(&[(
            "pkg/types.go",
            "package p\ntype I interface{ M(int) }\ntype T struct{}\nfunc (T) M(int) {}\n",
        )]);
        let interface_owner = cg
            .go_interface_declarations
            .keys()
            .find(|owner| owner.name == "I")
            .expect("I owner")
            .clone();

        cg.go_interface_live_types.insert("T".to_string());
        assert_eq!(
            cg.go_proven_interface_consult(&interface_owner, "M", "pkg/types.go", Some(1), false,)
                .route,
            GoProvenInterfaceRoute::LiveHit
        );

        let mut fallback = cg.clone();
        fallback.go_interface_live_types.clear();
        assert_eq!(
            fallback
                .go_proven_interface_consult(&interface_owner, "M", "pkg/types.go", Some(1), false,)
                .route,
            GoProvenInterfaceRoute::FallbackHit
        );

        let invalid_owner = GoOwnerIdentity {
            package_dir: "pkg".to_string(),
            package_clause: "p".to_string(),
            name: "Missing".to_string(),
        };
        assert_eq!(
            cg.go_proven_interface_consult(&invalid_owner, "M", "pkg/types.go", Some(1), false,)
                .route,
            GoProvenInterfaceRoute::InvalidDrop
        );

        let mut no_methods = cg.clone();
        no_methods.go_method_declarations.clear();
        assert_eq!(
            no_methods
                .go_proven_interface_consult(&interface_owner, "M", "pkg/types.go", Some(1), false,)
                .route,
            GoProvenInterfaceRoute::WalkDrop
        );

        assert_eq!(
            cg.go_proven_interface_consult(&interface_owner, "M", "pkg/types.go", Some(0), false,)
                .route,
            GoProvenInterfaceRoute::ArityDrop
        );
    }

    #[test]
    fn p16_consult_admits_guarded_promoted_supply_and_rejects_signature_mismatch() {
        let mut cg = build_go(&[(
            "pkg/types.go",
            "package p\ntype I interface{ M(int) }\ntype Base struct{}\nfunc (Base) M(int) {}\ntype T struct{ Base }\n",
        )]);
        let interface_owner = cg
            .go_interface_declarations
            .keys()
            .find(|owner| owner.name == "I")
            .expect("I owner")
            .clone();
        cg.go_interface_live_types.insert("T".to_string());

        let promoted =
            cg.go_proven_interface_consult(&interface_owner, "M", "pkg/types.go", Some(1), false);
        assert_eq!(promoted.route, GoProvenInterfaceRoute::LiveHit);
        assert_eq!(promoted.all_satisfiers, 1);
        assert_eq!(promoted.live_satisfiers, 1);
        assert_eq!(promoted.final_satisfiers, 1);
        assert_eq!(promoted.promoted, Default::default());
        let target = promoted.selection.value.expect("promoted target")[0];
        assert_eq!(target.name, "M");
        assert_eq!(target.file, "pkg/types.go");

        let mismatch = build_go(&[(
            "pkg/types.go",
            "package p\ntype I interface{ M(int) }\ntype Base struct{}\nfunc (Base) M(string) {}\ntype T struct{ Base }\n",
        )]);
        let mismatch_owner = mismatch
            .go_interface_declarations
            .keys()
            .find(|owner| owner.name == "I")
            .expect("mismatch I owner");
        let rejected = mismatch.go_proven_interface_consult(
            mismatch_owner,
            "M",
            "pkg/types.go",
            Some(1),
            false,
        );
        assert_eq!(rejected.route, GoProvenInterfaceRoute::WalkDrop);
        assert_eq!(rejected.final_satisfiers, 0);
        assert_eq!(rejected.promoted.signature_drop, 1);
    }

    #[test]
    fn p16_consult_filters_uncertain_owner_before_live_selection() {
        let mut cg = build_go(&[
            ("pkg/interface.go", "package p\ntype I interface{ M() }\n"),
            (
                "pkg/good.go",
                "package p\ntype Good struct{}\nfunc (Good) M() {}\n",
            ),
            (
                "pkg/bad.go",
                "package p\ntype Bad struct{}\nfunc (Bad) M() {}\n",
            ),
        ]);
        let interface_owner = cg
            .go_interface_declarations
            .keys()
            .find(|owner| owner.name == "I")
            .expect("I owner")
            .clone();
        cg.go_file_profiles.remove("pkg/bad.go");
        cg.go_interface_live_types.insert("Bad".to_string());

        let consult = cg.go_proven_interface_consult(
            &interface_owner,
            "M",
            "pkg/interface.go",
            Some(0),
            false,
        );
        assert_eq!(consult.route, GoProvenInterfaceRoute::FallbackHit);
        assert_eq!(consult.all_satisfiers, 1);
        assert_eq!(consult.live_satisfiers, 0);
        assert_eq!(consult.final_satisfiers, 1);
        assert!(consult.selection.evidence.recovered);
        assert!(!consult.selection.evidence.conflict);
        assert!(!consult.selection.evidence.uncertain);
        let target = consult.selection.value.expect("fallback target")[0];
        assert_eq!(target.file, "pkg/good.go");
    }

    #[test]
    fn p16_census_target_identity_is_package_qualified() {
        let cg = build_go(&[(
            "pkg/types.go",
            "package p\ntype T struct{}\nfunc (T) M() {}\n",
        )]);
        let target = cg
            .method_owners
            .iter()
            .find_map(|(target, owner)| (owner == "T").then_some(target))
            .expect("T.M target");
        let identities = interface_manifest_target_identities(&cg, [target]);
        let values = interface_manifest_target_identity_values(&identities);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["name"], "T");
        assert_eq!(values[0]["file"], "pkg/types.go");
        assert_eq!(values[0]["package_dir"], "pkg");
        assert_eq!(values[0]["package_clause"], "p");
        assert_eq!(
            values[0]["span"],
            serde_json::json!([target.start_line, target.end_line])
        );
        let census_values = p16_target_identity_values(&identities);
        assert_eq!(census_values[0]["owner_name"], "T");
        assert_eq!(census_values[0]["function_name"], "M");
    }

    #[test]
    fn p16_production_retires_actual_resolver_legacy_arm() {
        let mut cg = build_go(&[
            (
                "p/main.go",
                "package p\ntype I struct{}\nfunc run(v I) { v.M() }\n",
            ),
            (
                "q/types.go",
                "package q\ntype I interface{ M() }\ntype Impl struct{}\nfunc (Impl) M() {}\nvar _ I = Impl{}\n",
            ),
        ]);
        let call = cg
            .calls
            .values()
            .flatten()
            .find(|site| site.caller.name == "run" && site.callee_name == "M")
            .expect("run->M site")
            .clone();
        let owner = call
            .receiver_owner_identity
            .as_ref()
            .expect("proven p.I owner")
            .clone();
        assert_eq!(owner.package_dir, "p");
        cg.go_declaration_kind_index.remove(&owner);

        let current = cg.resolve_call_site_full(&call);
        assert_eq!(
            current.telemetry.go_unproven_receiver_bare_fallback_sites,
            0
        );
        assert_eq!(current.telemetry.go_unproven_receiver_bare_fallback_hits, 0);
        assert_eq!(
            current.telemetry.go_unproven_receiver_bare_fallback_edges,
            0
        );
        assert!(current.resolved.is_empty(), "{current:?}");
        assert_eq!(current.drop, Some(DropReason::ExternalReceiver));

        let manifest = interface_dispatch_manifest_inner(&cg, true);
        let candidates = manifest["p16_candidates"]
            .as_array()
            .expect("candidate ledger");
        assert!(candidates.is_empty(), "{manifest:#}");
        let rows = manifest["sites"].as_array().expect("ordinary sites");
        let row = rows
            .iter()
            .find(|row| row["file"] == "p/main.go" && row["method"] == "M")
            .expect("run M row");
        assert_eq!(row["dispatch_route"], "unproven_drop", "{manifest:#}");
        assert_eq!(row["fanout"], 0, "{manifest:#}");
    }

    #[test]
    fn p16_census_ownerless_site_never_enters_candidate_ledger() {
        let mut cg = build_go(&[(
            "pkg/main.go",
            "package p\ntype I interface{ M() }\nfunc run(v I) { v.M() }\n",
        )]);
        for site_set in cg.calls.values_mut() {
            *site_set = site_set
                .iter()
                .cloned()
                .map(|mut site| {
                    if site.caller.name == "run" && site.callee_name == "M" {
                        site.receiver_owner_identity = None;
                    }
                    site
                })
                .collect::<BTreeSet<_>>();
        }

        let manifest = interface_dispatch_manifest_inner(&cg, true);
        let rows = manifest["p16_prerequisite"]
            .as_array()
            .expect("prerequisite ledger");
        let row = rows
            .iter()
            .find(|row| row["method"] == "M")
            .expect("ownerless prerequisite row");
        assert_eq!(row["partition"], "ownerless_terminal");
        assert!(row["owner"].is_null());
        assert!(manifest["p16_candidates"]
            .as_array()
            .expect("candidate ledger")
            .is_empty());
        assert!(manifest["sites"]
            .as_array()
            .expect("ordinary sites")
            .iter()
            .all(|site| site["method"] != "M"));

        let disabled = interface_dispatch_manifest_inner(&cg, false);
        assert!(disabled.get("p16_prerequisite").is_none());
        assert!(disabled.get("p16_candidates").is_none());
    }
}

/// Phase-IP PR-2 in-scope interface-dispatch manifest (spec §8a, structural — no oracle).
///
/// A call-site is *in-scope* iff its receiver was syntactically recovered
/// (typed_param / constructor_local / type_assertion / var_local) AND the called
/// method appears on some known Go interface (`cg.interface_method_names`). Each
/// in-scope site is keyed by its byte-span (`file:start_byte:end_byte`) and stratified
/// by receiver class. `implementers` (Slice E) is the sorted, deduped set of implementer
/// owner *type names* prism mints for that (interface, method) — the RTA-pruned live set;
/// `fanout` is its cardinality (0 / `[]` for a concrete owner-resolved receiver). The
/// Slice-E gopls oracle (`eval/tools/dispatch_oracle.py`) compares this set per site to
/// gopls's `textDocument/implementation` satisfier set to decide sound vs over-approx.
///
/// The §5 `slice_candidate` (range-element) class is a manifest-only AST scan that is
/// **deferred** (see the PR-2 deferred doc); the recovered classes above do not depend
/// on it. The `corrected_fp` line of the gate report (the Python harness consumes this
/// JSON) is provisional until the Slice-E re-adjudication.
pub fn interface_dispatch_manifest(cg: &CallGraph) -> serde_json::Value {
    interface_dispatch_manifest_inner(cg, std::env::var_os("PRISM_P16_CENSUS").is_some())
}

fn interface_dispatch_manifest_inner(
    cg: &CallGraph,
    p16_census_enabled: bool,
) -> serde_json::Value {
    use crate::resolution::ReceiverRecovery;
    let mut p16_prerequisite = Vec::new();
    let mut p16_candidates = Vec::new();
    // receiver_class wire strings (the Rust→JSON→Python contract; pinned by the
    // `interface_manifest_receiver_class_strings` test). NOTE (review MAJOR 5):
    // `SliceElem`/"slice_elem" is the RESERVED variant (Slice F) — the classifier returns
    // None for it, so it never appears on a real site. It is DISTINCT from the spec-§5
    // deferred manifest-only "slice_candidate" range class (a CpgContext AST scan,
    // deferred — see the PR-2 deferred doc); the manifest currently emits only the four
    // recovered classes below.
    let class = |r: ReceiverRecovery| match r {
        ReceiverRecovery::TypedParam => "typed_param",
        ReceiverRecovery::ConstructorLocal => "constructor_local",
        ReceiverRecovery::TypeAssertion => "type_assertion",
        ReceiverRecovery::VarDecl => "var_local",
        ReceiverRecovery::SliceElem => "slice_elem",
        // P11 S5 (telemetry honesty fix): FieldTyped/ReturnTyped used to
        // collapse into the catch-all "rust_receiver" bucket, hiding the new
        // Go S1/S2 recoveries from this manifest entirely. Split out; only
        // the genuinely Rust-only forms stay lumped.
        ReceiverRecovery::FieldTyped => "field_typed",
        ReceiverRecovery::ReturnTyped => "return_typed",
        ReceiverRecovery::StdWrapperPeel | ReceiverRecovery::TypedLet => "rust_receiver",
    };
    let mut sites = Vec::new();
    for site_set in cg.calls.values() {
        for site in site_set {
            let (Some(recv_ty), Some(recovery)) =
                (site.receiver_type.as_deref(), site.receiver_recovery)
            else {
                continue;
            };
            // Go-caller gate (review MAJOR 4): real interface dispatch is Go-gated in
            // resolution.rs (caller.file is Go), so only count Go-caller sites. A non-Go
            // caller that syntactically recovers a same-named receiver type is not a real
            // interface-dispatch site and would inflate the denominator.
            if crate::languages::Language::from_path(&site.caller.file)
                != Some(crate::languages::Language::Go)
            {
                continue;
            }
            // Denominator predicate (§8a): the called method is on some known interface.
            if !cg.interface_method_names.contains(&site.callee_name) {
                continue;
            }
            let terminally_unproven =
                crate::resolution::go_receiver_owner_is_terminally_unproven(site);
            if p16_census_enabled {
                p16_prerequisite.push(serde_json::json!({
                    "key": format!("{}:{}:{}", site.caller.file, site.start_byte, site.end_byte),
                    "file": site.caller.file,
                    "start_byte": site.start_byte,
                    "end_byte": site.end_byte,
                    "line": site.line,
                    "receiver_type": recv_ty,
                    "method": site.callee_name,
                    "partition": if terminally_unproven {
                        "ownerless_terminal"
                    } else {
                        "owner_bearing"
                    },
                    "owner": site.receiver_owner_identity,
                }));
            }
            if terminally_unproven {
                continue;
            }
            let p16_current_outcome = p16_census_enabled.then(|| cg.resolve_call_site_full(site));
            // P17: this is the same full R1/R2/R3 verdict the resolver consumes.
            // It must be consulted immediately after the denominator predicate,
            // before the manifest's legacy R3 interface ladder.
            let go_route = cg.go_concrete_receiver_route(
                recv_ty,
                site.receiver_owner_identity.as_ref(),
                site.receiver_local_type_shadowed,
                site.receiver_newly_recovered,
                &site.callee_name,
                &site.caller.file,
            );
            let mut visibility_interface = None;
            let legacy_bare = false;
            let mut terminal_targets: Option<Vec<&FunctionId>> = None;
            let dispatch_route;
            let impls: &[FunctionId];
            match &go_route {
                GoConcreteReceiverRoute::ConcreteDirect { .. } => {
                    dispatch_route = "concrete_direct";
                    impls = &[];
                }
                GoConcreteReceiverRoute::ConcretePromoted { .. } => {
                    dispatch_route = "concrete_promoted";
                    impls = &[];
                }
                GoConcreteReceiverRoute::ConcretePromotedDeferred { owner } => {
                    dispatch_route = match consult_promoted_snapshot(
                        cg.go_promoted_selector_snapshot(),
                        &cg.go_method_declarations,
                        owner,
                        &site.callee_name,
                    ) {
                        GoPromotedSnapshotConsult::Hit(_) => "concrete_promoted_snapshot",
                        _ => "concrete_promoted_deferred_drop",
                    };
                    impls = &[];
                }
                GoConcreteReceiverRoute::EmbeddedInterfaceDispatch {
                    owner,
                    interface_name,
                    ..
                } => {
                    dispatch_route = "embedded_interface_dispatch";
                    visibility_interface = Some((Some(owner.clone()), interface_name.clone()));
                    impls = cg
                        .interface_impls
                        .get(&(interface_name.clone(), site.callee_name.clone()))
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                }
                GoConcreteReceiverRoute::FuncValueField { .. } => {
                    dispatch_route = "func_value_field";
                    impls = &[];
                }
                GoConcreteReceiverRoute::ConcreteNoSelector { .. } => {
                    dispatch_route = "concrete_no_selector_drop";
                    impls = &[];
                }
                GoConcreteReceiverRoute::InterfaceDispatch {
                    owner,
                    interface_name,
                } => {
                    dispatch_route = "interface_dispatch";
                    visibility_interface = Some((Some(owner.clone()), interface_name.clone()));
                    impls = cg
                        .interface_impls
                        .get(&(interface_name.clone(), site.callee_name.clone()))
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                }
                GoConcreteReceiverRoute::R2OnDemandNameCollisionBail => {
                    dispatch_route = "unproven_drop";
                    impls = &[];
                }
                GoConcreteReceiverRoute::ExternalNewRecoveryDrop => {
                    dispatch_route = "unproven_drop";
                    impls = &[];
                }
                GoConcreteReceiverRoute::Unproven => {
                    // Retain the S4 and carried-interface screens, then route the
                    // surviving owner-bearing R3 seam through the #16 exact-owner consult.
                    let s4_route = cg.go_embedded_interface_route(
                        recv_ty,
                        site.receiver_owner_identity.as_ref(),
                        &site.callee_name,
                        &site.caller.file,
                    );
                    let s4_blocked = s4_route.evidence.conflict || s4_route.evidence.uncertain;
                    let interface_presence = site
                        .receiver_owner_identity
                        .as_ref()
                        .map(|owner| cg.go_visible_interface_owner(owner, &site.caller.file));
                    let direct_interface_blocked =
                        interface_presence.as_ref().is_some_and(|selection| {
                            selection.evidence.conflict || selection.evidence.uncertain
                        });
                    let proven_interface_owner = site
                        .receiver_owner_identity
                        .as_ref()
                        .filter(|_| {
                            interface_presence
                                .as_ref()
                                .is_some_and(|selection| selection.value == Some(true))
                        })
                        .cloned();
                    if let Some(interface_name) = s4_route.value {
                        dispatch_route = "embedded_interface_dispatch";
                        visibility_interface =
                            Some((site.receiver_owner_identity.clone(), interface_name.clone()));
                        impls = cg
                            .interface_impls
                            .get(&(interface_name, site.callee_name.clone()))
                            .map(Vec::as_slice)
                            .unwrap_or(&[]);
                    } else if s4_blocked || direct_interface_blocked {
                        dispatch_route = "unproven_drop";
                        impls = &[];
                    } else if let Some(owner) = proven_interface_owner {
                        dispatch_route = "interface_dispatch";
                        let interface_name = owner.name.clone();
                        impls = cg
                            .interface_impls
                            .get(&(interface_name.clone(), site.callee_name.clone()))
                            .map(Vec::as_slice)
                            .unwrap_or(&[]);
                        visibility_interface =
                            Some((site.receiver_owner_identity.clone(), interface_name));
                    } else {
                        let owner = site.receiver_owner_identity.as_ref().expect(
                            "terminal Go receiver predicate admitted an owner-bearing site",
                        );
                        let (consult, outcome) = cg.go_proven_interface_outcome(site, owner);
                        dispatch_route = if consult.final_satisfiers > 0 {
                            "interface_dispatch"
                        } else if outcome.resolved.iter().any(|resolved| {
                            resolved.kind == crate::resolution::ResolutionKind::FuncValueField
                        }) {
                            "func_value_field"
                        } else {
                            "unproven_drop"
                        };
                        terminal_targets = Some(
                            outcome
                                .resolved
                                .iter()
                                .map(|resolved| resolved.target)
                                .collect(),
                        );
                        impls = &[];
                    }
                }
            }
            // Arity-disambiguate the name-keyed candidate set BEFORE the owner-name
            // mapping, so `fanout` (= implementers cardinality) reflects the filtered
            // set the resolver would mint. Same shared helper as the resolution mint;
            // an emptied set yields implementers: [] / fanout: 0. The oracle reads this
            // manifest, so the filter MUST run here too, not just in resolution.rs.
            let terminal_targets_authoritative = terminal_targets.is_some();
            let kept = if let Some(targets) = terminal_targets {
                targets
            } else {
                let kept = crate::resolution::arity_filter(
                    impls,
                    site.arg_count,
                    site.arg_spread,
                    &cg.method_arity,
                );
                let kept = if let Some((owner, interface_name)) = visibility_interface.as_ref() {
                    cg.go_visible_s4_implementers(
                        recv_ty,
                        owner.as_ref(),
                        interface_name,
                        &site.callee_name,
                        &site.caller.file,
                        kept,
                    )
                    .value
                    .unwrap_or_default()
                } else {
                    kept
                };
                kept.into_iter()
                    .filter(|target| {
                        crate::resolution::arity_admits(
                            site.arg_count,
                            site.arg_spread,
                            cg.method_arity.get(*target),
                        )
                    })
                    .collect()
            };
            if let Some(current_outcome) = p16_current_outcome
                .as_ref()
                .filter(|outcome| outcome.telemetry.go_unproven_receiver_bare_fallback_sites > 0)
            {
                let owner = site
                    .receiver_owner_identity
                    .as_ref()
                    .expect("terminal provenance predicate admitted owner-bearing site");
                let (consult, candidate_outcome) = cg.go_proven_interface_outcome(site, owner);
                let current_table_identities = if current_outcome
                    .telemetry
                    .go_unproven_receiver_bare_fallback_hits
                    > 0
                {
                    interface_manifest_target_identities(
                        cg,
                        current_outcome
                            .resolved
                            .iter()
                            .map(|resolved| resolved.target),
                    )
                } else {
                    BTreeSet::new()
                };
                let current_terminal_identities = interface_manifest_target_identities(
                    cg,
                    current_outcome
                        .resolved
                        .iter()
                        .map(|resolved| resolved.target),
                );
                let candidate_interface_identities = interface_manifest_target_identities(
                    cg,
                    consult
                        .selection
                        .value
                        .as_ref()
                        .into_iter()
                        .flatten()
                        .copied(),
                );
                let candidate_terminal_identities = interface_manifest_target_identities(
                    cg,
                    candidate_outcome
                        .resolved
                        .iter()
                        .map(|resolved| resolved.target),
                );
                let lost: BTreeSet<_> = current_terminal_identities
                    .difference(&candidate_terminal_identities)
                    .cloned()
                    .collect();
                let added: BTreeSet<_> = candidate_terminal_identities
                    .difference(&current_terminal_identities)
                    .cloned()
                    .collect();
                let route = consult.route.as_str();
                let evidence = consult.selection.evidence;
                p16_candidates.push(serde_json::json!({
                    "key": format!("{}:{}:{}", site.caller.file, site.start_byte, site.end_byte),
                    "file": site.caller.file,
                    "start_byte": site.start_byte,
                    "end_byte": site.end_byte,
                    "line": site.line,
                    "receiver_type": recv_ty,
                    "method": site.callee_name,
                    "owner": owner,
                    "manifest_dispatch_route": dispatch_route,
                    "manifest_legacy_bare": legacy_bare,
                    "route": route,
                    "all_satisfiers": consult.all_satisfiers,
                    "live_satisfiers": consult.live_satisfiers,
                    "final_satisfiers": consult.final_satisfiers,
                    "partition_evidence": {
                        "visible_declarations": evidence.visible_declarations,
                        "filtered_declarations": evidence.filtered_declarations,
                        "distinct_visible_values": evidence.distinct_visible_values,
                        "recovered": evidence.recovered,
                        "conflict": evidence.conflict,
                        "uncertain": evidence.uncertain,
                    },
                    "promoted_supply": {
                        "conflict_drop": consult.promoted.conflict_drop,
                        "variant_drop": consult.promoted.variant_drop,
                        "invariant_drop": consult.promoted.invariant_drop,
                        "signature_drop": consult.promoted.signature_drop,
                    },
                    "current_table_targets": p16_target_identity_values(
                        &current_table_identities,
                    ),
                    "current_terminal_outcome": p16_terminal_outcome(current_outcome),
                    "current_terminal_targets": p16_target_identity_values(
                        &current_terminal_identities,
                    ),
                    "candidate_interface_targets": p16_target_identity_values(
                        &candidate_interface_identities,
                    ),
                    "candidate_terminal_outcome": p16_terminal_outcome(&candidate_outcome),
                    "candidate_terminal_targets": p16_target_identity_values(
                        &candidate_terminal_identities,
                    ),
                    "lost_targets": p16_target_identity_values(&lost),
                    "added_targets": p16_target_identity_values(&added),
                }));
            }
            let implementers: BTreeSet<String> = kept
                .iter()
                .map(|fid| {
                    cg.method_owners
                        .get(*fid)
                        .cloned()
                        .unwrap_or_else(|| crate::resolution::file_stem(&fid.file).to_string())
                })
                .collect();
            let implementers: Vec<String> = implementers.into_iter().collect();
            // #14 slice 1: preserve the legacy owner-name wire field above, and add
            // enough target evidence for the oracle to distinguish same-named Go
            // implementers across packages, clauses, and build-tagged files. This
            // intentionally dedupes only identical full target tuples: two methods
            // named Impl in different build-tag files are distinct Exact-edge targets.
            let implementer_identities = interface_manifest_target_identity_values(
                &interface_manifest_target_identities(cg, kept.iter().copied()),
            );
            // Consult-terminal rows carry the resolver's exact targets. Their legacy
            // owner labels may collapse multiple callbacks from the same file, so
            // report exact target cardinality without changing interface-route fanout.
            let fanout = if terminal_targets_authoritative {
                implementer_identities.len()
            } else {
                implementers.len()
            };
            sites.push(serde_json::json!({
                "file": site.caller.file,
                "start_byte": site.start_byte,
                "end_byte": site.end_byte,
                "line": site.line,
                "receiver_class": class(recovery),
                "method": site.callee_name,
                "dispatch_route": dispatch_route,
                "fanout": fanout,
                "implementers": implementers,
                "implementer_identities": implementer_identities,
            }));
        }
    }
    // `interface_dispatch_computed` (review MINOR 6): false on a raw build_direct_subset
    // graph (apply_go_interface_dispatch never ran) → an empty `sites` means "not computed",
    // not "no dispatch found". The CLI feeds a full-build graph, so this is true in practice.
    let mut manifest = serde_json::json!({
        "sites": sites,
        "interface_dispatch_computed": cg.interface_dispatch_computed,
        "go_promoted_snapshot": {
            "owners": cg.go_promoted_selector_snapshot().owners.len(),
            "profile_conflicts": cg.go_promoted_selector_snapshot().profile_conflicts(),
            "promoted_methods": cg.go_promoted_selector_snapshot().promoted_methods(),
        },
    });
    if p16_census_enabled {
        let object = manifest
            .as_object_mut()
            .expect("interface dispatch manifest is an object");
        object.insert("p16_prerequisite".to_string(), p16_prerequisite.into());
        object.insert("p16_candidates".to_string(), p16_candidates.into());
    }
    manifest
}

fn confidence_score(c: crate::resolution::ResolutionConfidence) -> f32 {
    match c {
        crate::resolution::ResolutionConfidence::Exact => 1.0,
        crate::resolution::ResolutionConfidence::NameOnly => 0.6,
    }
}

fn resolution_reason(kind: crate::resolution::ResolutionKind) -> Reason {
    Reason::Resolution {
        kind: kind.as_str().to_string(),
    }
}

fn function_bytes(s: &NavigationSession, fid: &FunctionId) -> (usize, usize) {
    s.index
        .cpg
        .function_candidates(&fid.file, &fid.name)
        .into_iter()
        .find_map(|idx| match s.index.cpg.node(idx) {
            CpgNode::Function {
                start_line,
                start_byte,
                end_byte,
                ..
            } if *start_line == fid.start_line => Some((*start_byte, *end_byte)),
            _ => None,
        })
        .unwrap_or((0, 0))
}

/// `sites` is a deterministic (sorted), capped-at-5 sample of the dropped
/// `(file, line)` locations (P3) so a consumer can jump straight to a
/// site instead of only seeing a count.
fn collision_warning(count: usize, sites: &[(String, usize)]) -> Warning {
    let mut message = format!(
        "{count} same-name receiver call site(s) with unknown receiver type across multiple owner types; not attributed as callers"
    );
    if !sites.is_empty() {
        let named: Vec<String> = sites
            .iter()
            .take(5)
            .map(|(file, line)| format!("{file}:{line}"))
            .collect();
        message.push_str(&format!(" ({})", named.join(", ")));
    }
    Warning {
        kind: WarningKind::Collision,
        message,
        location: None,
    }
}

/// Exact, read-only callable edit coordinates for a uniquely resolved seed.
pub fn symbol_spans(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
) -> Result<SymbolSpans, QueryError> {
    fn ast_location(file: &str, span: &crate::ast::AstNodeSpan) -> Location {
        Location {
            file: file.to_string(),
            start_line: span.start_line,
            end_line: span.end_line,
            start_byte: span.start_byte,
            end_byte: span.end_byte,
        }
    }

    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let SymbolRef::Function {
        file,
        name,
        start_line,
        end_line,
        start_byte,
        end_byte,
        ..
    } = &resolved.symbol
    else {
        unreachable!("resolve_fn returns only function symbols")
    };
    let symbol_span = Location {
        file: file.clone(),
        start_line: *start_line,
        end_line: *end_line,
        start_byte: *start_byte,
        end_byte: *end_byte,
    };
    let mut unavailable = BTreeMap::new();
    let structure = s
        .repo
        .files
        .get(file)
        .and_then(|parsed| parsed.function_ast_spans(*start_byte, *end_byte));

    let (name_span, body_span, symbol_indentation, body_indentation) = match structure {
        Some(structure) => {
            let name_span = structure.name.as_ref().map(|span| ast_location(file, span));
            if name_span.is_none() {
                unavailable.insert(
                    "name_span".into(),
                    "callable grammar node has no name".into(),
                );
            }
            let body_span = structure.body.as_ref().map(|span| ast_location(file, span));
            if body_span.is_none() {
                unavailable.insert(
                    "body_span".into(),
                    "callable grammar node has no body field".into(),
                );
            }
            let symbol_indentation = match structure.symbol_indentation {
                Ok(value) => Some(value),
                Err(reason) => {
                    unavailable.insert("indentation.symbol".into(), reason.into());
                    None
                }
            };
            let body_indentation = match structure.body_indentation {
                Ok(value) => Some(value),
                Err(reason) => {
                    unavailable.insert("indentation.body".into(), reason.into());
                    None
                }
            };
            (name_span, body_span, symbol_indentation, body_indentation)
        }
        None => {
            let reason = "current parsed file has no exact callable byte identity";
            unavailable.insert("name_span".into(), reason.into());
            unavailable.insert("body_span".into(), reason.into());
            unavailable.insert("indentation.symbol".into(), reason.into());
            unavailable.insert("indentation.body".into(), "body span unavailable".into());
            (None, None, None, None)
        }
    };

    Ok(SymbolSpans {
        schema_version: "1.0",
        query: format!("symbol-spans:{name}@{file}"),
        symbol: resolved.symbol.clone(),
        symbol_span: symbol_span.clone(),
        name_span,
        body_span,
        insert_before: InsertionAnchor {
            file: file.clone(),
            line: symbol_span.start_line,
            byte: symbol_span.start_byte,
        },
        insert_after: InsertionAnchor {
            file: file.clone(),
            line: symbol_span.end_line,
            byte: symbol_span.end_byte,
        },
        indentation: IndentationContext {
            symbol: symbol_indentation,
            body: body_indentation,
        },
        unavailable,
        warnings: vec![],
    })
}

/// Exact CPG nodes at `file:line` (Function/Variable only, spec §8 R3-M3) plus the
/// innermost enclosing function as `EnclosingFunction` evidence.
pub fn nodes_at(s: &NavigationSession, file: &str, line: usize) -> Evidence {
    let query = format!("nodes-at:{file}:{line}");
    if !s.repo.files.contains_key(file) {
        let message = s
            .repo
            .skipped
            .iter()
            .find(|skipped| skipped.path == file)
            .map(|skipped| format!("file excluded: {:?}: {file}", skipped.reason))
            .unwrap_or_else(|| format!("file not in nav index: {file}"));
        return Evidence {
            query,
            items: vec![],
            truncated: false,
            warnings: vec![Warning {
                kind: WarningKind::SkippedPath,
                message,
                location: Some(Location {
                    file: file.into(),
                    start_line: line,
                    end_line: line,
                    start_byte: 0,
                    end_byte: 0,
                }),
            }],
            graph: None,
            reasoning: None,
        };
    }
    let mut items = Vec::new();
    for idx in nav_visible_nodes_at(s, file, line) {
        match s.index.cpg.node(idx) {
            CpgNode::Function {
                name,
                file: f,
                start_line,
                end_line,
                start_byte,
                end_byte,
                ..
            } => items.push(item_fn(
                f,
                name,
                *start_line,
                *end_line,
                *start_byte,
                *end_byte,
            )),
            CpgNode::Variable {
                path,
                file: f,
                function,
                line: l,
                access,
                start_byte,
                end_byte,
                ..
            } => items.push(EvidenceItem {
                symbol: Some(SymbolRef::Variable {
                    file: f.clone(),
                    function: function.clone(),
                    line: *l,
                    path: format!("{path:?}"),
                    access: format!("{access:?}"),
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                    ordinal: 0,
                }),
                location: Location {
                    file: f.clone(),
                    start_line: *l,
                    end_line: *l,
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: vec![],
                snippet: None,
            }),
            CpgNode::Statement { .. } | CpgNode::ReturnValue { .. } => {} // statements/synthetic return values are not first-class nodes-at evidence
        }
    }
    // Enclosing function (innermost), as evidence on the line.
    if let Some((eidx, _)) = s.index.enclosing_function(file, line) {
        if let CpgNode::Function {
            name,
            file: f,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } = s.index.cpg.node(eidx)
        {
            let func = SymbolRef::Function {
                file: f.clone(),
                name: name.clone(),
                start_line: *start_line,
                end_line: *end_line,
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            };
            items.push(EvidenceItem {
                symbol: Some(func.clone()),
                // The enclosing-function evidence's Location mirrors the function's FULL span so
                // line and byte coordinates are coherent (S2 review MAJOR: queried-line + full-
                // function bytes was incoherent). The queried line is the query input, not the
                // evidence extent; the symbol + location both describe the function.
                location: Location {
                    file: f.clone(),
                    start_line: *start_line,
                    end_line: *end_line,
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: vec![Reason::EnclosingFunction { function: func }],
                snippet: None,
            });
        }
    }
    Evidence {
        query,
        items,
        truncated: false,
        warnings: vec![],
        graph: None,
        reasoning: None,
    }
}

/// Whether a CPG node is an anonymous-callable identity: the callable's own
/// Function node, or a Variable / ReturnValue row it owns (`<anon@L:C>`).
fn is_anonymous_identity_node(s: &NavigationSession, idx: NodeIndex) -> bool {
    match s.index.cpg.node(idx) {
        CpgNode::Function { name, .. } => crate::anon_owner::is_anonymous_owner_name(name),
        CpgNode::Variable { function, .. } | CpgNode::ReturnValue { function, .. } => {
            crate::anon_owner::is_anonymous_owner_name(function)
        }
        CpgNode::Statement { .. } => false,
    }
}

/// js-param-defs PR-B: DFG rows of anonymous JS/TS callables carry a synthetic
/// `<anon@L:C>` owner. Navigation stays byte-identical by default, so those
/// nodes (and, once they exist, the anonymous Function nodes themselves) are
/// never navigation evidence or ego seeds (SPEC-prB D7).
fn nav_visible_nodes_at(s: &NavigationSession, file: &str, line: usize) -> Vec<NodeIndex> {
    s.index
        .cpg
        .nodes_at(file, line)
        .into_iter()
        .filter(|&idx| !is_anonymous_identity_node(s, idx))
        .collect()
}

fn fid_of(sym: &SymbolRef) -> FunctionId {
    match sym {
        // `..` — SymbolRef::Function also has `ordinal` (R2-B1).
        SymbolRef::Function {
            name,
            file,
            start_line,
            end_line,
            ..
        } => FunctionId {
            name: name.clone(),
            file: file.clone(),
            start_line: *start_line,
            end_line: *end_line,
        },
        _ => unreachable!("seed resolves to a Function"),
    }
}

/// Direct callers of `target` (qualifier-aware identity filter): callers index is keyed by name,
/// so each candidate CallSite is resolved from its caller's file and kept only if it reaches THIS target.
fn direct_callers<'a>(
    s: &'a NavigationSession,
    target: &FunctionId,
) -> &'a [crate::navigation::IndexedIncomingCall] {
    s.index.direct_callers(target)
}

struct SortableEvidenceItem {
    item: EvidenceItem,
    call_site_line: usize,
    call_site_start_byte: usize,
    call_site_end_byte: usize,
    resolution_kind: &'static str,
    name: String,
    qualifier: Option<String>,
}

fn sort_evidence_items(items: &mut [SortableEvidenceItem]) {
    items.sort_by(|a, b| {
        b.item
            .score
            .partial_cmp(&a.item.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.item.location.file.cmp(&b.item.location.file))
            .then(a.item.location.start_line.cmp(&b.item.location.start_line))
            .then(a.call_site_line.cmp(&b.call_site_line))
            .then(a.call_site_start_byte.cmp(&b.call_site_start_byte))
            .then(a.call_site_end_byte.cmp(&b.call_site_end_byte))
            .then(a.resolution_kind.cmp(&b.resolution_kind))
            .then(a.name.cmp(&b.name))
            .then(a.qualifier.cmp(&b.qualifier))
    });
}

pub fn callers(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
) -> Result<Evidence, QueryError> {
    callers_with_confidence(s, symbol, file, location, depth, false)
}

pub fn callers_with_confidence(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let target = fid_of(&resolved.symbol);
    let target_for_warning = target.clone();
    let query = format!("callers:{}@{}", target.name, target.file); // @file identity (R3-M4)
    let mut items = Vec::new();
    let mut visited: std::collections::BTreeSet<FunctionId> = std::collections::BTreeSet::new();
    visited.insert(target.clone());
    let mut frontier = vec![target];
    for hop in 0..depth {
        // hop 0 = direct hit → score 1.0 (R3-M2); depth=2 → hops 0,1
        let mut next = Vec::new();
        for fid in &frontier {
            for edge in direct_callers(s, fid) {
                if exact_only && edge.confidence != ResolutionConfidence::Exact {
                    continue;
                }
                let caller = &edge.caller;
                let (start_byte, end_byte) = function_bytes(s, caller);
                let (symbol, location) =
                    super::module_owner::caller_evidence(caller, edge, (start_byte, end_byte));
                // One item PER CALL SITE (m7 symmetry with callees); `visited` only gates BFS recursion.
                items.push(SortableEvidenceItem {
                    item: EvidenceItem {
                        symbol: Some(symbol),
                        location,
                        score: confidence_score(edge.confidence) / (1.0 + hop as f32),
                        source: Source::PrismCpg,
                        fallback: false,
                        why: vec![Reason::CalledBy {
                            caller: if crate::anon_owner::is_module_owner_name(&caller.name) {
                                "module-level".into()
                            } else {
                                caller.name.clone()
                            },
                            call_site_line: edge.call_site_line,
                        }]
                        .into_iter()
                        .chain(std::iter::once(resolution_reason(edge.kind)))
                        .collect(),
                        snippet: None,
                    },
                    call_site_line: edge.call_site_line,
                    call_site_start_byte: edge.start_byte,
                    call_site_end_byte: edge.end_byte,
                    resolution_kind: edge.kind.as_str(),
                    name: edge.callee_name.clone(),
                    qualifier: edge.qualifier.clone(),
                });
                if visited.insert((*caller).clone()) {
                    next.push((*caller).clone());
                }
            }
        }
        frontier = next;
    }
    sort_evidence_items(&mut items);
    let items = items.into_iter().map(|i| i.item).collect();
    let dropped_sites = s
        .index
        .collision_dropped_site_locations(&target_for_warning.name);
    let warnings = if !dropped_sites.is_empty() {
        vec![collision_warning(dropped_sites.len(), &dropped_sites)]
    } else {
        vec![]
    };
    Ok(Evidence {
        query,
        items,
        truncated: false,
        warnings,
        graph: None,
        reasoning: None,
    })
}

fn direct_callees<'a>(
    s: &'a NavigationSession,
    caller: &FunctionId,
) -> &'a [crate::navigation::IndexedOutgoingCallSite] {
    s.index.direct_callees(caller)
}

pub fn callees(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
) -> Result<Evidence, QueryError> {
    callees_with_confidence(s, symbol, file, location, depth, false)
}

pub fn callees_with_confidence(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let seed_fid = fid_of(&resolved.symbol);
    let query = format!("callees:{}@{}", seed_fid.name, seed_fid.file); // @file identity (R3-M4)
    let mut items = Vec::new();
    let mut visited: std::collections::BTreeSet<FunctionId> = std::collections::BTreeSet::new();
    visited.insert(seed_fid.clone());
    let mut frontier = vec![seed_fid];
    for hop in 0..depth {
        // hop 0 = direct hit → score 1.0 (R3-M2); depth=2 → hops 0,1
        let mut next = Vec::new();
        for fid in &frontier {
            for site in direct_callees(s, fid) {
                if site.resolved.is_empty() {
                    if exact_only {
                        continue;
                    }
                    items.push(SortableEvidenceItem {
                        item: EvidenceItem {
                            symbol: None,
                            location: Location {
                                file: fid.file.clone(),
                                start_line: site.call_site_line,
                                end_line: site.call_site_line,
                                start_byte: 0,
                                end_byte: 0,
                            },
                            score: 1.0 / (1.0 + hop as f32),
                            source: Source::PrismCpg,
                            fallback: false,
                            why: vec![Reason::Calls {
                                callee: site.callee_name.clone(),
                                call_site_line: site.call_site_line,
                                qualifier: site.qualifier.clone(),
                            }],
                            snippet: None,
                        },
                        call_site_line: site.call_site_line,
                        call_site_start_byte: site.start_byte,
                        call_site_end_byte: site.end_byte,
                        resolution_kind: "",
                        name: site.callee_name.clone(),
                        qualifier: site.qualifier.clone(),
                    });
                    continue;
                }

                for edge in &site.resolved {
                    if exact_only && edge.confidence != ResolutionConfidence::Exact {
                        continue;
                    }
                    let d = &edge.target;
                    let (start_byte, end_byte) = function_bytes(s, d);
                    let sym = Some(SymbolRef::Function {
                        file: d.file.clone(),
                        name: d.name.clone(),
                        start_line: d.start_line,
                        end_line: d.end_line,
                        start_byte,
                        end_byte,
                        ordinal: 0,
                    });
                    let loc = Location {
                        file: d.file.clone(),
                        start_line: d.start_line,
                        end_line: d.end_line,
                        start_byte,
                        end_byte,
                    };
                    items.push(SortableEvidenceItem {
                        item: EvidenceItem {
                            symbol: sym,
                            location: loc,
                            score: confidence_score(edge.confidence) / (1.0 + hop as f32),
                            source: Source::PrismCpg,
                            fallback: false,
                            why: vec![
                                Reason::Calls {
                                    callee: site.callee_name.clone(),
                                    call_site_line: site.call_site_line,
                                    qualifier: site.qualifier.clone(),
                                },
                                resolution_reason(edge.kind),
                            ],
                            snippet: None,
                        },
                        call_site_line: site.call_site_line,
                        call_site_start_byte: site.start_byte,
                        call_site_end_byte: site.end_byte,
                        resolution_kind: edge.kind.as_str(),
                        name: d.name.clone(),
                        qualifier: site.qualifier.clone(),
                    });
                    if visited.insert(d.clone()) {
                        next.push(d.clone());
                    }
                }
            }
        }
        frontier = next;
    }
    sort_evidence_items(&mut items);
    let items = items.into_iter().map(|i| i.item).collect();
    Ok(Evidence {
        query,
        items,
        truncated: false,
        warnings: vec![],
        graph: None,
        reasoning: None,
    })
}

fn edge_kind(e: &CpgEdge) -> &'static str {
    match e {
        CpgEdge::DataFlow(_) => "DataFlow",
        CpgEdge::ControlFlow => "ControlFlow",
        CpgEdge::Call(_) => "Call",
        CpgEdge::Return(_) => "Return",
        CpgEdge::ReturnFlow { .. } => "ReturnFlow",
        CpgEdge::ReturnInput => "ReturnInput",
        CpgEdge::Contains => "Contains",
        CpgEdge::FieldOf => "FieldOf",
    }
}

fn parse_ego_edges(edges: &[&str]) -> Result<BTreeSet<&'static str>, QueryError> {
    let mut parsed = BTreeSet::new();
    for edge in edges {
        let edge = edge.trim();
        let kind = match edge {
            "DataFlow" => "DataFlow",
            "ControlFlow" => "ControlFlow",
            "Call" => "Call",
            "Return" => "Return",
            "ReturnFlow" => "ReturnFlow",
            "ReturnInput" => "ReturnInput",
            "Contains" => "Contains",
            "FieldOf" => "FieldOf",
            _ => {
                return Err(QueryError::UnknownEdge {
                    edge: edge.to_string(),
                });
            }
        };
        parsed.insert(kind);
    }
    Ok(parsed)
}

fn node_symbol_loc(s: &NavigationSession, ni: NodeIndex) -> (SymbolRef, Location) {
    match s.index.cpg.node(ni) {
        CpgNode::Function {
            name,
            file,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } => (
            super::module_owner::function_symbol(
                file,
                name,
                *start_line,
                *end_line,
                *start_byte,
                *end_byte,
            ),
            Location {
                file: file.clone(),
                start_line: *start_line,
                end_line: *end_line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
        CpgNode::Variable {
            path,
            file,
            function,
            line,
            access,
            start_byte,
            end_byte,
            ..
        } => (
            SymbolRef::Variable {
                file: file.clone(),
                function: function.clone(),
                line: *line,
                path: format!("{path:?}"),
                access: format!("{access:?}"),
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
            Location {
                file: file.clone(),
                start_line: *line,
                end_line: *line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
        CpgNode::Statement {
            file,
            line,
            kind,
            start_byte,
            end_byte,
            ..
        } => (
            SymbolRef::Statement {
                file: file.clone(),
                line: *line,
                kind: format!("{kind:?}"),
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
            Location {
                file: file.clone(),
                start_line: *line,
                end_line: *line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
        CpgNode::ReturnValue {
            file,
            line,
            start_byte,
            end_byte,
            ..
        } => (
            SymbolRef::Statement {
                file: file.clone(),
                line: *line,
                kind: "ReturnValue".into(),
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
            Location {
                file: file.clone(),
                start_line: *line,
                end_line: *line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
    }
}

struct EgoSeed {
    query: String,
    nodes: Vec<NodeIndex>,
}

fn parse_location(location: &str) -> Result<(String, usize), QueryError> {
    location
        .rsplit_once(':')
        .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
        .ok_or_else(|| QueryError::SymbolNotFound {
            seed: format!("loc:{location}"),
        })
}

fn resolve_ego_seed(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
) -> Result<EgoSeed, QueryError> {
    if let Some(loc) = location {
        let (f, line) = parse_location(loc)?;
        let mut nodes = nav_visible_nodes_at(s, &f, line);
        nodes.sort_by_key(|i| i.index());
        nodes.dedup();
        if nodes.is_empty() {
            let (idx, _) = s
                .index
                .enclosing_function(&f, line)
                .ok_or_else(|| seed::missing_callable_error(s, &f, line))?;
            nodes.push(idx);
        }
        return Ok(EgoSeed {
            query: format!("ego:{f}:{line}"),
            nodes,
        });
    }

    let resolved = seed::resolve_fn(s, symbol, file, None)?;
    let ego_fid = fid_of(&resolved.symbol);
    Ok(EgoSeed {
        query: format!("ego:{}@{}", ego_fid.name, ego_fid.file),
        nodes: vec![resolved.idx],
    })
}

pub fn ego_graph(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    hops: usize,
    edges: &[&str],
) -> Result<Evidence, QueryError> {
    let seed = resolve_ego_seed(s, symbol, file, location)?;
    let edge_filter = parse_ego_edges(edges)?;
    let g = &s.index.cpg.graph;
    let mut order: BTreeMap<NodeIndex, usize> = BTreeMap::new();
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut ego_edges: Vec<GraphEdge> = Vec::new();
    // returns (index, is_new) so we only enqueue freshly-discovered nodes (m9).
    let intern = |s: &NavigationSession,
                  ni: NodeIndex,
                  order: &mut BTreeMap<NodeIndex, usize>,
                  nodes: &mut Vec<GraphNode>|
     -> (usize, bool) {
        if let Some(&i) = order.get(&ni) {
            return (i, false);
        }
        let i = nodes.len();
        let (symbol, location) = node_symbol_loc(s, ni);
        nodes.push(GraphNode {
            symbol: Some(symbol),
            location,
        });
        order.insert(ni, i);
        (i, true)
    };
    let mut q = VecDeque::new();
    for seed_node in &seed.nodes {
        intern(s, *seed_node, &mut order, &mut nodes);
        q.push_back((*seed_node, 0usize));
    }
    while let Some((ni, d)) = q.pop_front() {
        if d >= hops {
            continue;
        }
        for dir in [Direction::Outgoing, Direction::Incoming] {
            for er in g.edges_directed(ni, dir) {
                if !edge_filter.contains(edge_kind(er.weight())) {
                    continue;
                }
                let other = if er.source() == ni {
                    er.target()
                } else {
                    er.source()
                };
                // Contains hops (and any other edge) must not surface a synthetic
                // Variable / ReturnValue owned by an anonymous callable.
                if matches!(
                    s.index.cpg.node(other),
                    CpgNode::Variable { .. } | CpgNode::ReturnValue { .. }
                ) && is_anonymous_identity_node(s, other)
                {
                    continue;
                }
                let (from, _) = intern(s, ni, &mut order, &mut nodes);
                let (to, is_new) = intern(s, other, &mut order, &mut nodes);
                let (a, b) = if dir == Direction::Outgoing {
                    (from, to)
                } else {
                    (to, from)
                };
                ego_edges.push(GraphEdge {
                    from: a,
                    to: b,
                    kind: edge_kind(er.weight()).into(),
                });
                if is_new {
                    q.push_back((other, d + 1));
                }
            }
        }
    }
    ego_edges.sort_by(|x, y| (x.from, x.to, &x.kind).cmp(&(y.from, y.to, &y.kind)));
    ego_edges.dedup();
    let dropped_sites: Vec<(String, usize)> = if hops > 0 && edge_filter.contains("Call") {
        let mut sites: Vec<(String, usize)> = seed
            .nodes
            .iter()
            .filter_map(|ni| match s.index.cpg.node(*ni) {
                CpgNode::Function { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .flat_map(|name| s.index.collision_dropped_site_locations(name))
            .collect();
        sites.sort();
        sites
    } else {
        Vec::new()
    };
    let warnings = if !dropped_sites.is_empty() {
        vec![collision_warning(dropped_sites.len(), &dropped_sites)]
    } else {
        vec![]
    };
    Ok(Evidence {
        query: seed.query,
        items: vec![],
        truncated: false,
        warnings,
        graph: Some(GraphPayload {
            nodes,
            edges: ego_edges,
        }),
        reasoning: None,
    })
}

fn item_fn(
    file: &str,
    name: &str,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
) -> EvidenceItem {
    let sym = SymbolRef::Function {
        file: file.into(),
        name: name.into(),
        start_line,
        end_line,
        start_byte,
        end_byte,
        ordinal: 0,
    };
    EvidenceItem {
        symbol: Some(sym),
        location: Location {
            file: file.into(),
            start_line,
            end_line,
            start_byte,
            end_byte,
        },
        score: 1.0,
        source: Source::PrismCpg,
        fallback: false,
        why: vec![],
        snippet: None,
    }
}

#[cfg(test)]
mod call_site_dump_stream_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;

    #[test]
    fn empty_and_partial_stream_keep_source_then_callback_custody_order() {
        let empty = CallGraph::build(&BTreeMap::new());
        assert_eq!(call_site_dump_iter(&empty).next(), None);
        let parsed = ParsedFile::parse(
            "callbacks.go",
            "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc accepted() { invoke(safe) }\nfunc dropped() { safe := func() {}; invoke(safe) }\n",
            Language::Go,
        ).unwrap();
        let cg = CallGraph::build(&BTreeMap::from([("callbacks.go".into(), parsed)]));
        let mut rows = call_site_dump_iter(&cg);
        let first = rows.next().unwrap();
        assert_eq!(first["record_kind"], "call_site");
        assert_eq!(first["caller"]["name"], "accepted");
        let remaining: Vec<_> = rows.collect();
        let candidates: Vec<_> = remaining
            .iter()
            .filter(|r| r["record_kind"] == "go_level3_b1_candidate")
            .collect();
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            remaining.last().unwrap()["record_kind"],
            "go_level3_b1_candidate"
        );
        assert!(candidates.iter().any(|r| r["decision"] == "accepted"));
        assert!(candidates.iter().any(|r| r["decision"] == "dropped"));
        let collected = call_site_dump(&cg);
        assert_eq!(collected[0], first);
        assert_eq!(collected[1..], remaining);
    }
}
