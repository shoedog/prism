use super::{CallGraph, CallSite, FunctionId};
use crate::ast::ParsedFile;
use crate::go_concrete_receiver::GoConcreteReceiverRoute;
use crate::go_promoted_snapshot::{GoPromotedOwnerSnapshot, GoPromotedSnapshotVerdict};
use crate::languages::Language;
use crate::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
use serde_json::Value;
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

fn build_go_module(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    let repo = tempfile::tempdir().expect("temporary Go module root");
    std::fs::write(
        repo.path().join("go.mod"),
        "module example.test/root\n\ngo 1.24\n",
    )
    .expect("write go.mod fixture");
    let inputs = crate::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn deferred_fixture(source: &str) -> CallGraph {
    build_go_module(&[
        ("q/types.go", source),
        ("p/marker.go", "package p\ntype Marker interface{ M() }\n"),
        (
            "app/use.go",
            "package app\nimport q \"example.test/root/q\"\nfunc run(s q.S) { s.M() }\n",
        ),
    ])
}

fn depth_one_fixture() -> CallGraph {
    deferred_fixture("package q\ntype B struct{}\nfunc (B) M() {}\ntype S struct{ B }\n")
}

fn site(cg: &CallGraph) -> &CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "run" && site.callee_name == "M")
        .expect("run->M call site")
}

fn receiver_owner(cg: &CallGraph) -> crate::resolution::GoOwnerIdentity {
    let call = site(cg);
    match cg.go_concrete_receiver_route(
        call.receiver_type.as_deref().expect("receiver type"),
        call.receiver_owner_identity.as_ref(),
        call.receiver_local_type_shadowed,
        call.receiver_newly_recovered,
        &call.callee_name,
        &call.caller.file,
    ) {
        GoConcreteReceiverRoute::ConcretePromotedDeferred { owner } => owner,
        route => panic!("pre-consult route must be ConcretePromotedDeferred, got {route:?}"),
    }
}

fn assert_preconsult_deferred(cg: &CallGraph) {
    let call = site(cg);
    let route = cg.go_concrete_receiver_route(
        call.receiver_type.as_deref().expect("receiver type"),
        call.receiver_owner_identity.as_ref(),
        call.receiver_local_type_shadowed,
        call.receiver_newly_recovered,
        &call.callee_name,
        &call.caller.file,
    );
    assert!(
        matches!(
            route,
            GoConcreteReceiverRoute::ConcretePromotedDeferred { .. }
        ),
        "pre-consult route must be ConcretePromotedDeferred, got {route:?}"
    );
}

fn owner_snapshot(cg: &CallGraph, name: &str) -> GoPromotedOwnerSnapshot {
    cg.go_promoted_selector_snapshot
        .owners
        .iter()
        .find(|(owner, _)| owner.name == name)
        .map(|(_, snapshot)| snapshot.clone())
        .unwrap_or_else(|| panic!("missing promoted snapshot owner {name}"))
}

fn deserialize_owner(value: Value) -> GoPromotedOwnerSnapshot {
    serde_json::from_value(value).expect("deserialize promoted owner snapshot")
}

fn replace_receiver_snapshot(cg: &mut CallGraph, snapshot: GoPromotedOwnerSnapshot) {
    let owner = receiver_owner(cg);
    cg.go_promoted_selector_snapshot
        .owners
        .insert(owner, snapshot);
}

fn graft_deserialized_owner(mut cg: CallGraph, snapshot: GoPromotedOwnerSnapshot) -> CallGraph {
    let serialized = serde_json::to_value(snapshot).expect("serialize promoted owner snapshot");
    replace_receiver_snapshot(&mut cg, deserialize_owner(serialized));
    cg
}

fn mutate_deserialized_owner(mut cg: CallGraph, mutate: impl FnOnce(&mut Value)) -> CallGraph {
    let owner = receiver_owner(&cg);
    let snapshot = cg
        .go_promoted_selector_snapshot
        .owners
        .get(&owner)
        .expect("receiver snapshot");
    let mut serialized = serde_json::to_value(snapshot).expect("serialize promoted owner snapshot");
    mutate(&mut serialized);
    replace_receiver_snapshot(&mut cg, deserialize_owner(serialized));
    cg
}

