//! Syntax facts only. Labels and anchors never replace FunctionId identity.
use crate::call_graph::FunctionId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnonKind {
    Arrow,
    Function,
    Generator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BoundAs {
    JsxProp { element: String, prop: String },
    ObjectKey { key: String },
    ClassField { class: String, field: String },
    DefaultExport,
    VariableParenthesized { name: String },
    DefaultParam { function: FunctionId, param: String },
    Returned,
    Iife,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PassedTo {
    pub callee_text: String,
    pub arg_index: usize,
    pub call_line: usize,
    pub call_span: (usize, usize),
    /// Syntax extraction has no resolution authority. Always None here.
    pub callee: Option<FunctionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AnchorRoot {
    Named { function: FunctionId },
    Scope { file: String, scope: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AnonAnchor {
    pub version: u8,
    pub root: AnchorRoot,
    pub ordinals: Vec<usize>,
    pub callee_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AnonMeta {
    pub anonymous: bool,
    pub col: usize,
    pub kind: AnonKind,
    pub r#async: bool,
    pub enclosing: Option<FunctionId>,
    pub enclosing_scope: Option<String>,
    pub passed_to: Option<PassedTo>,
    pub bound_as: Option<BoundAs>,
    pub label: String,
    pub anchor: AnonAnchor,
}

mod label;
mod metadata;
#[cfg_attr(not(test), allow(unused_imports))] // S3a1b supplies the production consumer.
pub(crate) use metadata::extract;

#[cfg(test)]
mod tests;
