use crate::access_path::AccessPath;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

/// A definition or use of a variable at a specific location.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VarLocation {
    pub file: String,
    pub function: String,
    pub function_start_line: usize,
    pub line: usize,
    /// Structured access path for this variable reference.
    pub path: AccessPath,
    pub start_byte: usize,
    pub end_byte: usize,
    /// Backward-compatible accessor: returns the base variable name.
    /// Algorithms that don't need field sensitivity can use this.
    pub kind: VarAccessKind,
}

impl VarLocation {
    /// Identity tuple — byte excluded. Ord/Eq/Hash ALL derive from this so they
    /// cannot disagree (would corrupt BTreeMap/HashMap keys). Pinned by §7.6.
    fn identity_key(&self) -> (&str, &str, usize, usize, &AccessPath, VarAccessKind) {
        (
            &self.file,
            &self.function,
            self.function_start_line,
            self.line,
            &self.path,
            self.kind,
        )
    }

    /// Backward-compatible accessor: returns the base variable name.
    pub fn var_name(&self) -> &str {
        &self.path.base
    }
}

impl PartialEq for VarLocation {
    fn eq(&self, other: &Self) -> bool {
        self.identity_key() == other.identity_key()
    }
}

impl Eq for VarLocation {}

impl PartialOrd for VarLocation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VarLocation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identity_key().cmp(&other.identity_key())
    }
}

impl Hash for VarLocation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity_key().hash(state);
    }
}

/// Whether a variable access is a definition or a use.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum VarAccessKind {
    /// Variable is being assigned to (written).
    Def,
    /// Variable is being read.
    Use,
}

/// Crate-internal occurrence identity for producer facts that must distinguish
/// multiple references sharing the legacy `(path, line)` identity.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub(crate) struct ExactVarLocation {
    pub(crate) file: String,
    pub(crate) function: String,
    pub(crate) function_start_line: usize,
    pub(crate) line: usize,
    pub(crate) path: AccessPath,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) kind: VarAccessKind,
}

impl ExactVarLocation {
    pub(crate) fn from_legacy(loc: &VarLocation) -> Self {
        Self {
            file: loc.file.clone(),
            function: loc.function.clone(),
            function_start_line: loc.function_start_line,
            line: loc.line,
            path: loc.path.clone(),
            start_byte: loc.start_byte,
            end_byte: loc.end_byte,
            kind: loc.kind,
        }
    }

    pub(crate) fn as_legacy(&self) -> VarLocation {
        VarLocation {
            file: self.file.clone(),
            function: self.function.clone(),
            function_start_line: self.function_start_line,
            line: self.line,
            path: self.path.clone(),
            start_byte: self.start_byte,
            end_byte: self.end_byte,
            kind: self.kind,
        }
    }
}

/// An edge in the data flow graph: a definition flows to a use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct FlowEdge {
    pub from: VarLocation,
    pub to: VarLocation,
}

/// Exact byte-bearing identity for one internal supplemental producer edge.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub(crate) struct ExactFlowEdge {
    pub(crate) from: ExactVarLocation,
    pub(crate) to: ExactVarLocation,
}

impl ExactFlowEdge {
    pub(crate) fn from_legacy(edge: &FlowEdge) -> Self {
        Self {
            from: ExactVarLocation::from_legacy(&edge.from),
            to: ExactVarLocation::from_legacy(&edge.to),
        }
    }

    pub(crate) fn as_legacy(&self) -> FlowEdge {
        FlowEdge {
            from: self.from.as_legacy(),
            to: self.to.as_legacy(),
        }
    }

    pub(crate) fn binding_key(&self) -> (String, String, usize, AccessPath, usize, usize) {
        (
            self.from.file.clone(),
            self.from.function.clone(),
            self.from.function_start_line,
            self.from.path.clone(),
            self.from.start_byte,
            self.from.end_byte,
        )
    }
}

/// A path through the data flow graph.
#[derive(Debug, Clone)]
pub struct FlowPath {
    pub edges: Vec<FlowEdge>,
    /// Sanitizer categories this flow has been cleansed for. A sink whose
    /// category is in this set causes the finding to be suppressed at
    /// evaluation time. Per spec §3.5 — path-sensitive: different flows
    /// reaching the same use-site may disagree on cleansing status.
    pub cleansed_for: BTreeSet<crate::frameworks::SanitizerCategory>,
}