fn promoted_target(cg: &CallGraph) -> FunctionId {
    let owner = receiver_owner(cg);
    cg.go_promoted_selector_snapshot
        .owners
        .get(&owner)
        .expect("receiver snapshot")
        .declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "M")
        .expect("promoted M")
        .target
        .clone()
}

fn mutate_target_declaration(
    mut cg: CallGraph,
    mut mutate: impl FnMut(&mut crate::go_owner_partition::GoMethodDeclaration),
) -> CallGraph {
    let target = promoted_target(&cg);
    let mut matches = 0;
    for declarations in cg.go_method_declarations.values_mut() {
        let original = std::mem::take(declarations);
        *declarations = original
            .into_iter()
            .map(|mut declaration| {
                if declaration.function_id == target {
                    matches += 1;
                    mutate(&mut declaration);
                }
                declaration
            })
            .collect();
    }
    assert_eq!(matches, 1, "fixture must have one target declaration");
    cg
}

fn manifest_route(cg: &CallGraph) -> String {
    let manifest = crate::navigation::queries::interface_dispatch_manifest(cg);
    let sites = manifest["sites"].as_array().expect("manifest sites");
    assert_eq!(sites.len(), 1, "{manifest:#}");
    sites[0]["dispatch_route"]
        .as_str()
        .expect("manifest dispatch route")
        .to_string()
}

fn assert_negative(cg: &CallGraph, expected_counter: Option<&str>) {
    assert_preconsult_deferred(cg);
    let outcome = cg.resolve_call_site_full(site(cg));
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(
        outcome.drop,
        Some(DropReason::ConcreteReceiverPromotedDeferred)
    );
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_deferred, 1);
    assert_eq!(manifest_route(cg), "concrete_promoted_deferred_drop");

    let stats = crate::navigation::queries::call_stats(cg);
    assert_eq!(stats["go_concrete_receiver_promoted_deferred"], 1);
    assert_eq!(stats["go_promoted_snapshot_hits"], 0);
    for counter in [
        "go_promoted_snapshot_conflict_drop",
        "go_promoted_snapshot_variant_drop",
        "go_promoted_snapshot_invariant_drop",
    ] {
        assert_eq!(
            stats[counter],
            usize::from(expected_counter == Some(counter)),
            "unexpected {counter}: {stats:#}"
        );
    }
}

fn assert_positive(cg: &CallGraph, expected_file: &str) {
    assert_preconsult_deferred(cg);
    let outcome = cg.resolve_call_site_full(site(cg));
    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.file, expected_file);
    assert_eq!(outcome.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_deferred, 0);
    assert_eq!(manifest_route(cg), "concrete_promoted_snapshot");

    let stats = crate::navigation::queries::call_stats(cg);
    assert_eq!(stats["go_concrete_receiver_promoted_deferred"], 0);
    assert_eq!(stats["go_promoted_snapshot_hits"], 1);
    assert_eq!(stats["go_promoted_snapshot_conflict_drop"], 0);
    assert_eq!(stats["go_promoted_snapshot_variant_drop"], 0);
    assert_eq!(stats["go_promoted_snapshot_invariant_drop"], 0);
    assert_eq!(stats["kinds"]["embedded_promotion"], 1);
}

fn assert_conflict_donor(donor: &CallGraph) -> GoPromotedOwnerSnapshot {
    let snapshot = owner_snapshot(donor, "S");
    assert_eq!(snapshot.verdict, GoPromotedSnapshotVerdict::ProfileConflict);
    snapshot
}

#[test]
fn promoted_snapshot_unique_single_variant_resolves_resolver_and_manifest() {
    assert_positive(&depth_one_fixture(), "q/types.go");
}

