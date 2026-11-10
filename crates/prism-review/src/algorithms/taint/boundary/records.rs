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
