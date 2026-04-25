//! Peer Consistency Slice — signature-matched guard divergence.
//!
//! **Question answered:** "A cluster of sibling functions shares a signature and
//! a dereference pattern — do they all handle the parameter safely, or do some
//! skip a guard that the rest have (or that they all lack)?"
//!
//! Motivating case (FRR CVE-2025-61102, T1-002): seven `show_vty_*` functions
//! in `ospfd/ospf_ext.c` all take `struct vty *vty` as their first parameter
//! and all call `vty_out(vty, ...)` without any `if (vty)` guard. When an
//! upstream dispatcher invokes one of them with `vty = NULL`, the daemon
//! crashes. Any single function looks fine in isolation; the signal is in the
//! *cluster* — 7 siblings uniformly unguarded is actionable even without
//! knowing the NULL-passing caller.
//!
//! Strategy (AST-only, C/C++ only):
//! 1. For each diff-touched function, find peers in the same file whose first
//!    parameter shares the same identifier name (proxy for matching type).
//! 2. For each member of the cluster, classify as:
//!      - *dereferences first param*: body contains `p->` or `fn(p,` / `fn(p)`
//!      - *guards first param*: body contains `if (p)`, `if (!p)`,
//!        `if (p ==`, `if (p !=`, or `if (p)` at any depth
//! 3. Emit one cluster finding if:
//!      - cluster size ≥ 3
//!      - ≥ 80 % of siblings dereference the first parameter
//!      - zero siblings guard it (uniform gap)
//!    or:
//!      - ≥ 3 siblings guard the first parameter and ≥ 1 dereferences without guarding
//!        (divergent gap — flag the divergent ones)

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

