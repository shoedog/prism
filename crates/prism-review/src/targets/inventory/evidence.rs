//! Optional source evidence captured before DTO projection.

use super::{python::source::Source, UnsupportedReason};
use crate::targets::Target;
use serde::Serialize;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Root,
    R1,
    R2,
    R3,
    R4,
    R5,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Span {
    pub range: [usize; 4],
    pub text: String,
}

impl Span {
    pub(in crate::targets::inventory) fn new(source: &Source<'_>, node: Node<'_>) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Self {
            range: [start.row + 1, start.column, end.row + 1, end.column],
            text: source.original_text(node).to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Proof {
    pub route: Route,
    pub call: Span,
    pub callee: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<ReceiverProof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<ClassSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<Box<FieldProof>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldProof {
    pub class: String,
    pub owner: Span,
    pub initializer: Span,
    pub parameter: Span,
    pub annotation: Span,
    pub store: Span,
    pub receiver_use: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReceiverProof {
    pub construction: ClassSource,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicting_declarations: Vec<ClassSource>,
}

impl ReceiverProof {
    pub(super) fn constructed(class: &str, expression: Node<'_>) -> Self {
        Self {
            construction: ClassSource::new(class, expression),
            conflicting_declarations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClassSource {
    pub class: String,
    pub range: [usize; 4],
}

impl ClassSource {
    pub(super) fn new(class: &str, node: Node<'_>) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Self {
            class: class.to_owned(),
            range: [start.row + 1, start.column, end.row + 1, end.column],
        }
    }
}

pub struct Occurrence {
    pub file: String,
    pub symbol: Option<String>,
    pub canonical: String,
    pub reason: Option<UnsupportedReason>,
    pub proof: Proof,
}

#[derive(Clone, Debug, Serialize)]
pub struct Projected {
    pub revision: &'static str,
    pub build_identity: &'static str,
    pub file: String,
    pub id: String,
    pub occurrence: usize,
    pub proof: Proof,
}

/// Attach proof to the actual projected IDs, including every occurrence of a
/// grouped unsupported row. The route and spans come from the proof branch.
pub fn sidecar(rows: &[Target], occurrences: &[Occurrence]) -> Vec<Projected> {
    let build = prism::api::build_info();
    let mut result = Vec::new();
    for row in rows
        .iter()
        .filter(|row| row.source_algorithm == "prism.inventory/1")
    {
        for (index, occurrence) in occurrences.iter().enumerate() {
            let reason = row
                .expected
                .detail
                .as_deref()
                .and_then(|s| s.split(';').next());
            if occurrence.file == row.site.file
                && occurrence.proof.callee.range[0] == row.site.line
                && occurrence.symbol == row.site.symbol
                && row
                    .dependency_hint
                    .as_ref()
                    .and_then(|hint| hint.callee.as_ref())
                    == Some(&occurrence.canonical)
                && match occurrence.reason {
                    Some(expected) => row.kind == "other" && reason == Some(expected.as_str()),
                    None => row.kind == "external_call",
                }
            {
                result.push(Projected {
                    revision: build.git_sha,
                    build_identity: build.build_identity,
                    file: occurrence.file.clone(),
                    id: row.id.clone(),
                    occurrence: index,
                    proof: occurrence.proof.clone(),
                });
            }
        }
    }
    result
}
