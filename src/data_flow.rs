//! Data flow graph construction for intraprocedural and interprocedural analysis.
//!
//! Builds def-use chains within functions and tracks flow through function
//! arguments and return values. Used by chopping, taint analysis, circular
//! slice, and delta slice.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A definition or use of a variable at a specific location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct VarLocation {
    pub file: String,
    pub function: String,
    pub line: usize,
    /// Structured access path for this variable reference.
    pub path: AccessPath,
    /// Backward-compatible accessor: returns the base variable name.
    /// Algorithms that don't need field sensitivity can use this.
    pub kind: VarAccessKind,
}

impl VarLocation {
    /// Backward-compatible accessor: returns the base variable name.
    pub fn var_name(&self) -> &str {
        &self.path.base
    }
}

/// Whether a variable access is a definition or a use.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
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
}

/// The data flow graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFlowGraph {
    /// All def-use edges.
    pub edges: Vec<FlowEdge>,
    /// Index: (file, function, access_path) → definitions
    pub defs: BTreeMap<(String, String, AccessPath), Vec<VarLocation>>,
    /// Index: (file, function, access_path) → uses
    pub uses: BTreeMap<(String, String, AccessPath), Vec<VarLocation>>,
    /// Forward adjacency: def location → use locations it reaches
    pub forward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// Backward adjacency: use location → def locations it comes from
    pub backward: BTreeMap<VarLocation, Vec<VarLocation>>,
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
        }
    }

    // -----------------------------------------------------------------------
    // Incremental cache support (Phase 2)
    // -----------------------------------------------------------------------

    /// Remove all entries originating from the given files.
    ///
    /// Strips out edges, defs, and uses where the file matches any in `exclude`,
    /// then rebuilds the forward/backward adjacency maps from the retained edges.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        // Remove edges that involve excluded files (either end).
        self.edges
            .retain(|e| !exclude.contains(&e.from.file) && !exclude.contains(&e.to.file));

        // Remove defs/uses entries for excluded files.
        self.defs.retain(|(file, _, _), _| !exclude.contains(file));
        self.uses.retain(|(file, _, _), _| !exclude.contains(file));

        // Rebuild adjacency from retained edges.
        self.rebuild_adjacency();
    }

    /// Merge another DataFlowGraph into this one.
    ///
    /// Adds all edges, defs, and uses from `other`, then rebuilds adjacency.
    pub fn merge(&mut self, other: DataFlowGraph) {
        self.edges.extend(other.edges);
        for (key, locs) in other.defs {
            self.defs.entry(key).or_default().extend(locs);
        }
        for (key, locs) in other.uses {
            self.uses.entry(key).or_default().extend(locs);
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
        let mut defs: BTreeMap<(String, String, AccessPath), Vec<VarLocation>> = BTreeMap::new();
        let mut uses: BTreeMap<(String, String, AccessPath), Vec<VarLocation>> = BTreeMap::new();
        let mut edges = Vec::new();

        for (file_path, parsed) in files {
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

                // Register defs for destructuring aliases that resolve to field paths.
                // `const { name } = device` doesn't appear as an assignment L-value,
                // so we create defs from the alias map directly.
                for (alias, _target, alias_line) in &raw_aliases {
                    if let Some(resolved_str) = alias_map.get(alias) {
                        let resolved_ap = AccessPath::from_expr(resolved_str);
                        if resolved_ap.has_fields() {
                            let loc = VarLocation {
                                file: file_path.clone(),
                                function: func_name.clone(),
                                line: *alias_line,
                                path: resolved_ap.clone(),
                                kind: VarAccessKind::Def,
                            };
                            defs.entry((file_path.clone(), func_name.clone(), resolved_ap))
                                .or_default()
                                .push(loc);
                        }
                    }
                }

                // Register function parameters as Defs at the function start line.
                // Parameters are variable definitions that receive values from callers.
                // Without this, interprocedural data flow edges have no target.
                //
                // Skip parameters that are only used via field access (e.g. `dev.name`)
                // to preserve field isolation — a base-only Def would let taint on
                // `dev.name` leak to unrelated fields like `dev.id`.
                let param_names = parsed.function_parameter_names(&func_node);
                for param_name in &param_names {
                    let path = AccessPath::simple(param_name);
                    // Skip parameters only used via field access (e.g. `dev.name`)
                    // to preserve field isolation.
                    if !parsed.has_bare_references(&func_node, param_name) {
                        continue;
                    }
                    let refs = parsed.find_path_references_scoped(&func_node, &path, start);

                    let loc = VarLocation {
                        file: file_path.clone(),
                        function: func_name.clone(),
                        line: start,
                        path: path.clone(),
                        kind: VarAccessKind::Def,
                    };
                    defs.entry((file_path.clone(), func_name.clone(), path.clone()))
                        .or_default()
                        .push(loc.clone());

                    // Create edges from param def to all uses in the function body.
                    for ref_line in &refs {
                        if *ref_line == start {
                            continue;
                        }
                        let use_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            line: *ref_line,
                            path: path.clone(),
                            kind: VarAccessKind::Use,
                        };
                        uses.entry((file_path.clone(), func_name.clone(), path.clone()))
                            .or_default()
                            .push(use_loc.clone());
                        edges.push(FlowEdge {
                            from: loc.clone(),
                            to: use_loc,
                        });
                    }
                }

                // Find all definitions (L-values) with structured access paths
                let lvalue_paths = parsed.assignment_lvalue_paths_on_lines(&func_node, &all_lines);
                for (path, line) in &lvalue_paths {
                    let loc = VarLocation {
                        file: file_path.clone(),
                        function: func_name.clone(),
                        line: *line,
                        path: path.clone(),
                        kind: VarAccessKind::Def,
                    };
                    defs.entry((file_path.clone(), func_name.clone(), path.clone()))
                        .or_default()
                        .push(loc);

                    // Phase 3: If this path's base is aliased, also register a def under
                    // the resolved path so edges connect through aliases.
                    // For field paths (ptr->field → dev->field): resolves base through alias.
                    // For simple paths from destructuring (name → device.name): creates
                    // a field-qualified def so taint connects through destructured variables.
                    if let Some(resolved) = Self::resolve_path(&alias_map, path) {
                        if resolved != *path {
                            let resolved_loc = VarLocation {
                                file: file_path.clone(),
                                function: func_name.clone(),
                                line: *line,
                                path: resolved.clone(),
                                kind: VarAccessKind::Def,
                            };
                            defs.entry((file_path.clone(), func_name.clone(), resolved.clone()))
                                .or_default()
                                .push(resolved_loc);
                        }
                    }
                }

                // Find all uses of each defined variable/path
                for (path, def_line) in &lvalue_paths {
                    let refs = parsed.find_path_references_scoped(&func_node, path, *def_line);
                    for ref_line in &refs {
                        if *ref_line == *def_line {
                            continue; // Skip self-reference
                        }
                        let use_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            line: *ref_line,
                            path: path.clone(),
                            kind: VarAccessKind::Use,
                        };
                        uses.entry((file_path.clone(), func_name.clone(), path.clone()))
                            .or_default()
                            .push(use_loc.clone());

                        let def_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            line: *def_line,
                            path: path.clone(),
                            kind: VarAccessKind::Def,
                        };
                        edges.push(FlowEdge {
                            from: def_loc,
                            to: use_loc,
                        });
                    }

                    // Phase 3: Also create edges for the alias-resolved path
                    if let Some(resolved) = Self::resolve_path(&alias_map, path) {
                        if resolved != *path {
                            let resolved_refs = parsed
                                .find_path_references_scoped(&func_node, &resolved, *def_line);
                            for ref_line in &resolved_refs {
                                if *ref_line == *def_line {
                                    continue;
                                }
                                let use_loc = VarLocation {
                                    file: file_path.clone(),
                                    function: func_name.clone(),
                                    line: *ref_line,
                                    path: resolved.clone(),
                                    kind: VarAccessKind::Use,
                                };
                                uses.entry((
                                    file_path.clone(),
                                    func_name.clone(),
                                    resolved.clone(),
                                ))
                                .or_default()
                                .push(use_loc.clone());

                                let def_loc = VarLocation {
                                    file: file_path.clone(),
                                    function: func_name.clone(),
                                    line: *def_line,
                                    path: resolved.clone(),
                                    kind: VarAccessKind::Def,
                                };
                                edges.push(FlowEdge {
                                    from: def_loc,
                                    to: use_loc,
                                });
                            }
                        }
                    }
                }

                // R-values: variables/paths read on each line
                let rvalue_paths = parsed.rvalue_identifier_paths_on_lines(&func_node, &all_lines);
                for (path, line) in &rvalue_paths {
                    let use_loc = VarLocation {
                        file: file_path.clone(),
                        function: func_name.clone(),
                        line: *line,
                        path: path.clone(),
                        kind: VarAccessKind::Use,
                    };
                    uses.entry((file_path.clone(), func_name.clone(), path.clone()))
                        .or_default()
                        .push(use_loc);
                }
            }
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
                for ((f, func, _path), def_locs) in &self.defs {
                    if f == &loc.file && func == &loc.function {
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
        for ((f, _func, path), locs) in &self.defs {
            if f == file && path.base == var_name {
                result.extend(locs.clone());
            }
        }
        result
    }
}