#[test]
fn promoted_snapshot_depth_two_resolves_when_stored_snapshot_lists_it() {
    let cg = deferred_fixture(
        "package q\ntype C struct{}\nfunc (C) M() {}\ntype B struct{ C }\ntype S struct{ B }\n",
    );
    let snapshot = owner_snapshot(&cg, "S");
    let promoted = snapshot.declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "M")
        .expect("stored depth-two M");
    assert_eq!(promoted.depth, 2);
    assert_positive(&cg, "q/types.go");
}

#[test]
fn promoted_snapshot_pointer_method_on_value_receiver_resolves_without_shape_check() {
    let cg = deferred_fixture("package q\ntype B struct{}\nfunc (*B) M() {}\ntype S struct{ B }\n");
    let snapshot = owner_snapshot(&cg, "S");
    let promoted = snapshot.declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "M")
        .expect("stored pointer-receiver M");
    assert!(!promoted.value_method_set);
    assert_positive(&cg, "q/types.go");
}

#[test]
fn promoted_snapshot_embedded_target_qualifier_conflict_stays_dropped() {
    let donor = build_go_module(&[
        ("q/b.go", "package q\ntype B struct{}\nfunc (B) M() {}\n"),
        ("r/b.go", "package r\ntype B struct{}\nfunc (B) M() {}\n"),
        (
            "outer/s_linux.go",
            "package outer\nimport q \"example.test/root/q\"\ntype S struct{ q.B }\n",
        ),
        (
            "outer/s_windows.go",
            "package outer\nimport r \"example.test/root/r\"\ntype S struct{ r.B }\n",
        ),
    ]);
    let cg = graft_deserialized_owner(depth_one_fixture(), assert_conflict_donor(&donor));
    assert_negative(&cg, Some("go_promoted_snapshot_conflict_drop"));
}

#[test]
fn promoted_snapshot_conflict_dual_counts_and_hit_conservation_hold() {
    let donor = build_go(&[
        ("base.go", "package p\ntype B struct{}\nfunc (B) M() {}\n"),
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B; M int }\n"),
    ]);
    let conflict = graft_deserialized_owner(depth_one_fixture(), assert_conflict_donor(&donor));
    let conflict_outcome = conflict.resolve_call_site_full(site(&conflict));
    assert_eq!(
        conflict_outcome
            .telemetry
            .go_promoted_snapshot_conflict_drop,
        1
    );
    assert_eq!(
        conflict_outcome
            .telemetry
            .go_concrete_receiver_promoted_deferred,
        1
    );
    let conflict_stats = crate::navigation::queries::call_stats(&conflict);
    assert_eq!(conflict_stats["go_promoted_snapshot_conflict_drop"], 1);
    assert_eq!(conflict_stats["go_concrete_receiver_promoted_deferred"], 1);

    let mut control = depth_one_fixture();
    let owner = receiver_owner(&control);
    control.go_promoted_selector_snapshot.owners.remove(&owner);
    let control_stats = crate::navigation::queries::call_stats(&control);
    let candidate_stats = crate::navigation::queries::call_stats(&depth_one_fixture());
    assert_eq!(
        control_stats["go_concrete_receiver_promoted_deferred"]
            .as_u64()
            .expect("control deferred counter"),
        candidate_stats["go_concrete_receiver_promoted_deferred"]
            .as_u64()
            .expect("candidate deferred counter")
            + candidate_stats["go_promoted_snapshot_hits"]
                .as_u64()
                .expect("candidate snapshot hit counter")
    );
}

#[test]
fn promoted_snapshot_ordinary_field_profile_conflict_stays_dropped() {
    let donor = build_go(&[
        ("base.go", "package p\ntype B struct{}\nfunc (B) M() {}\n"),
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B; M func() }\n"),
    ]);
    let cg = graft_deserialized_owner(depth_one_fixture(), assert_conflict_donor(&donor));
    assert_negative(&cg, Some("go_promoted_snapshot_conflict_drop"));
}

