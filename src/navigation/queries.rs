use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::cpg::{CpgEdge, CpgNode};
use crate::navigation::seed;
use crate::navigation::types::*;
use crate::navigation::NavigationSession;
use crate::resolution::ResolutionConfidence;
use crate::resolution_identity::{resolve_type_path_to_type_scope, TypeKey};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    crate::name_resolution::glob_stats::GLOBAL.reset();

    use crate::resolution::{DropReason, ResolutionConfidence};

    use crate::call_graph::MethodKind;

    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut demoted = 0usize;
    let mut total = 0usize;
    let (mut multi, mut external, mut import_ext, mut unknown) = (0usize, 0usize, 0usize, 0usize);
    let mut func_value_fanout = 0usize;
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
    for sites in cg.calls.values() {
        for site in sites {
            total += 1;
            let out = cg.resolve_call_site_full(site);
            match out.drop {
                Some(DropReason::MultiOwnerCollision) => multi += 1,
                Some(DropReason::ExternalReceiver) => external += 1,
                Some(DropReason::ImportExternal) => import_ext += 1,
                Some(DropReason::UnknownName) => unknown += 1,
                Some(DropReason::FuncValueFanout) => func_value_fanout += 1,
                None => {}
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
    let ge = crate::name_resolution::glob_stats::GLOBAL.snapshot();
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

    serde_json::json!({
        "total_call_sites": total,
        "kinds": kinds,
        "demoted_edges": demoted,
        "dropped_multi_owner": multi,
        "dropped_external_receiver": external,
        "dropped_import_external": import_ext,
        "unresolved_unknown_name": unknown,
        "dropped_func_value_fanout": func_value_fanout,
        "callback_registrations_recorded": cg.go_registrations.len(),
        "callback_registration_shadowed_skips": cg.go_registration_shadowed_skips,
        "callback_registration_ambiguous_owner_skips": cg.go_registration_ambiguous_owner_skips,
        "callback_registration_unknown_owner_recorded": cg.go_registration_unknown_owner_recorded,
        "property_accesses_recorded": cg.property_accesses.len(),
        "property_access_fanout_skips": cg.property_access_fanout_skips,
        "property_access_store_skips": cg.property_access_store_skips,
        "property_access_cached_property_recorded": property_access_cached_property_recorded,
        // P4: JS/TS export-fact re-export chain/barrel telemetry (js_exports::
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
    })
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
    use crate::resolution::ReceiverRecovery;
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
        ReceiverRecovery::FieldTyped
        | ReceiverRecovery::ReturnTyped
        | ReceiverRecovery::StdWrapperPeel
        | ReceiverRecovery::TypedLet => "rust_receiver",
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
            // Slice E: emit the minted implementer SET (owner type names), not just the
            // count. The fanned-out FunctionIds are mapped to their owning type via
            // method_owners (fall back to the FunctionId's file stem if absent — a method
            // with no recorded owner is keyed by its file). Deduped + sorted so the wire
            // shape is deterministic and `fanout == implementers.len()`. A concrete
            // (fanout == 0) receiver yields the empty set.
            let impls: &[FunctionId] = crate::resolution::iface_key(recv_ty)
                .and_then(|k| cg.interface_impls.get(&(k, site.callee_name.clone())))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            // Arity-disambiguate the name-keyed candidate set BEFORE the owner-name
            // mapping, so `fanout` (= implementers cardinality) reflects the filtered
            // set the resolver would mint. Same shared helper as the resolution mint;
            // an emptied set yields implementers: [] / fanout: 0. The oracle reads this
            // manifest, so the filter MUST run here too, not just in resolution.rs.
            let kept = crate::resolution::arity_filter(
                impls,
                site.arg_count,
                site.arg_spread,
                &cg.method_arity,
            );
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
            sites.push(serde_json::json!({
                "file": site.caller.file,
                "start_byte": site.start_byte,
                "end_byte": site.end_byte,
                "line": site.line,
                "receiver_class": class(recovery),
                "method": site.callee_name,
                "fanout": implementers.len(),
                "implementers": implementers,
            }));
        }
    }
    // `interface_dispatch_computed` (review MINOR 6): false on a raw build_direct_subset
    // graph (apply_go_interface_dispatch never ran) → an empty `sites` means "not computed",
    // not "no dispatch found". The CLI feeds a full-build graph, so this is true in practice.
    serde_json::json!({
        "sites": sites,
        "interface_dispatch_computed": cg.interface_dispatch_computed,
    })
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
    for idx in s.index.cpg.nodes_at(file, line) {
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
            CpgNode::Statement { .. } => {} // statements not first-class in v1 (spec §8 R3-M3)
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
                // One item PER CALL SITE (m7 symmetry with callees); `visited` only gates BFS recursion.
                items.push(SortableEvidenceItem {
                    item: EvidenceItem {
                        symbol: Some(SymbolRef::Function {
                            file: caller.file.clone(),
                            name: caller.name.clone(),
                            start_line: caller.start_line,
                            end_line: caller.end_line,
                            start_byte,
                            end_byte,
                            ordinal: 0,
                        }),
                        location: Location {
                            file: caller.file.clone(),
                            start_line: caller.start_line,
                            end_line: caller.end_line,
                            start_byte,
                            end_byte,
                        },
                        score: confidence_score(edge.confidence) / (1.0 + hop as f32),
                        source: Source::PrismCpg,
                        fallback: false,
                        why: vec![Reason::CalledBy {
                            caller: caller.name.clone(),
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
        CpgEdge::DataFlow => "DataFlow",
        CpgEdge::ControlFlow => "ControlFlow",
        CpgEdge::Call(_) => "Call",
        CpgEdge::Return(_) => "Return",
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
            SymbolRef::Function {
                file: file.clone(),
                name: name.clone(),
                start_line: *start_line,
                end_line: *end_line,
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
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
        let mut nodes = s.index.cpg.nodes_at(&f, line);
        nodes.sort_by_key(|i| i.index());
        nodes.dedup();
        if nodes.is_empty() {
            let (idx, _) =
                s.index
                    .enclosing_function(&f, line)
                    .ok_or(QueryError::LocationOutOfRange {
                        file: f.clone(),
                        line,
                    })?;
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
