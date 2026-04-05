//! 3D Slice — temporal-structural integration.
//!
//! Combines structural analysis (horizontal breadth, vertical depth) with
//! temporal data from git history (churn rate, recent modifications, active
//! contributors) into a unified risk model.
//!
//! Three axes:
//! - X: Structural breadth (peer relationships via horizontal slice)
//! - Y: Structural depth (caller/callee via call graph)
//! - Z: Temporal (git history)

use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::{anyhow, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Configuration for 3D slicing.
#[derive(Debug, Clone)]
pub struct ThreeDConfig {
    /// How many days back to look in git history.
    pub temporal_days: usize,
    /// Path to the git repository.
    pub git_dir: String,
}

impl Default for ThreeDConfig {
    fn default() -> Self {
        Self {
            temporal_days: 90,
            git_dir: ".".to_string(),
        }
    }
}

/// Risk score for a function.
#[derive(Debug, Clone)]
pub struct RiskScore {
    pub file: String,
    pub function_name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Number of callers + callees.
    pub structural_coupling: usize,
    /// Number of commits touching this file in the temporal window.
    pub temporal_activity: usize,
    /// Number of diff lines in this function.
    pub change_complexity: usize,
    /// Combined risk: structural * temporal * complexity.
    pub risk: f64,
}

pub fn slice(ctx: &CpgContext, diff: &DiffInput, config: &ThreeDConfig) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::ThreeDSlice);

    // Get temporal data from git
    let git_churn = get_git_churn(&config.git_dir, config.temporal_days)?;

    // Calculate risk scores for all functions containing diff lines
    let mut scores: Vec<RiskScore> = Vec::new();

    for diff_info in &diff.files {
        let _parsed = match ctx.files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        let mut scored_funcs: BTreeSet<String> = BTreeSet::new();

        for &line in &diff_info.diff_lines {
            if let Some((_idx, func_id)) = ctx.cpg.function_at(&diff_info.file_path, line) {
                if scored_funcs.contains(&func_id.name) {
                    continue;
                }
                scored_funcs.insert(func_id.name.clone());

                // Structural coupling: callers + callees (file-scoped to
                // disambiguate static functions with the same name)
                let callers =
                    ctx.cpg
                        .callers_of_in_file(&func_id.name, 1, Some(&diff_info.file_path));
                let callees = ctx.cpg.callees_of(&func_id.name, &diff_info.file_path, 1);
                let structural_coupling = callers.len() + callees.len();

                // Temporal activity
                let temporal_activity = git_churn.get(&diff_info.file_path).copied().unwrap_or(0);

                // Change complexity
                let change_complexity = diff_info
                    .diff_lines
                    .iter()
                    .filter(|&&l| l >= func_id.start_line && l <= func_id.end_line)
                    .count();

                // Risk score
                let risk = (structural_coupling.max(1) as f64)
                    * (temporal_activity.max(1) as f64)
                    * (change_complexity.max(1) as f64);

                scores.push(RiskScore {
                    file: diff_info.file_path.clone(),
                    function_name: func_id.name.clone(),
                    start_line: func_id.start_line,
                    end_line: func_id.end_line,
                    structural_coupling,
                    temporal_activity,
                    change_complexity,
                    risk,
                });
            }
        }
    }

    // Also score connected functions (callers/callees)
    let diff_funcs: Vec<(String, String)> = scores
        .iter()
        .map(|s| (s.file.clone(), s.function_name.clone()))
        .collect();
    for (func_file, func_name) in &diff_funcs {
        let callers = ctx.cpg.callers_of_in_file(func_name, 2, Some(func_file));
        for (caller_id, _) in &callers {
            if scores
                .iter()
                .any(|s| s.function_name == caller_id.name && s.file == caller_id.file)
            {
                continue;
            }

            let temporal_activity = git_churn.get(&caller_id.file).copied().unwrap_or(0);
            let callers_of_caller =
                ctx.cpg
                    .callers_of_in_file(&caller_id.name, 1, Some(&caller_id.file));
            let callees_of_caller = ctx.cpg.callees_of(&caller_id.name, &caller_id.file, 1);

            scores.push(RiskScore {
                file: caller_id.file.clone(),
                function_name: caller_id.name.clone(),
                start_line: caller_id.start_line,
                end_line: caller_id.end_line,
                structural_coupling: callers_of_caller.len() + callees_of_caller.len(),
                temporal_activity,
                change_complexity: 0,
                risk: (callers_of_caller.len() + callees_of_caller.len()).max(1) as f64
                    * temporal_activity.max(1) as f64,
            });
        }
    }

    // Sort by risk descending
    scores.sort_by(|a, b| {
        b.risk
            .partial_cmp(&a.risk)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Build output blocks sorted by risk
    for (block_id, score) in scores.iter().enumerate() {
        let mut block = DiffBlock::new(block_id, score.file.clone(), ModifyType::Modified);

        // Include function boundaries
        block.add_line(&score.file, score.start_line, false);
        block.add_line(&score.file, score.end_line, false);

        // Include diff lines within this function
        for diff_info in &diff.files {
            if diff_info.file_path == score.file {
                for &line in &diff_info.diff_lines {
                    if line >= score.start_line && line <= score.end_line {
                        block.add_line(&score.file, line, true);
                    }
                }
            }
        }

        result.blocks.push(block);
    }

    Ok(result)
}

/// Query git for file churn data.
fn get_git_churn(git_dir: &str, days: usize) -> Result<BTreeMap<String, usize>> {
    let mut churn = BTreeMap::new();

    let output = Command::new("git")
        .args([
            "log",
            "--format=",
            "--name-only",
            &format!("--since={} days ago", days),
        ])
        .current_dir(git_dir)
        .output()
        .map_err(|e| anyhow!("git is not available: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!(
            "git log failed (is this a git repository?): {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    if let Ok(stdout) = String::from_utf8(output.stdout) {
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() {
                *churn.entry(line.to_string()).or_insert(0) += 1;
            }
        }
    }

    Ok(churn)
}
