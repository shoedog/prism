//! Membrane Slice — module boundary impact analysis.
//!
//! **Question answered:** "Who depends on this API, and will they break if its contract changes?"
//!
//! When a change alters an exported/public function's contract (parameters,
//! return type, error behavior), shows every import site and how callers
//! consume the changed API. Like barrier slice inverted — instead of stopping
//! at the boundary, it *only* shows the boundary crossings.
//!
//! Catches breaking changes, missing error handling at call sites, and
//! parameter mismatches.

use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::MembraneSlice);
    let call_graph = CallGraph::build(files);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find functions with diff lines
        let mut changed_functions: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    let name = parsed.node_text(&name_node).to_string();
                    let range = parsed.node_line_range(&func_node);
                    changed_functions.insert(name, range);
                }
            }
        }

        for (func_name, (func_start, func_end)) in &changed_functions {
            // Find all callers in OTHER files (cross-module boundary),
            // respecting static linkage so file-local functions don't create
            // false cross-file edges.
            let callers = call_graph.callers_of_in_file(func_name, 1, Some(&diff_info.file_path));
            let cross_file_callers: Vec<_> = callers
                .iter()
                .filter(|(caller_id, _)| caller_id.file != diff_info.file_path)
                .collect();

            if cross_file_callers.is_empty() {
                continue;
            }

            let mut block =
                DiffBlock::new(block_id, diff_info.file_path.clone(), ModifyType::Modified);

            // Include the changed function (the API being modified)
            for line in *func_start..=*func_end {
                let is_diff = diff_info.diff_lines.contains(&line);
                block.add_line(&diff_info.file_path, line, is_diff);
            }

            // Include each cross-file caller with surrounding context
            for (caller_id, _) in &cross_file_callers {
                if let Some(caller_parsed) = files.get(&caller_id.file) {
                    // Include the caller function
                    block.add_line(&caller_id.file, caller_id.start_line, false);
                    block.add_line(&caller_id.file, caller_id.end_line, false);

                    // Find the specific call site line(s)
                    if let Some(sites) = call_graph.callers.get(func_name) {
                        for site in sites {
                            if site.caller.name == caller_id.name
                                && site.caller.file == caller_id.file
                            {
                                // Include the call site and a few lines of context
                                let ctx_start = site.line.saturating_sub(2);
                                let ctx_end = site.line + 2;
                                for l in ctx_start..=ctx_end {
                                    if l >= caller_id.start_line && l <= caller_id.end_line {
                                        block.add_line(&caller_id.file, l, false);
                                    }
                                }
                            }
                        }
                    }

                    // Check if the caller handles errors from the changed function
                    let caller_func = caller_parsed.find_function_by_name(&caller_id.name);
                    if let Some(cf) = caller_func {
                        let (cs, ce) = caller_parsed.node_line_range(&cf);
                        let caller_source: Vec<&str> = caller_parsed.source.lines().collect();

                        // Look for error handling around the call site
                        let has_error_handling = (cs..=ce).any(|l| {
                            if l == 0 || l > caller_source.len() {
                                return false;
                            }
                            let lt = caller_source[l - 1].trim();
                            // === Cross-language / generic ===
                            lt.contains("try")
                                || lt.contains("catch")
                                || lt.contains("except")
                                || lt.contains("if err")
                                || lt.contains("if error")
                                || lt.contains(".catch(")
                                // === Python ===
                                || lt.contains("raise_for_status(") // requests library
                                || lt.contains("raise ")            // re-raising exceptions
                                // === JavaScript / TypeScript ===
                                || lt.contains("throw ")            // throwing errors
                                || lt.contains("Promise.reject")    // promise rejection
                                || lt.contains(".finally(")         // cleanup handler
                                // === Go ===
                                || lt.contains("errors.Is(")        // Go 1.13+ error wrapping
                                || lt.contains("errors.As(")        // Go 1.13+ error type assertion
                                || lt.contains("log.Fatal")         // fatal error logging
                                || lt.contains("log.Panic")         // panic logging
                                || lt.contains("panic(")
                                // C/C++ return-value error handling
                                || lt.contains("if (ret < 0)")
                                || lt.contains("if (ret == -1)")
                                || lt.contains("if (ret != 0)")
                                || lt.contains("if (rc < 0)")
                                || lt.contains("if (rc != 0)")
                                || lt.contains("if (status < 0)")
                                || lt.contains("if (result < 0)")
                                // C/C++ NULL-pointer checks
                                || lt.contains("if (!") // covers if (!ptr), if (!ret)
                                || lt.contains("if (NULL")
                                || lt.contains("== NULL)")
                                || lt.contains("!= NULL)")
                                || lt.contains("if (ptr == NULL")
                                || lt.contains("== nullptr)")
                                || lt.contains("!= nullptr)")
                                // C errno / perror
                                || lt.contains("errno")
                                || lt.contains("perror(")
                                || lt.contains("strerror(")
                                // C/C++ assert macros
                                || lt.contains("assert(")
                                || lt.contains("ASSERT_")
                                || lt.contains("CHECK_")
                                || lt.contains("WARN_ON(")
                                || lt.contains("WARN_ON_ONCE(")
                                || lt.contains("BUG_ON(")
                                // Go-style (already partially covered by "if err")
                                || lt.contains("if (err")
                                // === Rust ===
                                || lt.contains("?")             // ? operator for error propagation
                                || lt.contains(".unwrap(")       // explicit unwrap
                                || lt.contains(".expect(")       // unwrap with message
                                || lt.contains(".unwrap_or(")    // unwrap with default
                                || lt.contains(".unwrap_or_else(")
                                || lt.contains("if let Err(")    // pattern match on error
                                || lt.contains("if let Ok(")     // pattern match on success
                                || lt.contains("match ")         // match expression (may handle Result/Option)
                                || lt.contains(".map_err(")      // error transformation
                                || lt.contains("Err(")           // error construction
                                // === Lua ===
                                || lt.contains("pcall(")         // protected call
                                || lt.contains("xpcall(")        // extended protected call
                                || lt.contains("assert(")        // assertion (Lua-style)
                                || lt.contains("error(") // error raising
                        });

                        if !has_error_handling {
                            // Mark the call site as potentially unprotected
                            // (it's already included but highlight it)
                            if let Some(sites) = call_graph.callers.get(func_name) {
                                for site in sites {
                                    if site.caller.name == caller_id.name {
                                        block.add_line(&caller_id.file, site.line, true);
                                        result.findings.push(SliceFinding {
                                            algorithm: "membrane".to_string(),
                                            file: caller_id.file.clone(),
                                            line: site.line,
                                            severity: "concern".to_string(),
                                            description: format!(
                                                "unprotected call to '{}' from '{}'",
                                                func_name, caller_id.name
                                            ),
                                            function_name: Some(caller_id.name.clone()),
                                            related_lines: vec![],
                                            related_files: vec![diff_info.file_path.clone()],
                                            category: Some("unprotected_caller".to_string()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
