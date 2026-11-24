//! Private complete eligible frontier and owned continuation provenance for S4c2.
use crate::algorithms::taint::boundary::records::PhysicalSite;
use prism::{api::ReviewImportHint, languages::Language};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallProvenance {
    pub site: PhysicalSite,
    pub line: usize,
    pub callee: String,
    pub language: Language,
    pub hint: ReviewImportHint,
    pub first_round: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub target: String,
    pub member: Option<String>,
    pub depth: u8,
    pub origin: usize,
    pub visited: Vec<(String, Option<String>)>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    Requested,
    DepthBound,
    GoDeclarationUnknown,
    Unresolvable,
    TargetRefused,
    BarrelCycle,
    LoadedUnresolved,
    Resolved,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrontierState {
    pub provenance: Vec<CallProvenance>,
    pub open: Vec<Request>,
    pub load_order: Vec<String>,
    pub dispositions: Vec<(usize, String, Disposition)>,
}
mod plan;
pub(crate) use plan::plan_round;
#[cfg(test)]
mod tests;
