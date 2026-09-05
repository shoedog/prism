//! A3: single inline-CFG-filtered predecessor BFS over the production petgraph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::resolution::ResolutionConfidence;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};

type DescentGateKey = (
    String, // caller file
    String, // caller function
    usize,  // caller function start line
    usize,  // source occurrence line
    usize,  // source occurrence start byte
    String, // callee name
    String, // target function file
    usize,  // target function start line
);

pub const MAX_TAINT_DESCENT_DEPTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnFlowMode {
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    DataFlow,
    /// Cross-variable same-line assignment propagation: `x = y` — the use of `y` flows to the def of
    /// `x`. Plan B's strong-update/kill logic kills these on reassignment.
    AssignmentPropagation,
    /// A variable's own recovered def→use chain — re-supplying edges `data_flow.rs` drops: a simple
    /// path's *same-line* reference (`y = u; sink(y)`), and a field path's loop-carried / earlier
    /// reference (`o.data` used on a line ≤ its def). Distinct from `AssignmentPropagation` because
    /// strong-update never kills a variable's own def-use chain — kept a separate serialized `kind`
    /// before any Plan B consumer freezes the wire shape. (Not "same-line"-only since round 9 widened
    /// it to the cross-line field-path recovery.)
    RecoveredDefUse,
    /// Cross-function descent hop that is both an arg->param edge and proven Exact
    /// across one recovery gate. Stage A keeps it as a taint parent edge for witness
    /// and leaves a future stage to expose this depth in evidence.
    CallDescent,
    /// A semantic Use inside a non-simple return expression flows to its
    /// non-seedable ReturnValue endpoint.
    ReturnInput,
    /// A callee return endpoint ascends to the exact caller LHS.
    ReturnFlow,
}

