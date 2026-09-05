//! The serialized SARIF 2.1 document model — the typed structs `sarif.rs`
//! fills in and hands to `serde_json::to_value` (design §2.2.1).
//!
//! The structs fix the key SET of each object and, through
//! `skip_serializing_if`, which keys are omitted; they do NOT fix key ORDER
//! (`serde_json::Map` is a `BTreeMap` here, so emitted keys are alphabetical).
//! Determinism therefore comes from the array ordering in `sarif.rs`, not from
//! this file. Split out of `sarif.rs` to keep both files under the repo's
//! 600-line limit (CLAUDE.md §7); the module is private to `output`, so none
//! of these types appears in a public signature.

use crate::finding_confidence::{FindingConfidence, FindingTier};
use crate::slice::AlgorithmError;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct Text {
    pub text: String,
}

#[derive(Serialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: &'static str,
    pub version: &'static str,
    pub runs: Vec<Run>,
}

#[derive(Serialize)]
pub struct Run {
    pub tool: Tool,
    pub invocations: Vec<Invocation>,
    pub results: Vec<SarifResult>,
    pub properties: RunProperties,
}

#[derive(Serialize)]
pub struct Tool {
    pub driver: Driver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    pub name: &'static str,
    pub version: &'static str,
    pub semantic_version: &'static str,
    pub information_uri: &'static str,
    pub rules: Vec<Rule>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub short_description: Text,
    pub full_description: Text,
    pub properties: RuleProperties,
}

#[derive(Serialize)]
pub struct RuleProperties {
    pub algorithm: String,
    pub category: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    pub execution_successful: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_execution_notifications: Vec<Notification>,
}

#[derive(Serialize)]
pub struct Notification {
    pub level: &'static str,
    pub message: Text,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    pub rule_index: usize,
    pub level: &'static str,
    pub message: Text,
    pub locations: Vec<Location>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_locations: Vec<RelatedLocation>,
    pub partial_fingerprints: BTreeMap<&'static str, String>,
    pub properties: ResultProperties,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub physical_location: PhysicalLocation,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub logical_locations: Vec<LogicalLocation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    /// Omitted when the finding's line is 0: SARIF requires `startLine >= 1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Region>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLocation {
    pub uri: String,
    pub uri_base_id: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub start_line: usize,
}

#[derive(Serialize)]
pub struct LogicalLocation {
    pub name: String,
    pub kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedLocation {
    pub id: usize,
    pub physical_location: PhysicalLocation,
}

#[derive(Serialize)]
pub struct ResultProperties {
    pub algorithm: String,
    pub category: String,
    /// The ORIGINAL severity string, including values outside the four-value
    /// vocabulary that `level` conservatively maps to `error`.
    pub severity: String,
    pub confidence: FindingConfidence,
    pub tier: FindingTier,
    /// Runtime emitter projection selected by `--resolution`.
    pub resolution_mode: &'static str,
    pub parse_quality: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_files: Vec<String>,
    /// Related lines that could not be attributed to a file (§2.2.2). Data
    /// preserved rather than pinned to a location that may be wrong.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_lines: Vec<usize>,
}

#[derive(Serialize)]
pub struct RunProperties {
    pub mapping_version: &'static str,
    pub algorithms_run: Vec<String>,
    /// Runtime emitter projection shared by every result in this run.
    pub resolution_mode: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<AlgorithmError>,
    pub prism_build_identity: &'static str,
    pub prism_git_sha: &'static str,
    pub binary_input_dirty: bool,
}
