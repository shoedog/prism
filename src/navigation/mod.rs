pub mod cache;
pub mod call_edge_cache;
pub mod call_resolve;
pub mod code_context;
pub mod inventory;
pub mod module_graph;
mod partition_site_dump;
pub mod queries;
pub mod seed;
pub mod types;

use crate::call_graph::{CallGraph, CallKind, CallSite, CallSiteOrigin, FunctionId};
use crate::cpg::{CodePropertyGraph, CpgContext, CpgNode};
use crate::repo_loader::LoadedRepo;
use crate::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
use crate::type_provider::TypeRegistry;
use petgraph::graph::NodeIndex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
struct CallSiteKey {
    caller: FunctionId,
    callee_name: String,
    source_callee_name: Option<String>,
    line: usize,
    kind: CallKind,
    start_byte: usize,
    end_byte: usize,
    qualifier: Option<String>,
    receiver_type: Option<String>,
    receiver_owner_identity: Option<crate::resolution::GoOwnerIdentity>,
    receiver_recovery: Option<String>,
    receiver_materialized: bool,
    receiver_newly_recovered: bool,
    arg_count: Option<usize>,
    arg_spread: bool,
    receiver_outcome: Option<String>,
    origin: CallSiteOrigin,
    pre_resolved_target: Option<FunctionId>,
}

