//! Data flow graph construction for intraprocedural and interprocedural analysis.
//!
//! Builds def-use chains within functions and tracks flow through function
//! arguments and return values. Used by chopping, taint analysis, circular
//! slice, and delta slice.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{
    reaching_definitions, DefId, DefSite, FlowConfidence, FlowDoubt, RdFileStats, RdOutcome,
};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::{Hash, Hasher};

/// A definition or use of a variable at a specific location.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VarLocation {
    pub file: String,
    pub function: String,
    pub function_start_line: usize,
    pub line: usize,
    /// Structured access path for this variable reference.
    pub path: AccessPath,
    pub start_byte: usize,
    pub end_byte: usize,
    /// Backward-compatible accessor: returns the base variable name.
    /// Algorithms that don't need field sensitivity can use this.
    pub kind: VarAccessKind,
}

impl VarLocation {
    /// Identity tuple — byte excluded. Ord/Eq/Hash ALL derive from this so they
    /// cannot disagree (would corrupt BTreeMap/HashMap keys). Pinned by §7.6.
    fn identity_key(&self) -> (&str, &str, usize, usize, &AccessPath, VarAccessKind) {
        (
            &self.file,
            &self.function,
            self.function_start_line,
            self.line,
            &self.path,
            self.kind,
        )
    }

    /// Backward-compatible accessor: returns the base variable name.
    pub fn var_name(&self) -> &str {
        &self.path.base
    }
}

impl PartialEq for VarLocation {
    fn eq(&self, other: &Self) -> bool {
        self.identity_key() == other.identity_key()
    }
}

impl Eq for VarLocation {}

impl PartialOrd for VarLocation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VarLocation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identity_key().cmp(&other.identity_key())
    }
}

impl Hash for VarLocation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity_key().hash(state);
    }
}

/// Whether a variable access is a definition or a use.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum VarAccessKind {
    /// Variable is being assigned to (written).
    Def,
    /// Variable is being read.
    Use,
}

/// An edge in the data flow graph: a definition flows to a use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct FlowEdge {
    pub from: VarLocation,
    pub to: VarLocation,
}

/// A path through the data flow graph.
#[derive(Debug, Clone)]
pub struct FlowPath {
    pub edges: Vec<FlowEdge>,
    /// Sanitizer categories this flow has been cleansed for. A sink whose
    /// category is in this set causes the finding to be suppressed at
    /// evaluation time. Per spec §3.5 — path-sensitive: different flows
    /// reaching the same use-site may disagree on cleansing status.
    pub cleansed_for: BTreeSet<crate::frameworks::SanitizerCategory>,
}

/// The data flow graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFlowGraph {
    /// All def-use edges.
    pub edges: Vec<FlowEdge>,
    /// Index: (file, function, function_start_line, access_path) → definitions
    pub defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Index: (file, function, function_start_line, access_path) → uses
    pub uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Forward adjacency: def location → use locations it reaches
    pub forward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// Backward adjacency: use location → def locations it comes from
    pub backward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// One label per unique `(from, to)` FlowEdge endpoint pair. Primary state
    /// with the same file-partitioned lifecycle as `edges`.
    pub labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    /// Per-file reaching-definitions availability counters.
    pub rd_function_stats: BTreeMap<String, RdFileStats>,
}

