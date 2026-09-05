//! Serializable DTOs for `docs/contracts/targets.schema.json` v1.0.
//!
//! Every type here is `#[non_exhaustive]` (§2.3.1): they are re-exported from
//! `prism::api`, they are produced only by `targets::project`, and consumers
//! read or deserialize them — so a v1.1 field must not break an embedder that
//! compiled against v1.0. `#[non_exhaustive]` constrains only OTHER crates;
//! `project` still builds them with struct literals inside this one.

use crate::finding_confidence::{FindingConfidence, FindingTier};
use crate::slice::AlgorithmError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct Producer {
    pub tool: String,
    pub version: String,
    /// Runtime emitter projection selected for this document.
    pub resolution_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_identity: Option<String>,
    pub algorithms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Repo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Diff {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub struct Expected {
    pub property: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DependencyHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterpart: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Related {
    pub lines: Vec<usize>,
    pub files: Vec<String>,
}
