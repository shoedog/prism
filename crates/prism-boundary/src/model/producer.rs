//! Producer source records added by P2: annotation observations and the edge grade.
//! Every field is optional on the wire, so P1d documents decode and serialize unchanged.

use super::SourceSite;
use serde::{Deserialize, Serialize};

/// How an annotation names its operation. `Bound` is the only unique grade: one handler
/// declaration names the operation and every possible competitor was accounted for (no tie, no
/// expected file left unread, no gap that is keyless or names the operation). A `Candidate` keeps
/// the observed site and carries the condition in the row's `evidence`
/// (`competing_handlers`, `competitor_unread`).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EdgeGrade {
    Bound,
    Candidate,
}

/// The function or method declaration an annotation is attached to.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HandlerSite {
    pub package: Option<String>,
    pub name: String,
    /// The receiver type exactly as written, `*T[A]` included.
    pub receiver: Option<String>,
    /// The receiver type name without pointer or type arguments.
    pub receiver_type: Option<String>,
    /// Tree, file, blob, declaration start line and the half-open byte span of the declaration.
    pub site: SourceSite,
}

/// What became of one annotation directive.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationDisposition {
    Bound,
    Candidate,
    AnnotationWithoutOperation,
    Unattached,
    Malformed,
    UnsupportedForm,
    NotChecked,
}

/// One recognised annotation directive, whether or not it bound.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AnnotationRecord {
    /// The comment line: physical one-based line and half-open byte span, trailing `\r` excluded.
    pub site: SourceSite,
    pub raw: String,
    pub raw_method: Option<String>,
    pub raw_path: Option<String>,
    pub disposition: AnnotationDisposition,
    pub reason: Option<String>,
    pub handler: Option<HandlerSite>,
    /// Operation IDs the directive selected, empty unless bound or candidate.
    #[serde(default)]
    pub operation_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}
