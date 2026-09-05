//! Schema types for the Code Property Graph: node and edge enums plus their
//! accessors.

use super::flow_confidence::FlowConfidence;
use crate::access_path::AccessPath;
use crate::resolution::ResolutionConfidence;

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/// A node in the Code Property Graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CpgNode {
    /// A function definition.
    Function {
        name: String,
        file: String,
        start_line: usize,
        end_line: usize,
        start_byte: usize,
        end_byte: usize,
    },

    /// A statement or expression at a specific source location.
    Statement {
        file: String,
        line: usize,
        kind: StmtKind,
        start_byte: usize,
        end_byte: usize,
    },

    /// A variable access (definition or use) with a structured access path.
    Variable {
        path: AccessPath,
        file: String,
        function: String,
        function_start_line: usize,
        line: usize,
        access: VarAccess,
        start_byte: usize,
        end_byte: usize,
    },

    /// A non-seedable endpoint for a non-simple returned expression. Identity
    /// is the fenced function plus return-statement span and child slot.
    ReturnValue {
        file: String,
        function: String,
        function_start_line: usize,
        line: usize,
        return_start_byte: usize,
        return_end_byte: usize,
        child_slot: usize,
        start_byte: usize,
        end_byte: usize,
    },
}

impl PartialEq for CpgNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                CpgNode::Function {
                    name,
                    file,
                    start_line,
                    end_line,
                    ..
                },
                CpgNode::Function {
                    name: other_name,
                    file: other_file,
                    start_line: other_start_line,
                    end_line: other_end_line,
                    ..
                },
            ) => {
                name == other_name
                    && file == other_file
                    && start_line == other_start_line
                    && end_line == other_end_line
            }
            (
                CpgNode::Statement {
                    file, line, kind, ..
                },
                CpgNode::Statement {
                    file: other_file,
                    line: other_line,
                    kind: other_kind,
                    ..
                },
            ) => file == other_file && line == other_line && kind == other_kind,
            (
                CpgNode::Variable {
                    path,
                    file,
                    function,
                    function_start_line,
                    line,
                    access,
                    ..
                },
                CpgNode::Variable {
                    path: other_path,
                    file: other_file,
                    function: other_function,
                    function_start_line: other_function_start_line,
                    line: other_line,
                    access: other_access,
                    ..
                },
            ) => {
                path == other_path
                    && file == other_file
                    && function == other_function
                    && function_start_line == other_function_start_line
                    && line == other_line
                    && access == other_access
            }
            (
                CpgNode::ReturnValue {
                    file,
                    function,
                    function_start_line,
                    return_start_byte,
                    return_end_byte,
                    child_slot,
                    ..
                },
                CpgNode::ReturnValue {
                    file: other_file,
                    function: other_function,
                    function_start_line: other_function_start_line,
                    return_start_byte: other_return_start_byte,
                    return_end_byte: other_return_end_byte,
                    child_slot: other_child_slot,
                    ..
                },
            ) => {
                file == other_file
                    && function == other_function
                    && function_start_line == other_function_start_line
                    && return_start_byte == other_return_start_byte
                    && return_end_byte == other_return_end_byte
                    && child_slot == other_child_slot
            }
            _ => false,
        }
    }
}

impl Eq for CpgNode {}

/// Classification of statements relevant for analysis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StmtKind {
    /// Variable assignment: `x = expr`
    Assignment,
    /// Function/method call.
    Call { callee: String },
    /// Return statement.
    Return,
    /// Conditional branch: if, switch, match.
    Branch,
    /// Loop: for, while, loop, do-while.
    Loop,
    /// Goto statement (C/C++).
    Goto { target: String },
    /// Label (C/C++ goto target).
    Label { name: String },
    /// Variable/type declaration.
    Declaration,
    /// Any other statement.
    Other,
}

/// Whether a variable access is a definition (write) or use (read).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum VarAccess {
    /// Variable is written to (assigned, declared with initializer).
    Def,
    /// Variable is read.
    Use,
}

