//! Provenance Slice — trace each variable back to its ultimate origin.
//!
//! **Question answered:** "Where did this data originally come from, and does that origin require special handling?"
//!
//! For each variable on a diff line, traces backward through assignments to
//! classify its ultimate source: user input, config value, database result,
//! hardcoded constant, environment variable, or function parameter. Different
//! origins require different levels of scrutiny — a variable from request.body
//! needs validation, one from a constant doesn't.
//!
//! This is backward taint analysis with origin classification.

use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Classification of a data origin.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// User/client input (request body, query params, form data, stdin)
    UserInput,
    /// Configuration values (config files, settings)
    Config,
    /// Database query results
    Database,
    /// Hardcoded constant or literal
    Constant,
    /// Environment variable
    EnvVar,
    /// Function parameter (origin unknown without interprocedural analysis)
    FunctionParam,
    /// Return value from external/library function
    ExternalCall,
    /// Hardware / device I/O (ioctl, mmap, register reads) — C/C++ embedded
    Hardware,
    /// Could not determine origin
    Unknown,
}

impl Origin {
    pub fn risk_level(&self) -> &str {
        match self {
            Self::UserInput => "HIGH — requires validation/sanitization",
            Self::Hardware => "HIGH — raw device data, validate before use",
            Self::Database => "MEDIUM — may contain user-supplied data",
            Self::ExternalCall => "MEDIUM — verify return contract",
            Self::FunctionParam => "MEDIUM — depends on caller",
            Self::EnvVar => "LOW — trusted but may be misconfigured",
            Self::Config => "LOW — typically trusted",
            Self::Constant => "NONE — hardcoded value",
            Self::Unknown => "UNKNOWN — could not trace",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::UserInput => "user_input",
            Self::Config => "config",
            Self::Database => "database",
            Self::Constant => "constant",
            Self::EnvVar => "env_var",
            Self::FunctionParam => "function_param",
            Self::ExternalCall => "external_call",
            Self::Hardware => "hardware",
            Self::Unknown => "unknown",
        }
    }
}

/// Heuristic patterns for origin classification.
const USER_INPUT_PATTERNS: &[&str] = &[
    "request",
    "req.",
    "body",
    "params",
    "query",
    "form",
    "input",
    "stdin",
    "args",
    "argv",
    "getParameter",
    "ReadBody",
    "FormValue",
    "PostForm",
    "r.Body",
    "event.target",
    "prompt(",
    "readline",
    // C/C++ network and file input
    "recv(",
    "recvfrom(",
    "read(",
    "fgets(",
    "fread(",
    "scanf(",
    "fscanf(",
    "gets(",
    "getline(",
    "accept(",
];

const DATABASE_PATTERNS: &[&str] = &[
    "query",
    "execute",
    "fetch",
    "cursor",
    "find(",
    "findOne",
    "select",
    "SELECT",
    "db.",
    "DB.",
    "repository",
    "dao",
    "store.get",
];

const CONFIG_PATTERNS: &[&str] = &[
    "config",
    "Config",
    "settings",
    "Settings",
    "getConfig",
    "get_config",
    "cfg.",
    "conf.",
    "properties",
    "yaml",
    "toml",
    // C/C++ command-line option and config file parsing
    "getopt(",
    "fopen(",
];

const ENV_PATTERNS: &[&str] = &[
    "os.environ",
    "os.Getenv",
    "process.env",
    "System.getenv",
    "env.",
    "ENV[",
    "getenv(",
    "dotenv",
    // C/C++ command-line arguments
    "argv[",
    "argv ",
];

/// C/C++ hardware / device I/O patterns (embedded and kernel).
const HARDWARE_PATTERNS: &[&str] = &["ioctl(", "mmap(", "inb(", "outb(", "readl(", "writel("];

fn classify_line(line_text: &str) -> Origin {
    // Check for literal/constant assignment
    let trimmed = line_text.trim();
    if trimmed.contains("= \"")
        || trimmed.contains("= '")
        || trimmed.contains("= 0")
        || trimmed.contains("= 1")
        || trimmed.contains("= true")
        || trimmed.contains("= false")
        || trimmed.contains("= nil")
        || trimmed.contains("= null")
        || trimmed.contains("= None")
        || trimmed.contains("= []")
        || trimmed.contains("= {}")
        || trimmed.contains("= ()")
    {
        return Origin::Constant;
    }

    if USER_INPUT_PATTERNS.iter().any(|p| line_text.contains(p)) {
        return Origin::UserInput;
    }
    if DATABASE_PATTERNS.iter().any(|p| line_text.contains(p)) {
        return Origin::Database;
    }
    if HARDWARE_PATTERNS.iter().any(|p| line_text.contains(p)) {
        return Origin::Hardware;
    }
    if CONFIG_PATTERNS.iter().any(|p| line_text.contains(p)) {
        return Origin::Config;
    }
    if ENV_PATTERNS.iter().any(|p| line_text.contains(p)) {
        return Origin::EnvVar;
    }

    Origin::Unknown
}

