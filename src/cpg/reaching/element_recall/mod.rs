//! Opt-in element-recall diagnostics (lane 10, #847). I0 syntax inventory:
//! indexed-store sources (W) and whole-operand consumers (R) of one Python owner.
mod join;
#[cfg(test)]
mod join_tests;
mod limits;
mod loops;
mod preview;
#[cfg(test)]
mod preview_tests;
mod python;
mod rebind;
mod route;
#[cfg(test)]
mod route_tests;
#[cfg(test)]
mod syntax_tests;
mod trace;
mod transfer;
#[cfg(test)]
mod transfer_tests;

pub(crate) use python::scan_file;
pub(crate) use trace::{FileTrace, Final};

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
    /// or a static selector prefix of its container, lazily and in write-then-operand order.
    fn admitted(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let writes = self
            .writes
            .iter()
            .enumerate()
            .filter(|(_, w)| w.verdict.is_ok());
        writes.flat_map(move |(w, write)| {
            let reads = self.operands.iter().enumerate();
            let fed = move |(_, o): &(usize, &Operand)| {
                let (a, b) = (&write.path, &o.path);
                o.verdict.is_ok() && a.root == b.root && a.fields.starts_with(&b.fields)
            };
            reads.filter(fed).map(move |(r, _)| (w, r))
        })
    }

    /// Every admitted pair, materialized. Callers that may face a huge owner count first
    /// ([`OwnerScan::pair_count`]) and take at most [`OwnerScan::pair_head`].
    pub(crate) fn pairs(&self) -> Vec<(usize, usize)> {
        self.pair_head(usize::MAX)
    }

    /// The first `n` admitted pairs, allocating only those.
    pub(crate) fn pair_head(&self, n: usize) -> Vec<(usize, usize)> {
        let out: Vec<_> = self.admitted().take(n).collect();
        #[cfg(test)]
        {
            let mut seen = PAIRS_BUILT.lock().unwrap();
            let slot = seen.entry(self.name.clone()).or_default();
            *slot = (*slot).max(out.len());
        }
        out
    }

    /// The number of admitted pairs, from counts of the operands of each selector path, without
    /// building a pair. Saturates at `u64::MAX`.
    pub(crate) fn pair_count(&self) -> u64 {
        let mut groups: std::collections::BTreeMap<(&Option<String>, &Vec<String>), u64> =
            std::collections::BTreeMap::new();
        for o in self.operands.iter().filter(|o| o.verdict.is_ok()) {
            *groups.entry((&o.path.root, &o.path.fields)).or_default() += 1;
        }
        let mut total = 0u64;
        for w in self.writes.iter().filter(|w| w.verdict.is_ok()) {
            for ((root, fields), n) in &groups {
                if **root == w.path.root && w.path.fields.starts_with(fields) {
                    total = total.saturating_add(*n);
                }
            }
        }
        total
    }
}

/// Tests read the largest pair list any owner of that name ever materialized.
#[cfg(test)]
pub(super) static PAIRS_BUILT: std::sync::Mutex<std::collections::BTreeMap<String, usize>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());
