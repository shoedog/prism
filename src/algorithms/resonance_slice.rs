//! Resonance Slice — change coupling from repository mining.
//!
//! **Question answered:** "What files usually change together with these, and are any missing from this diff?"
//!
//! When file A changes, what other files historically always change with it?
//! Uses git log co-change frequency to detect implicit coupling. If a file that
//! normally co-changes isn't in the current diff, that's a signal something may
//! be missing.
//!
//! Different from 3D slice which scores risk — this detects *absent* changes.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Configuration for resonance slicing.
#[derive(Debug, Clone)]
pub struct ResonanceConfig {
    /// Path to the git repository.
    pub git_dir: String,
    /// How many days back to look.
    pub days: usize,
    /// Minimum co-change count to be considered coupled.
    pub min_co_changes: usize,
    /// Minimum co-change ratio (0.0-1.0): of all commits touching file A,
    /// what fraction also touched file B?
    pub min_ratio: f64,
}

impl Default for ResonanceConfig {
    fn default() -> Self {
        Self {
            git_dir: ".".to_string(),
            days: 180,
            min_co_changes: 3,
            min_ratio: 0.3,
        }
    }
}

/// A co-change finding: a file that usually changes alongside the diff but isn't in it.
#[derive(Debug, Clone)]
pub struct ResonanceFinding {
    pub changed_file: String,
    pub missing_file: String,
    pub co_change_count: usize,
    pub total_changes: usize,
    pub ratio: f64,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &ResonanceConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::ResonanceSlice);

    let changed_files: BTreeSet<String> = diff.files.iter().map(|f| f.file_path.clone()).collect();

    // Get co-change data from git
    let co_changes = get_co_change_data(&config.git_dir, config.days);

    // For each changed file, find files that usually co-change
    let mut findings: Vec<ResonanceFinding> = Vec::new();

    for changed_file in &changed_files {
        if let Some(partners) = co_changes.get(changed_file) {
            for (partner_file, &count) in partners {
                if changed_files.contains(partner_file) {
                    continue; // Already in the diff, not missing
                }

                // Get total changes for the changed file
                let total = partners.values().sum::<usize>().max(count);
                let ratio = count as f64 / total.max(1) as f64;

                if count >= config.min_co_changes && ratio >= config.min_ratio {
                    findings.push(ResonanceFinding {
                        changed_file: changed_file.clone(),
                        missing_file: partner_file.clone(),
                        co_change_count: count,
                        total_changes: total,
                        ratio,
                    });
                }
            }
        }
    }

    // Sort by co-change count descending (strongest coupling first)
    findings.sort_by(|a, b| b.co_change_count.cmp(&a.co_change_count));

    // Build output blocks — one per missing file
    for (block_id, finding) in findings.iter().enumerate() {
        let mut block =
            DiffBlock::new(block_id, finding.missing_file.clone(), ModifyType::Modified);

        // If we have the missing file parsed, include its function signatures
        if let Some(parsed) = files.get(&finding.missing_file) {
            for func_node in parsed.all_functions() {
                let (start, end) = parsed.node_line_range(&func_node);
                block.add_line(&finding.missing_file, start, false);
                block.add_line(&finding.missing_file, end, false);
            }
        }

        // Include the changed file's diff lines for reference
        if let Some(diff_info) = diff
            .files
            .iter()
            .find(|f| f.file_path == finding.changed_file)
        {
            for &line in &diff_info.diff_lines {
                block.add_line(&finding.changed_file, line, true);
            }
        }

        result.blocks.push(block);
    }

    Ok(result)
}

/// Query git for co-change data: which files change together.
fn get_co_change_data(git_dir: &str, days: usize) -> BTreeMap<String, BTreeMap<String, usize>> {
    let mut co_changes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    // Get commit hashes in the time window
    let output = Command::new("git")
        .args(["log", "--format=%H", &format!("--since={} days ago", days)])
        .current_dir(git_dir)
        .output();

    let commits: Vec<String> = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => return co_changes,
    };

    // For each commit, get the list of changed files
    for commit in &commits {
        let output = Command::new("git")
            .args(["diff-tree", "--no-commit-id", "--name-only", "-r", commit])
            .current_dir(git_dir)
            .output();

        let files_in_commit: Vec<String> = match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Err(_) => continue,
        };

        // Record co-changes: for each pair of files in this commit
        for i in 0..files_in_commit.len() {
            for j in (i + 1)..files_in_commit.len() {
                let a = &files_in_commit[i];
                let b = &files_in_commit[j];
                *co_changes
                    .entry(a.clone())
                    .or_default()
                    .entry(b.clone())
                    .or_insert(0) += 1;
                *co_changes
                    .entry(b.clone())
                    .or_default()
                    .entry(a.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    co_changes
}
