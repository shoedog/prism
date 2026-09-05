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

use crate::call_graph::FunctionId;
use crate::cpg::query::ConfidenceFilter;
use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::resolution::{ResolutionKind, ResolvedCallEdge};
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

pub fn slice(ctx: &CpgContext, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::MembraneSlice);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let _parsed = match ctx.files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find functions with diff lines
        let mut changed_functions: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();
        for &line in &diff_info.diff_lines {
            if let Some((_, func_id)) = ctx.cpg.function_at(&diff_info.file_path, line) {
                changed_functions.insert(func_id.clone(), (func_id.start_line, func_id.end_line));
            }
        }

        for (func_id, (func_start, func_end)) in &changed_functions {
            let Some(func_idx) = ctx.cpg.function_node_for_id(func_id) else {
                continue;
            };
            // Find all callers in OTHER files (cross-module boundary),
            // preserving Exact and NameOnly recall for callback/fptr paths.
            let callers: Vec<_> = ctx
                .cpg
                .callers_of_node(func_idx, 1, ConfidenceFilter::All)
                .into_iter()
                .filter_map(|(idx, depth)| ctx.cpg.to_function_id(idx).map(|id| (id, depth)))
                .collect();

            // resolved_caller_edges scans every repo call site — compute once, not per caller.
            let all_caller_edges = ctx.cpg.call_graph.resolved_caller_edges(func_id);
            // P3 (F2): R6MultiOwnerCandidate is an unverified, capped NameOnly maybe-edge
            // (nav-only) — exclude it so Membrane doesn't assert an unconfirmed cross-module
            // caller as fact. Other NameOnly kinds are untouched (pre-existing recall).
            let caller_edges: Vec<_> = all_caller_edges
                .iter()
                .filter(|edge| edge.kind != ResolutionKind::R6MultiOwnerCandidate)
                .cloned()
                .collect();
            // Review-fix (F2 was too broad): only exclude a caller when it HAS raw
            // CallGraph evidence (a literal call site scanned by resolved_caller_edges)
            // AND every one of those raw edges is R6MultiOwnerCandidate. A caller with
            // NO raw edges at all — e.g. a CPG-graph-only CHA virtual-dispatch edge
            // minted in cpg/build.rs (C/C++ with a TypeDatabase), which never appears
            // in resolved_caller_edges since it bypasses CallGraph::calls entirely —
            // must pass through unfiltered, exactly as before the P3 candidate fix.
            let mut raw_edges_by_caller: BTreeMap<&FunctionId, Vec<&ResolvedCallEdge>> =
                BTreeMap::new();
            for edge in &all_caller_edges {
                raw_edges_by_caller
                    .entry(&edge.caller)
                    .or_default()
                    .push(edge);
            }
            let cross_file_callers: Vec<_> = callers
                .iter()
                .filter(|(caller_id, _)| {
                    if caller_id.file == diff_info.file_path {
                        return false;
                    }
                    match raw_edges_by_caller.get(caller_id) {
                        Some(edges) => edges
                            .iter()
                            .any(|e| e.kind != ResolutionKind::R6MultiOwnerCandidate),
                        None => true,
                    }
                })
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
                if let Some(caller_parsed) = ctx.files.get(&caller_id.file) {
                    // Include the caller function
                    block.add_line(&caller_id.file, caller_id.start_line, false);
                    block.add_line(&caller_id.file, caller_id.end_line, false);

                    // Find the specific call site line(s), keeping Exact and NameOnly.
                    for edge in &caller_edges {
                        if edge.caller == *caller_id {
                            // Include the call site and a few lines of context
                            let ctx_start = edge.call_site_line.saturating_sub(2);
                            let ctx_end = edge.call_site_line + 2;
                            for l in ctx_start..=ctx_end {
                                if l >= caller_id.start_line && l <= caller_id.end_line {
                                    block.add_line(&caller_id.file, l, false);
                                }
                            }
                        }
                    }

                    // Check if the caller handles errors — scan the EXACT caller body by its
                    // FunctionId range, NOT find_function_by_name (first same-name match), which
                    // would scan the wrong body in a file with multiple same-named functions.
                    {
                        let (cs, ce) = (caller_id.start_line, caller_id.end_line);
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
                                // C++ exception handling
                                || lt.contains("throw ")          // throwing exceptions
                                || lt.contains("catch (")         // catch with specific type
                                || lt.contains("catch(")          // catch without space
                                || lt.contains("noexcept")        // noexcept specification
                                || lt.contains("std::exception")  // standard exception type
                                // C++ RAII smart pointers (responsible resource handling at call site)
                                || lt.contains("unique_ptr")
                                || lt.contains("shared_ptr")
                                || lt.contains("make_unique")
                                || lt.contains("make_shared")
                                // C++ RAII lock guards
                                || lt.contains("lock_guard")
                                || lt.contains("unique_lock")
                                || lt.contains("scoped_lock")
                                // C++ optional/expected error handling
                                || lt.contains(".has_value(")
                                || lt.contains(".value_or(")
                                || lt.contains("std::optional")
                                || lt.contains("std::expected")
                                // C++ error code handling
                                || lt.contains("std::error_code")
                                || lt.contains("std::errc")
                                // Go-style (already partially covered by "if err")
                                || lt.contains("if (err")
                                // === Rust ===
                                || lt.contains(")?")            // Rust ? operator (foo()?) without matching JS/C ternary
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
                            // (it's already included but highlight it).
                            for edge in &caller_edges {
                                if edge.caller == *caller_id {
                                    block.add_line(&caller_id.file, edge.call_site_line, true);
                                    result.findings.push(SliceFinding {
                                        algorithm: "membrane".to_string(),
                                        file: caller_id.file.clone(),
                                        line: edge.call_site_line,
                                        severity: "concern".to_string(),
                                        description: format!(
                                            "unprotected call to '{}' from '{}'",
                                            func_id.name, caller_id.name
                                        ),
                                        function_name: Some(caller_id.name.clone()),
                                        related_lines: vec![],
                                        related_files: vec![diff_info.file_path.clone()],
                                        category: Some("unprotected_caller".to_string()),
                                        parse_quality: None,
                                        diagrams: vec![],
                                    });
                                    result.evidence.push(Some(EvidencePath {
                                        hops: vec![EvidenceHop::Call {
                                            edge: edge.clone(),
                                            confidence: edge.confidence,
                                        }],
                                        crossed_unlabeled: false,
                                    }));
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
