use prism::{api::ReviewImportHint, call_graph::FunctionId, languages::Language};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PhysicalSite {
    pub caller: FunctionId,
    pub start_byte: usize,
    pub end_byte: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Availability {
    Available,
    ListingUnavailable(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BoundaryKind {
    InRepo,
    GoDeclarationUnknown,
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BoundaryRecord {
    pub site: PhysicalSite,
    pub line: usize,
    pub callee: String,
    pub language: Language,
    pub hint: ReviewImportHint,
    pub kind: BoundaryKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BoundaryCollection {
    pub availability: Availability,
    pub records: Vec<BoundaryRecord>,
}
