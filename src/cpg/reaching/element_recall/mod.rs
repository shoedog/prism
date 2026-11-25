//! Opt-in element-recall diagnostics (lane 10, #847). I0 syntax inventory:
//! indexed-store sources (W) and whole-operand consumers (R) of one Python owner.
mod python;
#[cfg(test)]
mod syntax_tests;

pub(crate) use python::scan_file;

/// Half-open UTF-8 byte span of a node plus its 1-indexed start line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteKind {
    Assign,
    Augmented,
}

/// Why a store is not a value-carrying single-target indexed W.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteRefusal {
    Slice,
    MultiIndex,
    Unpack,
    BindingTarget,
    Delete,
    NoValue,
    NonNameRoot,
    DotAfterBracket,
    Redirected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Argument,
    Keyword,
    Return,
    AssignmentRhs,
}

/// Why an occurrence of a store's root is not a whole operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandRefusal {
    Projected,
    Slice,
    Key,
    Receiver,
    Callee,
    ExcludedCallee,
    Wrapped,
    Fenced,
}

/// Static path of a store or operand: root name, selector fields, bracket count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StaticPath {
    pub root: Option<String>,
    pub fields: Vec<String>,
    pub brackets: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Write {
    pub span: Span,
    pub path: StaticPath,
    pub verdict: Result<WriteKind, WriteRefusal>,
}

/// `span` is the operand with transparent parentheses stripped, `outer` keeps them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Operand {
    pub span: Span,
    pub outer: Span,
    pub path: StaticPath,
    pub verdict: Result<Role, OperandRefusal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OwnerScan {
    pub name: String,
    pub span: Span,
    pub writes: Vec<Write>,
    pub operands: Vec<Operand>,
}

impl OwnerScan {
    /// Admitted `(write, operand)` index pairs where the operand is the store's root
    /// or a static selector prefix of its container.
    pub(crate) fn pairs(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (w, write) in self.writes.iter().enumerate() {
            for (r, operand) in self.operands.iter().enumerate() {
                let (a, b) = (&write.path, &operand.path);
                let prefix = a.fields.starts_with(&b.fields);
                if write.verdict.is_ok() && operand.verdict.is_ok() && a.root == b.root && prefix {
                    out.push((w, r));
                }
            }
        }
        out
    }
}
