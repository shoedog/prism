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
}

impl CallGraph {
    /// Build a call graph from all parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();

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
                    functions.entry(name).or_default().push(func_id);
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
                    calls.entry(caller_id.clone()).or_default().insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
        }

        CallGraph {
            functions,
            calls,
            callers,
        }
    }

    /// Find all callers of a function by name, up to a given depth.
    pub fn callers_of(&self, func_name: &str, max_depth: usize) -> Vec<(FunctionId, usize)> {
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
                    if !visited.contains(&site.caller.name) {
                        visited.insert(site.caller.name.clone());
                        queue.push_back((site.caller.name.clone(), depth + 1));
                    }
                }
            }
        }

        result
    }

    /// Find all callees of a function by name, up to a given depth.
    pub fn callees_of(&self, func_name: &str, file: &str, max_depth: usize) -> Vec<(FunctionId, usize)> {
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
                        // Resolve callee to FunctionId
                        if let Some(callee_ids) = self.functions.get(&site.callee_name) {
                            for callee_id in callee_ids {
                                queue.push_back((callee_id.clone(), depth + 1));
                            }
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
                if let Some(callee_ids) = self.functions.get(&site.callee_name) {
                    for callee_id in callee_ids {
                        self.dfs_cycles(callee_id, path, visited, cycles);
                    }
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
