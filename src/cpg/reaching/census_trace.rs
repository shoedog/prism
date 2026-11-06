//! #660: `PRISM_RD_CENSUS_TRACE=1` writes one stderr line for every edge whose label a census
//! mechanism decided, so a measurement can join each moved edge to the rule that moved it. The
//! switch is read once; unset, nothing is formatted and nothing is written. The trace never changes
//! an output. S1a records the capture mechanisms; later slices add theirs here.
//!
//! Line: `rd-census\t<mechanism>\t<file>\t<owner>\t<from path>@<line>:<byte>\t<to path>@<line>:<byte>`.
use crate::data_flow::{FlowEdge, VarLocation};
use std::io::Write;
use std::sync::OnceLock;

/// A use in a nested callable's parameter list.
pub(super) const CAPTURE_PARAM: &str = "capture-param";
/// A use in an instance field initializer.
pub(super) const CAPTURE_FIELD: &str = "capture-field";
/// A use in a nested body that main's collector did not place (a read behind a wrapped receiver).
pub(super) const CAPTURE_BODY: &str = "capture-body";
/// The innermost-region belt: the def lies in a different region than the use.
pub(super) const CAPTURE_REGION: &str = "capture-region";

/// Whether the environment asked for the trace (read once per process).
pub(super) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("PRISM_RD_CENSUS_TRACE").is_ok_and(|v| v == "1"))
}

fn place(loc: &VarLocation) -> String {
    format!("{}@{}:{}", loc.path, loc.line, loc.start_byte)
}

pub(super) fn line(mechanism: &str, edge: &FlowEdge) -> String {
    format!(
        "rd-census\t{mechanism}\t{}\t{}\t{}\t{}",
        edge.to.file,
        edge.from.function,
        place(&edge.from),
        place(&edge.to)
    )
}

pub(super) fn emit(out: &mut impl Write, mechanism: &str, edge: &FlowEdge) {
    let _ = writeln!(out, "{}", line(mechanism, edge));
}

#[cfg(test)]
#[path = "census_trace_tests.rs"]
mod tests;
