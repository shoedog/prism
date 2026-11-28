//! `call_stats`: resolution-kind histogram and drop classification over a call graph.

use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::resolution_identity::{resolve_type_path_to_type_scope, TypeKey};
use std::collections::{BTreeMap, BTreeSet};

/// Shape of a multi-target-Exact call site — what *kind* of receiver/qualifier
/// minted the colliding pool. This decides which lever could address it:
/// `type_path` (`T::m`) is the only shape the proposed Option-B owner-key
/// narrowing targets; `receiver_typed` (`x.m()` recovered) is a different path
/// (the receiver `methods_by_scope` hook); `qualifier_field` is the ambiguous
/// `pkg.f()`/`x.m()` form (`CallSite.qualifier` cannot tell type from value).
fn multi_target_shape(site: &CallSite) -> &'static str {
    if site.callee_name.contains("::") {
        let mut segs: Vec<&str> = site.callee_name.split("::").collect();
        if segs.pop().is_none() {
            return "unshaped";
        }
        while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
            segs.remove(0);
        }
        if segs.as_slice() == ["self"] || segs.last() == Some(&"Self") {
            return "self_path";
        }
        if segs.is_empty() {
            return "unshaped";
        }
        return "type_path";
    }
    if site.receiver_outcome.is_some() {
        return "receiver_typed";
    }
    if site.qualifier.is_some() {
        return "qualifier_field";
    }
    "unshaped"
}

/// Shadow (measurement-only) of the Option-B narrowing for a genuine `T::m`
/// type-path site: resolve the owner type-path through the scope graph and report
/// whether `methods_by_scope` would narrow the colliding pool to a singleton (the
/// realized precision win), to a still-multiple set (would demote to NameOnly), or
/// cannot (fail-open, split by cause). Only called for `type_path` sites — never
/// changes resolution behavior.
fn shadow_narrow_type_path(cg: &CallGraph, site: &CallSite) -> &'static str {
    let mut segs: Vec<&str> = site.callee_name.split("::").collect();
    let Some(method) = segs.pop() else {
        return "failopen_type_unresolved";
    };
    while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
        segs.remove(0);
    }
    let owner_syntax = segs.join("::");
    let Some(graph) = cg.scope_graph.as_ref() else {
        return "failopen_no_graph";
    };
    let resolved = graph
        .file_paths
        .get(&site.caller.file)
        .copied()
        .and_then(|file| CallGraph::module_scope_for_byte(graph, file, site.start_byte))
        .and_then(|from| resolve_type_path_to_type_scope(graph, from, &owner_syntax));
    match resolved {
        Some(TypeKey::InRepo(scope)) => {
            match cg
                .methods_by_scope
                .get(&(scope, method.to_string()))
                .map(|c| c.len())
            {
                Some(1) => "singleton",
                Some(n) if n > 1 => "multiple",
                _ => "failopen_no_method",
            }
        }
        _ => "failopen_type_unresolved",
    }
}