/// A provenance finding for a variable.
#[derive(Debug, Clone)]
pub struct ProvenanceFinding {
    pub var_name: String,
    pub use_file: String,
    pub use_line: usize,
    pub origin: Origin,
    pub origin_file: String,
    pub origin_line: usize,
    pub path: Vec<(String, usize)>,
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::ProvenanceSlice);
    let dfg = DataFlowGraph::build(files);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        let source_lines: Vec<&str> = parsed.source.lines().collect();

        for &line in &diff_info.diff_lines {
            // Get all variables referenced on this line
            let identifiers = parsed.identifiers_on_line(line);
            let mut seen_vars: BTreeSet<String> = BTreeSet::new();

            for id_node in &identifiers {
                let var_name = parsed.node_text(id_node).to_string();
                if seen_vars.contains(&var_name) || var_name.len() <= 1 {
                    continue;
                }
                seen_vars.insert(var_name.clone());

                // Trace backward through data flow to find the origin
                let locs = dfg.all_defs_of(&diff_info.file_path, &var_name);
                let mut origin = Origin::Unknown;
                let mut origin_line = line;
                let mut origin_file = diff_info.file_path.clone();

                // Check each definition site
                for loc in &locs {
                    if loc.line > 0 && loc.line <= source_lines.len() {
                        let lt = source_lines[loc.line - 1];
                        let classified = classify_line(lt);
                        if classified != Origin::Unknown {
                            origin = classified;
                            origin_line = loc.line;
                            origin_file = loc.file.clone();
                            break;
                        }
                    }

                    // Also trace backward from this def
                    let reachable = dfg.backward_reachable(loc);
                    for r in &reachable {
                        if let Some(rparsed) = files.get(&r.file) {
                            let rlines: Vec<&str> = rparsed.source.lines().collect();
                            if r.line > 0 && r.line <= rlines.len() {
                                let lt = rlines[r.line - 1];
                                let classified = classify_line(lt);
                                if classified != Origin::Unknown {
                                    origin = classified;
                                    origin_line = r.line;
                                    origin_file = r.file.clone();
                                    break;
                                }
                            }
                        }
                    }

                    if origin != Origin::Unknown {
                        break;
                    }
                }

                // If still unknown, check if it's a function parameter
                if origin == Origin::Unknown {
                    if let Some(func_node) = parsed.enclosing_function(line) {
                        let func_text = parsed.node_text(&func_node);
                        if func_text.contains(&var_name) {
                            // Rough heuristic: if the var appears in the function signature
                            let (start, _) = parsed.node_line_range(&func_node);
                            if start > 0 && start <= source_lines.len() {
                                let sig = source_lines[start - 1];
                                if sig.contains(&var_name) {
                                    origin = Origin::FunctionParam;
                                    origin_line = start;
                                }
                            }
                        }
                    }
                }

                // Emit a finding for untrusted-origin variables
                let severity = match &origin {
                    Origin::UserInput | Origin::Hardware => Some("concern"),
                    Origin::Database | Origin::ExternalCall => Some("warning"),
                    Origin::FunctionParam | Origin::EnvVar | Origin::Config => Some("info"),
                    Origin::Constant | Origin::Unknown => None,
                };
                if let Some(sev) = severity {
                    result.findings.push(SliceFinding {
                        algorithm: "provenance".to_string(),
                        file: diff_info.file_path.clone(),
                        line,
                        severity: sev.to_string(),
                        description: format!(
                            "variable '{}' has {} origin: {}",
                            var_name,
                            origin.name(),
                            origin.risk_level()
                        ),
                        function_name: None,
                        related_lines: locs.iter().map(|l| l.line).collect(),
                        related_files: if origin_file != diff_info.file_path {
                            vec![origin_file.clone()]
                        } else {
                            vec![]
                        },
                        category: Some("untrusted_origin".to_string()),
                    });
                }

                // Build a block showing the provenance chain
                let mut block =
                    DiffBlock::new(block_id, diff_info.file_path.clone(), ModifyType::Modified);

                // The use site (diff line)
                block.add_line(&diff_info.file_path, line, true);

                // The origin site
                if origin_file == diff_info.file_path {
                    block.add_line(&origin_file, origin_line, false);
                } else {
                    block.add_line(&origin_file, origin_line, false);
                }

                // Include intermediate def sites
                for loc in &locs {
                    if loc.line != line && loc.line != origin_line {
                        block.add_line(&loc.file, loc.line, false);
                    }
                }

                result.blocks.push(block);
                block_id += 1;
            }
        }
    }

    Ok(result)
}
