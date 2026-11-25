use crate::cpg::{BoundaryEdge, BoundaryKind, CodePropertyGraph, LossReason, Trace};
use crate::data_flow::VarLocation;
use crate::navigation::types::{Warning, WarningKind};
use crate::reasoning::types::ReasoningWarning;

use super::boundary_targets::BoundaryAttribution;
use super::taint_reaches::location_for_var;

pub(super) fn boundary_warnings(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    frontier: bool,
) -> Vec<Warning> {
    trace
        .boundary
        .iter()
        .filter_map(|edge| {
            let target = cpg.to_var_location(edge.to)?;
            Some(boundary_warning(
                edge,
                &target,
                frontier.then_some(edge.reason),
            ))
        })
        .collect()
}

fn boundary_warning(
    edge: &BoundaryEdge,
    target: &VarLocation,
    reason: Option<LossReason>,
) -> Warning {
    let kind = match edge.kind {
        BoundaryKind::CrossFunction => "cross-function",
        BoundaryKind::SelfFunctionParam => "self-function parameter",
        BoundaryKind::SelfFunctionReturn => "self-function return",
    };
    Warning {
        kind: WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary {
            sink: target.path.to_string(),
            reason,
        }),
        message: format!("taint reached a {kind} boundary at {}", target.path),
        location: Some(location_for_var(target)),
    }
}

pub(super) fn attribution_warning(
    cpg: &CodePropertyGraph,
    source: &VarLocation,
    sink: &VarLocation,
    attribution: BoundaryAttribution<'_>,
) -> Option<Warning> {
    match attribution {
        BoundaryAttribution::None => None,
        BoundaryAttribution::Matched(edge) => {
            let target = cpg.to_var_location(edge.to)?;
            Some(boundary_warning(edge, &target, Some(edge.reason)))
        }
        BoundaryAttribution::Incomplete { .. } => Some(Warning {
            kind: WarningKind::SkippedPath,
            message: format!(
                "taint boundary attribution incomplete (unexplained) for {}:{} [{}..{}] to {}",
                source.file, source.line, source.start_byte, source.end_byte, sink.path
            ),
            location: Some(location_for_var(sink)),
        }),
    }
}
