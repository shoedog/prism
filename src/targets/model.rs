//! Serializable DTOs for `docs/contracts/targets.schema.json` v1.0.

use crate::finding_confidence::{FindingConfidence, FindingTier};
use crate::slice::AlgorithmError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetsDocument {
    pub schema_version: String,
    pub producer: Producer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<Repo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    pub targets: Vec<Target>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<AlgorithmError>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Producer {
    pub tool: String,
    pub version: String,
    pub resolution_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_identity: Option<String>,
    pub algorithms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub id: String,
    pub site: Site,
    pub kind: String,
    pub category: String,
    pub expected: Expected,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_hint: Option<DependencyHint>,
    pub source_algorithm: String,
    pub confidence: FindingConfidence,
    pub tier: FindingTier,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Related>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_quality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub file: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expected {
    pub property: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterpart: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Related {
    pub lines: Vec<usize>,
    pub files: Vec<String>,
}