/// A def-use edge crossing a `(file,function,function_start_line)` boundary, recorded but not
/// traversed in v1.
/// `Ord` so [`Trace::boundary`] can be a set — parallel DataFlow edges and multi-root traces
/// would otherwise push duplicate `(root, from, to)` triples and double-count downstream warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoundaryKind {
    CrossFunction,
    SelfFunctionParam,
    SelfFunctionReturn,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundaryEdge {
    pub root: NodeIndex,
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub kind: BoundaryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderingDecision {
    Admit,
    AdmitWithWarning { warning: OrderingWarning },
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderingUnavailableReason {
    DuplicateOccurrences,
    AstUnavailable,
    OccurrenceMismatch,
    UnsupportedSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderingWarning {
    pub file: String,
    pub line: usize,
    pub path: String,
    pub reason: OrderingUnavailableReason,
}

pub trait SameLineOrderView {
    fn admit_same_line_recovered_def_use(
        &self,
        def: NodeIndex,
        use_: NodeIndex,
    ) -> OrderingDecision;
}

#[derive(Debug, Clone, Default)]
pub struct Trace {
    /// Reached nodes per seeded root. The frontier is the union of these — see [`Trace::in_frontier`]
    /// / [`Trace::frontier`]. Keeping only the per-root maps avoids a frontier/per-root desync (which
    /// would re-introduce the "Reached with no witness" dead-end bug A3 exists to prevent).
    pub frontier_by_root: BTreeMap<NodeIndex, BTreeSet<NodeIndex>>,
    pub parents_by_root: BTreeMap<(NodeIndex, NodeIndex), (NodeIndex, Relation)>,
    /// Set (not Vec) so repeated `(root, from, to)` triples from parallel edges / multi-root
    /// traversal collapse — Plan B counts `InterproceduralBoundary` warnings off this.
    pub boundary: BTreeSet<BoundaryEdge>,
    pub degraded: bool,
    pub warnings: Vec<String>,
    pub ordering_warnings: Vec<OrderingWarning>,
}

impl Trace {
    /// Is `node` reached from any seed? (the frontier is the union of `frontier_by_root`).
    pub fn in_frontier(&self, node: NodeIndex) -> bool {
        self.frontier_by_root.values().any(|f| f.contains(&node))
    }

    /// The full frontier (union over roots). Allocates; prefer [`Trace::in_frontier`] for membership.
    pub fn frontier(&self) -> BTreeSet<NodeIndex> {
        self.frontier_by_root.values().flatten().copied().collect()
    }
}

impl CodePropertyGraph {
    /// Total, deterministic same-line ordering key (S2 §3): byte range, then access (Def<Use),
    /// then build-order NodeIndex. Non-Variable nodes sort last.
    fn node_sort_key(&self, idx: NodeIndex) -> (usize, usize, u8, usize) {
        match &self.graph[idx] {
            CpgNode::Variable {
                start_byte,
                end_byte,
                access,
                ..
            } => (
                *start_byte,
                *end_byte,
                match access {
                    VarAccess::Def => 0,
                    VarAccess::Use => 1,
                },
                idx.index(),
            ),
            _ => (usize::MAX, usize::MAX, 2, idx.index()),
        }
    }

    /// Single inline-CFG-filtered predecessor BFS. Every frontier member is CFG-reachable from
    /// the seed, so the parent walk-back never dead-ends. Determinism: neighbors sorted by
    /// NodeIndex; first enqueue per root wins the parent slot; DataFlow beats same-line.
    pub fn taint_trace(&self, sources: &[(String, usize)]) -> Trace {
        self.taint_trace_with_mode(sources, ReturnFlowMode::Off)
    }

    pub fn taint_trace_with_mode(
        &self,
        sources: &[(String, usize)],
        return_flow_mode: ReturnFlowMode,
    ) -> Trace {
        let has_cfg = self.has_cfg_edges();
        let mut trace = Trace::default();
        let mut descent_gate_cache = BTreeMap::new();
        let mut depths = BTreeMap::new();

        // Dedup `(file,line)` seeds: a repeated seed would re-run an identical BFS and (for a degraded
        // line) push a duplicate warning. `BTreeSet` also keeps iteration deterministic.
        let seeds: BTreeSet<(&str, usize)> =
            sources.iter().map(|(f, l)| (f.as_str(), *l)).collect();
        for (file, line) in seeds {
            let roots: Vec<NodeIndex> = self
                .nodes_at(file, line)
                .into_iter()
                .filter(|&n| matches!(self.graph[n], CpgNode::Variable { .. }))
                .collect();
            if roots.is_empty() {
                // Surface unresolved seeds instead of dropping them silently — otherwise a call-only
                // or blank seed line is indistinguishable from "resolved and reached nothing." (A
                // structured per-seed `SeedUnresolved` is a tracked Plan B follow-up.)
                trace
                    .warnings
                    .push(format!("Seed {file}:{line} resolved to no variable nodes"));
                continue;
            }
            // Compute the CFG scope once per seed `(file,line)` rather than once per variable node on
            // it (so a degraded line warns once, not once per variable).
            //
            // A minified line can host roots from *multiple* functions. The per-line CFG scope is
            // built from the line's Statement nodes, which carry no function field, so it cannot be
            // split per function: a root whose function has no statement on the line would inherit
            // another function's scope and be CFG-pruned into a false `NotReached` (an unsafe
            // within-`Reached`-contract miss). When the roots span more than one function, degrade the
            // whole line to pure taint — the safe over-approximation — instead of mis-attributing a
            // scope. (Node/location-precise seeding, a tracked Plan B follow-up, removes the ambiguity
            // and restores per-function precision.)
            let multi_function = roots
                .iter()
                .filter_map(|&n| match &self.graph[n] {
                    CpgNode::Variable {
                        function,
                        function_start_line,
                        ..
                    } => Some((function.as_str(), *function_start_line)),
                    _ => None,
                })
                .collect::<BTreeSet<(&str, usize)>>()
                .len()
                > 1;
            // A seed line that is a function's signature line is also unsafe to CFG-scope: its only
            // statement (if any) may belong to an *enclosing* function (a named nested function:
            // `function outer(){ return function inner(p){` — `inner`'s param seed would inherit
            // `outer`'s scope and prune `inner`'s body). Statement nodes carry no function field, so
            // degrade these to pure taint too (the param-seed fallback already covers the no-statement
            // sub-case; this makes the nested case robust rather than incidental).
            let cfg_scope = if has_cfg && (multi_function || self.function_starts_at(file, line)) {
                trace.degraded = true;
                trace.warnings.push(format!(
                    "Seed line {file}:{line} is a function signature / hosts multiple functions; using pure-taint fallback"
                ));
                None
            } else {
                self.cfg_scope_for_seed(file, line, has_cfg, &mut trace)
            };
            let no_exclusions = BTreeSet::new();
            for root in roots {
                self.taint_trace_from_root(
                    &mut trace,
                    root,
                    file,
                    line,
                    cfg_scope.as_ref(),
                    has_cfg,
                    None,
                    &mut descent_gate_cache,
                    &mut depths,
                    &no_exclusions,
                    return_flow_mode,
                );
            }
        }
        trace
    }

    /// Node-precise sibling of [`Self::taint_trace`]. It traverses only the supplied roots, while
    /// still computing the line-level CFG degradation guards over all variable nodes on each root's
    /// line so a single-node seed cannot bypass known minified/shared-line ambiguity.
    pub fn taint_trace_nodes(
        &self,
        roots: &[NodeIndex],
        order: Option<&dyn SameLineOrderView>,
    ) -> Trace {
        let no_exclusions = BTreeSet::new();
        self.taint_trace_nodes_impl(roots, order, &no_exclusions, ReturnFlowMode::Off)
    }

    pub fn taint_trace_nodes_with_mode(
        &self,
        roots: &[NodeIndex],
        order: Option<&dyn SameLineOrderView>,
        return_flow_mode: ReturnFlowMode,
    ) -> Trace {
        let no_exclusions = BTreeSet::new();
        self.taint_trace_nodes_impl(roots, order, &no_exclusions, return_flow_mode)
    }

    /// P14 fix-wave BLOCKER fix: like [`Self::taint_trace_nodes`], but every hop `(from, to)` in
    /// `excluded_hops` is skipped outright, as if the edge did not exist — it is not merely refused
    /// as a boundary, it is never even considered (no `BoundaryEdge` is recorded for it either).
    /// Used by `reasoning::taint_reaches::witness_mode` to test whether a sink stays reachable from
    /// a single source root after cutting every proven sanitizer-transition hop, i.e. whether a
    /// `Sanitized` downgrade is bypass-proven (see
    /// `reasoning::sanitizer_walk::sanitizer_bypass_exclusions`). Shares the SAME core
    /// (`taint_trace_from_root`) as [`Self::taint_trace`]/[`Self::taint_trace_nodes`] — doctrine-6,
    /// exactly one walk implementation.
    pub fn taint_trace_nodes_excluding(
        &self,
        roots: &[NodeIndex],
        order: Option<&dyn SameLineOrderView>,
        excluded_hops: &BTreeSet<(NodeIndex, NodeIndex)>,
    ) -> Trace {
        self.taint_trace_nodes_impl(roots, order, excluded_hops, ReturnFlowMode::Off)
    }

    pub fn taint_trace_nodes_excluding_with_mode(
        &self,
        roots: &[NodeIndex],
        order: Option<&dyn SameLineOrderView>,
        excluded_hops: &BTreeSet<(NodeIndex, NodeIndex)>,
        return_flow_mode: ReturnFlowMode,
    ) -> Trace {
        self.taint_trace_nodes_impl(roots, order, excluded_hops, return_flow_mode)
    }

    fn taint_trace_nodes_impl(
        &self,
        roots: &[NodeIndex],
        order: Option<&dyn SameLineOrderView>,
        excluded_hops: &BTreeSet<(NodeIndex, NodeIndex)>,
        return_flow_mode: ReturnFlowMode,
    ) -> Trace {
        let has_cfg = self.has_cfg_edges();
        let mut trace = Trace::default();
        let mut descent_gate_cache = BTreeMap::new();
        let mut depths = BTreeMap::new();

        let mut by_line: BTreeMap<(String, usize), BTreeSet<NodeIndex>> = BTreeMap::new();
        for &root in roots {
            let CpgNode::Variable { file, line, .. } = &self.graph[root] else {
                continue;
            };
            by_line
                .entry((file.clone(), *line))
                .or_default()
                .insert(root);
        }

        for ((file, line), selected_roots) in by_line {
            let line_roots: Vec<NodeIndex> = self
                .nodes_at(&file, line)
                .into_iter()
                .filter(|&n| matches!(self.graph[n], CpgNode::Variable { .. }))
                .collect();
            if line_roots.is_empty() {
                trace
                    .warnings
                    .push(format!("Seed {file}:{line} resolved to no variable nodes"));
                continue;
            }

            let multi_function = line_roots
                .iter()
                .filter_map(|&n| match &self.graph[n] {
                    CpgNode::Variable {
                        function,
                        function_start_line,
                        ..
                    } => Some((function.as_str(), *function_start_line)),
                    _ => None,
                })
                .collect::<BTreeSet<(&str, usize)>>()
                .len()
                > 1;
            let cfg_scope = if has_cfg && (multi_function || self.function_starts_at(&file, line)) {
                trace.degraded = true;
                trace.warnings.push(format!(
                    "Seed line {file}:{line} is a function signature / hosts multiple functions; using pure-taint fallback"
                ));
                None
            } else {
                self.cfg_scope_for_seed(&file, line, has_cfg, &mut trace)
            };

            for root in selected_roots {
                self.taint_trace_from_root(
                    &mut trace,
                    root,
                    &file,
                    line,
                    cfg_scope.as_ref(),
                    has_cfg,
                    order,
                    &mut descent_gate_cache,
                    &mut depths,
                    excluded_hops,
                    return_flow_mode,
                );
            }
        }
        trace
    }

    #[allow(clippy::too_many_arguments)]
    fn taint_trace_from_root(
        &self,
        trace: &mut Trace,
        root: NodeIndex,
        src_file: &str,
        src_line: usize,
        cfg_scope: Option<&BTreeSet<(String, usize)>>,
        has_cfg: bool,
        order: Option<&dyn SameLineOrderView>,
        descent_gate_cache: &mut BTreeMap<DescentGateKey, bool>,
        depths: &mut BTreeMap<(NodeIndex, NodeIndex), usize>,
        excluded_hops: &BTreeSet<(NodeIndex, NodeIndex)>,
        return_flow_mode: ReturnFlowMode,
    ) {
        if !matches!(self.graph[root], CpgNode::Variable { .. }) {
            return;
        }
        let mut enqueued: BTreeSet<NodeIndex> = BTreeSet::new();
        let mut queue = VecDeque::new();
        if enqueued.insert(root) {
            trace.frontier_by_root.entry(root).or_default().insert(root);
            depths.insert((root, root), 0);
            queue.push_back(root);
        }

        while let Some(node) = queue.pop_front() {
            let node_depth = *depths.get(&(root, node)).unwrap_or(&0);
            for (next, rel) in self.taint_neighbors(node, return_flow_mode) {
                // P14 fix-wave BLOCKER fix: an excluded hop is simply not taken — no ordering
                // check, no boundary/descent/CFG handling, no `BoundaryEdge` record. Checked first
                // so `taint_trace_nodes_excluding`'s re-walk (used to bypass-prove a `Sanitized`
                // downgrade, `reasoning::sanitizer_walk::sanitizer_bypass_exclusions`) behaves as
                // if the excluded edge did not exist at all.
                if excluded_hops.contains(&(node, next)) {
                    continue;
                }
                if !self.ordering_admits(node, next, rel, order, trace) {
                    continue;
                }
                let Some(node_fn) = self.node_file_fn(node) else {
                    continue;
                };
                let Some(next_fn) = self.node_file_fn(next) else {
                    continue;
                };

                // Return ascent is handled before the generic cross-function
                // branch: it has its own singleton-Exact gate and consumes the
                // same bounded interprocedural depth budget as call descent.
                if rel == Relation::ReturnFlow {
                    let boundary_kind = if next_fn == node_fn {
                        BoundaryKind::SelfFunctionReturn
                    } else {
                        BoundaryKind::CrossFunction
                    };
                    let can_ascend = next_fn != node_fn
                        && node_depth < MAX_TAINT_DESCENT_DEPTH
                        && self.return_flow_ascent_allowed(node, next);
                    if can_ascend {
                        if enqueued.insert(next) {
                            trace.frontier_by_root.entry(root).or_default().insert(next);
                            trace
                                .parents_by_root
                                .insert((root, next), (node, Relation::ReturnFlow));
                            depths.insert((root, next), node_depth + 1);
                            queue.push_back(next);
                        }
                    } else {
                        trace.boundary.insert(BoundaryEdge {
                            root,
                            from: node,
                            to: next,
                            kind: boundary_kind,
                        });
                    }
                    continue;
                }

                // A neighbor is a boundary (taint exits into a callee in v1) if it crosses into a
                // different function, or if a recursive/self call's arg→param DataFlow edge lands back in
                // the same function. The edge-sensitive parameter test avoids classifying one-line body
                // locals as call boundaries.
                if next_fn != node_fn || self.is_parameter_binding_from(node, next, rel) {
                    let boundary_kind = if next_fn != node_fn {
                        BoundaryKind::CrossFunction
                    } else {
                        BoundaryKind::SelfFunctionParam
                    };

                    let can_descend = matches!(boundary_kind, BoundaryKind::CrossFunction)
                        && node_depth < MAX_TAINT_DESCENT_DEPTH
                        && self.descend_target(node, next, rel, descent_gate_cache);

                    if can_descend {
                        if enqueued.insert(next) {
                            let next_depth = node_depth + 1;
                            trace.frontier_by_root.entry(root).or_default().insert(next);
                            trace
                                .parents_by_root
                                .insert((root, next), (node, Relation::CallDescent));
                            depths.insert((root, next), next_depth);
                            queue.push_back(next);
                        }
                        continue;
                    }

                    trace.boundary.insert(BoundaryEdge {
                        root,
                        from: node,
                        to: next,
                        kind: boundary_kind,
                    });
                    continue;
                }

                // S3: CFG pruning is depth-rooted. Any descended hop is walked pure-taint.
                if node_depth == 0 && !self.cfg_valid(src_file, src_line, cfg_scope, has_cfg, next)
                {
                    continue;
                }

                if enqueued.insert(next) {
                    trace.frontier_by_root.entry(root).or_default().insert(next);
                    trace.parents_by_root.insert((root, next), (node, rel));
                    depths.insert((root, next), node_depth);
                    queue.push_back(next);
                }
            }
        }
    }

    fn ordering_admits(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        rel: Relation,
        order: Option<&dyn SameLineOrderView>,
        trace: &mut Trace,
    ) -> bool {
        let mut warnings = Vec::new();
        let admitted = self.ordering_admits_collect(from, to, rel, order, &mut warnings);
        trace.ordering_warnings.extend(warnings);
        admitted
    }

    fn ordering_admits_collect(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        rel: Relation,
        order: Option<&dyn SameLineOrderView>,
        warnings: &mut Vec<OrderingWarning>,
    ) -> bool {
        if rel != Relation::RecoveredDefUse || !self.same_line_variables(from, to) {
            return true;
        }
        let Some(order) = order else {
            return true;
        };
        match order.admit_same_line_recovered_def_use(from, to) {
            OrderingDecision::Admit => true,
            OrderingDecision::AdmitWithWarning { warning } => {
                warnings.push(warning);
                true
            }
        }
    }

    fn same_line_variables(&self, a: NodeIndex, b: NodeIndex) -> bool {
        matches!(
            (&self.graph[a], &self.graph[b]),
            (
                CpgNode::Variable {
                    file: af,
                    line: al,
                    ..
                },
                CpgNode::Variable {
                    file: bf,
                    line: bl,
                    ..
                }
            ) if af == bf && al == bl
        )
    }

    fn node_file_fn(&self, idx: NodeIndex) -> Option<(String, String, usize)> {
        match &self.graph[idx] {
            CpgNode::Variable {
                file,
                function,
                function_start_line,
                ..
            } => Some((file.clone(), function.clone(), *function_start_line)),
            CpgNode::ReturnValue {
                file,
                function,
                function_start_line,
                ..
            } => Some((file.clone(), function.clone(), *function_start_line)),
            _ => None,
        }
    }

    /// Does a function's signature start at `(file, line)`? Function nodes are indexed at their
    /// `start_line`, so this is where parameters are bound and where a callee's CFG entry is murky.
    fn function_starts_at(&self, file: &str, line: usize) -> bool {
        self.nodes_at(file, line).iter().any(|&n| {
            matches!(&self.graph[n], CpgNode::Function { start_line, .. } if *start_line == line)
        })
    }

    /// Is `to` a parameter binding reached by an arg→param call edge from `from`?
    ///
    /// Parameter defs and one-line body locals can share the function start line. Statement nodes are
    /// also line-deduped, so byte/statement heuristics misclassify locals in minified multi-function
    /// lines. The boundary we need to preserve is narrower and edge-sensitive: a recursive/self call
    /// whose DataFlow edge lands on the callee parameter. If call resolution misses that self-call,
    /// this stays intra-function and fails open rather than manufacturing a false `NotReached`.
    fn is_parameter_binding_from(&self, from: NodeIndex, to: NodeIndex, rel: Relation) -> bool {
        if rel != Relation::DataFlow {
            return false;
        }
        let CpgNode::Variable {
            access: VarAccess::Def,
            function: callee_name,
            ..
        } = &self.graph[to]
        else {
            return false;
        };
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start_line,
            line: source_line,
            start_byte: source_start_byte,
            end_byte: source_end_byte,
            ..
        } = &self.graph[from]
        else {
            return false;
        };
        self.call_graph
            .callers
            .get(callee_name)
            .is_some_and(|sites| {
                sites.iter().any(|site| {
                    Self::call_site_matches_source_occurrence(
                        site,
                        caller_file,
                        caller_name,
                        *caller_start_line,
                        *source_line,
                        *source_start_byte,
                        *source_end_byte,
                    )
                })
            })
    }

    /// Some(..) iff this cross-function `DataFlow` hop is backed by exactly one matching
    /// CallSite and that site's `resolve_call_site` includes an Exact edge to the target `to` owner.
    fn descend_target(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        rel: Relation,
        descent_gate_cache: &mut BTreeMap<DescentGateKey, bool>,
    ) -> bool {
        if rel != Relation::DataFlow {
            return false;
        }

        let CpgNode::Variable {
            file: target_file,
            function: target_fn,
            function_start_line: target_fn_start_line,
            access: VarAccess::Def,
            ..
        } = &self.graph[to]
        else {
            return false;
        };

        let CpgNode::Variable {
            file: caller_file,
            function: caller_fn,
            function_start_line: caller_fn_start_line,
            line: source_line,
            start_byte: source_start_byte,
            end_byte: source_end_byte,
            ..
        } = &self.graph[from]
        else {
            return false;
        };

        let cache_key = (
            caller_file.clone(),
            caller_fn.clone(),
            *caller_fn_start_line,
            *source_line,
            *source_start_byte,
            target_fn.clone(),
            target_file.clone(),
            *target_fn_start_line,
        );
        if let Some(can_descend) = descent_gate_cache.get(&cache_key) {
            return *can_descend;
        }

        let can_descend = self
            .call_graph
            .callers
            .get(target_fn)
            .and_then(|sites| {
                let matching_sites: Vec<_> = sites
                    .iter()
                    .filter(|site| {
                        Self::call_site_matches_source_occurrence(
                            site,
                            caller_file,
                            caller_fn,
                            *caller_fn_start_line,
                            *source_line,
                            *source_start_byte,
                            *source_end_byte,
                        )
                    })
                    .collect();

                match matching_sites.len() {
                    1 => Some(matching_sites[0]),
                    _ => None,
                }
            })
            .is_some_and(|site| {
                self.call_graph
                    .resolve_call_site(site)
                    .into_iter()
                    .any(|resolved| {
                        resolved.confidence == ResolutionConfidence::Exact
                            && resolved.target.file == *target_file
                            && resolved.target.name == *target_fn
                            && resolved.target.start_line == *target_fn_start_line
                    })
            });

        descent_gate_cache.insert(cache_key, can_descend);
        can_descend
    }

    /// Match a Step-5b source occurrence to its CallSite. Existing same-line
    /// behavior remains line-based, preserving the refusal on same-line
    /// same-name call collisions. When the occurrence is on a continuation
    /// line, the CallSite's byte span is the only available identity proof.
    fn call_site_matches_source_occurrence(
        site: &crate::call_graph::CallSite,
        caller_file: &str,
        caller_name: &str,
        caller_start_line: usize,
        source_line: usize,
        source_start_byte: usize,
        source_end_byte: usize,
    ) -> bool {
        site.caller.file == caller_file
            && site.caller.name == caller_name
            && site.caller.start_line == caller_start_line
            && (site.line == source_line
                || (site.start_byte <= source_start_byte && source_end_byte <= site.end_byte))
    }

    fn return_flow_ascent_allowed(&self, from: NodeIndex, to: NodeIndex) -> bool {
        let Some((callee_file, callee_name, callee_start)) = self.node_file_fn(from) else {
            return false;
        };
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start,
            ..
        } = &self.graph[to]
        else {
            return false;
        };
        let caller = crate::call_graph::FunctionId {
            file: caller_file.clone(),
            name: caller_name.clone(),
            start_line: *caller_start,
            end_line: usize::MAX,
        };
        self.call_graph
            .calls
            .iter()
            .filter(|(id, _)| {
                id.file == caller.file
                    && id.name == caller.name
                    && id.start_line == caller.start_line
            })
            .flat_map(|(_, sites)| sites)
            .any(|site| {
                super::build::singleton_exact(&self.call_graph.resolve_call_site_full(site))
                    .is_some_and(|resolved| {
                        resolved.target.file == callee_file
                            && resolved.target.name == callee_name
                            && resolved.target.start_line == callee_start
                    })
            })
    }

    fn assignment_shortcut_suppressed(&self, use_idx: NodeIndex, def_idx: NodeIndex) -> bool {
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start,
            start_byte: use_start,
            end_byte: use_end,
            ..
        } = &self.graph[use_idx]
        else {
            return false;
        };

        self.graph
            .edges_directed(def_idx, Direction::Incoming)
            .filter(|edge| {
                matches!(
                    edge.weight(),
                    CpgEdge::ReturnFlow {
                        suppress_shortcut: true
                    }
                )
            })
            .any(|edge| {
                let Some((callee_file, callee_name, callee_start)) =
                    self.node_file_fn(edge.source())
                else {
                    return false;
                };
                self.call_graph
                    .calls
                    .iter()
                    .filter(|(id, _)| {
                        id.file == *caller_file
                            && id.name == *caller_name
                            && id.start_line == *caller_start
                    })
                    .flat_map(|(_, sites)| sites)
                    .any(|site| {
                        site.start_byte <= *use_start
                            && *use_end <= site.end_byte
                            && super::build::singleton_exact(
                                &self.call_graph.resolve_call_site_full(site),
                            )
                            .is_some_and(|resolved| {
                                resolved.target.file == callee_file
                                    && resolved.target.name == callee_name
                                    && resolved.target.start_line == callee_start
                            })
                    })
            })
    }

    fn taint_neighbors(
        &self,
        node: NodeIndex,
        return_flow_mode: ReturnFlowMode,
    ) -> Vec<(NodeIndex, Relation)> {
        let mut out = Vec::new();
        let mut df: Vec<NodeIndex> = self
            .graph
            .edges(node)
            .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
            .map(|e| e.target())
            .collect();
        // General DFG neighbors can cross lines; NodeIndex order is build-deterministic.
        df.sort_by_key(|i| i.index());
        out.extend(df.into_iter().map(|t| (t, Relation::DataFlow)));

        if return_flow_mode == ReturnFlowMode::On {
            let mut returned: Vec<(NodeIndex, Relation)> = self
                .graph
                .edges(node)
                .filter_map(|edge| match edge.weight() {
                    CpgEdge::ReturnInput => Some((edge.target(), Relation::ReturnInput)),
                    CpgEdge::ReturnFlow { .. } => Some((edge.target(), Relation::ReturnFlow)),
                    _ => None,
                })
                .collect();
            returned.sort_by_key(|(target, relation)| {
                (
                    match relation {
                        Relation::ReturnInput => 0,
                        Relation::ReturnFlow => 1,
                        _ => 2,
                    },
                    target.index(),
                )
            });
            out.extend(returned);
        }

        match &self.graph[node] {
            // Use → same-line same-function `Def`s (assignment propagation: `x = y`, the use of `y`
            // flows to the def of `x`). Must stay within the SAME function — keying on (file,line)
            // alone leaks across functions (minified one-line-per-file JS/TS degenerates to
            // everything-taints-everything). Statement/byte-range scoping within a function is a
            // tracked follow-up (line-granular over-approximation).
            CpgNode::Variable {
                access: VarAccess::Use,
                file,
                line,
                function,
                function_start_line,
                ..
            } => {
                if let Some(at) = self.location_index.get(&(file.clone(), *line)) {
                    let mut same: Vec<NodeIndex> = at
                        .iter()
                        .copied()
                        .filter(|&o| {
                            matches!(
                                &self.graph[o],
                                CpgNode::Variable {
                                    access: VarAccess::Def,
                                    function: def_fn,
                                    function_start_line: def_start_line,
                                    ..
                                } if def_fn == function && def_start_line == function_start_line
                            ) && (return_flow_mode == ReturnFlowMode::Off
                                || !self.assignment_shortcut_suppressed(node, o))
                        })
                        .collect();
                    same.sort_by_key(|&i| self.node_sort_key(i));
                    out.extend(
                        same.into_iter()
                            .map(|t| (t, Relation::AssignmentPropagation)),
                    );
                }
            }
            // `Def` → same-function SAME-PATH `Use`s, re-supplying def→use edges `data_flow.rs` drops:
            //  - SIMPLE path (`x`): the DFG carries cross-line def-use, but the variable's *same-line*
            //    edge is dropped (param `ref_line == start`; regular `ref_line == def_line`), so a
            //    same-line `y = u; sink(y)` / `q = p; sink(q)` would dead-end. Recover same-line uses.
            //  - FIELD path (`o.data`): the DFG filters a field's refs to lines AFTER the def
            //    (`collect_path_refs`, `line > def_line`), so a loop-carried `def@N → use@M (M<N)` edge
            //    never exists and the loop's earlier use is a false negative. Recover ALL same-path uses
            //    on ANY line; the back-edge-aware `cfg_valid` prunes the CFG-infeasible ones (loop
            //    back-edges keep the loop-carried use). Same composition simple paths get from the DFG.
            // Same-path only (the variable's own def-use chain, not an assignment), so no cross-variable
            // leak; the safe over-approximation is consistent with the line-granular stance.
            CpgNode::Variable {
                access: VarAccess::Def,
                path,
                ..
            } => {
                let uses = if path.fields.is_empty() {
                    self.same_line_same_path_uses(node)
                } else {
                    self.same_function_same_path_uses_any_line(node)
                };
                for u in uses {
                    out.push((u, Relation::RecoveredDefUse));
                }
            }
            _ => {}
        }
        out
    }

    /// Function-scoped forward reachability over the SAME edges [`CodePropertyGraph::taint_trace`]
    /// traverses: DataFlow edges plus function-scoped same-line assignment propagation (including the
    /// Def → same-line-Use arm that re-supplies `data_flow.rs`'s dropped same-line def→use edges),
    /// staying within `start`'s `(file,function,function_start_line)`. Boundary classification
    /// ([`crate::reasoning::shape::reachability_for_node`]) uses this instead of the legacy
    /// `dfg_forward_reachable`, whose same-line propagation keys on `(file,line)` alone and so leaks
    /// across functions that share a line in minified one-line-per-file JS/TS — there it would
    /// classify a sink in a *different* function as `BoundaryExited` off an unrelated boundary.
    /// Routing both the trace BFS and boundary classification through one function-scoped primitive
    /// — and through one shared `taint_neighbors` — keeps them from diverging. The same-line def→use
    /// recovery lives in `taint_neighbors`, so this primitive needs no special seeding.
    pub(crate) fn forward_reachable_in_function(&self, start: NodeIndex) -> BTreeSet<NodeIndex> {
        let mut warnings = Vec::new();
        self.forward_reachable_in_function_ordered(start, None, &mut warnings)
    }

    /// Tier-1 opt-in adapter: preserve the existing FlowPath fan shape while
    /// sourcing reachability from the same mode-aware trace walker as Tier-2.
    pub fn taint_forward_cfg_with_return_flow(
        &self,
        sources: &[(String, usize)],
    ) -> Vec<crate::data_flow::FlowPath> {
        let trace = self.taint_trace_with_mode(sources, ReturnFlowMode::On);
        let mut paths = Vec::new();
        for (root, frontier) in &trace.frontier_by_root {
            let Some(source) = self.to_var_location(*root) else {
                continue;
            };
            let mut targets: Vec<_> = frontier
                .iter()
                .filter(|&&node| node != *root)
                .filter_map(|&node| self.to_var_location(node))
                .collect();
            targets.sort();
            targets.dedup();
            if targets.is_empty() {
                continue;
            }
            paths.push(crate::data_flow::FlowPath {
                edges: targets
                    .into_iter()
                    .map(|target| crate::data_flow::FlowEdge {
                        from: source.clone(),
                        to: target,
                    })
                    .collect(),
                cleansed_for: BTreeSet::new(),
            });
        }
        paths
    }

    pub(crate) fn forward_reachable_in_function_ordered(
        &self,
        start: NodeIndex,
        order: Option<&dyn SameLineOrderView>,
        ordering_warnings: &mut Vec<OrderingWarning>,
    ) -> BTreeSet<NodeIndex> {
        let Some(start_fn) = self.node_file_fn(start) else {
            return BTreeSet::new();
        };
        let mut visited: BTreeSet<NodeIndex> = BTreeSet::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();
        queue.push_back(start);
        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            for (next, rel) in self.taint_neighbors(node, ReturnFlowMode::Off) {
                if !self.ordering_admits_collect(node, next, rel, order, ordering_warnings) {
                    continue;
                }
                if self.node_file_fn(next).as_ref() == Some(&start_fn) && !visited.contains(&next) {
                    queue.push_back(next);
                }
            }
        }
        visited.remove(&start);
        visited
    }

    /// Same-function, same-path `Use` nodes for a `Def` on ANY line — re-supplies the field def→use
    /// edges `data_flow.rs` drops (a field path's refs are filtered to lines after the def, so
    /// loop-carried / earlier uses have no edge). `cfg_valid` (back-edge aware) prunes the
    /// CFG-infeasible ones. Empty unless `def` is a Variable `Def`.
    fn same_function_same_path_uses_any_line(&self, def: NodeIndex) -> Vec<NodeIndex> {
        let CpgNode::Variable {
            access: VarAccess::Def,
            file,
            function,
            function_start_line,
            path,
            ..
        } = &self.graph[def]
        else {
            return Vec::new();
        };
        let mut out: Vec<NodeIndex> = self
            .location_index
            .range((file.clone(), 0)..=(file.clone(), usize::MAX))
            .flat_map(|(_, nodes)| nodes.iter().copied())
            .filter(|&o| {
                matches!(
                    &self.graph[o],
                    CpgNode::Variable {
                        access: VarAccess::Use,
                        function: f2,
                        function_start_line: fsl2,
                        path: p2,
                        ..
                    } if f2 == function && fsl2 == function_start_line && p2 == path
                )
            })
            .collect();
        out.sort_by_key(|&i| self.node_sort_key(i));
        out
    }

    /// Same-line, same-function, same-path `Use` nodes for a `Def` — re-supplies the def→use edges
    /// `data_flow.rs` drops for a variable's own same-line references (param `ref_line == start`;
    /// regular `ref_line == def_line`). Empty unless `def` is a Variable `Def`. Used by
    /// [`Self::taint_neighbors`] so every hop (not just a boundary target) recovers these edges.
    fn same_line_same_path_uses(&self, def: NodeIndex) -> Vec<NodeIndex> {
        let CpgNode::Variable {
            access: VarAccess::Def,
            file,
            line,
            function,
            function_start_line,
            path,
            ..
        } = &self.graph[def]
        else {
            return Vec::new();
        };
        let Some(at) = self.location_index.get(&(file.clone(), *line)) else {
            return Vec::new();
        };
        let mut out: Vec<NodeIndex> = at
            .iter()
            .copied()
            .filter(|&o| {
                matches!(
                    &self.graph[o],
                    CpgNode::Variable {
                        access: VarAccess::Use,
                        function: f2,
                        function_start_line: fsl2,
                        path: p2,
                        ..
                    } if f2 == function && fsl2 == function_start_line && p2 == path
                )
            })
            .collect();
        out.sort_by_key(|&i| self.node_sort_key(i));
        out
    }

    fn cfg_scope_for_seed(
        &self,
        file: &str,
        line: usize,
        has_cfg: bool,
        trace: &mut Trace,
    ) -> Option<BTreeSet<(String, usize)>> {
        if !has_cfg {
            return Some(BTreeSet::new());
        }
        if self.statement_at(file, line).is_some() {
            // Union over ALL statements at the line, not just the first: a minified line can host
            // statements from two functions, and seeding the scope from one would CFG-prune the
            // other's roots into a false `NotReached` (the minified-JS class). The union is the safe
            // over-approximation.
            return Some(self.cfg_reachable_lines_unioned(file, line));
        }
        // A seed with no CFG statement node — e.g. a parameter def at the function-start line, or a
        // continuation line — falls back to pure taint: an over-approximation (no CFG pruning for
        // this seed's flow, the conservative/safe direction). CFG-precise parameter body-entry
        // handling is a tracked follow-up (docs/archive/plans/prism-query-layer/planA-followups.md).
        trace.degraded = true;
        trace.warnings.push(format!(
            "No CFG statement node at {file}:{line}; using pure-taint fallback for this seed"
        ));
        None
    }

    /// Per-node CFG validity. No CFG or a degraded seed means pure-taint fallback. Same source
    /// line is admitted explicitly because `cfg_reachable_lines` excludes the start line.
    ///
    /// PRECONDITION: this check applies only to intraprocedural hops reached at `depth == 0`.
    /// Once a node enters via a CallDescent edge, BFS intentionally walks callee-side flow in
    /// pure-taint mode to avoid falsely pruning legitimate re-entries into the seed function (e.g.
    /// mutual recursion). Stage B can refine this if a callee-scoped CFG strategy is introduced.
    fn cfg_valid(
        &self,
        src_file: &str,
        src_line: usize,
        cfg_set: Option<&BTreeSet<(String, usize)>>,
        has_cfg: bool,
        target: NodeIndex,
    ) -> bool {
        if !has_cfg {
            return true;
        }
        let Some(cfg_set) = cfg_set else {
            return true;
        };
        let (tfile, tline) = match &self.graph[target] {
            CpgNode::Variable { file, line, .. } => (file.clone(), *line),
            _ => return false,
        };
        if tfile == src_file && tline == src_line {
            return true;
        }
        // Reasoning path: fail-open continuation scan (unbounded, any reachable preceding statement),
        // so a long multi-line call or one with an inline nested callback before the tainted argument
        // is not a false negative. Production `taint_forward_cfg` keeps the byte-stable nearest-only
        // 20-line cap.
        cfg_set.contains(&(tfile.clone(), tline))
            || self.cfg_reachable_including_continuation(
                &tfile,
                tline,
                cfg_set,
                super::cfg_queries::ContinuationScan::ReasoningFailOpen,
            )
    }
}
