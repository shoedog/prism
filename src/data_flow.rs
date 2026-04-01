//! Data flow graph construction for intraprocedural and interprocedural analysis.
//!
//! Builds def-use chains within functions and tracks flow through function
//! arguments and return values. Used by chopping, taint analysis, circular
//! slice, and delta slice.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A definition or use of a variable at a specific location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VarAccessKind {
    /// Variable is being assigned to (written).
    Def,
    /// Variable is being read.
    Use,
}

/// An edge in the data flow graph: a definition flows to a use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug)]
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
    /// Build a data flow graph from parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
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
                let alias_map = Self::build_alias_map(parsed, &func_node, &all_lines);

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

                    // Phase 3: If this path has fields and its base is aliased, also register
                    // a def under the resolved path so edges connect through aliases.
                    // Only applies to field paths (ptr->field → dev->field), not simple
                    // variables which are already handled by def-use chains.
                    if path.has_fields() {
                        if let Some(resolved) = Self::resolve_path(&alias_map, path) {
                            let resolved_loc = VarLocation {
                                file: file_path.clone(),
                                function: func_name.clone(),
                                line: *line,
                                path: resolved.clone(),
                                kind: VarAccessKind::Def,
                            };
                            defs.entry((
                                file_path.clone(),
                                func_name.clone(),
                                resolved.clone(),
                            ))
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

                    // Phase 3: Also create edges for the alias-resolved path (field paths only)
                    if path.has_fields() {
                        if let Some(resolved) = Self::resolve_path(&alias_map, path) {
                            let resolved_refs = parsed.find_path_references_scoped(
                                &func_node,
                                &resolved,
                                *def_line,
                            );
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
    /// Returns a map from alias name to resolved target name, following chains.
    fn build_alias_map(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> BTreeMap<String, String> {
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
            // Don't create self-aliases
            if alias != &resolved {
                alias_map.insert(alias.clone(), resolved);
            }
        }

        alias_map
    }

    /// Phase 3: If a path's base is aliased, return the resolved path.
    /// e.g., if alias_map has ptr → dev, then path `ptr.field` resolves to `dev.field`.
    fn resolve_path(alias_map: &BTreeMap<String, String>, path: &AccessPath) -> Option<AccessPath> {
        if let Some(target) = alias_map.get(&path.base) {
            Some(AccessPath {
                base: target.clone(),
                fields: path.fields.clone(),
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