impl From<&CallSite> for CallSiteKey {
    fn from(site: &CallSite) -> Self {
        Self {
            caller: site.caller.clone(),
            callee_name: site.callee_name.clone(),
            source_callee_name: site.source_callee_name.clone(),
            line: site.line,
            kind: site.kind,
            start_byte: site.start_byte,
            end_byte: site.end_byte,
            qualifier: site.qualifier.clone(),
            receiver_type: site.receiver_type.clone(),
            receiver_owner_identity: site.receiver_owner_identity.clone(),
            receiver_recovery: site.receiver_recovery.map(|r| format!("{r:?}")),
            receiver_materialized: site.receiver_materialized,
            receiver_newly_recovered: site.receiver_newly_recovered,
            arg_count: site.arg_count,
            arg_spread: site.arg_spread,
            receiver_outcome: site.receiver_outcome.as_ref().map(|o| format!("{o:?}")),
            origin: site.origin,
            pre_resolved_target: site.pre_resolved_target.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IndexedResolvedTarget {
    pub target: FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

#[derive(Debug, Clone)]
struct ResolvedCallSiteOutcome {
    resolved: Vec<IndexedResolvedTarget>,
    drop: Option<DropReason>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IndexedOutgoingCallSite {
    pub callee_name: String,
    pub call_site_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub resolved: Vec<IndexedResolvedTarget>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct IndexedIncomingCall {
    pub caller: FunctionId,
    pub callee_name: String,
    pub call_site_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub qualifier: Option<String>,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct NavigationCallEdgeIndex {
    outgoing_by_caller: BTreeMap<FunctionId, Vec<IndexedOutgoingCallSite>>,
    incoming_by_target: BTreeMap<FunctionId, Vec<IndexedIncomingCall>>,
    multi_owner_collision_sites: BTreeSet<CallSiteKey>,
}

pub struct NavigationIndex {
    pub(crate) cpg: CodePropertyGraph,
    pub types: TypeRegistry,
    pub live_types: BTreeSet<String>,
    // nav-local, derived from CpgNode::Function (spec section 3):
    pub line_range_index: BTreeMap<String, Vec<(usize, usize, NodeIndex)>>,
    pub name_index: BTreeMap<(String, String), Vec<NodeIndex>>,
    resolved_call_edges: OnceLock<NavigationCallEdgeIndex>,
    call_edge_cache_store: Option<call_edge_cache::NavigationCallEdgeCacheStore>,
}

pub struct NavigationSession {
    pub repo: Arc<LoadedRepo>,
    pub index: Arc<NavigationIndex>,
}

impl NavigationIndex {
    /// Build a whole-repo index. Uses `CpgContext::build` (scope == None), never
    /// `build_scoped` (spec section 17 Step 3 / R3-M5). Moves the owned cpg/types/live_types
    /// out of the borrowing context so the index owns them.
    pub fn build(repo: &LoadedRepo) -> Self {
        let ctx = CpgContext::build_with_scope_graph_inputs(
            &repo.files,
            repo.type_db.as_ref(),
            repo.scope_graph_inputs.as_ref(),
        );
        Self::from_ctx(ctx)
    }

    /// Build a whole-repo index by incrementally refreshing the CPG from a
    /// previously published whole-repo navigation index.
    ///
    /// This is intentionally separate from `cache::build_cached_at`, whose
    /// partial-hit behavior remains conservative for normal nav cache users.
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    pub(crate) fn build_incremental_from_previous(
        previous: &NavigationIndex,
        repo: &LoadedRepo,
        changed_files: &BTreeSet<String>,
    ) -> Self {
        let cpg = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
            previous.cpg.call_graph.clone(),
            previous.cpg.dfg.clone(),
            changed_files,
            &repo.files,
            repo.type_db.clone(),
            repo.scope_graph_inputs.as_ref(),
        );
        // P15a-fix2 provenance boundary: this CPG was FRESHLY REBUILT from
        // the same current `repo.files`, so its stashed plain Go provider
        // transfers into the registry exactly like the full path (one plain
        // extraction total, peak live = 1, and nothing retained on the graph).
        // A truly deserialized cache hit must keep using
        // `CpgContext::build_with_cached_cpg`, which constructs fresh because
        // the cached provider's provenance cannot be trusted.
        Self::from_ctx(CpgContext::build_with_fresh_cpg(
            &repo.files,
            cpg,
            repo.type_db.as_ref(),
        ))
    }

    pub(crate) fn from_ctx(ctx: CpgContext<'_>) -> Self {
        Self::from_ctx_with_call_edge_cache(ctx, None)
    }

    pub(crate) fn from_ctx_with_call_edge_cache(
        ctx: CpgContext<'_>,
        call_edge_cache_store: Option<call_edge_cache::NavigationCallEdgeCacheStore>,
    ) -> Self {
        debug_assert!(ctx.scope.is_none(), "nav index must be whole-repo");
        let (mut line_range_index, mut name_index) = (
            BTreeMap::<String, Vec<(usize, usize, NodeIndex)>>::new(),
            BTreeMap::<(String, String), Vec<NodeIndex>>::new(),
        );
        for idx in ctx.cpg.node_indices() {
            if let CpgNode::Function {
                name,
                file,
                start_line,
                end_line,
                ..
            } = ctx.cpg.node(idx)
            {
                line_range_index.entry(file.clone()).or_default().push((
                    *start_line,
                    *end_line,
                    idx,
                ));
                name_index
                    .entry((file.clone(), name.clone()))
                    .or_default()
                    .push(idx);
            }
        }
        for v in line_range_index.values_mut() {
            v.sort_by_key(|&(s, _, _)| s);
        }
        NavigationIndex {
            cpg: ctx.cpg,
            types: ctx.types,
            live_types: ctx.live_types,
            line_range_index,
            name_index,
            resolved_call_edges: OnceLock::new(),
            call_edge_cache_store,
        }
    }

    pub fn cpg(&self) -> &CodePropertyGraph {
        &self.cpg
    }

    pub fn call_graph(&self) -> &CallGraph {
        &self.cpg.call_graph
    }

    /// Test-only escape hatch for call graph/scope graph mutations. This resets
    /// the lazy call-edge index; it must not be used to mutate CPG function nodes
    /// because `line_range_index` and `name_index` would need rebuilding too.
    #[doc(hidden)]
    pub fn with_modified_cpg_for_testing(mut self, f: impl FnOnce(&mut CodePropertyGraph)) -> Self {
        f(&mut self.cpg);
        self.resolved_call_edges = OnceLock::new();
        self.call_edge_cache_store = None;
        self
    }

    pub(crate) fn resolved_call_edges(&self) -> &NavigationCallEdgeIndex {
        self.resolved_call_edges.get_or_init(|| {
            if let Some(store) = &self.call_edge_cache_store {
                if let Some(index) = store.load_best_effort() {
                    return index;
                }
            }
            let index = self.build_resolved_call_edges();
            if let Some(store) = &self.call_edge_cache_store {
                store.save_best_effort(&index);
            }
            index
        })
    }

    pub(crate) fn direct_callers(&self, target: &FunctionId) -> &[IndexedIncomingCall] {
        self.resolved_call_edges()
            .incoming_by_target
            .get(target)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn direct_callees(&self, caller: &FunctionId) -> &[IndexedOutgoingCallSite] {
        self.resolved_call_edges()
            .outgoing_by_caller
            .get(caller)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn outgoing_call_edges(
        &self,
    ) -> impl Iterator<Item = (&FunctionId, &IndexedOutgoingCallSite)> {
        self.resolved_call_edges()
            .outgoing_by_caller
            .iter()
            .flat_map(|(caller, sites)| sites.iter().map(move |site| (caller, site)))
    }

    /// Deterministic, sorted `(file, line)` locations for the R6
    /// multi-owner-collision sites still dropped for `seed_name` (P3): lets
    /// the nav warning name a few sites instead of only a bare count. The
    /// count consumers want is simply `.len()` on the result — not
    /// deduplicated, so it matches the prior count-only helper exactly.
    pub(crate) fn collision_dropped_site_locations(&self, seed_name: &str) -> Vec<(String, usize)> {
        let idx = self.resolved_call_edges();
        let mut locations: Vec<(String, usize)> =
            crate::navigation::call_resolve::scoped_caller_sites(&self.cpg.call_graph, seed_name)
                .into_iter()
                .filter(|site| {
                    idx.multi_owner_collision_sites
                        .contains(&CallSiteKey::from(*site))
                })
                .map(|site| (site.caller.file.clone(), site.line))
                .collect();
        locations.sort();
        locations
    }

    fn build_resolved_call_edges(&self) -> NavigationCallEdgeIndex {
        let cg = &self.cpg.call_graph;
        let mut idx = NavigationCallEdgeIndex::default();
        let mut resolved_by_site: BTreeMap<CallSiteKey, ResolvedCallSiteOutcome> = BTreeMap::new();

        for (bucket_key, sites) in &cg.callers {
            for site in sites {
                let key = CallSiteKey::from(site);
                let outcome = resolved_by_site.entry(key.clone()).or_insert_with(|| {
                    let outcome = cg.resolve_call_site_full(site);
                    ResolvedCallSiteOutcome {
                        resolved: outcome
                            .resolved
                            .into_iter()
                            .map(|resolved| IndexedResolvedTarget {
                                target: resolved.target.clone(),
                                confidence: resolved.confidence,
                                kind: resolved.kind,
                            })
                            .collect(),
                        drop: outcome.drop,
                    }
                });
                if outcome.drop == Some(DropReason::MultiOwnerCollision) {
                    idx.multi_owner_collision_sites.insert(key);
                }
                for resolved in &outcome.resolved {
                    // P5 S3: `func_value_field` deliberately breaks the
                    // name-correlation invariant every other `ResolutionKind`
                    // upholds (`cmd.Run()` legitimately resolves to a target
                    // named `helper`, not `Run`) — `scoped_caller_site_match_count`
                    // exists to disambiguate MODULE-SCOPED variants of the SAME
                    // callee name (`bucket_key == target_name` / a `::name`
                    // suffix), which does not apply here. The site is already
                    // fully precise (one CallSite -> its resolved targets), so
                    // count it directly rather than gating on name match.
                    let count = if resolved.kind == ResolutionKind::FuncValueField {
                        1
                    } else {
                        let suffix = format!("::{}", resolved.target.name);
                        crate::navigation::call_resolve::scoped_caller_site_match_count(
                            cg,
                            bucket_key,
                            site,
                            &resolved.target.name,
                            &suffix,
                        )
                    };
                    for _ in 0..count {
                        idx.incoming_by_target
                            .entry(resolved.target.clone())
                            .or_default()
                            .push(IndexedIncomingCall {
                                caller: site.caller.clone(),
                                callee_name: site.callee_name.clone(),
                                call_site_line: site.line,
                                start_byte: site.start_byte,
                                end_byte: site.end_byte,
                                qualifier: site.qualifier.clone(),
                                confidence: resolved.confidence,
                                kind: resolved.kind,
                            });
                    }
                }
            }
        }

        for (caller, sites) in &cg.calls {
            for site in sites {
                let key = CallSiteKey::from(site);
                let outcome = resolved_by_site.entry(key).or_insert_with(|| {
                    let outcome = cg.resolve_call_site_full(site);
                    ResolvedCallSiteOutcome {
                        resolved: outcome
                            .resolved
                            .into_iter()
                            .map(|resolved| IndexedResolvedTarget {
                                target: resolved.target.clone(),
                                confidence: resolved.confidence,
                                kind: resolved.kind,
                            })
                            .collect(),
                        drop: outcome.drop,
                    }
                });
                idx.outgoing_by_caller
                    .entry(caller.clone())
                    .or_default()
                    .push(IndexedOutgoingCallSite {
                        callee_name: site.callee_name.clone(),
                        call_site_line: site.line,
                        start_byte: site.start_byte,
                        end_byte: site.end_byte,
                        qualifier: site.qualifier.clone(),
                        resolved: outcome.resolved.clone(),
                    });
            }
        }

        // P5 S2 (re-review MINOR-3): merge Go function-value registrations in
        // deterministically (`cg.go_registrations` is a `BTreeSet`, so this
        // loop's insertion order is already deterministic). Registrations are
        // NOT `CallSite`s — see the architecture note on
        // `CallGraph::go_registrations` — so they never appear in the
        // `cg.callers`/`cg.calls` loops above; this is the only place they
        // reach `direct_callers`/`direct_callees` (`queries::{callers,callees}`
        // read those unchanged).
        for reg in &cg.go_registrations {
            idx.incoming_by_target
                .entry(reg.target.clone())
                .or_default()
                .push(IndexedIncomingCall {
                    caller: reg.enclosing.clone(),
                    callee_name: reg.target.name.clone(),
                    call_site_line: reg.site.line,
                    start_byte: reg.site.start_byte,
                    end_byte: reg.site.end_byte,
                    qualifier: None,
                    confidence: ResolutionConfidence::NameOnly,
                    kind: ResolutionKind::CallbackRegistration,
                });
            idx.outgoing_by_caller
                .entry(reg.enclosing.clone())
                .or_default()
                .push(IndexedOutgoingCallSite {
                    callee_name: reg.target.name.clone(),
                    call_site_line: reg.site.line,
                    start_byte: reg.site.start_byte,
                    end_byte: reg.site.end_byte,
                    qualifier: None,
                    resolved: vec![IndexedResolvedTarget {
                        target: reg.target.clone(),
                        confidence: ResolutionConfidence::NameOnly,
                        kind: ResolutionKind::CallbackRegistration,
                    }],
                });
        }

        // P7 S3: merge Python property-access records in beside the P5
        // registration loop above (same rationale — `cg.property_accesses`
        // is a `BTreeSet`, so insertion order is already deterministic;
        // these are NOT `CallSite`s, so they never appear in the
        // `cg.callers`/`cg.calls` loops above either — this is the only
        // place they reach `direct_callers`/`direct_callees`).
        for acc in &cg.property_accesses {
            idx.incoming_by_target
                .entry(acc.getter.clone())
                .or_default()
                .push(IndexedIncomingCall {
                    caller: acc.enclosing.clone(),
                    callee_name: acc.getter.name.clone(),
                    call_site_line: acc.site.line,
                    start_byte: acc.site.start_byte,
                    end_byte: acc.site.end_byte,
                    qualifier: None,
                    confidence: ResolutionConfidence::NameOnly,
                    kind: ResolutionKind::PropertyAccess,
                });
            idx.outgoing_by_caller
                .entry(acc.enclosing.clone())
                .or_default()
                .push(IndexedOutgoingCallSite {
                    callee_name: acc.getter.name.clone(),
                    call_site_line: acc.site.line,
                    start_byte: acc.site.start_byte,
                    end_byte: acc.site.end_byte,
                    qualifier: None,
                    resolved: vec![IndexedResolvedTarget {
                        target: acc.getter.clone(),
                        confidence: ResolutionConfidence::NameOnly,
                        kind: ResolutionKind::PropertyAccess,
                    }],
                });
        }

        // P9 S3: merge framework-entry (Flask/FastAPI/Express route
        // registration) records in beside the P7 loop above — same
        // rationale: `cg.framework_entries` is a `BTreeSet` (deterministic
        // insertion order), records are NOT `CallSite`s, so they never
        // appear in the `cg.callers`/`cg.calls` loops above either.
        //
        // Module-level incoming-only rule (codex-adjudicated): every record
        // feeds `incoming_by_target[handler]` regardless of `caller`, but
        // `outgoing_by_caller[caller]` is populated ONLY when `caller` is a
        // REAL enclosing function — the `<module>` pseudo-caller
        // (`framework_entries::MODULE_PSEUDO_CALLER_NAME`) is not navigable
        // (no CPG node, no Module symbol kind in nav — navigation/types.rs)
        // and must never appear as an outgoing/callees entry.
        for entry in &cg.framework_entries {
            idx.incoming_by_target
                .entry(entry.handler.clone())
                .or_default()
                .push(IndexedIncomingCall {
                    caller: entry.caller.clone(),
                    callee_name: entry.handler.name.clone(),
                    call_site_line: entry.site.line,
                    start_byte: entry.site.start_byte,
                    end_byte: entry.site.end_byte,
                    qualifier: None,
                    confidence: ResolutionConfidence::NameOnly,
                    kind: ResolutionKind::FrameworkEntry,
                });
            if entry.caller.name == crate::framework_entries::MODULE_PSEUDO_CALLER_NAME {
                continue;
            }
            idx.outgoing_by_caller
                .entry(entry.caller.clone())
                .or_default()
                .push(IndexedOutgoingCallSite {
                    callee_name: entry.handler.name.clone(),
                    call_site_line: entry.site.line,
                    start_byte: entry.site.start_byte,
                    end_byte: entry.site.end_byte,
                    qualifier: None,
                    resolved: vec![IndexedResolvedTarget {
                        target: entry.handler.clone(),
                        confidence: ResolutionConfidence::NameOnly,
                        kind: ResolutionKind::FrameworkEntry,
                    }],
                });
        }

        idx
    }

    /// Innermost enclosing function (smallest [start,end] containing `line`).
    pub fn enclosing_function(&self, file: &str, line: usize) -> Option<(NodeIndex, String)> {
        let ranges = self.line_range_index.get(file)?;
        ranges
            .iter()
            .filter(|&&(s, e, _)| s <= line && line <= e)
            .min_by_key(|&&(s, e, _)| e - s)
            .map(|&(_, _, idx)| {
                let name = match self.cpg.node(idx) {
                    CpgNode::Function { name, .. } => name.clone(),
                    _ => String::new(),
                };
                (idx, name)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::queries;
    use crate::navigation::types::{Reason, SymbolRef};
    use crate::repo_loader::load_repo;
    use std::path::Path;

    fn write_files(root: &Path, files: &[(&str, &str)]) {
        for (name, source) in files {
            let path = root.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, source).unwrap();
        }
    }

    fn full_session(root: &Path) -> NavigationSession {
        let repo = Arc::new(load_repo(root).unwrap());
        let index = Arc::new(NavigationIndex::build(&repo));
        NavigationSession { repo, index }
    }

    #[test]
    fn test_cpg_mutation_helper_resets_lazy_call_edge_index() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[(
                "a.py",
                "def target():\n    return 1\n\ndef caller():\n    return target()\n",
            )],
        );
        let repo = Arc::new(load_repo(dir.path()).unwrap());
        let index = NavigationIndex::build(&repo);
        let caller = index
            .call_graph()
            .functions
            .get("caller")
            .and_then(|ids| ids.first())
            .cloned()
            .unwrap();
        assert!(!index.direct_callees(&caller).is_empty());

        let index = index.with_modified_cpg_for_testing(|cpg| {
            cpg.call_graph.calls.clear();
            cpg.call_graph.callers.clear();
        });
        assert!(index.direct_callees(&caller).is_empty());
    }

    fn incremental_session(
        previous: &NavigationSession,
        root: &Path,
        changed_files: &[&str],
    ) -> NavigationSession {
        let repo = Arc::new(load_repo(root).unwrap());
        let changed_files = changed_files.iter().map(|f| (*f).to_string()).collect();
        let index = Arc::new(NavigationIndex::build_incremental_from_previous(
            previous.index.as_ref(),
            &repo,
            &changed_files,
        ));
        NavigationSession { repo, index }
    }

    /// P15a-fix2: the incremental REBUILD path (`build_incremental_from_previous`,
    /// used by MCP `refresh_index`) constructs a fresh CPG from the current
    /// `repo.files`, so it must transfer that CPG's stashed plain Go provider
    /// into the registry — exactly ONE plain construction and peak live = 1.
    /// RED on the pre-fix code, which routed through `build_with_cached_cpg`
    /// (registry got None) and did dispatch + TWO plain extractions with two
    /// live datasets.
    #[test]
    fn p15a_fix2_incremental_rebuild_single_plain_extraction() {
        use crate::type_providers::go::test_counters::MeasurementToken;

        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                (
                    "types.go",
                    "package main\ntype Greeter interface{ Greet() string }\n",
                ),
                (
                    "main.go",
                    "package main\nfunc (g Greeter) greet() {}\nfunc main() {}\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "main.go",
                "package main\nfunc (g Greeter) greet() {}\nfunc main() { _ = 1 }\n",
            )],
        );
        let repo = Arc::new(load_repo(dir.path()).unwrap());
        let token = MeasurementToken::acquire();
        let index = NavigationIndex::build_incremental_from_previous(
            v1.index.as_ref(),
            &repo,
            &BTreeSet::from(["main.go".to_string()]),
        );
        assert_eq!(
            token.plain_constructions(),
            1,
            "incremental rebuild must reuse the rebuild's single plain extraction"
        );
        assert_eq!(
            token.peak_live(),
            1,
            "at most 1 full GoTypeData may be alive at a time"
        );
        assert!(
            index.cpg.call_graph.shared_plain_go_provider.is_none(),
            "incrementally rebuilt index must not retain the shared provider"
        );
    }

    fn function_names_for_callers(
        session: &NavigationSession,
        target: &str,
        file: Option<&str>,
        depth: usize,
    ) -> Vec<String> {
        let evidence = queries::callers(session, Some(target), file, None, depth).unwrap();
        evidence
            .items
            .into_iter()
            .filter_map(|item| match item.symbol {
                Some(SymbolRef::Function { name, .. }) => Some(name),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn incremental_from_previous_chained_edits_match_full_queries() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                ("api.py", "def target():\n    return 1\n"),
                (
                    "callers.py",
                    "from api import target\n\ndef first():\n    return target()\n\ndef second():\n    return 0\n",
                ),
                (
                    "outer.py",
                    "from callers import second\n\ndef third():\n    return 0\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "callers.py",
                "from api import target\n\ndef first():\n    return target()\n\ndef second():\n    return target()\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["callers.py"]);

        write_files(
            dir.path(),
            &[(
                "outer.py",
                "from callers import second\n\ndef third():\n    return second()\n",
            )],
        );
        let v3_incremental = incremental_session(&v2_incremental, dir.path(), &["outer.py"]);
        let v3_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v3_full, Some("target"), Some("api.py"), None, 2).unwrap(),
            queries::callers(&v3_incremental, Some("target"), Some("api.py"), None, 2).unwrap()
        );
        assert_eq!(
            queries::callees(&v3_full, Some("third"), Some("outer.py"), None, 2).unwrap(),
            queries::callees(&v3_incremental, Some("third"), Some("outer.py"), None, 2).unwrap()
        );

        let caller_names = function_names_for_callers(&v3_incremental, "target", Some("api.py"), 2);
        assert!(caller_names.contains(&"first".to_string()));
        assert!(caller_names.contains(&"second".to_string()));
        assert!(caller_names.contains(&"third".to_string()));
    }

    #[test]
    fn incremental_from_previous_recomputes_indirect_edges_for_unchanged_caller() {
        let dir = tempfile::tempdir().unwrap();
        let device_src =
            "struct Device { void (*callback)(); };\nvoid run(struct Device *d) { d->callback(); }\n";
        write_files(
            dir.path(),
            &[
                ("device.c", device_src),
                (
                    "setup.c",
                    "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = old_handler; }\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "setup.c",
                "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = new_handler; }\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["setup.c"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("new_handler"), Some("setup.c"), None, 1).unwrap(),
            queries::callers(
                &v2_incremental,
                Some("new_handler"),
                Some("setup.c"),
                None,
                1
            )
            .unwrap()
        );
        assert_eq!(
            queries::callers(&v2_full, Some("old_handler"), Some("setup.c"), None, 1).unwrap(),
            queries::callers(
                &v2_incremental,
                Some("old_handler"),
                Some("setup.c"),
                None,
                1
            )
            .unwrap()
        );

        let new_handler_callers = queries::callers(
            &v2_incremental,
            Some("new_handler"),
            Some("setup.c"),
            None,
            1,
        )
        .unwrap();
        assert!(
            new_handler_callers.items.iter().any(|item| matches!(
                &item.symbol,
                Some(SymbolRef::Function { file, name, .. }) if file == "device.c" && name == "run"
            )),
            "incremental nav index should expose the recomputed indirect caller"
        );
        assert!(
            new_handler_callers
                .items
                .iter()
                .any(|item| item.why.iter().any(|reason| {
                    matches!(
                        reason,
                        Reason::CalledBy {
                            caller,
                            call_site_line
                        } if caller == "run" && *call_site_line == 2
                    )
                })),
            "caller evidence should point at the unchanged callback call site"
        );

        let old_handler_callers =
            function_names_for_callers(&v2_incremental, "old_handler", Some("setup.c"), 1);
        assert!(!old_handler_callers.contains(&"run".to_string()));
    }

    /// P5 plumbing: the S1 func-typed-field index + S2 registration table are
    /// whole-program derived (mirroring Go embedding/interface dispatch), so
    /// `build_incremental_with_scope_graph_inputs` must explicitly clear +
    /// re-apply them (`apply_go_func_value_fields` / `apply_go_registrations`)
    /// rather than leaving stale/empty state after `remove_files` + `merge`.
    /// Changes an UNRELATED file (`Command` stays declared in the untouched
    /// `types.go`) and confirms the registration edge still surfaces — a
    /// missing re-apply call would silently drop it, since `remove_files`
    /// unconditionally clears this whole-program state.
    #[test]
    fn incremental_from_previous_recomputes_go_func_value_registrations() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                (
                    "types.go",
                    "package main\ntype Command struct {\n\tRun func()\n}\n",
                ),
                (
                    "main.go",
                    "package main\nfunc helper() {}\nfunc unrelated() {}\nfunc main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "main.go",
                "package main\nfunc helper() {}\nfunc unrelated() { _ = 1 }\nfunc main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["main.go"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("helper"), None, None, 1).unwrap(),
            queries::callers(&v2_incremental, Some("helper"), None, None, 1).unwrap()
        );
        let callers = function_names_for_callers(&v2_incremental, "helper", None, 1);
        assert!(
            callers.contains(&"main".to_string()),
            "incremental rebuild must recompute the callback_registration edge \
             for a struct declared in an unchanged file, not leave it cleared"
        );
    }

    /// F2 (review-fix wave, MINOR — coverage gap): the test above only pins
    /// the S2 `callback_registration` edge surviving an incremental rebuild.
    /// The S3 `func_value_field` INVOCATION edge (a separate function calling
    /// through the registered field, not the registration site itself) needs
    /// its own coverage of the same clear+re-apply plumbing — same fixture
    /// shape (an unrelated file edit; `Command` stays declared in the
    /// untouched `types.go`), but with a `register`/`invoke` split so the
    /// surviving caller is reached via `apply_go_func_value_fields` +
    /// `CallGraph::resolve_call_site` rather than the registration-site edge
    /// synthesized directly from `go_registrations`.
    #[test]
    fn incremental_from_previous_recomputes_go_func_value_field_invocation() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                (
                    "types.go",
                    "package main\ntype Command struct {\n\tRun func()\n}\n",
                ),
                (
                    "main.go",
                    "package main\nfunc helper() {}\nfunc unrelated() {}\nfunc register() *Command { return &Command{Run: helper} }\nfunc invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "main.go",
                "package main\nfunc helper() {}\nfunc unrelated() { _ = 1 }\nfunc register() *Command { return &Command{Run: helper} }\nfunc invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["main.go"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("helper"), None, None, 1).unwrap(),
            queries::callers(&v2_incremental, Some("helper"), None, None, 1).unwrap()
        );
        let callers = function_names_for_callers(&v2_incremental, "helper", None, 1);
        assert!(
            callers.contains(&"invoke".to_string()),
            "incremental rebuild must recompute the func_value_field invocation edge \
             for a struct declared in an unchanged file, not leave it cleared"
        );

        let evidence = queries::callers(&v2_incremental, Some("helper"), None, None, 1).unwrap();
        let hit = evidence
            .items
            .iter()
            .find(
                |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "invoke"),
            )
            .expect("invoke() surfaces as a caller of helper via func_value_field resolution");
        assert!(hit.why.iter().any(|r| matches!(r,
            Reason::Resolution { kind } if kind == "func_value_field"
        )));
    }

    #[test]
    fn p16_incremental_refresh_preserves_proven_interface_guard() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                ("go.mod", "module example.com/root\n\ngo 1.22\n"),
                (
                    "app/types.go",
                    "package app\ntype S struct{ Run func() }\nfunc workerA() {}\nfunc workerB() {}\nfunc retainA() { _ = S{Run: workerA} }\nfunc retainB() { _ = S{Run: workerB} }\nfunc New() S { return S{} }\n",
                ),
                (
                    "app/use.go",
                    "package app\nfunc invoke() {\n  type S struct{}\n  c := New()\n  c.Run()\n}\n",
                ),
                ("app/other.go", "package app\nfunc unrelated() {}\n"),
                (
                    "decoy/types.go",
                    "package decoy\ntype S interface{ Run() }\ntype Wrong struct{}\nfunc (Wrong) Run() {}\nfunc retain() { var _ S = Wrong{} }\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[("app/other.go", "package app\nfunc unrelated() { _ = 1 }\n")],
        );
        let refreshed = incremental_session(&v1, dir.path(), &["app/other.go"]);
        let full = full_session(dir.path());

        assert!(refreshed
            .index
            .call_graph()
            .go_interface_live_types
            .contains("S"));
        let refreshed_edges =
            queries::callees(&refreshed, Some("invoke"), Some("app/use.go"), None, 1).unwrap();
        let full_edges =
            queries::callees(&full, Some("invoke"), Some("app/use.go"), None, 1).unwrap();
        assert_eq!(full_edges, refreshed_edges);

        let has_target = |file: &str, name: &str| {
            refreshed_edges.items.iter().any(|item| {
                matches!(
                    &item.symbol,
                    Some(SymbolRef::Function {
                        file: target_file,
                        name: target_name,
                        ..
                    }) if target_file == file && target_name == name
                )
            })
        };
        assert!(has_target("app/types.go", "workerA"));
        assert!(has_target("app/types.go", "workerB"));
        assert!(!has_target("decoy/types.go", "Run"));
    }

    /// P7 plumbing: the S1 property-getter index + S2 access-site table are
    /// whole-program derived (mirroring the P5 Go func-value state above), so
    /// `build_incremental_with_scope_graph_inputs` must explicitly clear +
    /// re-apply them (`apply_python_property_accesses`) rather than leaving
    /// stale/empty state after `remove_files` + `merge`. Changes an UNRELATED
    /// file (the getter stays declared in the untouched `resp.py`) and
    /// confirms the property_access edge still surfaces — a missing re-apply
    /// call would silently drop it, since `remove_files` unconditionally
    /// clears this whole-program state.
    #[test]
    fn incremental_from_previous_recomputes_python_property_accesses() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                (
                    "resp.py",
                    "class Response:\n    @property\n    def text(self):\n        return self._text\n",
                ),
                (
                    "main.py",
                    "def unrelated():\n    return 1\n\n\ndef f(r):\n    return r.text\n",
                ),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[(
                "main.py",
                "def unrelated():\n    return 2\n\n\ndef f(r):\n    return r.text\n",
            )],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["main.py"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("text"), None, None, 1).unwrap(),
            queries::callers(&v2_incremental, Some("text"), None, None, 1).unwrap()
        );
        let callers = function_names_for_callers(&v2_incremental, "text", None, 1);
        assert!(
            callers.contains(&"f".to_string()),
            "incremental rebuild must recompute the property_access edge for a \
             getter declared in an unchanged file, not leave it cleared"
        );

        let evidence = queries::callers(&v2_incremental, Some("text"), None, None, 1).unwrap();
        let hit = evidence
            .items
            .iter()
            .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "f"))
            .expect("f() surfaces as a caller of text via property_access resolution");
        assert!(hit.why.iter().any(|r| matches!(r,
            Reason::Resolution { kind } if kind == "property_access"
        )));
    }

    /// P9 plumbing: framework-entry state (Flask/FastAPI/Express route
    /// registrations) is whole-program derived exactly like the P5/P7 state
    /// above, so `build_incremental_with_scope_graph_inputs` must explicitly
    /// clear + re-apply it (`apply_framework_entries`) rather than leaving
    /// stale/empty state after `remove_files` + `merge`. Changes an
    /// UNRELATED file (the route decorator stays declared in the untouched
    /// `app.py`) and confirms the framework_entry edge still surfaces — a
    /// missing re-apply call would silently drop it.
    #[test]
    fn incremental_from_previous_recomputes_framework_entries() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                (
                    "app.py",
                    "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\")\ndef handler():\n    return \"ok\"\n",
                ),
                ("main.py", "def unrelated():\n    return 1\n"),
            ],
        );
        let v1 = full_session(dir.path());

        write_files(
            dir.path(),
            &[("main.py", "def unrelated():\n    return 2\n")],
        );
        let v2_incremental = incremental_session(&v1, dir.path(), &["main.py"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("handler"), None, None, 1).unwrap(),
            queries::callers(&v2_incremental, Some("handler"), None, None, 1).unwrap()
        );
        let callers = function_names_for_callers(&v2_incremental, "handler", None, 1);
        assert!(
            callers.contains(&"<module>".to_string()),
            "incremental rebuild must recompute the framework_entry edge for a \
             route decorator declared in an unchanged file, not leave it cleared"
        );

        let evidence = queries::callers(&v2_incremental, Some("handler"), None, None, 1).unwrap();
        let hit = evidence
            .items
            .iter()
            .find(
                |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "<module>"),
            )
            .expect("<module> surfaces as a caller of handler via framework_entry resolution");
        assert!(hit.why.iter().any(|r| matches!(r,
            Reason::Resolution { kind } if kind == "framework_entry"
        )));
    }

    /// Whitebox pin (same-crate access to `outgoing_by_caller`) for the
    /// module-level incoming-only rule: a module-level registration's
    /// `<module>` pseudo-caller must NEVER become an `outgoing_by_caller`
    /// key. `<module>` has no CPG node and is never a valid nav seed (no
    /// Module symbol kind exists), so a leaked key can't be observed via
    /// `nav_callees`; it's also invisible to `nav module-deps`/`nav
    /// repo-map` (`collect_module_edges`, src/navigation/module_graph.rs,
    /// unconditionally drops same-file edges, and a framework-entry
    /// handler is always in the same file as its registration) — this
    /// whitebox test is the ONLY place that can pin the invariant at all.
    #[test]
    fn module_level_framework_entry_never_populates_outgoing_by_caller() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[(
                "app.py",
                "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\")\ndef handler():\n    return \"ok\"\n",
            )],
        );
        let session = full_session(dir.path());
        assert!(
            !session
                .index
                .outgoing_call_edges()
                .any(|(caller, _)| caller.name
                    == crate::framework_entries::MODULE_PSEUDO_CALLER_NAME),
            "the <module> pseudo-caller must never appear as an outgoing_by_caller key"
        );
    }

    /// P8 F1 fix (codex re-review BLOCKER): Rust macro-arg call extraction
    /// depends on the repo-wide macro shadow set
    /// (`rust_macro_args::collect_macro_shadow_set`), but this incremental
    /// path only re-extracts `changed_files` — an unchanged file's call
    /// sites/`macro_arg_facts` are RETAINED as-is (see `remove_files`'s P8
    /// comment). Adding a `macro_rules! vec` definition in a brand-new file
    /// (`b.rs`) must suppress the pre-existing `vec![check()]` macro-arg
    /// call site in the UNCHANGED `a.rs`, exactly like a full rebuild would
    /// — a naive incremental rebuild that never re-examines `a.rs` would
    /// leave that site stale (still minted). This pins the
    /// `build_incremental_with_scope_graph_inputs` mismatch guard that falls
    /// back to a full rebuild instead.
    #[test]
    fn incremental_from_previous_falls_back_to_full_rebuild_on_new_macro_shadow() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[(
                "a.rs",
                "fn check() -> bool {\n    true\n}\nfn host() {\n    let _ = vec![check()];\n}\n",
            )],
        );
        let v1 = full_session(dir.path());
        let v1_callers = function_names_for_callers(&v1, "check", None, 1);
        assert!(
            v1_callers.contains(&"host".to_string()),
            "v1 (no macro_rules! vec anywhere) must mint the vec![check()] macro-arg call site"
        );

        write_files(dir.path(), &[("b.rs", "macro_rules! vec { () => {}; }\n")]);
        let v2_incremental = incremental_session(&v1, dir.path(), &["b.rs"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("check"), None, None, 1).unwrap(),
            queries::callers(&v2_incremental, Some("check"), None, None, 1).unwrap(),
            "incremental and full rebuilds must agree once a shadowing macro_rules! \
             appears anywhere in the repo, even in a brand-new unrelated file"
        );
        let v2_callers = function_names_for_callers(&v2_incremental, "check", None, 1);
        assert!(
            !v2_callers.contains(&"host".to_string()),
            "adding a repo-wide shadowing macro_rules! vec must fall back to a full \
             rebuild and drop the now-suppressed vec! macro-arg call site in the \
             unchanged a.rs, not retain it stale"
        );
    }

    /// Reverse direction of the test above: REMOVING the shadowing file must
    /// also trigger the full-rebuild fallback so the previously-suppressed
    /// `vec![check()]` site in the unchanged `a.rs` reappears, matching a
    /// full rebuild — not stay suppressed because `a.rs` was never
    /// re-examined.
    #[test]
    fn incremental_from_previous_falls_back_to_full_rebuild_on_removed_macro_shadow() {
        let dir = tempfile::tempdir().unwrap();
        write_files(
            dir.path(),
            &[
                (
                    "a.rs",
                    "fn check() -> bool {\n    true\n}\nfn host() {\n    let _ = vec![check()];\n}\n",
                ),
                ("b.rs", "macro_rules! vec { () => {}; }\n"),
            ],
        );
        let v1 = full_session(dir.path());
        let v1_callers = function_names_for_callers(&v1, "check", None, 1);
        assert!(
            !v1_callers.contains(&"host".to_string()),
            "v1 (macro_rules! vec shadowed repo-wide) must suppress the vec![check()] site"
        );

        std::fs::remove_file(dir.path().join("b.rs")).unwrap();
        let v2_incremental = incremental_session(&v1, dir.path(), &["b.rs"]);
        let v2_full = full_session(dir.path());

        assert_eq!(
            queries::callers(&v2_full, Some("check"), None, None, 1).unwrap(),
            queries::callers(&v2_incremental, Some("check"), None, None, 1).unwrap(),
            "incremental and full rebuilds must agree once the shadowing macro_rules! \
             is removed from the repo"
        );
        let v2_callers = function_names_for_callers(&v2_incremental, "check", None, 1);
        assert!(
            v2_callers.contains(&"host".to_string()),
            "removing the shadowing macro_rules! vec must fall back to a full rebuild \
             and reinstate the vec! macro-arg call site in the unchanged a.rs, not \
             leave it suppressed"
        );
    }
}
