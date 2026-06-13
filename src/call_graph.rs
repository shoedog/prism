//! Call graph construction from parsed files.
//!
//! Builds both forward (caller→callee) and reverse (callee→caller) call graphs
//! across all parsed files. Used by barrier slice, spiral slice, vertical slice,
//! circular slice, and 3D slice.

use crate::ast::ParsedFile;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A node in the call graph: a function identified by file path and name.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FunctionId {
    pub file: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// A call site: where a function is called from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CallSite {
    pub caller: FunctionId,
    pub callee_name: String,
    pub line: usize,
    /// Module/object qualifier for the call (e.g., `utils` in `utils.process()`).
    /// `None` for unqualified calls like `process()`.
    pub qualifier: Option<String>,
    /// S3 P6-lite: receiver type recovered syntactically at extraction time
    /// (typed param / constructor local, peeled). None = unrecovered.
    #[serde(default)]
    pub receiver_type: Option<String>,
    /// S3 P6-lite: which syntactic fact recovered `receiver_type`
    /// (telemetry + ResolutionKind split). Excluded from cmp_key —
    /// derived from the same scan as receiver_type.
    #[serde(default)]
    pub receiver_recovery: Option<crate::resolution::ReceiverRecovery>,
}

/// The call graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Per-file import maps: file_path → (local_alias → module_path).
    /// Used for import-aware call resolution (E6).
    pub imports: BTreeMap<String, BTreeMap<String, String>>,
    /// S3: (owner_key, method_name) → definitions. Trait impls dual-keyed
    /// under both the impl type and the trait name.
    #[serde(default)]
    pub methods: BTreeMap<(String, String), Vec<FunctionId>>,
    /// S3: owning type per method FunctionId (primary owner, not the trait).
    #[serde(default)]
    pub method_owners: BTreeMap<FunctionId, String>,
    /// S3 (Go): receiver variable name per method FunctionId.
    #[serde(default)]
    pub receiver_vars: BTreeMap<FunctionId, String>,
}

impl CallGraph {
    /// Create an empty call graph with no functions or edges.
    pub fn empty() -> Self {
        CallGraph {
            functions: BTreeMap::new(),
            calls: BTreeMap::new(),
            callers: BTreeMap::new(),
            static_functions: BTreeSet::new(),
            imports: BTreeMap::new(),
            methods: BTreeMap::new(),
            method_owners: BTreeMap::new(),
            receiver_vars: BTreeMap::new(),
        }
    }

    /// Build a lightweight call graph with only direct calls (Phases 1-2).
    ///
    /// Skips Phase 3 (indirect call resolution: function pointers, dispatch
    /// tables, parameter-passed callbacks). Used by `CpgContext::build_scoped()`
    /// to quickly identify which files are in the dependency neighborhood of a
    /// diff, before committing to a full CPG build on the scoped subset.
    pub fn build_skeleton(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();

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
                    functions
                        .entry(name.clone())
                        .or_default()
                        .push(func_id.clone());
                    let (owner, trait_key, recv_var) = Self::method_metadata(parsed, &func_node);
                    if let Some(o) = owner {
                        methods
                            .entry((o.clone(), name.clone()))
                            .or_default()
                            .push(func_id.clone());
                        method_owners.insert(func_id.clone(), o);
                    }
                    if let Some(t) = trait_key {
                        methods
                            .entry((t, name.clone()))
                            .or_default()
                            .push(func_id.clone());
                    }
                    if let Some(rv) = recv_var {
                        receiver_vars.insert(func_id.clone(), rv);
                    }

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
                        qualifier: Self::recover_self_receiver_qualifier(
                            parsed,
                            &callee_name,
                            line,
                            None,
                        ),
                        receiver_type: None,
                        receiver_recovery: None,
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
        }

        // Skip Phase 3 (indirect call resolution) — skeleton only needs direct calls.