#[test]
fn promoted_snapshot_own_method_profile_conflict_stays_dropped() {
    let donor = build_go(&[
        ("base.go", "package p\ntype B struct{}\nfunc (B) M() {}\n"),
        (
            "s_linux.go",
            "package p\ntype S struct{ B }\nfunc (S) M() {}\n",
        ),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    let cg = graft_deserialized_owner(depth_one_fixture(), assert_conflict_donor(&donor));
    assert_negative(&cg, Some("go_promoted_snapshot_conflict_drop"));
}

#[test]
fn promoted_snapshot_embedded_alias_selector_profile_conflict_stays_dropped() {
    let donor = build_go(&[
        ("base.go", "package p\ntype B struct{}\nfunc (B) M() {}\n"),
        ("alias.go", "package p\ntype A = B\n"),
        ("s_linux.go", "package p\ntype S struct{ A }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    let cg = graft_deserialized_owner(depth_one_fixture(), assert_conflict_donor(&donor));
    assert_negative(&cg, Some("go_promoted_snapshot_conflict_drop"));
}

#[test]
fn promoted_snapshot_multiple_profile_variants_stay_dropped() {
    let donor = build_go(&[
        (
            "b_linux.go",
            "package p\ntype B struct{}\nfunc (B) M() {}\n",
        ),
        (
            "b_windows.go",
            "package p\ntype B struct{}\nfunc (B) M() {}\n",
        ),
        ("s.go", "package p\ntype S struct{ B }\n"),
    ]);
    let snapshot = owner_snapshot(&donor, "S");
    let promoted = snapshot.declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "M")
        .expect("profile-variant M");
    assert!(promoted.profile_variants.len() > 1);
    let cg = graft_deserialized_owner(depth_one_fixture(), snapshot);
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}

#[test]
fn promoted_snapshot_field_shadowed_flag_stays_dropped() {
    let donor = build_go(&[
        (
            "base.go",
            "package p\ntype D struct{}\nfunc (D) M() {}\ntype B struct{ D }\ntype C struct{ M int }\ntype S struct{ B; C }\n",
        ),
    ]);
    let snapshot = owner_snapshot(&donor, "S");
    assert!(snapshot.declarations[0]
        .promoted_methods
        .iter()
        .any(|method| method.method == "M" && method.field_shadowed));
    let cg = graft_deserialized_owner(depth_one_fixture(), snapshot);
    assert_negative(&cg, None);
}

#[test]
fn promoted_snapshot_equal_depth_field_method_collision_stays_dropped() {
    let donor = build_go(&[(
        "types.go",
        "package p\ntype A struct{}\nfunc (A) M() {}\ntype B struct{ M int }\ntype T struct{ A; B }\n",
    )]);
    let snapshot = owner_snapshot(&donor, "T");
    assert!(
        snapshot.declarations[0]
            .promoted_methods
            .iter()
            .all(|method| method.method != "M" || method.field_shadowed),
        "equal-depth field must leave no usable promoted M: {snapshot:#?}"
    );
    let cg = graft_deserialized_owner(depth_one_fixture(), snapshot);
    assert_negative(&cg, None);
}

#[test]
fn promoted_snapshot_ambiguous_method_flag_stays_dropped() {
    let donor = build_go(&[(
        "types.go",
        "package p\ntype B struct{}\nfunc (B) M() {}\ntype C struct{}\nfunc (C) M() {}\ntype S struct{ B; C }\n",
    )]);
    let snapshot = owner_snapshot(&donor, "S");
    assert!(snapshot.declarations[0]
        .ambiguous_promoted_methods
        .contains("M"));
    let cg = graft_deserialized_owner(depth_one_fixture(), snapshot);
    assert_negative(&cg, None);
}

#[test]
fn promoted_snapshot_zero_declarations_deserialized_invariant_stays_dropped() {
    let cg = mutate_deserialized_owner(depth_one_fixture(), |snapshot| {
        snapshot["declarations"] = serde_json::json!([]);
    });
    assert_negative(&cg, Some("go_promoted_snapshot_invariant_drop"));
}

#[test]
fn promoted_snapshot_two_declarations_deserialized_invariant_stays_dropped() {
    let cg = mutate_deserialized_owner(depth_one_fixture(), |snapshot| {
        let declaration = snapshot["declarations"][0].clone();
        snapshot["declarations"] = serde_json::json!([declaration.clone(), declaration]);
    });
    assert_negative(&cg, Some("go_promoted_snapshot_invariant_drop"));
}

#[test]
fn promoted_snapshot_empty_profile_variants_deserialized_stays_dropped() {
    let cg = mutate_deserialized_owner(depth_one_fixture(), |snapshot| {
        snapshot["declarations"][0]["promoted_methods"][0]["profile_variants"] =
            serde_json::json!([]);
    });
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}

#[test]
fn promoted_snapshot_singleton_variant_not_target_deserialized_stays_dropped() {
    let cg = mutate_deserialized_owner(depth_one_fixture(), |snapshot| {
        let mut other = snapshot["declarations"][0]["promoted_methods"][0]["target"].clone();
        other["file"] = serde_json::json!("q/other.go");
        snapshot["declarations"][0]["promoted_methods"][0]["profile_variants"] =
            serde_json::json!([other]);
    });
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}

#[test]
fn promoted_snapshot_generic_receiver_declaration_stays_dropped() {
    let donor = build_go(&[(
        "q/generic.go",
        "package q\ntype B[T any] struct{}\nfunc (B[T]) M() {}\n",
    )]);
    let (owner, declaration) = donor
        .go_method_declarations
        .iter()
        .flat_map(|(owner, declarations)| {
            declarations
                .iter()
                .map(move |declaration| (owner, declaration))
        })
        .find(|(_, declaration)| declaration.method_name == "M" && declaration.generic)
        .expect("generic receiver method declaration");
    let target = declaration.function_id.clone();
    let target_value = serde_json::to_value(&target).expect("serialize generic target");
    let mut cg = mutate_deserialized_owner(depth_one_fixture(), |snapshot| {
        let promoted = &mut snapshot["declarations"][0]["promoted_methods"][0];
        promoted["target"] = target_value.clone();
        promoted["profile_variants"] = serde_json::json!([target_value]);
    });
    cg.go_method_declarations
        .entry(owner.clone())
        .or_default()
        .insert(declaration.clone());
    assert!(cg
        .go_method_declarations
        .values()
        .flatten()
        .any(|declaration| declaration.function_id == target && declaration.generic));
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}

#[test]
fn promoted_snapshot_declaration_join_miss_deserialized_stays_dropped() {
    let cg = mutate_deserialized_owner(depth_one_fixture(), |snapshot| {
        let promoted = &mut snapshot["declarations"][0]["promoted_methods"][0];
        let mut missing = promoted["target"].clone();
        missing["file"] = serde_json::json!("q/missing.go");
        promoted["target"] = missing.clone();
        promoted["profile_variants"] = serde_json::json!([missing]);
    });
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}

#[test]
fn promoted_snapshot_missing_signature_declaration_stays_dropped() {
    let cg = mutate_target_declaration(depth_one_fixture(), |declaration| {
        declaration.signature = None;
    });
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}

#[test]
fn promoted_snapshot_non_unique_function_id_join_stays_dropped() {
    let mut cg = depth_one_fixture();
    let target = promoted_target(&cg);
    let duplicate = cg
        .go_method_declarations
        .values()
        .flatten()
        .find(|declaration| declaration.function_id == target)
        .expect("target declaration")
        .clone();
    cg.go_method_declarations
        .entry(crate::resolution::GoOwnerIdentity {
            package_dir: "duplicate".to_string(),
            package_clause: "duplicate".to_string(),
            name: "Duplicate".to_string(),
        })
        .or_default()
        .insert(duplicate);
    assert_negative(&cg, Some("go_promoted_snapshot_variant_drop"));
}
