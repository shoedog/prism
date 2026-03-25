//! Symmetry Slice — detect broken symmetries in paired operations.
//!
//! **Question answered:** "If I changed one side of a symmetric pair, is the other side still consistent?"
//!
//! Many code patterns are symmetric: serialize/deserialize, encode/decode,
//! push/pop, marshal/unmarshal, toJSON/fromJSON, save/load, encrypt/decrypt.
//! If a change modifies one side without touching the other, the symmetry may
//! be broken. This slice surfaces both sides for comparison.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// A symmetric function pair pattern.
#[derive(Debug, Clone)]
struct SymmetricPair {
    left: &'static [&'static str],
    right: &'static [&'static str],
}

/// Built-in symmetric pairs.
fn default_symmetric_pairs() -> Vec<SymmetricPair> {
    vec![
        SymmetricPair {
            left: &[
                "serialize",
                "marshal",
                "encode",
                "encrypt",
                "compress",
                "pack",
                "stringify",
            ],
            right: &[
                "deserialize",
                "unmarshal",
                "decode",
                "decrypt",
                "decompress",
                "unpack",
                "parse",
            ],
        },
        SymmetricPair {
            left: &["save", "store", "write", "put", "set", "insert", "create"],
            right: &[
                "load", "fetch", "read", "get", "delete", "remove", "destroy",
            ],
        },
        SymmetricPair {
            left: &[
                "to_json",
                "toJSON",
                "to_dict",
                "toMap",
                "to_string",
                "ToString",
            ],
            right: &[
                "from_json",
                "fromJSON",
                "from_dict",
                "fromMap",
                "from_string",
                "FromString",
            ],
        },
        SymmetricPair {
            left: &[
                "open", "start", "begin", "connect", "init", "setup", "mount",
            ],
            right: &[
                "close",
                "stop",
                "end",
                "disconnect",
                "cleanup",
                "teardown",
                "unmount",
            ],
        },
        SymmetricPair {
            left: &["subscribe", "register", "bind", "attach", "listen", "on"],
            right: &[
                "unsubscribe",
                "unregister",
                "unbind",
                "detach",
                "unlisten",
                "off",
            ],
        },
        SymmetricPair {
            left: &["request", "send", "upload", "push", "produce", "emit"],
            right: &[
                "response", "receive", "download", "pull", "consume", "handle",
            ],
        },
        SymmetricPair {
            left: &["lock", "acquire", "enter"],
            right: &["unlock", "release", "exit"],
        },
        SymmetricPair {
            left: &["increment", "increase", "add", "deposit"],
            right: &["decrement", "decrease", "subtract", "withdraw"],
        },
    ]
}

/// Find the counterpart name for a given function name.
fn find_counterpart(func_name: &str, pairs: &[SymmetricPair]) -> Option<Vec<String>> {
    let name_lower = func_name.to_lowercase();

    for pair in pairs {
        // Check if it matches a left pattern
        for &left_pat in pair.left {
            if name_lower.contains(&left_pat.to_lowercase()) {
                // Generate possible counterpart names
                let counterparts: Vec<String> = pair
                    .right
                    .iter()
                    .map(|r| {
                        // Try to preserve the original casing/naming convention
                        let replaced = func_name
                            .to_lowercase()
                            .replace(&left_pat.to_lowercase(), &r.to_lowercase());
                        // Also try direct substitution
                        replaced
                    })
                    .collect();
                return Some(counterparts);
            }
        }

        // Check if it matches a right pattern
        for &right_pat in pair.right {
            if name_lower.contains(&right_pat.to_lowercase()) {
                let counterparts: Vec<String> = pair
                    .left
                    .iter()
                    .map(|l| {
                        func_name
                            .to_lowercase()
                            .replace(&right_pat.to_lowercase(), &l.to_lowercase())
                    })
                    .collect();
                return Some(counterparts);
            }
        }
    }

    None
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::SymmetrySlice);
    let pairs = default_symmetric_pairs();
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find functions that contain diff lines
        let mut changed_functions: BTreeSet<String> = BTreeSet::new();
        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    changed_functions.insert(parsed.node_text(&name_node).to_string());
                }
            }
        }

        // For each changed function, look for its symmetric counterpart
        for func_name in &changed_functions {
            let counterpart_candidates = match find_counterpart(func_name, &pairs) {
                Some(c) => c,
                None => continue,
            };

            // Search all files for the counterpart
            let mut _counterpart_found = false;
            let mut _counterpart_changed = false;

            for (file_path, other_parsed) in files {
                for candidate in &counterpart_candidates {
                    // Search by lowercased name comparison
                    for func_node in other_parsed.all_functions() {
                        if let Some(name_node) = other_parsed.language.function_name(&func_node) {
                            let other_name = other_parsed.node_text(&name_node);
                            if other_name.to_lowercase() == *candidate {
                                _counterpart_found = true;
                                let (start, end) = other_parsed.node_line_range(&func_node);

                                // Check if the counterpart is also in the diff
                                let is_also_changed = diff.files.iter().any(|d| {
                                    d.file_path == *file_path
                                        && d.diff_lines.iter().any(|&l| l >= start && l <= end)
                                });

                                if is_also_changed {
                                    _counterpart_changed = true;
                                }

                                // Build a block showing both sides
                                let mut block = DiffBlock::new(
                                    block_id,
                                    diff_info.file_path.clone(),
                                    ModifyType::Modified,
                                );

                                // Include the changed function
                                if let Some(changed_node) = parsed.find_function_by_name(func_name)
                                {
                                    let (cs, ce) = parsed.node_line_range(&changed_node);
                                    for line in cs..=ce {
                                        let is_diff = diff_info.diff_lines.contains(&line);
                                        block.add_line(&diff_info.file_path, line, is_diff);
                                    }
                                }

                                // Include the counterpart function
                                for line in start..=end {
                                    let is_diff = diff.files.iter().any(|d| {
                                        d.file_path == *file_path && d.diff_lines.contains(&line)
                                    });
                                    block.add_line(file_path, line, is_diff);
                                }

                                if !is_also_changed {
                                    // Counterpart wasn't changed — potential broken symmetry
                                    result.blocks.push(block);
                                    block_id += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}