impl DataFlowGraph {
    /// Create an empty data flow graph with no edges.
    pub fn empty() -> Self {
        DataFlowGraph {
            edges: Vec::new(),
            defs: BTreeMap::new(),
            uses: BTreeMap::new(),
            forward: BTreeMap::new(),
            backward: BTreeMap::new(),
            labels: BTreeMap::new(),
            rd_function_stats: BTreeMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Incremental cache support (Phase 2)
    // -----------------------------------------------------------------------

    /// Remove all entries originating from the given files.
    ///
    /// Strips out edges, labels, defs, uses, and RD stats where the file matches
    /// any in `exclude`, then rebuilds adjacency from the retained edges.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        // Remove edges that involve excluded files (either end).
        self.edges
            .retain(|e| !exclude.contains(&e.from.file) && !exclude.contains(&e.to.file));

        // Remove defs/uses entries for excluded files.
        self.defs
            .retain(|(file, _, _, _), _| !exclude.contains(file));
        self.uses
            .retain(|(file, _, _, _), _| !exclude.contains(file));
        self.labels
            .retain(|(from, to), _| !exclude.contains(&from.file) && !exclude.contains(&to.file));
        self.rd_function_stats
            .retain(|file, _| !exclude.contains(file));

        // Rebuild adjacency from retained edges.
        self.rebuild_adjacency();
    }

    /// Merge another DataFlowGraph into this one.
    ///
    /// Adds all primary per-file state from `other`, then rebuilds adjacency.
    pub fn merge(&mut self, other: DataFlowGraph) {
        self.edges.extend(other.edges);
        for (key, locs) in other.defs {
            self.defs.entry(key).or_default().extend(locs);
        }
        for (key, locs) in other.uses {
            self.uses.entry(key).or_default().extend(locs);
        }
        for (key, label) in other.labels {
            Self::insert_label(&mut self.labels, key, label);
        }
        for (file, stats) in other.rd_function_stats {
            self.rd_function_stats.entry(file).or_default().merge(stats);
        }
        self.rebuild_adjacency();
    }

    /// Rebuild the forward/backward adjacency maps from the current edges.
    fn rebuild_adjacency(&mut self) {
        self.forward.clear();
        self.backward.clear();
        for edge in &self.edges {
            self.forward
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            self.backward
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }
    }

    fn insert_label(
        labels: &mut BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
        key: (VarLocation, VarLocation),
        label: FlowConfidence,
    ) {
        labels
            .entry(key)
            .and_modify(|stored| *stored = stored.worst(label))
            .or_insert(label);
    }

    fn push_rd_def(rd_defs: &mut Vec<DefSite>, loc: &VarLocation, alias_derived: bool) {
        let id = DefId(
            u32::try_from(rd_defs.len())
                .expect("reaching-definition count exceeds the DefId representation"),
        );
        rd_defs.push(DefSite {
            id,
            path: loc.path.clone(),
            line: loc.line,
            start_byte: loc.start_byte,
            alias_derived,
        });
    }

    /// Build a data flow graph from only the specified files.
    ///
    /// Identical to `build()` but skips files not in `only_files`. Used by
    /// incremental cache update to process only changed files.
    pub fn build_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        // Filter to only the requested files and delegate to build.
        let filtered: BTreeMap<String, &ParsedFile> = files
            .iter()
            .filter(|(k, _)| only_files.contains(*k))
            .map(|(k, v)| (k.clone(), v))
            .collect();
        Self::build_from_refs(&filtered)
    }