/// Recovery instrument (spec §7 / review MAJOR 5). For an owner-method `T::m`
/// site, report what the disproof prune ACTUALLY decided (re-derived measurement-
/// only, exactly as `shadow_narrow_type_path` re-derives the narrowing — never
/// read from final edge counts, which cannot tell a prune-demote from a fail-open
/// demote):
///   `singleton`        — the prune disproved down to a single survivor (the
///                        recovered Exact: ①C+②B held and the id-set pinned one).
///   `pruned_multiple`  — the prune disproved ≥1 but >1 survived (a real prune
///                        that still demotes to NameOnly).
///   `failopen_singleton` — the predicate proved nothing; the bare pool is a
///                        singleton (the #120 floor mints Exact — not a recovery).
///   `failopen_demote`  — the predicate proved nothing; the bare pool collides
///                        (the #120 floor demotes to NameOnly — the un-recovered
///                        residue this slice aims to shrink).
///   `not_owner_method` — no bare `(owner, method)` pool / unresolvable scope.
/// Keyed off the owner-`::` population, not the >=2-Exact population the legacy
/// `shadow_typepath_narrow` requires.
fn classify_recovery_typepath(cg: &CallGraph, site: &CallSite) -> &'static str {
    use crate::name_resolution::rust_populator::enclosing_scope;
    use crate::resolution::{prune, DisproofCx, DisproofPredicate, ScopeResolution};
    // Owner-method key `(T, m)` from `mod::T::m` (mirror the resolver's split;
    // `crate`/`super` stripped, `self`/`Self` heads excluded).
    let mut segs: Vec<&str> = site.callee_name.split("::").collect();
    let Some(method) = segs.pop() else {
        return "not_owner_method";
    };
    while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
        segs.remove(0);
    }
    let Some(&owner) = segs.last() else {
        return "not_owner_method";
    };
    if owner == "self" || owner == "Self" {
        return "not_owner_method";
    }
    let Some(pool_ids) = cg.methods.get(&(owner.to_string(), method.to_string())) else {
        return "not_owner_method";
    };
    // Re-derive the authoritative (file, enclosing-scope) the prune ran from. If
    // the graph is absent/incomplete or the byte has no scope, this site never
    // reached the prune -> not a recovery outcome.
    let Some(graph) = cg.scope_graph.as_ref() else {
        return "not_owner_method";
    };
    if !graph.complete {
        return "not_owner_method";
    }
    let Some(file) = graph.file_paths.get(&site.caller.file).copied() else {
        return "not_owner_method";
    };
    let Some(from) = enclosing_scope(graph, file, site.start_byte) else {
        return "not_owner_method";
    };
    let pool: Vec<&FunctionId> = pool_ids.iter().collect();
    let pred = ScopeResolution::new(cg);
    let cx = DisproofCx { graph, file, from };
    let kept = prune(pool.clone(), site, &cx, &[&pred as &dyn DisproofPredicate]);
    if kept.len() < pool.len() {
        // The predicate disproved at least one candidate (a real prune).
        if kept.len() == 1 {
            "singleton"
        } else {
            "pruned_multiple"
        }
    } else if pool.len() == 1 {
        "failopen_singleton"
    } else {
        "failopen_demote"
    }
}

