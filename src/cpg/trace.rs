//! A3: single inline-CFG-filtered predecessor BFS over the production petgraph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};

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
}

/// A def-use edge crossing a `(file,function)` boundary, recorded but not traversed in v1.
/// `Ord` so [`Trace::boundary`] can be a set — parallel DataFlow edges and multi-root traces
/// would otherwise push duplicate `(root, from, to)` triples and double-count downstream warnings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundaryEdge {
    pub root: NodeIndex,
    pub from: NodeIndex,
    pub to: NodeIndex,
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
    /// Single inline-CFG-filtered predecessor BFS. Every frontier member is CFG-reachable from
    /// the seed, so the parent walk-back never dead-ends. Determinism: neighbors sorted by
    /// NodeIndex; first enqueue per root wins the parent slot; DataFlow beats same-line.
    pub fn taint_trace(&self, sources: &[(String, usize)]) -> Trace {
        let has_cfg = self.has_cfg_edges();
        let mut trace = Trace::default();

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
                    CpgNode::Variable { function, .. } => Some(function.as_str()),
                    _ => None,
                })
                .collect::<BTreeSet<&str>>()
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
                    "Seed line {file}:{line} is a function signature / hosts multiple functions; \
                     using pure-taint fallback"
                ));
                None
            } else {
                self.cfg_scope_for_seed(file, line, has_cfg, &mut trace)
            };
            for root in roots {
                let Some(src_fn) = self.node_file_fn(root) else {
                    continue;
                };
                let mut enqueued: BTreeSet<NodeIndex> = BTreeSet::new();
                let mut queue = VecDeque::new();
                if enqueued.insert(root) {
                    trace.frontier_by_root.entry(root).or_default().insert(root);
                    queue.push_back(root);
                }
                while let Some(node) = queue.pop_front() {
                    for (next, rel) in self.taint_neighbors(node) {
                        // Non-Variable DataFlow targets are intentionally skipped (DataFlow currently
                        // connects Variables; a Statement-mediated edge is not a taint hop). This is
                        // covered by `test_taint_trace_skips_non_variable_dataflow_neighbors`.
                        let Some(next_fn) = self.node_file_fn(next) else {
                            continue;
                        };
                        // A neighbor is a boundary (taint exits into a callee v1 doesn't trace) if it
                        // crosses into a different function, OR it is a parameter binding — a Variable
                        // `Def` on a function's signature line, i.e. the target of an arg→param edge.
                        // The parameter test is what catches a *recursive* self-call (`f` → `f`), where
                        // `next_fn == src_fn` makes the name-keyed function check miss the boundary and
                        // the param's signature line is not CFG-reachable, dropping a real flow. This
                        // pairs the boundary and CFG-admit decisions so the ordering is structural, not
                        // a documented convention (see `cfg_valid`).
                        if next_fn != src_fn || self.is_parameter_binding(next) {
                            trace.boundary.insert(BoundaryEdge {
                                root,
                                from: node,
                                to: next,
                            });
                            continue;
                        }
                        if !self.cfg_valid(&src_fn.0, line, &cfg_scope, has_cfg, next) {
                            continue;
                        }
                        if enqueued.insert(next) {
                            trace.frontier_by_root.entry(root).or_default().insert(next);
                            trace.parents_by_root.insert((root, next), (node, rel));
                            queue.push_back(next);
                        }
                    }
                }
            }
        }
        trace
    }

    fn node_file_fn(&self, idx: NodeIndex) -> Option<(String, String)> {
        match &self.graph[idx] {
            CpgNode::Variable { file, function, .. } => Some((file.clone(), function.clone())),
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

    /// Is `node` a parameter binding — a Variable `Def` on a function's signature line? Such a node
    /// is only ever written by a caller's arg→param edge, so reaching it is a call boundary (true
    /// even for a recursive self-call, where name-keyed function identity would mask it).
    fn is_parameter_binding(&self, node: NodeIndex) -> bool {
        match &self.graph[node] {
            CpgNode::Variable {
                access: VarAccess::Def,
                file,
                line,
                ..
            } => self.function_starts_at(file, *line),
            _ => false,
        }
    }

    fn taint_neighbors(&self, node: NodeIndex) -> Vec<(NodeIndex, Relation)> {
        let mut out = Vec::new();
        let mut df: Vec<NodeIndex> = self
            .graph
            .edges(node)
            .filter(|e| matches!(e.weight(), CpgEdge::DataFlow))
            .map(|e| e.target())
            .collect();
        df.sort_by_key(|i| i.index());
        out.extend(df.into_iter().map(|t| (t, Relation::DataFlow)));

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
                                    ..
                                } if def_fn == function
                            )
                        })
                        .collect();
                    same.sort_by_key(|i| i.index());
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
    /// staying within `start`'s `(file,function)`. Boundary classification
    /// ([`crate::reasoning::shape::reachability_for_node`]) uses this instead of the legacy
    /// `dfg_forward_reachable`, whose same-line propagation keys on `(file,line)` alone and so leaks
    /// across functions that share a line in minified one-line-per-file JS/TS — there it would
    /// classify a sink in a *different* function as `BoundaryExited` off an unrelated boundary.
    /// Routing both the trace BFS and boundary classification through one function-scoped primitive
    /// — and through one shared `taint_neighbors` — keeps them from diverging. The same-line def→use
    /// recovery lives in `taint_neighbors`, so this primitive needs no special seeding.
    pub(crate) fn forward_reachable_in_function(&self, start: NodeIndex) -> BTreeSet<NodeIndex> {
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
            for (next, _rel) in self.taint_neighbors(node) {
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
                        path: p2,
                        ..
                    } if f2 == function && p2 == path
                )
            })
            .collect();
        out.sort_by_key(|i| i.index());
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
                        path: p2,
                        ..
                    } if f2 == function && p2 == path
                )
            })
            .collect();
        out.sort_by_key(|i| i.index());
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
        // handling is a tracked follow-up (docs/prism-query-layer/planA-followups.md).
        trace.degraded = true;
        trace.warnings.push(format!(
            "No CFG statement node at {file}:{line}; using pure-taint fallback for this seed"
        ));
        None
    }

    /// Per-node CFG validity. No CFG or a degraded seed means pure-taint fallback. Same source
    /// line is admitted explicitly because `cfg_reachable_lines` excludes the start line.
    ///
    /// PRECONDITION: this only gates *intraprocedural* targets. The BFS in `taint_trace` records and
    /// `continue`s cross-function neighbors as `BoundaryEdge`s *before* reaching this check, so
    /// `cfg_valid` never sees a cross-function target — its `cfg_set` (the seed function's CFG-
    /// reachable lines) contains no callee lines, and admitting one would be meaningless. Plan B's
    /// transitive callee-chain chase must preserve that ordering (or fold the function check in here)
    /// rather than relaxing the boundary `continue`, else a caller-seeded `cfg_set` silently prunes
    /// every callee line, or a widened union leaks cross-function admits.
    fn cfg_valid(
        &self,
        src_file: &str,
        src_line: usize,
        cfg_set: &Option<BTreeSet<(String, usize)>>,
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
