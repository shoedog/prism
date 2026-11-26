use crate::cpg::{BoundaryKind, CodePropertyGraph, Trace};
use crate::navigation::types::{Warning, WarningKind};
use crate::reasoning::types::ReasoningWarning;

use super::taint_reaches::location_for_var;

pub(super) fn boundary_warnings(cpg: &CodePropertyGraph, trace: &Trace) -> Vec<Warning> {
    trace
        .boundary
        .iter()
        .filter_map(|edge| {
            let target = cpg.to_var_location(edge.to)?;
            let kind = match edge.kind {
                BoundaryKind::CrossFunction => "cross-function",
                BoundaryKind::SelfFunctionParam => "self-function parameter",
                BoundaryKind::SelfFunctionReturn => "self-function return",
            };
            Some(Warning {
                kind: WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary {
                    sink: target.path.to_string(),
                    reason: Some(edge.reason),
                }),
                message: format!("taint reached a {kind} boundary at {}", target.path),
                location: Some(location_for_var(&target)),
            })
        })
        .collect()
}
