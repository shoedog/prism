//! Code Property Graph schema types.
//!
//! Defines the node and edge types for a unified graph that merges AST
//! structure, data flow, call graph, and (future) control flow into a single
//! queryable representation built on `petgraph`.
//!
//! This module defines the **schema only** — the type definitions and basic
//! operations. The graph builder (which populates a CPG from parsed files)
//! will be added in Phase 4 of the CPG architecture.
//!
//! See `docs/cpg-architecture.md` for the full design.

use crate::access_path::AccessPath;

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/// A node in the Code Property Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpgNode {
    /// A function definition.
    Function {
        name: String,
        file: String,
        start_line: usize,
        end_line: usize,
    },

    /// A statement or expression at a specific source location.
    Statement {
        file: String,
        line: usize,
        kind: StmtKind,
    },

    /// A variable access (definition or use) with a structured access path.
    Variable {
        path: AccessPath,
        file: String,
        function: String,
        line: usize,
        access: VarAccess,
    },
}

/// Classification of statements relevant for analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpgEdge {
    /// Data flow: a definition reaches this use (def-use chain).
    DataFlow,

    /// Control flow: execution can proceed from source to target.
    /// Added in Phase 6.
    ControlFlow,

    /// Call: a call site invokes a callee function.
    Call,

    /// Return: a function returns to the call site.
    Return,

    /// Containment: a function contains this statement or variable.
    Contains,

    /// Field relationship: a variable is a field access on another variable.
    FieldOf,
}

// ---------------------------------------------------------------------------
// Node accessors
// ---------------------------------------------------------------------------

impl CpgNode {
    /// The file path this node belongs to.
    pub fn file(&self) -> &str {
        match self {
            CpgNode::Function { file, .. } => file,
            CpgNode::Statement { file, .. } => file,
            CpgNode::Variable { file, .. } => file,
        }
    }

    /// The primary line number of this node.
    pub fn line(&self) -> usize {
        match self {
            CpgNode::Function { start_line, .. } => *start_line,
            CpgNode::Statement { line, .. } => *line,
            CpgNode::Variable { line, .. } => *line,
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
    /// Whether this is a data flow edge.
    pub fn is_data_flow(&self) -> bool {
        matches!(self, CpgEdge::DataFlow)
    }

    /// Whether this is a call or return edge.
    pub fn is_interprocedural(&self) -> bool {
        matches!(self, CpgEdge::Call | CpgEdge::Return)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_accessors() {
        let func = CpgNode::Function {
            name: "main".into(),
            file: "src/main.c".into(),
            start_line: 1,
            end_line: 10,
        };
        assert_eq!(func.file(), "src/main.c");
        assert_eq!(func.line(), 1);
        assert!(func.is_function());

        let var_def = CpgNode::Variable {
            path: AccessPath::from_expr("dev->name"),
            file: "src/dev.c".into(),
            function: "init".into(),
            line: 5,
            access: VarAccess::Def,
        };
        assert!(var_def.is_def());
        assert!(!var_def.is_use());

        let call = CpgNode::Statement {
            file: "src/main.c".into(),
            line: 3,
            kind: StmtKind::Call {
                callee: "init".into(),
            },
        };
        assert!(call.is_call());
    }

    #[test]
    fn test_edge_classification() {
        assert!(CpgEdge::DataFlow.is_data_flow());
        assert!(!CpgEdge::Call.is_data_flow());
        assert!(CpgEdge::Call.is_interprocedural());
        assert!(CpgEdge::Return.is_interprocedural());
        assert!(!CpgEdge::DataFlow.is_interprocedural());
        assert!(!CpgEdge::Contains.is_interprocedural());
        assert!(!CpgEdge::FieldOf.is_interprocedural());
        assert!(!CpgEdge::ControlFlow.is_data_flow());
    }

    #[test]
    fn test_variable_node_accessors() {
        let var_use = CpgNode::Variable {
            path: AccessPath::from_expr("dev->id"),
            file: "src/dev.c".into(),
            function: "get_id".into(),
            line: 8,
            access: VarAccess::Use,
        };
        assert!(var_use.is_use());
        assert!(!var_use.is_def());
        assert!(!var_use.is_function());
        assert!(!var_use.is_call());
        assert_eq!(var_use.file(), "src/dev.c");
        assert_eq!(var_use.line(), 8);
    }

    #[test]
    fn test_statement_node_non_call() {
        let branch = CpgNode::Statement {
            file: "src/main.c".into(),
            line: 15,
            kind: StmtKind::Branch,
        };
        assert!(!branch.is_call());
        assert!(!branch.is_function());
        assert!(!branch.is_def());
        assert_eq!(branch.file(), "src/main.c");
        assert_eq!(branch.line(), 15);

        let ret = CpgNode::Statement {
            file: "src/main.c".into(),
            line: 20,
            kind: StmtKind::Return,
        };
        assert!(!ret.is_call());
    }
}
