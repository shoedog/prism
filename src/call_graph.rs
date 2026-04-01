//! Call graph construction from parsed files.
//!
//! Builds both forward (caller→callee) and reverse (callee→caller) call graphs
//! across all parsed files. Used by barrier slice, spiral slice, vertical slice,
//! circular slice, and 3D slice.

use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A node in the call graph: a function identified by file path and name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId {
    pub file: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// A call site: where a function is called from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    pub caller: FunctionId,
    pub callee_name: String,
    pub line: usize,
}

/// The call graph for a set of parsed files.
#[derive(Debug)]
pub struct CallGraph {
    /// All known functions.
    pub functions: BTreeMap<String, Vec<FunctionId>>,
    /// Forward edges: function → set of functions it calls.
    pub calls: BTreeMap<FunctionId, BTreeSet<CallSite>>,
    /// Reverse edges: function name → set of call sites that invoke it.
    pub callers: BTreeMap<String, Vec<CallSite>>,
    /// Functions with file-local (static) linkage: `(file, name)` pairs.
    /// Used to disambiguate same-named functions across files.
    pub static_functions: BTreeSet<(String, String)>,
}

impl CallGraph {
    /// Build a call graph from all parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();

        // Phase 1: Collect all function definitions
        for (file_path, parsed) in files {
            for func_node in parsed.all_functions() {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    let name = parsed.node_text(&name_node).to_string();
                    let (start, end) = parsed.node_line_range(&func_node);
                    let func_id = FunctionId {
                        file: file_path.clone(),
                        name: name.clone(),
                        start_line: start,
                        end_line: end,
                    };
                    functions.entry(name.clone()).or_default().push(func_id);

                    // Detect C/C++ static linkage
                    if matches!(
                        parsed.language,
                        crate::languages::Language::C | crate::languages::Language::Cpp
                    ) {
                        if has_static_specifier(parsed, &func_node) {
                            static_functions.insert((file_path.clone(), name));
                        }
                    }
                }
            }
        }

        // Phase 2: Find all call sites within each function
        for (file_path, parsed) in files {
            for func_node in parsed.all_functions() {
                let func_name = match parsed.language.function_name(&func_node) {
                    Some(n) => parsed.node_text(&n).to_string(),
                    None => continue,
                };
                let (start, end) = parsed.node_line_range(&func_node);
                let caller_id = FunctionId {
                    file: file_path.clone(),
                    name: func_name,
                    start_line: start,
                    end_line: end,
                };

                let all_lines: BTreeSet<usize> = (start..=end).collect();
                let call_sites = parsed.function_calls_on_lines(&func_node, &all_lines);

                for (callee_name, line) in call_sites {
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        line,
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
        }

        // Phase 3: Resolve indirect call sites (function pointer variables and dispatch tables).
        //
        // For each callee_name that doesn't match any known function:
        //   Level 1: scan the caller's source for `callee_name = known_func` assignments
        //   Level 2: if callee_name contains `[`, find the array initializer and add all entries
        let known_fn_names: BTreeSet<String> = functions.keys().cloned().collect();
        let mut extra_sites: Vec<(FunctionId, CallSite)> = Vec::new();

        for (caller_id, sites) in &calls {
            for site in sites {
                if functions.contains_key(&site.callee_name) {
                    continue; // Already resolved by direct name match
                }

                let parsed = match files.get(&caller_id.file) {
                    Some(p) => p,
                    None => continue,
                };

                // Level 2: array dispatch table — callee_name like "handlers[0]"
                if site.callee_name.contains('[') {
                    let array_name = site.callee_name.split('[').next().unwrap_or("");
                    if array_name.is_empty() {
                        continue;
                    }
                    // Search the caller function's source, then file scope
                    let func_source = Self::extract_func_source(parsed, caller_id);
                    let targets = crate::ast::resolve_array_dispatch(
                        &func_source,
                        array_name,
                        &known_fn_names,
                    );
                    // Also check file scope for global dispatch tables
                    let file_targets = if targets.is_empty() {
                        crate::ast::resolve_array_dispatch(
                            &parsed.source,
                            array_name,
                            &known_fn_names,
                        )
                    } else {
                        Vec::new()
                    };
                    for target in targets.iter().chain(file_targets.iter()) {
                        extra_sites.push((
                            caller_id.clone(),
                            CallSite {
                                caller: caller_id.clone(),
                                callee_name: target.clone(),
                                line: site.line,
                            },
                        ));
                    }
                    continue;
                }

                // Level 1: local variable function pointer — callee_name is a plain identifier
                if site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    let func_source = Self::extract_func_source(parsed, caller_id);
                    if let Some(resolved) = crate::ast::resolve_fptr_assignment(
                        &func_source,
                        &site.callee_name,
                        &known_fn_names,
                    ) {
                        extra_sites.push((
                            caller_id.clone(),
                            CallSite {
                                caller: caller_id.clone(),
                                callee_name: resolved,
                                line: site.line,
                            },
                        ));
                    }
                }
            }
        }

        // Level 3: parameter-passed function pointers (1-hop interprocedural).
        //
        // When a function calls through a parameter (`cb(data)` where `cb` is a
        // parameter), check all callers of that function to see what argument they
        // pass for that parameter position. If the argument is a known function
        // name, add an edge from the original function to that target.
        //
        // This resolves patterns like:
        //   void execute(callback_fn cb, int data) { cb(data); }
        //   execute(handler_a, 1);  // → adds edge: execute → handler_a
        let mut level3_sites: Vec<(FunctionId, CallSite)> = Vec::new();
        for (caller_id, sites) in &calls {
            let parsed = match files.get(&caller_id.file) {
                Some(p) => p,
                None => continue,
            };

            // Get parameter names for this function
            let func_node = parsed.find_function_by_name(&caller_id.name);
            let param_names = match func_node {
                Some(ref f) => parsed.function_parameter_names(f),
                None => continue,
            };
            if param_names.is_empty() {
                continue;
            }

            for site in sites {
                // Skip if already resolved to a known function
                if functions.contains_key(&site.callee_name) {
                    continue;
                }
                // Skip non-plain identifiers (already handled by Level 1/2)
                if !site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    continue;
                }
                // Check if already resolved by Level 1
                if extra_sites.iter().any(|(cid, _)| {
                    cid == caller_id
                        && calls.get(caller_id).map_or(false, |s| {
                            s.iter().any(|cs| {
                                cs.callee_name == site.callee_name && cs.line == site.line
                            })
                        })
                }) {
                    // Check if Level 1 already resolved this specific callee
                    let already_resolved = extra_sites.iter().any(|(cid, es)| {
                        cid == caller_id
                            && es.line == site.line
                            && known_fn_names.contains(&es.callee_name)
                    });
                    if already_resolved {
                        continue;
                    }
                }

                // Is this callee_name one of the function's parameters?
                let param_idx = match param_names.iter().position(|p| p == &site.callee_name) {
                    Some(idx) => idx,
                    None => continue,
                };

                // Find all callers of this function and extract the argument at param_idx
                if let Some(caller_sites) = callers.get(&caller_id.name) {
                    for caller_site in caller_sites {
                        let caller_parsed = match files.get(&caller_site.caller.file) {
                            Some(p) => p,
                            None => continue,
                        };

                        // Extract the argument text at the parameter position
                        if let Some(arg_text) = caller_parsed.call_argument_text_at(
                            caller_site.line,
                            &caller_id.name,
                            param_idx,
                        ) {
                            // Check if the argument is a known function name
                            if known_fn_names.contains(&arg_text) {
                                level3_sites.push((
                                    caller_id.clone(),
                                    CallSite {
                                        caller: caller_id.clone(),
                                        callee_name: arg_text,
                                        line: site.line,
                                    },
                                ));
                            } else {
                                // Try Level 1 at the caller site: arg might be a local fptr variable
                                let caller_func_source =
                                    Self::extract_func_source(caller_parsed, &caller_site.caller);
                                if let Some(resolved) = crate::ast::resolve_fptr_assignment(
                                    &caller_func_source,
                                    &arg_text,
                                    &known_fn_names,
                                ) {
                                    level3_sites.push((
                                        caller_id.clone(),
                                        CallSite {
                                            caller: caller_id.clone(),
                                            callee_name: resolved,
                                            line: site.line,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        extra_sites.extend(level3_sites);

        // Add resolved edges
        for (caller_id, site) in extra_sites {
            let callee_name = site.callee_name.clone();
            calls.entry(caller_id).or_default().insert(site.clone());
            callers.entry(callee_name).or_default().push(site);
        }

        CallGraph {
            functions,
            calls,
            callers,
            static_functions,
        }
    }

    /// Resolve a callee name to the appropriate FunctionId(s), considering static linkage.
    ///
    /// If the callee name has a `static` definition in `caller_file`, return only that one.
    /// Otherwise, return all non-static definitions (excluding static functions in other files).
    pub fn resolve_callees(&self, callee_name: &str, caller_file: &str) -> Vec<&FunctionId> {
        let func_ids = match self.functions.get(callee_name) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        // If there's a static function with this name in the caller's file, use only that one
        if self
            .static_functions
            .contains(&(caller_file.to_string(), callee_name.to_string()))
        {
            return func_ids
                .iter()
                .filter(|fid| fid.file == caller_file)
                .collect();
        }

        // Otherwise, return all definitions that are NOT static in other files
        func_ids
            .iter()
            .filter(|fid| {
                // Include if: it's in the same file, OR it's not static
                fid.file == caller_file
                    || !self
                        .static_functions
                        .contains(&(fid.file.clone(), callee_name.to_string()))
            })
            .collect()
    }

    /// Extract the source text for a function from its parsed file.
    fn extract_func_source(parsed: &ParsedFile, func_id: &FunctionId) -> String {
        let lines: Vec<&str> = parsed.source.lines().collect();
        let start = func_id.start_line.saturating_sub(1); // 1-indexed to 0-indexed
        let end = func_id.end_line.min(lines.len());
        lines[start..end].join("\n")
    }

    /// Find all callers of a function by name, up to a given depth.
    ///
    /// Respects static linkage: a call to `func_name` in file X only counts
    /// if `resolve_callees(func_name, X)` includes a function in `target_file`
    /// (when provided). This prevents static functions in other files from
    /// being falsely reported as callers.
    pub fn callers_of(&self, func_name: &str, max_depth: usize) -> Vec<(FunctionId, usize)> {
        self.callers_of_in_file(func_name, max_depth, None)
    }

    /// Like `callers_of`, but only returns callers whose call actually resolves
    /// to a function in `target_file`.
    pub fn callers_of_in_file(
        &self,
        func_name: &str,
        max_depth: usize,
        target_file: Option<&str>,
    ) -> Vec<(FunctionId, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        queue.push_back((func_name.to_string(), 0));
        visited.insert(func_name.to_string());

        while let Some((name, depth)) = queue.pop_front() {
            if depth > 0 {
                if let Some(func_ids) = self.functions.get(&name) {
                    for fid in func_ids {
                        result.push((fid.clone(), depth));
                    }
                }
            }

            if depth >= max_depth {
                continue;
            }

            if let Some(sites) = self.callers.get(&name) {
                for site in sites {
                    if visited.contains(&site.caller.name) {
                        continue;
                    }

                    // Static linkage filter: if the caller is in a different file
                    // and their call to `name` resolves to a static function in
                    // their own file (not target_file), skip this caller.
                    if let Some(tf) = target_file {
                        let resolved = self.resolve_callees(&name, &site.caller.file);
                        let resolves_to_target = resolved.iter().any(|fid| fid.file == tf);
                        if !resolves_to_target {
                            continue;
                        }
                    }

                    visited.insert(site.caller.name.clone());
                    queue.push_back((site.caller.name.clone(), depth + 1));
                }
            }
        }

        result
    }

    /// Find all callees of a function by name, up to a given depth.
    pub fn callees_of(
        &self,
        func_name: &str,
        file: &str,
        max_depth: usize,
    ) -> Vec<(FunctionId, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<(FunctionId, usize)> = VecDeque::new();

        // Find the starting function
        if let Some(func_ids) = self.functions.get(func_name) {
            for fid in func_ids {
                if fid.file == file {
                    queue.push_back((fid.clone(), 0));
                    visited.insert(fid.name.clone());
                }
            }
        }

        while let Some((func_id, depth)) = queue.pop_front() {
            if depth > 0 {
                result.push((func_id.clone(), depth));
            }

            if depth >= max_depth {
                continue;
            }

            if let Some(sites) = self.calls.get(&func_id) {
                for site in sites {
                    if !visited.contains(&site.callee_name) {
                        visited.insert(site.callee_name.clone());
                        // Resolve callee to FunctionId, respecting static linkage
                        let callee_ids = self.resolve_callees(&site.callee_name, &func_id.file);
                        for callee_id in callee_ids {
                            queue.push_back((callee_id.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        result
    }

    /// Find the function containing a specific line in a file.
    pub fn function_at(&self, file: &str, line: usize) -> Option<&FunctionId> {
        for func_ids in self.functions.values() {
            for fid in func_ids {
                if fid.file == file && line >= fid.start_line && line <= fid.end_line {
                    return Some(fid);
                }
            }
        }
        None
    }

    /// Detect cycles in the call graph reachable from a set of functions.
    pub fn find_cycles_from(&self, start_funcs: &[&str]) -> Vec<Vec<FunctionId>> {
        let mut cycles = Vec::new();

        for &start_name in start_funcs {
            let mut path: Vec<FunctionId> = Vec::new();
            let mut visited: BTreeSet<String> = BTreeSet::new();

            if let Some(func_ids) = self.functions.get(start_name) {
                for fid in func_ids {
                    self.dfs_cycles(fid, &mut path, &mut visited, &mut cycles);
                }
            }
        }

        cycles
    }

    fn dfs_cycles(
        &self,
        node: &FunctionId,
        path: &mut Vec<FunctionId>,
        visited: &mut BTreeSet<String>,
        cycles: &mut Vec<Vec<FunctionId>>,
    ) {
        if let Some(pos) = path.iter().position(|f| f.name == node.name) {
            // Found a cycle
            let cycle: Vec<FunctionId> = path[pos..].to_vec();
            if !cycle.is_empty() {
                cycles.push(cycle);
            }
            return;
        }

        if visited.contains(&node.name) {
            return;
        }

        visited.insert(node.name.clone());
        path.push(node.clone());

        if let Some(sites) = self.calls.get(node) {
            for site in sites {
                let callee_ids = self.resolve_callees(&site.callee_name, &node.file);
                for callee_id in callee_ids {
                    self.dfs_cycles(callee_id, path, visited, cycles);
                }
            }
        }

        path.pop();
    }
}

impl CallSite {
    fn cmp_key(&self) -> (&str, &str, usize) {
        (&self.caller.name, &self.callee_name, self.line)
    }
}

impl PartialOrd for CallSite {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CallSite {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_key().cmp(&other.cmp_key())
    }
}

/// Check if a C/C++ function definition has a `static` storage class specifier.
fn has_static_specifier(parsed: &ParsedFile, func_node: &tree_sitter::Node<'_>) -> bool {
    let mut cursor = func_node.walk();
    for child in func_node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" && parsed.node_text(&child) == "static" {
            return true;
        }
    }
    false
}
