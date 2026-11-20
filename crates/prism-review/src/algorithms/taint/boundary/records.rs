use prism::{api::ReviewImportHint, call_graph::FunctionId, languages::Language};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PhysicalSite {
    pub caller: FunctionId,
    pub start_byte: usize,
    pub end_byte: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Availability {
    #[default]
    Available,
    ListingUnavailable(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryKind {
    InRepo,
    GoDeclarationUnknown,
    Unresolved,
    LoadedUnresolved,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundaryRecord {
    pub site: PhysicalSite,
    pub line: usize,
    pub callee: String,
    pub language: Language,
    pub hint: ReviewImportHint,
    pub kind: BoundaryKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundaryCollection {
    pub availability: Availability,
    pub records: Vec<BoundaryRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextRunNoteKind {
    Ignored,
    Unauthorized,
    Unavailable,
    DepthBound,
    FilesBound,
    BytesBound,
    TimeGuard,
    Cancelled,
    StageFailed,
    UnionRefused,
    FileSkipped,
    SkippedSummary,
    ContextParse,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContextRunNote {
    pub(crate) kind: ContextRunNoteKind,
    pub(crate) message: String,
    pub(crate) location: Option<(String, usize)>,
}
impl ContextRunNote {
    pub fn kind(&self) -> ContextRunNoteKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn location(&self) -> Option<(&str, usize)> {
        self.location.as_ref().map(|(p, n)| (p.as_str(), *n))
    }
}