struct PeerInfo {
    name: String,
    start_line: usize,
    end_line: usize,
    first_param: String,
    dereferences: bool,
    guards: bool,
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::PeerConsistencySlice);
    let mut block_id = 0;
    let mut seen_clusters: BTreeSet<(String, String)> = BTreeSet::new();

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };
        if !matches!(parsed.language, Language::C | Language::Cpp) {
            continue;
        }

        // Collect all functions in this file with their first-parameter name.
        let all_peers = collect_peers_in_file(parsed);
        if all_peers.len() < 3 {
            continue;
        }

        // Which peer param names are touched by the diff?
        let touched_params: BTreeSet<String> = all_peers
            .iter()
            .filter(|p| diff_lines_overlap(&diff_info.diff_lines, p.start_line, p.end_line))
            .map(|p| p.first_param.clone())
            .filter(|n| !n.is_empty())
            .collect();

        for param in &touched_params {
            let key = (diff_info.file_path.clone(), param.clone());
            if !seen_clusters.insert(key) {
                continue;
            }

            let cluster: Vec<&PeerInfo> = all_peers
                .iter()
                .filter(|p| &p.first_param == param)
                .collect();
            if cluster.len() < 3 {
                continue;
            }

            let deref_count = cluster.iter().filter(|p| p.dereferences).count();
            let guard_count = cluster.iter().filter(|p| p.guards).count();
            let deref_ratio = deref_count as f64 / cluster.len() as f64;

            let (severity, description) = if guard_count == 0 && deref_ratio >= 0.8 {
                (
                    "concern",
                    format!(
                        "{} sibling functions in this file share first parameter `{}` and all dereference it via `{}->…` or `fn({}, …)`; none contains an `if ({})` / `if (!{})` NULL guard. If any caller (possibly in unchanged dispatcher code) passes NULL, every member of the cluster crashes. Review whether a NULL caller path exists — e.g. log/debug wrappers, callback tables, or zlog-style dispatchers.",
                        cluster.len(), param, param, param, param, param
                    ),
                )
            } else if guard_count >= 3 && deref_count > guard_count {
                let divergent: Vec<&&PeerInfo> = cluster
                    .iter()
                    .filter(|p| p.dereferences && !p.guards)
                    .collect();
                if divergent.is_empty() {
                    continue;
                }
                (
                    "warning",
                    format!(
                        "{}/{} sibling functions guard first parameter `{}` before dereferencing it, but {} divergent sibling(s) skip the guard: {}. Add a NULL check to match the rest of the cluster.",
                        guard_count,
                        cluster.len(),
                        param,
                        divergent.len(),
                        divergent
                            .iter()
                            .map(|p| p.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            } else {
                continue;
            };

            // Pick a representative line (the first cluster member touched by
            // the diff if any, else the first cluster member).
            let primary = cluster
                .iter()
                .find(|p| diff_lines_overlap(&diff_info.diff_lines, p.start_line, p.end_line))
                .copied()
                .unwrap_or(cluster[0]);

            let related_lines: Vec<usize> = cluster
                .iter()
                .filter(|p| p.name != primary.name)
                .map(|p| p.start_line)
                .collect();

            result.findings.push(SliceFinding {
                algorithm: "peer_consistency".to_string(),
                file: diff_info.file_path.clone(),
                line: primary.start_line,
                severity: severity.to_string(),
                description,
                function_name: Some(primary.name.clone()),
                related_lines,
                related_files: vec![],
                category: Some("peer_guard_divergence".to_string()),
                parse_quality: None,
            });

            // Emit a block showing the cluster (function signatures + body start).
            let mut block =
                DiffBlock::new(block_id, diff_info.file_path.clone(), ModifyType::Modified);
            for peer in &cluster {
                let preview_end = peer.end_line.min(peer.start_line + 8);
                for line in peer.start_line..=preview_end {
                    let is_diff = diff_info.diff_lines.contains(&line);
                    block.add_line(&diff_info.file_path, line, is_diff);
                }
            }
            if !block.file_line_map.is_empty() {
                result.blocks.push(block);
                block_id += 1;
            }
        }
    }

    Ok(result)
}

fn diff_lines_overlap(diff_lines: &BTreeSet<usize>, start: usize, end: usize) -> bool {
    diff_lines.range(start..=end).next().is_some()
}

fn collect_peers_in_file(parsed: &ParsedFile) -> Vec<PeerInfo> {
    let mut peers = Vec::new();
    let source_lines: Vec<&str> = parsed.source.lines().collect();

    for func in parsed.all_functions() {
        let name = match parsed.language.function_name(&func) {
            Some(n) => parsed.node_text(&n).to_string(),
            None => continue,
        };
        let params = parsed.function_parameter_names(&func);
        let first_param = match params.first() {
            Some(p) if !p.is_empty() => p.clone(),
            _ => continue,
        };
        let (start, end) = parsed.node_line_range(&func);
        let body_lines: &[&str] = if start == 0 || start > source_lines.len() {
            &[]
        } else {
            let lo = start - 1;
            let hi = end.min(source_lines.len());
            &source_lines[lo..hi]
        };
        let dereferences = body_mentions_deref(body_lines, &first_param);
        let guards = body_has_guard(body_lines, &first_param);
        peers.push(PeerInfo {
            name,
            start_line: start,
            end_line: end,
            first_param,
            dereferences,
            guards,
        });
    }
    peers
}

/// Does the body dereference `param`?  Heuristic: `param->`, `(*param)`, or
/// any call that passes `param` as the first argument (`ident(param,` / `ident(param)`).
fn body_mentions_deref(body_lines: &[&str], param: &str) -> bool {
    let arrow = format!("{}->", param);
    let star = format!("(*{}", param);
    let star_space = format!("* {}", param);
    for line in body_lines {
        let stripped = strip_line_comment(line);
        if stripped.contains(&arrow) || stripped.contains(&star) {
            return true;
        }
        // `fn(param,` or `fn(param)` — param passed as first arg to another call.
        if let Some(idx) = stripped.find(&format!("({}", param)) {
            let after = &stripped[idx + 1 + param.len()..];
            if after
                .chars()
                .next()
                .map(|c| c == ',' || c == ')')
                .unwrap_or(false)
            {
                // Make sure the `(` is preceded by an identifier character (a call),
                // not a parenthesised expression like `(vty == NULL)`.
                let before = &stripped[..idx];
                if before
                    .chars()
                    .last()
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
        // Dereferenced assignment: `*param = ...`
        if stripped.trim_start().starts_with(&star_space) {
            return true;
        }
    }
    false
}

/// Does the body contain any guard check on `param`?  Accepts the common forms
/// `if (param)`, `if (!param)`, `if (param == NULL)`, `if (param != NULL)`,
/// `if (!param)` / `if ( !param )`, plus ternary and assert variants.
fn body_has_guard(body_lines: &[&str], param: &str) -> bool {
    let patterns = [
        format!("if ({})", param),
        format!("if ({} ", param),
        format!("if (!{})", param),
        format!("if (!{}", param),
        format!("if ({}==", param),
        format!("if ({}!=", param),
        format!("assert({}", param),
        format!("assert ({}", param),
        format!("return {} ==", param),
        format!("return !{}", param),
    ];
    for line in body_lines {
        let stripped = strip_line_comment(line).replace(char::is_whitespace, "");
        for pat in &patterns {
            let pat_compact: String = pat.chars().filter(|c| !c.is_whitespace()).collect();
            if stripped.contains(&pat_compact) {
                return true;
            }
        }
    }
    false
}

fn strip_line_comment(line: &str) -> &str {
    if let Some(pos) = line.find("//") {
        &line[..pos]
    } else {
        line
    }
}
