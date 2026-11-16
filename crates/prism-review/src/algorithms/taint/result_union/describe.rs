//! Actual producer coordinates, admitted related files and anchor-aware wording.
use super::super::attribution::SinkEmission;
use prism::finding_confidence::{EvidenceHop, EvidencePath};
use prism::slice::SliceFinding;
use std::collections::BTreeSet;

/// A Call hop contains only the caller and the call site, not callee coordinates.
pub(super) fn locations(evidence: &EvidencePath) -> Vec<(String, usize)> {
    let mut locations = Vec::new();
    for hop in &evidence.hops {
        match hop {
            EvidenceHop::DataFlow { from, to, .. } => {
                locations.push((from.file.clone(), from.line));
                locations.push((to.file.clone(), to.line));
            }
            EvidenceHop::Call { edge, .. } => {
                locations.push((edge.caller.file.clone(), edge.call_site_line));
            }
            _ => {}
        }
    }
    locations
}

pub(super) fn allowed(file: &str, original: &BTreeSet<String>, context: &BTreeSet<String>) -> bool {
    original.contains(file) || context.contains(file)
}

pub(super) fn touches_context(
    finding: &SliceFinding,
    evidence: Option<&EvidencePath>,
    context: &BTreeSet<String>,
) -> bool {
    context.contains(&finding.file)
        || finding.related_files.iter().any(|f| context.contains(f))
        || evidence.is_some_and(|e| locations(e).iter().any(|(f, _)| context.contains(f)))
}

fn context_witnesses(
    finding: &SliceFinding,
    evidence: Option<&EvidencePath>,
    context: &BTreeSet<String>,
) -> BTreeSet<String> {
    std::iter::once(finding.file.clone())
        .chain(finding.related_files.iter().cloned())
        .chain(
            evidence
                .into_iter()
                .flat_map(locations)
                .map(|(file, _)| file),
        )
        .filter(|file| context.contains(file))
        .collect()
}

pub(super) fn decorate(
    finding: &mut SliceFinding,
    evidence: Option<&EvidencePath>,
    sink: &SinkEmission,
    original: &BTreeSet<String>,
    context: &BTreeSet<String>,
) {
    if context.contains(&finding.file) {
        finding
            .description
            .push_str(" (sink file outside the analysed file set; loaded as taint context)");
    } else {
        let witnesses = context_witnesses(finding, evidence, context)
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        finding.description.push_str(&format!(
            " (witness passes through taint context: {})",
            witnesses
        ));
    }
    let selected = sink.selected_source.iter().map(|(file, _)| file.clone());
    let hop_files = evidence
        .into_iter()
        .flat_map(locations)
        .map(|(file, _)| file);
    let mut seen = BTreeSet::new();
    finding.related_files = selected
        .chain(hop_files)
        .filter(|file| {
            file != &finding.file && allowed(file, original, context) && seen.insert(file.clone())
        })
        .collect();
}