// ---------------------------------------------------------------------------
// Edge types
// ---------------------------------------------------------------------------

/// An edge in the Code Property Graph.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CpgEdge {
    /// Data flow: a definition reaches this use (def-use chain), carrying
    /// the reaching-definitions pass's confidence that the definition
    /// actually reaches the use. `is_data_flow` and every edge-set
    /// traversal predicate ignore this payload (§3 non-goal 1): the edge
    /// SET is unchanged from before this label existed.
    DataFlow(FlowConfidence),

    /// Control flow: execution can proceed from source to target.
    /// Added in Phase 6.
    ControlFlow,

    /// Call: a call site invokes a callee function, tagged with resolution confidence.
    Call(ResolutionConfidence),

    /// Return: a function returns to the call site, with the same confidence as its Call.
    Return(ResolutionConfidence),

    /// A modeled callee return endpoint flows to the exact caller assignment
    /// target. The certificate bit is identical on every edge for an assignment.
    ReturnFlow { suppress_shortcut: bool },

    /// A semantic value/receiver Use flows into a non-simple ReturnValue node.
    ReturnInput,

    /// Containment: a function contains this statement or variable.
    Contains,

    /// Field relationship: a variable is a field access on another variable.
    FieldOf,
}

// ---------------------------------------------------------------------------
// Node accessors
// ---------------------------------------------------------------------------

impl CpgNode {
    /// Construct a variable occurrence with additive byte metadata.
    pub fn variable_occurrence(
        path: AccessPath,
        file: String,
        function: String,
        function_start_line: usize,
        line: usize,
        access: VarAccess,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        CpgNode::Variable {
            path,
            file,
            function,
            function_start_line,
            line,
            access,
            start_byte,
            end_byte,
        }
    }

    /// The file path this node belongs to.
    pub fn file(&self) -> &str {
        match self {
            CpgNode::Function { file, .. } => file,
            CpgNode::Statement { file, .. } => file,
            CpgNode::Variable { file, .. } => file,
            CpgNode::ReturnValue { file, .. } => file,
        }
    }

    /// The primary line number of this node.
    pub fn line(&self) -> usize {
        match self {
            CpgNode::Function { start_line, .. } => *start_line,
            CpgNode::Statement { line, .. } => *line,
            CpgNode::Variable { line, .. } => *line,
            CpgNode::ReturnValue { line, .. } => *line,
        }
    }

    /// Whether this node is a function definition.
    pub fn is_function(&self) -> bool {
        matches!(self, CpgNode::Function { .. })
    }

    /// Whether this node is a variable definition.
    pub fn is_def(&self) -> bool {
        matches!(
            self,
            CpgNode::Variable {
                access: VarAccess::Def,
                ..
            }
        )
    }

    /// Whether this node is a variable use.
    pub fn is_use(&self) -> bool {
        matches!(
            self,
            CpgNode::Variable {
                access: VarAccess::Use,
                ..
            }
        )
    }

    /// Whether this node is a call statement.
    pub fn is_call(&self) -> bool {
        matches!(
            self,
            CpgNode::Statement {
                kind: StmtKind::Call { .. },
                ..
            }
        )
    }
}

impl CpgEdge {
    /// Whether this is a data flow edge. Label-insensitive by design (§3
    /// non-goal 1): a `DataFlow` edge is selected regardless of its
    /// `FlowConfidence` payload. Pinned by
    /// `tests/integration/dfg_label_parity_test.rs`.
    pub fn is_data_flow(&self) -> bool {
        matches!(self, CpgEdge::DataFlow(_))
    }

    /// Whether this is a call or return edge.
    pub fn is_interprocedural(&self) -> bool {
        matches!(
            self,
            CpgEdge::Call(_) | CpgEdge::Return(_) | CpgEdge::ReturnFlow { .. }
        )
    }
}