pub fn call_stats(cg: &CallGraph) -> serde_json::Value {
    // Glob-expansion telemetry is scoped to the graph this pass resolves over,
    // never a process-global: the reset→snapshot window below is otherwise open
    // to any concurrent resolution elsewhere in the process (a `call_stats` on a
    // different graph, `call_site_dump`, another repo's index build), which is
    // exactly the CI flake this scoping fixed. A `None` scope_graph means no
    // scope graph exists, hence no glob expansion to count.
    let glob_stats = cg.scope_graph.as_ref().map(|g| &g.glob_stats);
    if let Some(stats) = glob_stats {
        stats.reset();
    }

    use crate::resolution::{DropReason, ResolutionConfidence};

    use crate::call_graph::MethodKind;

    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut demoted = 0usize;
    let mut total = 0usize;
    let (mut multi, mut external, mut import_ext, mut unknown) = (0usize, 0usize, 0usize, 0usize);
    let mut func_value_fanout = 0usize;
    let mut go_same_pkg_all_filtered_drop = 0usize;
    let mut go_pkg_clause_partition_exact = 0usize;
    let mut go_build_partition_exact = 0usize;
    let mut go_concrete_receiver_direct = 0usize;
    let mut go_concrete_receiver_promoted_existing = 0usize;
    let mut go_concrete_receiver_promoted_deferred = 0usize;
    let mut go_promoted_snapshot_hits = 0usize;
    let mut go_promoted_snapshot_conflict_drop = 0usize;
    let mut go_promoted_snapshot_variant_drop = 0usize;
    let mut go_promoted_snapshot_invariant_drop = 0usize;
    let mut go_concrete_receiver_no_selector_drop = 0usize;
    let mut go_r2_on_demand_name_collision_bail = 0usize;
    let mut go_external_receiver_new_recovery_drop = 0usize;
    let mut go_unproven_receiver_bare_fallback_sites = 0usize;
    let mut go_unproven_receiver_bare_fallback_hits = 0usize;
    let mut go_unproven_receiver_bare_fallback_edges = 0usize;
    let mut go_bare_value_ref_ambiguous = cg.go_bare_value_ref_ambiguous;
    let mut go_build_expr_unparsed: usize = cg.go_build_profile_unparsed.values().sum();
    let mut go_owner_identity_partition =
        crate::resolution::go::owner_partition::GoOwnerPartitionTelemetry::default();
    // Phase-3 stratification (re-measure for slice scoping): split each kind by
    // confidence, and stratify NameOnly demotes by (recovery, method-kind). This
    // isolates the #2-addressable universe — a NameOnly demote from `combine_kind`'s
    // single-candidate arm is a *receiver-recovery* kind (TypedParam/ConstructorLocal/
    // FieldTyped/ReturnTyped/TypedLet…) on a *trait* method — from the R6SingleOwner
    // residue + TraitCha multi (recovery "none", not combine_kind) and the #4
    // wrapper-peel `clone` edges. NOTE: the #2 count is an UPPER bound — it still
    // includes `dyn Trait` / trait-scope receivers that Slice 1 must exclude.
    let mut kind_exact: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut kind_nameonly: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut nameonly_recovery_mk: BTreeMap<String, usize> = BTreeMap::new();
    let mut wrapper_peel_clone = 0usize;
    let mut r6_rust = 0usize;
    // Same-site Exact over-attribution: a call site that resolves to >1 callee all
    // at Exact (1.0). The dominant precision-FP class this surfaces is the
    // same-bare-name owner-key collision (e.g. two distinct `Foo` structs both
    // owning `Foo::make` conflate under the bare `("Foo","make")` key and BOTH
    // resolve Exact — see resolution.rs `owner_lookup_in_modules`). NameOnly
    // (TraitCha) fanout is excluded — those are already demoted, not full-confidence.
    let mut multi_target_exact_sites = 0usize;
    let mut multi_target_exact_fanout: BTreeMap<usize, usize> = BTreeMap::new();
    let mut multi_target_exact_by_kind: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Pre-gate shadow over the multi-target-Exact set: stratify each site by SHAPE
    // (which lever could address it), then for genuine `type_path` (`T::m`) sites
    // run the Option-B scope-graph narrowing shadow (singleton = realized win,
    // multiple = would-demote, failopen_* = pool unchanged, split by cause).
    let mut multi_target_exact_shape: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut shadow_typepath_narrow: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Forward recovery instrument (spec §7): classify each owner-`::` `T::m` site
    // by what the scope path now yields, keyed off the qualified_owner population
    // (the demoted-NameOnly + recovered-Exact sites #120/this slice produce),
    // independent of the legacy >=2-Exact shadow guard.
    let mut recovery_typepath: BTreeMap<&'static str, usize> = BTreeMap::new();
    // P11 S5: Go drop-attribution stratification. `call_stats`'s existing
    // drop counters (multi/external/import_ext/unknown/func_value_fanout) are
    // language-blind — a Go-caller site that drops still lands in exactly one
    // of those buckets, but there's no way to tell "receiver recovered, then
    // still dropped downstream" from "no receiver recovery at all" without
    // this. Keyed by `site.receiver_recovery` (Debug-formatted; "none" when
    // absent) so Go misses stop landing unattributed.
    let mut dropped_go_receiver: BTreeMap<String, usize> = BTreeMap::new();
    let mut wrapped_non_jsx = 0usize;
    let mut jsx_intrinsic = 0usize;
    let mut local_binding_unproven = 0usize;
    // S1b-3b (SPEC §5): bindings kept at base behavior, by reason, observable regardless of
    // the resolution outcome a site lands on (a `MayCall`/`Position` site can still drop for
    // an unrelated reason downstream, for example `UnknownName`).
    let mut local_binding_unchecked_position: BTreeMap<String, usize> = BTreeMap::new();
    let mut local_binding_may_call: BTreeMap<String, usize> = BTreeMap::new();
    // Slice L: followed `const` aliases by terminal kind and outcome (`proven`: the alias route
    // bound exactly one Exact target; `kept_main`: it did not, and the site's records are
    // those of an unfollowed alias, i.e. main's).
    let mut js_alias_follow: BTreeMap<String, usize> = BTreeMap::new();
    for sites in cg.calls.values() {
        for site in sites {
            total += 1;
            match &site.local_binding {
                crate::call_graph::JsLocalBinding::Position(r) => {
                    *local_binding_unchecked_position
                        .entry(r.clone())
                        .or_default() += 1
                }
                crate::call_graph::JsLocalBinding::MayCall(r) => {
                    *local_binding_may_call.entry(r.clone()).or_default() += 1
                }
                followed if followed.is_followed() => {
                    let hit = cg.js_ts_alias_route_hit(site);
                    let kind = match followed {
                        crate::call_graph::JsLocalBinding::AliasCallable(_) => "callable",
                        crate::call_graph::JsLocalBinding::AliasImport { .. } => "import",
                        crate::call_graph::JsLocalBinding::RequireModule { .. } => {
                            // #598: an unproven bare callee still counts under the `MayCall`
                            // reason it had before the binding existed.
                            if !hit && site.qualifier.is_none() {
                                *local_binding_may_call
                                    .entry("alias".to_string())
                                    .or_default() += 1;
                            }
                            "cjs_module"
                        }
                        crate::call_graph::JsLocalBinding::Returned { kept, .. } => {
                            // Slice V: a returned-value site the fact does not prove still
                            // counts under the base reason it always did.
                            if !hit {
                                *local_binding_may_call.entry(kept.clone()).or_default() += 1;
                            }
                            "returned"
                        }
                        _ => "namespace_member",
                    };
                    let outcome = if hit { "proven" } else { "kept_main" };
                    *js_alias_follow
                        .entry(format!("{kind}_{outcome}"))
                        .or_default() += 1
                }
                _ => {}
            }
            let out = cg.resolve_call_site_full(site);
            match out.drop {
                Some(DropReason::MultiOwnerCollision) => multi += 1,
                Some(DropReason::ExternalReceiver) => external += 1,
                Some(DropReason::ImportExternal) => import_ext += 1,
                Some(DropReason::UnknownName) => unknown += 1,
                Some(DropReason::FuncValueFanout) => func_value_fanout += 1,
                Some(DropReason::GoSamePkgAllFiltered) => go_same_pkg_all_filtered_drop += 1,
                Some(DropReason::WrappedExportNonJsx) => wrapped_non_jsx += 1,
                Some(DropReason::JsxIntrinsic) => jsx_intrinsic += 1,
                Some(DropReason::LocalBindingUnproven) => local_binding_unproven += 1,
                Some(
                    DropReason::ConcreteReceiverPromotedDeferred
                    | DropReason::ConcreteReceiverNoSelector,
                ) => {}
                None => {}
            }
            go_pkg_clause_partition_exact += out.telemetry.go_pkg_clause_partition_exact;
            go_build_partition_exact += out.telemetry.go_build_partition_exact;
            go_bare_value_ref_ambiguous += out.telemetry.go_bare_value_ref_ambiguous;
            go_build_expr_unparsed += out.telemetry.go_build_expr_unparsed;
            go_concrete_receiver_direct += out.telemetry.go_concrete_receiver_direct;
            go_concrete_receiver_promoted_existing +=
                out.telemetry.go_concrete_receiver_promoted_existing;
            go_concrete_receiver_promoted_deferred +=
                out.telemetry.go_concrete_receiver_promoted_deferred;
            go_promoted_snapshot_hits += out.telemetry.go_promoted_snapshot_hits;
            go_promoted_snapshot_conflict_drop += out.telemetry.go_promoted_snapshot_conflict_drop;
            go_promoted_snapshot_variant_drop += out.telemetry.go_promoted_snapshot_variant_drop;
            go_promoted_snapshot_invariant_drop +=
                out.telemetry.go_promoted_snapshot_invariant_drop;
            go_concrete_receiver_no_selector_drop +=
                out.telemetry.go_concrete_receiver_no_selector_drop;
            go_r2_on_demand_name_collision_bail +=
                out.telemetry.go_r2_on_demand_name_collision_bail;
            go_external_receiver_new_recovery_drop +=
                out.telemetry.go_external_receiver_new_recovery_drop;
            go_unproven_receiver_bare_fallback_sites +=
                out.telemetry.go_unproven_receiver_bare_fallback_sites;
            go_unproven_receiver_bare_fallback_hits +=
                out.telemetry.go_unproven_receiver_bare_fallback_hits;
            go_unproven_receiver_bare_fallback_edges +=
                out.telemetry.go_unproven_receiver_bare_fallback_edges;
            let receiver_partition = cg
                .go_owner_identity_partition_sites
                .get(&crate::resolution::go::owner_partition::site_key(site))
                .copied()
                .unwrap_or_default();
            let site_partition =
                receiver_partition.coalesce_site(out.telemetry.go_owner_identity_partition);
            crate::navigation::partition_site_dump::emit_if_enabled(site, &out, site_partition);
            go_owner_identity_partition.merge(site_partition);
            if out.drop.is_some() && site.caller.file.ends_with(".go") {
                let key = site
                    .receiver_recovery
                    .map(|r| format!("{r:?}"))
                    .unwrap_or_else(|| "none".to_string());
                *dropped_go_receiver.entry(key).or_default() += 1;
            }
            if site.callee_name.contains("::") {
                let bucket = classify_recovery_typepath(cg, site);
                if bucket != "not_owner_method" {
                    *recovery_typepath.entry(bucket).or_default() += 1;
                }
            }
            let recovery = site
                .receiver_outcome
                .as_ref()
                .map(|o| format!("{:?}", o.recovery))
                .unwrap_or_else(|| "none".to_string());
            for c in &out.resolved {
                *kinds.entry(c.kind.as_str()).or_default() += 1;
                if c.kind.as_str() == "r6_single_owner" && site.caller.file.ends_with(".rs") {
                    r6_rust += 1;
                }
                if c.confidence == ResolutionConfidence::NameOnly {
                    demoted += 1;
                    *kind_nameonly.entry(c.kind.as_str()).or_default() += 1;
                    let mk = match cg.method_facts.get(c.target).map(|f| &f.kind) {
                        Some(MethodKind::Trait(_)) => "trait",
                        Some(MethodKind::Inherent) => "inherent",
                        None => "nonmethod",
                    };
                    *nameonly_recovery_mk
                        .entry(format!("{recovery}/{mk}"))
                        .or_default() += 1;
                    if recovery == "StdWrapperPeel" && c.target.name == "clone" {
                        wrapper_peel_clone += 1;
                    }
                } else {
                    *kind_exact.entry(c.kind.as_str()).or_default() += 1;
                }
            }
            let exact_kinds: Vec<&'static str> = out
                .resolved
                .iter()
                .filter(|c| c.confidence == ResolutionConfidence::Exact)
                .map(|c| c.kind.as_str())
                .collect();
            if exact_kinds.len() >= 2 {
                multi_target_exact_sites += 1;
                *multi_target_exact_fanout
                    .entry(exact_kinds.len())
                    .or_default() += 1;
                for k in exact_kinds.iter().copied().collect::<BTreeSet<_>>() {
                    *multi_target_exact_by_kind.entry(k).or_default() += 1;
                }
                let shape = multi_target_shape(site);
                *multi_target_exact_shape.entry(shape).or_default() += 1;
                if shape == "type_path" {
                    *shadow_typepath_narrow
                        .entry(shadow_narrow_type_path(cg, site))
                        .or_default() += 1;
                }
            }
        }
    }
    let ge = glob_stats.map(|stats| stats.snapshot()).unwrap_or_default();
    let mut interface_fanout: BTreeMap<usize, usize> = BTreeMap::new();
    for ids in cg.interface_impls.values() {
        *interface_fanout.entry(ids.len()).or_default() += 1;
    }

    // P5 S2 (re-review MAJOR-3): registrations are NOT `CallSite`s, so they
    // never flow through the `cg.calls`/`cg.calls` resolver-outcome loop
    // above. Count them explicitly into the same `kinds`/`kind_nameonly`/
    // `demoted` telemetry so callback_registration shows up in the standard
    // buckets, plus dedicated counters for the registration-build-time facts
    // that have no other way to reach call-stats.
    for _ in &cg.go_registrations {
        *kinds.entry("callback_registration").or_default() += 1;
        *kind_nameonly.entry("callback_registration").or_default() += 1;
        demoted += 1;
    }

    // P7 S3: Python property-access records are NOT `CallSite`s either (same
    // rationale as the go_registrations loop above) — count them explicitly
    // into the same `kinds`/`kind_nameonly`/`demoted` telemetry, plus
    // dedicated counters for the S1/S2 build-time facts that have no other
    // way to reach call-stats.
    let mut property_access_cached_property_recorded = 0usize;
    for acc in &cg.property_accesses {
        *kinds.entry("property_access").or_default() += 1;
        *kind_nameonly.entry("property_access").or_default() += 1;
        demoted += 1;
        if cg.cached_property_getters.contains(&acc.getter) {
            property_access_cached_property_recorded += 1;
        }
    }

    // P9 S3: framework-entry (Flask/FastAPI/Express route registration)
    // records are NOT `CallSite`s either (same rationale as the
    // go_registrations/property_accesses loops above) — count them
    // explicitly into the same `kinds`/`kind_nameonly`/`demoted` telemetry.
    for _ in &cg.framework_entries {
        *kinds.entry("framework_entry").or_default() += 1;
        *kind_nameonly.entry("framework_entry").or_default() += 1;
        demoted += 1;
    }

    // Built as its own value (not inlined) so the outer json!() call below stays
    // under the macro's recursion limit now that P4 added 2 more top-level keys.
    let glob_expand = serde_json::json!({
        "resolved_l1": ge.resolved_l1,
        "resolved_l2": ge.resolved_l2,
        "depth_exceeded": ge.depth_exceeded,
        "cycle": ge.cycle,
        "external": ge.external,
        "multi_target": ge.multi_target,
        "vis_unknown": ge.vis_unknown,
        "member_multi": ge.member_multi,
        "member_undecidable": ge.member_undecidable,
        "member_hidden_continued": ge.member_hidden_continued,
        "member_hidden_continue_hit": ge.member_hidden_continue_hit,
        "member_hidden_continue_empty": ge.member_hidden_continue_empty,
        "member_hidden_continue_poison": ge.member_hidden_continue_poison,
    });

    let mut decl_reasons: BTreeMap<String, usize> = BTreeMap::new();
    let (mut local_refusals, mut local_may_call) = (BTreeMap::<String, usize>::new(), 0);
    for facts in cg.js_ts_exports.values() {
        for (reason, n) in &facts.skipped_decl_reasons {
            *decl_reasons.entry(reason.clone()).or_default() += n;
        }
        for (reason, n) in &facts.local_export_refusals {
            *local_refusals.entry(reason.clone()).or_default() += n;
        }
        local_may_call += facts.local_export_may_call;
    }
    let mut stats = serde_json::json!({
        "total_call_sites": total,
        "kinds": kinds,
        "demoted_edges": demoted,
        "dropped_multi_owner": multi,
        "dropped_external_receiver": external,
        "dropped_import_external": import_ext,
        "unresolved_unknown_name": unknown,
        "dropped_func_value_fanout": func_value_fanout,
        "go_pkg_clause_partition_exact": go_pkg_clause_partition_exact,
        "go_build_partition_exact": go_build_partition_exact,
        "go_same_pkg_all_filtered_drop": go_same_pkg_all_filtered_drop,
        "go_bare_value_ref_ambiguous": go_bare_value_ref_ambiguous,
        "go_build_expr_unparsed": go_build_expr_unparsed,
        "go_owner_identity_profile_conflict": cg.go_owner_identity_profile_conflict,
        "go_alias_expanded": cg.go_alias_expanded,
        "go_alias_unresolved": cg.go_alias_unresolved,
        "go_promoted_snapshot_owners": cg.go_promoted_selector_snapshot().owners.len(),
        "go_promoted_snapshot_profile_conflicts": cg
            .go_promoted_selector_snapshot()
            .profile_conflicts(),
        "go_promoted_snapshot_promoted_methods": cg
            .go_promoted_selector_snapshot()
            .promoted_methods(),
        "dropped_go_receiver": dropped_go_receiver,
        "go_receiver_prereq_drops": cg.go_receiver_prereq_drops,
        "callback_registrations_recorded": cg.go_registrations.len(),
        "callback_registration_shadowed_skips": cg.go_registration_shadowed_skips,
        "callback_registration_ambiguous_owner_skips": cg.go_registration_ambiguous_owner_skips,
        "callback_registration_unknown_owner_recorded": cg.go_registration_unknown_owner_recorded,
        "property_accesses_recorded": cg.property_accesses.len(),
        "property_access_fanout_skips": cg.property_access_fanout_skips,
        "property_access_store_skips": cg.property_access_store_skips,
        "property_access_cached_property_recorded": property_access_cached_property_recorded,
        "framework_entries_recorded": cg.framework_entries.len(),
        "framework_entry_unresolved_handlers": cg.framework_entry_unresolved_handlers,
        // P8: Rust macro-argument call-extraction telemetry (call_graph::rust::macro_args)
        // -- per-file facts summed on demand, same pattern as
        // `js_export_skipped_exprs` below (`macro_arg_facts` is per-file-
        // derivable, so remove_files/merge keep it exactly correct with no
        // separate whole-program recompute step, unlike property_access_*
        // above).
        "macro_arg_calls_recorded": cg
            .macro_arg_facts
            .values()
            .map(|f| f.calls_recorded)
            .sum::<usize>(),
        "macro_arg_skipped_macros": cg
            .macro_arg_facts
            .values()
            .map(|f| f.skipped_macros)
            .sum::<usize>(),
        "macro_arg_ctor_skips": cg
            .macro_arg_facts
            .values()
            .map(|f| f.ctor_skips)
            .sum::<usize>(),
        // P4: JS/TS export-fact re-export chain/barrel telemetry (ast::js_ts::exports::
        // resolve_js_exports, depth-bounded at MAX_REEXPORT_DEPTH). The primary
        // signal is `kinds`/`kind_exact`/`kind_nameonly`'s "import_member" count
        // rising and `unresolved_unknown_name` dropping (no new ResolutionKind/
        // DropReason needed); these two counters cover the fail-closed cases
        // that leave no other trace (a chain that never resolves emits nothing
        // for R4c to count).
        "js_export_chain_unresolved": cg.js_export_chain_unresolved,
        "js_export_barrel_conflicts": cg.js_export_barrel_conflicts,
        // F6 (opus Minor 2, review-fix wave): aggregate per-file
        // `JsExportFacts::skipped_expr_count` -- populated but never
        // surfaced before this fix, and load-bearing now that F1-F4 added
        // more fail-closed skip paths (mutable destructured require is a
        // structural skip elsewhere, not counted here; spread-poisoned
        // literals, non-arrow/function-expr initializers, and arbitrary
        // default-export/CJS-assignment RHS all count here).
        // S1: span-verified wrapped React exports and declarator skip reasons (a
        // subset of `js_export_skipped_exprs`).
        "js_export_spanned_admitted": cg
            .js_ts_exports
            .values()
            .map(|f| f.spanned_admitted)
            .sum::<usize>(),
        "js_export_skipped_decl_reasons": decl_reasons,
        "js_export_local_refusals": local_refusals,
        "js_export_local_may_call": local_may_call,
        "dropped_wrapped_export_non_jsx": wrapped_non_jsx,
        // S1b: intrinsic JSX tags and unproven JS/TS lexical bindings.
        "dropped_jsx_intrinsic": jsx_intrinsic,
        "dropped_local_binding_unproven": local_binding_unproven,
        "local_binding_unchecked_position": local_binding_unchecked_position,
        "local_binding_may_call": local_binding_may_call,
        "js_alias_follow": js_alias_follow,
        "js_arg_follow": cg.js_anonymous.arg_follow.stats(),
        "js_export_skipped_exprs": cg
            .js_ts_exports
            .values()
            .map(|f| f.skipped_expr_count)
            .sum::<usize>(),
        "embedding_gaps": cg.embedding_gaps,
        "interface_gaps": cg.interface_gaps,
        "interface_overapprox": cg.interface_overapprox,
        "interface_fanout": interface_fanout,
        "kind_exact": kind_exact,
        "kind_nameonly": kind_nameonly,
        "nameonly_by_recovery_methodkind": nameonly_recovery_mk,
        "wrapper_peel_clone_demotes": wrapper_peel_clone,
        "r6_single_owner_rust": r6_rust,
        "multi_target_exact_sites": multi_target_exact_sites,
        "multi_target_exact_fanout": multi_target_exact_fanout,
        "multi_target_exact_by_kind": multi_target_exact_by_kind,
        "multi_target_exact_shape": multi_target_exact_shape,
        "shadow_typepath_narrow": shadow_typepath_narrow,
        "recovery_typepath": recovery_typepath,
        "glob_expand": glob_expand,
        "param_slots_unknown": cg.param_slots_unknown,
        "level3_indirect_resolved": cg.level3_indirect_resolved,
        "go_level3_b1_candidates": cg.go_level3_b1_telemetry.candidates,
        "go_level3_b1_exact_inbound_sites": cg.go_level3_b1_telemetry.exact_inbound_sites,
        "go_level3_b1_accepted_inbound_sites": cg.go_level3_b1_telemetry.accepted_inbound_sites,
        "go_level3_b1_unique_targets": cg.go_level3_b1_telemetry.unique_targets,
        "go_level3_b1_edges": cg.go_level3_b1_telemetry.edges,
        "go_level3_b1_drops": cg.go_level3_b1_telemetry.drops,
    });
    if cg.skipped_go_testdata_files > 0 {
        stats.as_object_mut().expect("call-stats object").insert(
            "skipped_go_testdata_files".to_string(),
            cg.skipped_go_testdata_files.into(),
        );
    }
    if cg.go_import_path_proven_files + cg.go_import_path_unproven_files > 0 {
        let object = stats.as_object_mut().expect("call-stats object");
        object.insert(
            "go_module_graph".to_string(),
            serde_json::json!({
                "modules": cg.go_module_graph.modules,
                "active": cg.go_module_graph.active,
                "replaces_parsed": cg.go_module_graph.replaces_parsed,
                "replaces_applied": cg.go_module_graph.replaces_applied,
                "workspace_invalid": cg.go_module_graph.workspace_invalid,
            }),
        );
        object.insert(
            "go_import_path_proven_files".to_string(),
            cg.go_import_path_proven_files.into(),
        );
        object.insert(
            "go_import_path_unproven_files".to_string(),
            cg.go_import_path_unproven_files.into(),
        );
        object.insert(
            "go_import_path_unproven_reasons".to_string(),
            serde_json::to_value(&cg.go_import_path_unproven_reasons)
                .expect("Go import-path reason histogram serializes"),
        );
    }
    if !cg.go_file_profiles.is_empty() {
        let object = stats.as_object_mut().expect("call-stats object");
        object.insert(
            "go_concrete_receiver_direct".to_string(),
            go_concrete_receiver_direct.into(),
        );
        object.insert(
            "go_concrete_receiver_promoted_existing".to_string(),
            go_concrete_receiver_promoted_existing.into(),
        );
        object.insert(
            "go_concrete_receiver_promoted_deferred".to_string(),
            go_concrete_receiver_promoted_deferred.into(),
        );
        object.insert(
            "go_promoted_snapshot_hits".to_string(),
            go_promoted_snapshot_hits.into(),
        );
        object.insert(
            "go_promoted_snapshot_conflict_drop".to_string(),
            go_promoted_snapshot_conflict_drop.into(),
        );
        object.insert(
            "go_promoted_snapshot_variant_drop".to_string(),
            go_promoted_snapshot_variant_drop.into(),
        );
        object.insert(
            "go_promoted_snapshot_invariant_drop".to_string(),
            go_promoted_snapshot_invariant_drop.into(),
        );
        object.insert(
            "go_concrete_receiver_no_selector_drop".to_string(),
            go_concrete_receiver_no_selector_drop.into(),
        );
        object.insert(
            "go_r2_on_demand_name_collision_bail".to_string(),
            go_r2_on_demand_name_collision_bail.into(),
        );
        object.insert(
            "go_external_receiver_new_recovery_drop".to_string(),
            go_external_receiver_new_recovery_drop.into(),
        );
        object.insert(
            "go_unproven_receiver_bare_fallback_sites".to_string(),
            go_unproven_receiver_bare_fallback_sites.into(),
        );
        object.insert(
            "go_unproven_receiver_bare_fallback_hits".to_string(),
            go_unproven_receiver_bare_fallback_hits.into(),
        );
        object.insert(
            "go_unproven_receiver_bare_fallback_edges".to_string(),
            go_unproven_receiver_bare_fallback_edges.into(),
        );
        object.insert(
            "go_owner_identity_partition_affected_owners".to_string(),
            cg.go_owner_identity_profile_conflict.into(),
        );
        object.insert(
            "go_owner_identity_partition_drop".to_string(),
            go_owner_identity_partition.drops.into(),
        );
        object.insert(
            "go_owner_identity_partition_recovered".to_string(),
            go_owner_identity_partition.recovered.into(),
        );
        object.insert(
            "go_owner_identity_partition_affected_sites".to_string(),
            go_owner_identity_partition.affected_sites().into(),
        );
        object.insert(
            "go_owner_identity_partition_affected_edges".to_string(),
            go_owner_identity_partition.affected_edges.into(),
        );
    }
    stats
}

#[cfg(test)]
#[path = "call_stats_tests.rs"]
mod tests;