        CallGraph {
            functions,
            calls,
            callers,
            static_functions,
            imports: BTreeMap::new(),
            methods,
            method_owners,
            receiver_vars,
        }
    }

    /// Build a call graph from all parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();

        // Collect per-file import maps for import-aware call resolution.
        let mut imports: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (file_path, parsed) in files {
            let file_imports = parsed.extract_imports();
            if !file_imports.is_empty() {
                imports.insert(file_path.clone(), file_imports);
            }
        }

        let ordered_files: Vec<(&String, &ParsedFile)> = files.iter().collect();

        struct FileFunctions {
            functions: Vec<(
                String,
                FunctionId,
                Option<String>,
                Option<String>,
                Option<String>,
            )>,
            static_functions: Vec<(String, String)>,
        }

        // Phase 1: Collect all function definitions per file in parallel, then
        // flatten serially in file order to preserve insertion order.
        let per_file_functions: Vec<FileFunctions> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_functions = Vec::new();
                let mut file_static_functions = Vec::new();

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
                        let (owner, trait_key, recv_var) =
                            Self::method_metadata(parsed, &func_node);
                        file_functions.push((name.clone(), func_id, owner, trait_key, recv_var));

                        // Detect C/C++ static linkage
                        if matches!(
                            parsed.language,
                            crate::languages::Language::C | crate::languages::Language::Cpp
                        ) {
                            if has_static_specifier(parsed, &func_node) {
                                file_static_functions.push((file_path.clone(), name));
                            }
                        }
                    }
                }

                FileFunctions {
                    functions: file_functions,
                    static_functions: file_static_functions,
                }
            })
            .collect();

        for file_functions in per_file_functions {
            for (name, func_id, owner, trait_key, recv_var) in file_functions.functions {
                functions
                    .entry(name.clone())
                    .or_default()
                    .push(func_id.clone());
                if let Some(o) = owner {
                    methods
                        .entry((o.clone(), name.clone()))
                        .or_default()
                        .push(func_id.clone());
                    method_owners.insert(func_id.clone(), o);
                }
                if let Some(t) = trait_key {
                    methods
                        .entry((t, name.clone()))
                        .or_default()
                        .push(func_id.clone());
                }
                if let Some(rv) = recv_var {
                    receiver_vars.insert(func_id.clone(), rv);
                }
            }
            for static_function in file_functions.static_functions {
                static_functions.insert(static_function);
            }
        }

        struct FileCallSites {
            call_sites: Vec<(FunctionId, CallSite)>,
        }

        // Phase 2: Find all call sites within each function in parallel, then
        // flatten serially in file order to preserve insertion order.
        let per_file_calls: Vec<FileCallSites> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_call_sites = Vec::new();

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
                    let call_sites =
                        parsed.function_calls_on_lines_with_qualifier(&func_node, &all_lines);

                    for (callee_name, line, qualifier) in call_sites {
                        let qualifier = Self::recover_self_receiver_qualifier(
                            parsed,
                            &callee_name,
                            line,
                            qualifier,
                        );
                        let site = CallSite {
                            caller: caller_id.clone(),
                            callee_name,
                            line,
                            qualifier,
                            receiver_type: None,
                            receiver_recovery: None,
                        };
                        file_call_sites.push((caller_id.clone(), site));
                    }
                }

                FileCallSites {
                    call_sites: file_call_sites,
                }
            })
            .collect();

        for file_calls in per_file_calls {
            for (caller_id, site) in file_calls.call_sites {
                calls
                    .entry(caller_id.clone())
                    .or_default()
                    .insert(site.clone());
                callers
                    .entry(site.callee_name.clone())
                    .or_default()
                    .push(site);
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
                                qualifier: None,
                                receiver_type: None,
                                receiver_recovery: None,
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
                                qualifier: None,
                                receiver_type: None,
                                receiver_recovery: None,
                            },
                        ));
                    }
                }
            }
        }

        // Level 4: struct field callback resolution (interprocedural).
        //
        // When a call goes through a struct field (timer->callback(data)),
        // the callee_name is the field name ("callback") and qualifier is set.
        // Search ALL functions and file scope for assignments like:
        //   anything->field_name = known_func
        //   anything.field_name = known_func
        //   .field_name = known_func  (designated initializer)
        //
        // Level-4 index (S1/B1): field -> file -> targets, built ONCE per build.
        // Reuses the legacy per-line core, so per-(field,file) results are
        // byte-identical to resolve_struct_field_assignment by construction.
        type Level4Index = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;
        let mut level4_index: Level4Index = BTreeMap::new();
        for (path, parsed) in files {
            let mut per_field: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for line in parsed.source.lines() {
                let trimmed = line.trim();
                for field in crate::ast::candidate_fields_on_line(trimmed) {
                    let targets = per_field.entry(field.clone()).or_default();
                    crate::ast::line_field_targets(trimmed, &field, &known_fn_names, targets);
                }
            }
            for (field, targets) in per_field {
                if !targets.is_empty() {
                    level4_index
                        .entry(field)
                        .or_default()
                        .insert(path.clone(), targets);
                }
            }
        }

        let mut level4_sites: Vec<(FunctionId, CallSite)> = Vec::new();
        for (caller_id, sites) in &calls {
            for site in sites {
                if functions.contains_key(&site.callee_name) {
                    continue;
                }
                if site.qualifier.is_none() {
                    continue; // Not a struct field call
                }
                if !site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    continue;
                }
                // Check if already resolved by earlier levels
                let already_resolved = extra_sites.iter().any(|(cid, es)| {
                    cid == caller_id
                        && es.line == site.line
                        && known_fn_names.contains(&es.callee_name)
                });
                if already_resolved {
                    continue;
                }

                // Search the prebuilt index for assignments to this field name
                let field_name = &site.callee_name;
                if let Some(by_file) = level4_index.get(field_name) {
                    for targets in by_file.values() {
                        for target in targets {
                            level4_sites.push((
                                caller_id.clone(),
                                CallSite {
                                    caller: caller_id.clone(),
                                    callee_name: target.clone(),
                                    line: site.line,
                                    qualifier: None,
                                    receiver_type: None,
                                    receiver_recovery: None,
                                },
                            ));
                        }
                    }
                }
            }
        }
        extra_sites.extend(level4_sites);

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
                // Skip if already resolved to a known function by Level 1
                let already_resolved = extra_sites.iter().any(|(cid, es)| {
                    cid == caller_id
                        && es.line == site.line
                        && known_fn_names.contains(&es.callee_name)
                });
                if already_resolved {
                    continue;
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
                                        qualifier: None,
                                        receiver_type: None,
                                        receiver_recovery: None,
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
                                            qualifier: None,
                                            receiver_type: None,
                                            receiver_recovery: None,
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
            imports,
            methods,
            method_owners,
            receiver_vars,
        }
    }

    // -----------------------------------------------------------------------
    // Incremental cache support (Phase 2)
    // -----------------------------------------------------------------------

    /// Remove all entries originating from the given files.
    ///
    /// Used by incremental cache update: when a file changes, its call graph
    /// contributions are stripped out before fresh data is merged in.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        // functions: remove FunctionId entries from excluded files.
        for func_ids in self.functions.values_mut() {
            func_ids.retain(|fid| !exclude.contains(&fid.file));
        }
        self.functions.retain(|_, v| !v.is_empty());

        // calls: remove entries where the caller is in an excluded file.
        self.calls
            .retain(|caller, _| !exclude.contains(&caller.file));

        // callers: remove CallSite entries where the caller is in an excluded file.
        for sites in self.callers.values_mut() {
            sites.retain(|s| !exclude.contains(&s.caller.file));
        }
        self.callers.retain(|_, v| !v.is_empty());

        // static_functions: remove entries for excluded files.
        self.static_functions.retain(|(f, _)| !exclude.contains(f));

        // imports: remove entries for excluded files.
        self.imports.retain(|f, _| !exclude.contains(f));

        // methods: remove FunctionId entries from excluded files.
        for func_ids in self.methods.values_mut() {
            func_ids.retain(|fid| !exclude.contains(&fid.file));
        }
        self.methods.retain(|_, v| !v.is_empty());

        // method_owners / receiver_vars: keyed by FunctionId.
        self.method_owners
            .retain(|fid, _| !exclude.contains(&fid.file));
        self.receiver_vars
            .retain(|fid, _| !exclude.contains(&fid.file));
    }

    /// Merge another CallGraph into this one.
    ///
    /// Entries from `other` are added to the existing data. Typically called
    /// after `remove_files` to splice in freshly-built data for changed files.
    pub fn merge(&mut self, other: CallGraph) {
        for (name, fids) in other.functions {
            self.functions.entry(name).or_default().extend(fids);
        }
        for (caller, sites) in other.calls {
            self.calls.entry(caller).or_default().extend(sites);
        }
        for (name, sites) in other.callers {
            self.callers.entry(name).or_default().extend(sites);
        }
        self.static_functions.extend(other.static_functions);
        self.imports.extend(other.imports);
        for (key, fids) in other.methods {
            self.methods.entry(key).or_default().extend(fids);
        }
        self.method_owners.extend(other.method_owners);
        self.receiver_vars.extend(other.receiver_vars);
    }

    /// Build a call graph from only the specified files (Phases 1+2: direct calls only).
    ///
    /// Unlike `build()`, this skips Phase 3 (indirect call resolution) because
    /// that requires knowledge of all functions, not just the subset. The caller
    /// should run `resolve_indirect` on the merged result.
    pub fn build_direct_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut imports: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();

        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
            let file_imports = parsed.extract_imports();
            if !file_imports.is_empty() {
                imports.insert(file_path.clone(), file_imports);
            }
        }

        // Phase 1: Collect function definitions from subset.
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
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
                    functions
                        .entry(name.clone())
                        .or_default()
                        .push(func_id.clone());
                    let (owner, trait_key, recv_var) = Self::method_metadata(parsed, &func_node);
                    if let Some(o) = owner {
                        methods
                            .entry((o.clone(), name.clone()))
                            .or_default()
                            .push(func_id.clone());
                        method_owners.insert(func_id.clone(), o);
                    }
                    if let Some(t) = trait_key {
                        methods
                            .entry((t, name.clone()))
                            .or_default()
                            .push(func_id.clone());
                    }
                    if let Some(rv) = recv_var {
                        receiver_vars.insert(func_id.clone(), rv);
                    }

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

        // Phase 2: Find call sites from subset.
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
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
                let call_sites =
                    parsed.function_calls_on_lines_with_qualifier(&func_node, &all_lines);

                for (callee_name, line, qualifier) in call_sites {
                    let qualifier = Self::recover_self_receiver_qualifier(
                        parsed,
                        &callee_name,
                        line,
                        qualifier,
                    );
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        line,
                        qualifier,
                        receiver_type: None,
                        receiver_recovery: None,
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
        }

        CallGraph {
            functions,
            calls,
            callers,
            static_functions,
            imports,
            methods,
            method_owners,
            receiver_vars,
        }
    }

    /// Resolve a callee name to the appropriate FunctionId(s), considering static linkage.
    ///
    /// If the callee name has a `static` definition in `caller_file`, return only that one.
    /// Otherwise, return all non-static definitions (excluding static functions in other files).
    pub fn resolve_callees(&self, callee_name: &str, caller_file: &str) -> Vec<&FunctionId> {
        self.resolve_callees_qualified(callee_name, caller_file, None)
    }

    /// Resolve a callee with an optional module qualifier (e.g., `utils` in `utils.process()`).
    ///
    /// When a qualifier is present, looks up the caller file's import map to find
    /// which module the qualifier refers to, then narrows candidates to functions
    /// in files whose path stem matches the module stem.
    ///
    /// **Known limitation:** if two files share the same stem (e.g., `lib/utils.py`
    /// and `src/utils.py`), both will match a qualifier resolving to `utils`.
    /// This is inherent to name-based resolution without full path normalization.
    pub fn resolve_callees_qualified(
        &self,
        callee_name: &str,
        caller_file: &str,
        qualifier: Option<&str>,
    ) -> Vec<&FunctionId> {
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

        // Import-aware narrowing: if a qualifier is present, resolve it via the import map.
        if let Some(qual) = qualifier {
            if let Some(file_imports) = self.imports.get(caller_file) {
                if let Some(module_path) = file_imports.get(qual) {
                    // Extract the module stem (last component) for file path matching.
                    let module_stem = module_path
                        .rsplit('/')
                        .next()
                        .unwrap_or(module_path)
                        .rsplit('.')
                        .last()
                        .unwrap_or(module_path);
                    let candidates: Vec<&FunctionId> = func_ids
                        .iter()
                        .filter(|fid| {
                            // Match if the file path contains the module stem
                            // e.g., module_path="utils" matches "src/utils.py"
                            let file_stem = fid
                                .file
                                .rsplit('/')
                                .next()
                                .unwrap_or(&fid.file)
                                .rsplit('.')
                                .last()
                                .unwrap_or(&fid.file);
                            file_stem == module_stem
                        })
                        .collect();
                    if !candidates.is_empty() {
                        return candidates;
                    }
                    // Fall through to standard resolution if no import match found
                }
            }
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

    fn method_metadata(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let owner = parsed
            .language
            .method_owner(func_node)
            .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
        let trait_key = parsed
            .language
            .rust_impl_trait(func_node)
            .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
        let recv_var = parsed
            .language
            .go_receiver_var(func_node)
            .map(|n| parsed.node_text(&n).to_string());
        (owner, trait_key, recv_var)
    }

    fn recover_self_receiver_qualifier(
        parsed: &ParsedFile,
        callee_name: &str,
        line: usize,
        qualifier: Option<String>,
    ) -> Option<String> {
        if qualifier.is_some() {
            return qualifier;
        }
        if !matches!(parsed.language, crate::languages::Language::Rust) {
            return None;
        }
        let line_text = parsed.source.lines().nth(line.saturating_sub(1))?;
        ["self", "this", "cls"]
            .into_iter()
            .find(|receiver| line_text.contains(&format!("{receiver}.{callee_name}")))
            .map(str::to_string)
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

    /// Resolve callers of a specific function, respecting static linkage.
    ///
    /// Returns only CallSites where the call to `callee_name` from the caller's
    /// file actually resolves to the function in `target_file`.
    pub fn resolve_callers(&self, callee_name: &str, target_file: &str) -> Vec<&CallSite> {
        match self.callers.get(callee_name) {
            None => Vec::new(),
            Some(sites) => sites
                .iter()
                .filter(|site| {
                    let resolved = self.resolve_callees(callee_name, &site.caller.file);
                    resolved.iter().any(|fid| fid.file == target_file)
                })
                .collect(),
        }
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
    fn cmp_key(&self) -> (&str, &str, usize, Option<&str>, Option<&str>) {
        (
            &self.caller.name,
            &self.callee_name,
            self.line,
            self.qualifier.as_deref(),
            self.receiver_type.as_deref(),
        )
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

#[cfg(test)]
mod tests {
    #[test]
    fn level4_index_matches_legacy_oracle_over_full_universe() {
        // Corpus: quirk fixtures + prism's own top-level src/*.rs sources.
        let mut sources: Vec<(String, String)> = vec![(
            "quirks.c".into(),
            "s.cb = f; t->cb = g;\ns.cb = f; t->cbx = g;\nstatic struct ops o = { .open = do_open, .close = do_close };\na.cb == nope;\nb.cb = &handler;\n".into(),
        )];
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(&src_dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                sources.push((
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                ));
            }
        }
        let known: std::collections::BTreeSet<String> = [
            "f", "g", "do_open", "do_close", "handler", "build", "new", "slice", "run",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        // Universe: ALL post-accessor identifiers across the corpus (index-independent)
        // + explicit negatives.
        let mut universe: std::collections::BTreeSet<String> =
            ["no_such_field", "cb", "cbx", "open", "close"]
                .iter()
                .map(|s| s.to_string())
                .collect();
        for (_, src) in &sources {
            for line in src.lines() {
                universe.extend(crate::ast::candidate_fields_on_line(line.trim()));
            }
        }

        // Build the index exactly as CallGraph::build does.
        let mut index: std::collections::BTreeMap<
            String,
            std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
        > = Default::default();
        for (path, src) in &sources {
            let mut per_field: std::collections::BTreeMap<
                String,
                std::collections::BTreeSet<String>,
            > = Default::default();
            for line in src.lines() {
                let trimmed = line.trim();
                for field in crate::ast::candidate_fields_on_line(trimmed) {
                    let t = per_field.entry(field.clone()).or_default();
                    crate::ast::line_field_targets(trimmed, &field, &known, t);
                }
            }
            for (field, t) in per_field {
                if !t.is_empty() {
                    index.entry(field).or_default().insert(path.clone(), t);
                }
            }
        }

        // Half 1 — EXCESS: every (field, file) the index claims must equal the legacy scan.
        for (field, by_file) in &index {
            for (path, targets) in by_file {
                let src = &sources.iter().find(|(p, _)| p == path).unwrap().1;
                let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
                let got: Vec<String> = targets.iter().cloned().collect();
                assert_eq!(got, legacy, "excess: field={field} file={path}");
            }
        }
        // Half 2 — MISSES: per universe field, legacy-scan ONLY files containing the
        // ->field / .field substring (the legacy has_field check hoisted to file level —
        // provably outcome-preserving), and assert the index agrees (absent key == empty).
        for field in &universe {
            let arrow = format!("->{field}");
            let dot = format!(".{field}");
            for (path, src) in &sources {
                if !(src.contains(&arrow) || src.contains(&dot)) {
                    continue; // legacy provably returns empty; index has no entry by construction
                }
                let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
                let from_index: Vec<String> = index
                    .get(field)
                    .and_then(|m| m.get(path))
                    .map(|s| s.iter().cloned().collect())
                    .unwrap_or_default();
                assert_eq!(from_index, legacy, "miss: field={field} file={path}");
            }
        }
    }
}