    /// Build a data flow graph from parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let refs: BTreeMap<String, &ParsedFile> =
            files.iter().map(|(k, v)| (k.clone(), v)).collect();
        Self::build_from_refs(&refs)
    }

    /// Build a DFG from file references (shared implementation for build and build_subset).
    fn build_from_refs(files: &BTreeMap<String, &ParsedFile>) -> Self {
        let mut defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
            BTreeMap::new();
        let mut uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
            BTreeMap::new();
        let mut edges = Vec::new();
        let mut labels = BTreeMap::new();
        let mut rd_function_stats = BTreeMap::new();

        struct FileRefs {
            file_path: String,
            defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
            uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
            edges: Vec<FlowEdge>,
            labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
            rd_function_stats: RdFileStats,
        }

        let ordered_files: Vec<(&String, &ParsedFile)> =
            files.iter().map(|(path, parsed)| (path, *parsed)).collect();
        let per_file_refs: Vec<FileRefs> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
                    BTreeMap::new();
                let mut uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
                    BTreeMap::new();
                let mut edges = Vec::new();
                let mut labels = BTreeMap::new();
                let mut rd_function_stats = RdFileStats::default();

                for func_node in parsed.all_functions() {
                    let func_name = match parsed.language.function_name(&func_node) {
                        Some(n) => parsed.node_text(&n).to_string(),
                        None => continue,
                    };
                    let (start, end) = parsed.node_line_range(&func_node);
                    let all_lines: BTreeSet<usize> = (start..=end).collect();

                    // Phase 3: Build local alias map for this function.
                    // Tracks `ptr = dev` so that `ptr->field` resolves to `dev->field`.
                    // Also tracks destructuring: `const { name } = obj` → name resolves to obj.name.
                    let (alias_map, raw_aliases) =
                        Self::build_alias_map(parsed, &func_node, &all_lines);
                    let lvalue_spans =
                        parsed.assignment_lvalue_spans_on_lines(&func_node, &all_lines);
                    let mut def_locs_by_occurrence: BTreeMap<(AccessPath, usize), VarLocation> =
                        BTreeMap::new();
                    let mut resolved_def_locs_by_occurrence: BTreeMap<
                        (AccessPath, usize),
                        VarLocation,
                    > = BTreeMap::new();
                    let mut param_ref_jobs = Vec::new();
                    let mut rd_defs = Vec::new();
                    let function_edge_start = edges.len();

                    // Register function parameters as Defs at the function start line.
                    // Parameters are variable definitions that receive values from callers.
                    // Without this, interprocedural data flow edges have no target.
                    //
                    // Skip parameters that are only used via field access (e.g. `dev.name`)
                    // to preserve field isolation — a base-only Def would let taint on
                    // `dev.name` leak to unrelated fields like `dev.id`.
                    let param_occurrences = parsed.function_parameter_occurrences(&func_node);
                    for (param_name, param_start_byte, param_end_byte) in &param_occurrences {
                        let path = AccessPath::simple(param_name.clone());
                        // Skip parameters only used via field access (e.g. `dev.name`)
                        // to preserve field isolation.
                        if !parsed.has_bare_references(&func_node, param_name) {
                            continue;
                        }
                        let refs = parsed.find_path_references_scoped(&func_node, &path, start);
                        let param_decl_line = parsed.line_for_byte(*param_start_byte);

                        // Parameter Defs are pinned to the function's `start` line for stable lookup.
                        // `cpg/trace.rs` treats the arg→param DataFlow edge itself as the boundary for
                        // recursive/self calls, so body locals on the signature line are not classified
                        // as parameters.
                        let loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: start,
                            path: path.clone(),
                            start_byte: *param_start_byte,
                            end_byte: *param_end_byte,
                            kind: VarAccessKind::Def,
                        };
                        Self::push_rd_def(&mut rd_defs, &loc, false);
                        defs.entry((file_path.clone(), func_name.clone(), start, path.clone()))
                            .or_default()
                            .push(loc.clone());
                        param_ref_jobs.push((path, loc, refs, param_decl_line));
                    }

                    // Find all definitions (L-values) with structured access paths
                    for span in &lvalue_spans {
                        let loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: span.line,
                            path: span.path.clone(),
                            start_byte: span.start_byte,
                            end_byte: span.end_byte,
                            kind: VarAccessKind::Def,
                        };
                        Self::push_rd_def(&mut rd_defs, &loc, false);
                        defs.entry((
                            file_path.clone(),
                            func_name.clone(),
                            start,
                            span.path.clone(),
                        ))
                        .or_default()
                        .push(loc.clone());
                        def_locs_by_occurrence
                            .entry((span.path.clone(), span.line))
                            .or_insert(loc.clone());

                        // Phase 3: If this path's base is aliased, also register a def under
                        // the resolved path so edges connect through aliases.
                        // For field paths (ptr->field → dev->field): resolves base through alias.
                        // For simple paths from destructuring (name → device.name): creates
                        // a field-qualified def so taint connects through destructured variables.
                        if let Some(resolved) = Self::resolve_path(&alias_map, &span.path) {
                            if resolved != span.path.clone() {
                                let resolved_loc = VarLocation {
                                    file: file_path.clone(),
                                    function: func_name.clone(),
                                    function_start_line: start,
                                    line: span.line,
                                    path: resolved.clone(),
                                    start_byte: span.start_byte,
                                    end_byte: span.end_byte,
                                    kind: VarAccessKind::Def,
                                };
                                Self::push_rd_def(&mut rd_defs, &resolved_loc, true);
                                defs.entry((
                                    file_path.clone(),
                                    func_name.clone(),
                                    start,
                                    resolved.clone(),
                                ))
                                .or_default()
                                .push(resolved_loc.clone());
                                resolved_def_locs_by_occurrence
                                    .entry((resolved, span.line))
                                    .or_insert(resolved_loc);
                            }
                        }
                    }

                    // Register defs for alias-resolved paths that did not surface
                    // through lvalue spans. When they do surface, the lvalue loop
                    // above already registered the resolved def with the raw span.
                    for (alias, _target, alias_line) in &raw_aliases {
                        if lvalue_spans.iter().any(|span| {
                            span.line == *alias_line
                                && span.path.is_simple()
                                && span.path.base == *alias
                        }) {
                            continue;
                        }
                        if let Some(resolved_str) = alias_map.get(alias) {
                            let resolved_ap = AccessPath::from_expr(resolved_str);
                            if resolved_ap.has_fields() {
                                let anchor = parsed.line_start_byte(*alias_line);
                                let loc = VarLocation {
                                    file: file_path.clone(),
                                    function: func_name.clone(),
                                    function_start_line: start,
                                    line: *alias_line,
                                    path: resolved_ap.clone(),
                                    start_byte: anchor,
                                    end_byte: anchor,
                                    kind: VarAccessKind::Def,
                                };
                                debug_assert_eq!(loc.start_byte, loc.end_byte);
                                Self::push_rd_def(&mut rd_defs, &loc, true);
                                defs.entry((
                                    file_path.clone(),
                                    func_name.clone(),
                                    start,
                                    resolved_ap.clone(),
                                ))
                                .or_default()
                                .push(loc.clone());
                                resolved_def_locs_by_occurrence
                                    .entry((resolved_ap, *alias_line))
                                    .or_insert(loc);
                            }
                        }
                    }

                    // R-values: insert real-span occurrences before any line
                    // anchors so CPG var-node dedup keeps the real span.
                    let rvalue_spans =
                        parsed.rvalue_identifier_spans_on_lines(&func_node, &all_lines);
                    let mut preferred_use_locs: BTreeMap<(AccessPath, usize), VarLocation> =
                        BTreeMap::new();
                    for span in &rvalue_spans {
                        let use_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: span.line,
                            path: span.path.clone(),
                            start_byte: span.start_byte,
                            end_byte: span.end_byte,
                            kind: VarAccessKind::Use,
                        };
                        preferred_use_locs
                            .entry((span.path.clone(), span.line))
                            .or_insert(use_loc.clone());
                        uses.entry((
                            file_path.clone(),
                            func_name.clone(),
                            start,
                            span.path.clone(),
                        ))
                        .or_default()
                        .push(use_loc);
                    }

                    let mut get_use = |path: &AccessPath, ref_line: usize, allow: bool| {
                        if let Some(existing) = preferred_use_locs.get(&(path.clone(), ref_line)) {
                            return Some(existing.clone());
                        }
                        if !allow {
                            return None;
                        }
                        let anchor = parsed.line_start_byte(ref_line);
                        let use_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: ref_line,
                            path: path.clone(),
                            start_byte: anchor,
                            end_byte: anchor,
                            kind: VarAccessKind::Use,
                        };
                        debug_assert_eq!(use_loc.start_byte, use_loc.end_byte);
                        uses.entry((file_path.clone(), func_name.clone(), start, path.clone()))
                            .or_default()
                            .push(use_loc.clone());
                        preferred_use_locs.insert((path.clone(), ref_line), use_loc.clone());
                        Some(use_loc)
                    };

                    // Create edges from param def to all uses in the function body.
                    for (path, loc, refs, param_decl_line) in &param_ref_jobs {
                        for ref_line in refs {
                            let in_signature_line =
                                *ref_line >= start && *ref_line <= *param_decl_line;
                            if let Some(use_loc) = get_use(path, *ref_line, !in_signature_line) {
                                edges.push(FlowEdge {
                                    from: loc.clone(),
                                    to: use_loc,
                                });
                            }
                        }
                    }

                    // Find all uses of each defined variable/path.
                    // Cross-line references remain line-anchored unless a real
                    // rvalue span already exists for the same identity key.
                    for span in &lvalue_spans {
                        let path = &span.path;
                        let def_line = span.line;
                        let refs = parsed.find_path_references_scoped(&func_node, path, def_line);
                        for ref_line in &refs {
                            if *ref_line == def_line {
                                continue; // Skip self-reference
                            }
                            let Some(use_loc) = get_use(path, *ref_line, true) else {
                                continue;
                            };
                            let def_loc = def_locs_by_occurrence
                                .get(&(path.clone(), def_line))
                                .cloned()
                                .unwrap_or_else(|| VarLocation {
                                    file: file_path.clone(),
                                    function: func_name.clone(),
                                    function_start_line: start,
                                    line: def_line,
                                    path: path.clone(),
                                    start_byte: span.start_byte,
                                    end_byte: span.end_byte,
                                    kind: VarAccessKind::Def,
                                });
                            edges.push(FlowEdge {
                                from: def_loc,
                                to: use_loc,
                            });
                        }

                        // Phase 3: Also create edges for the alias-resolved path.
                        if let Some(resolved) = Self::resolve_path(&alias_map, path) {
                            if resolved != *path {
                                let resolved_refs = parsed
                                    .find_path_references_scoped(&func_node, &resolved, def_line);
                                for ref_line in &resolved_refs {
                                    if *ref_line == def_line {
                                        continue;
                                    }
                                    let Some(use_loc) = get_use(&resolved, *ref_line, true) else {
                                        continue;
                                    };
                                    let def_loc = resolved_def_locs_by_occurrence
                                        .get(&(resolved.clone(), def_line))
                                        .cloned()
                                        .unwrap_or_else(|| VarLocation {
                                            file: file_path.clone(),
                                            function: func_name.clone(),
                                            function_start_line: start,
                                            line: def_line,
                                            path: resolved.clone(),
                                            start_byte: span.start_byte,
                                            end_byte: span.end_byte,
                                            kind: VarAccessKind::Def,
                                        });
                                    edges.push(FlowEdge {
                                        from: def_loc,
                                        to: use_loc,
                                    });
                                }
                            }
                        }
                    }

                    let function_edges = &edges[function_edge_start..];
                    match reaching_definitions(parsed, &func_node, &rd_defs, function_edges) {
                        RdOutcome::Available(result) => {
                            for (key, label) in result.labels {
                                Self::insert_label(&mut labels, key, label);
                            }
                            // Task 5 owns loop-carried telemetry. Consume the
                            // classified set here without changing edge shape.
                            drop(result.loop_carried_edges);
                        }
                        RdOutcome::Unavailable(reason) => {
                            if reason.is_def_cap() || reason.is_line_cap() {
                                rd_function_stats.record_over_cap(func_name.clone(), start);
                            } else {
                                rd_function_stats.record_without_cfg(func_name.clone(), start);
                            }
                            let fallback = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
                            for edge in function_edges {
                                Self::insert_label(
                                    &mut labels,
                                    (edge.from.clone(), edge.to.clone()),
                                    fallback,
                                );
                            }
                        }
                    }
                }

                FileRefs {
                    file_path: file_path.clone(),
                    defs,
                    uses,
                    edges,
                    labels,
                    rd_function_stats,
                }
            })
            .collect();

        for file_refs in per_file_refs {
            // Invariant: per-file passes only register keys for their own file —
            // cross-file registration would break merge disjointness (S1 review MAJOR 2).
            debug_assert!(file_refs
                .defs
                .keys()
                .chain(file_refs.uses.keys())
                .all(|key| key.0.as_str() == file_refs.file_path.as_str()));
            debug_assert!(file_refs.labels.keys().all(|(from, to)| {
                from.file.as_str() == file_refs.file_path.as_str()
                    && to.file.as_str() == file_refs.file_path.as_str()
            }));
            for (key, locs) in file_refs.defs {
                defs.entry(key).or_default().extend(locs);
            }
            for (key, locs) in file_refs.uses {
                uses.entry(key).or_default().extend(locs);
            }
            edges.extend(file_refs.edges);
            for (key, label) in file_refs.labels {
                Self::insert_label(&mut labels, key, label);
            }
            rd_function_stats.insert(file_refs.file_path, file_refs.rd_function_stats);
        }

        // Build adjacency maps
        let mut forward: BTreeMap<VarLocation, Vec<VarLocation>> = BTreeMap::new();
        let mut backward: BTreeMap<VarLocation, Vec<VarLocation>> = BTreeMap::new();
        for edge in &edges {
            forward
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            backward
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }

        DataFlowGraph {
            edges,
            defs,
            uses,
            forward,
            backward,
            labels,
            rd_function_stats,
        }
    }

    /// Phase 3: Build a local alias map from simple assignments like `ptr = dev`.
    /// Returns (alias_map, raw_aliases) where alias_map maps alias name to
    /// fully resolved target, and raw_aliases are the original (alias, target, line) triples.
    fn build_alias_map(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> (BTreeMap<String, String>, Vec<(String, String, usize)>) {
        let raw_aliases = parsed.collect_alias_assignments(func_node, lines);
        let mut alias_map: BTreeMap<String, String> = BTreeMap::new();

        // Process in line order so earlier aliases are available for chain resolution
        for (alias, target, _line) in &raw_aliases {
            // Follow chain: if target itself is an alias, resolve transitively
            let mut resolved = target.clone();
            let mut depth = 0;
            while let Some(next) = alias_map.get(&resolved) {
                resolved = next.clone();
                depth += 1;
                if depth > 10 {
                    break; // Prevent infinite loops in pathological cases
                }
            }

            // For dotted targets (from destructuring), also resolve the base component.
            // e.g., name → ptr.name where ptr → device  →  resolved becomes device.name
            let mut base_depth = 0;
            loop {
                let target_ap = AccessPath::from_expr(&resolved);
                if target_ap.fields.is_empty() {
                    break; // Simple target, already handled by chain resolution above
                }
                if let Some(base_target) = alias_map.get(&target_ap.base) {
                    let base_ap = AccessPath::from_expr(base_target);
                    let mut fields = base_ap.fields;
                    fields.extend(target_ap.fields);
                    resolved = std::iter::once(base_ap.base.as_str())
                        .chain(fields.iter().map(|f| f.as_str()))
                        .collect::<Vec<_>>()
                        .join(".");
                    base_depth += 1;
                    if base_depth > 5 {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Don't create self-aliases
            if alias != &resolved {
                alias_map.insert(alias.clone(), resolved);
            }
        }

        (alias_map, raw_aliases)
    }

    /// Phase 3: If a path's base is aliased, return the resolved path.
    ///
    /// Simple alias: ptr → dev, path `ptr.field` resolves to `dev.field`.
    /// Destructuring alias: name → device.name, path `name` resolves to `device.name`.
    /// Destructuring + field: name → device.name, path `name.x` resolves to `device.name.x`.
    fn resolve_path(alias_map: &BTreeMap<String, String>, path: &AccessPath) -> Option<AccessPath> {
        if let Some(target) = alias_map.get(&path.base) {
            let target_path = AccessPath::from_expr(target);
            // Combine: target_path's fields + path's original fields
            let mut fields = target_path.fields;
            fields.extend(path.fields.iter().cloned());
            Some(AccessPath {
                base: target_path.base,
                fields,
            })
        } else {
            None
        }
    }

    /// Find all locations reachable forward from a given location (transitive).
    pub fn forward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());

        while let Some(loc) = queue.pop_front() {
            if !visited.insert(loc.clone()) {
                continue;
            }
            if let Some(nexts) = self.forward.get(&loc) {
                for next in nexts {
                    queue.push_back(next.clone());
                }
            }
            // Also follow through: if this is a use, find the def on the same line
            // (assignment propagation: x = y means use of y flows to def of x)
            if loc.kind == VarAccessKind::Use {
                let _key = (loc.file.clone(), loc.function.clone(), loc.path.clone());
                // Find defs of other variables on the same line
                for ((f, func, fsl, _path), def_locs) in &self.defs {
                    if f == &loc.file && func == &loc.function && *fsl == loc.function_start_line {
                        for dl in def_locs {
                            if dl.line == loc.line && !visited.contains(dl) {
                                queue.push_back(dl.clone());
                            }
                        }
                    }
                }
            }
        }

        visited.remove(from);
        visited
    }

    /// Find all locations reachable backward from a given location (transitive).
    pub fn backward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());

        while let Some(loc) = queue.pop_front() {
            if !visited.insert(loc.clone()) {
                continue;
            }
            if let Some(prevs) = self.backward.get(&loc) {
                for prev in prevs {
                    queue.push_back(prev.clone());
                }
            }
        }

        visited.remove(from);
        visited
    }

    /// Find all statements on any data flow path between source and sink.
    pub fn chop(
        &self,
        source_file: &str,
        source_line: usize,
        sink_file: &str,
        sink_line: usize,
    ) -> BTreeSet<(String, usize)> {
        // Forward reachable from source
        let source_locs: Vec<VarLocation> = self.all_locations_at(source_file, source_line);
        let mut forward_reachable = BTreeSet::new();
        for loc in &source_locs {
            forward_reachable.extend(self.forward_reachable(loc));
            forward_reachable.insert(loc.clone());
        }

        // Backward reachable from sink
        let sink_locs: Vec<VarLocation> = self.all_locations_at(sink_file, sink_line);
        let mut backward_reachable = BTreeSet::new();
        for loc in &sink_locs {
            backward_reachable.extend(self.backward_reachable(loc));
            backward_reachable.insert(loc.clone());
        }

        // Intersection: statements on paths between source and sink
        let on_path: BTreeSet<(String, usize)> = forward_reachable
            .intersection(&backward_reachable)
            .map(|loc| (loc.file.clone(), loc.line))
            .collect();

        // Always include source and sink
        let mut result = on_path;
        result.insert((source_file.to_string(), source_line));
        result.insert((sink_file.to_string(), sink_line));
        result
    }

    /// Get all VarLocations at a specific file and line.
    fn all_locations_at(&self, file: &str, line: usize) -> Vec<VarLocation> {
        let mut result = Vec::new();
        for locs in self.defs.values() {
            for loc in locs {
                if loc.file == file && loc.line == line {
                    result.push(loc.clone());
                }
            }
        }
        for locs in self.uses.values() {
            for loc in locs {
                if loc.file == file && loc.line == line {
                    result.push(loc.clone());
                }
            }
        }
        result
    }

    /// Forward taint propagation from a set of tainted locations.
    pub fn taint_forward(&self, taint_sources: &[(String, usize)]) -> Vec<FlowPath> {
        let mut paths = Vec::new();

        for (file, line) in taint_sources {
            let source_locs = self.all_locations_at(file, *line);
            for source in &source_locs {
                let reachable = self.forward_reachable(source);
                if !reachable.is_empty() {
                    // Build a path from source to each reachable sink
                    let path = FlowPath {
                        edges: reachable
                            .iter()
                            .map(|target| FlowEdge {
                                from: source.clone(),
                                to: target.clone(),
                            })
                            .collect(),
                        cleansed_for: BTreeSet::new(),
                    };
                    paths.push(path);
                }
            }
        }

        paths
    }

    /// Get all definition locations of a variable by base name in a file.
    /// Matches any AccessPath whose base equals `var_name` (field-insensitive lookup).
    pub fn all_defs_of(&self, file: &str, var_name: &str) -> Vec<VarLocation> {
        let mut result = Vec::new();
        for ((f, _func, _fsl, path), locs) in &self.defs {
            if f == file && path.base == var_name {
                result.extend(locs.clone());
            }
        }
        result
    }
}
