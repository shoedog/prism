//! Pure, baseline-preserving union of completed raw taint results (#602 S4d1-U).
use super::attribution::EmissionAttribution;
use prism::slice::{SliceFinding, SliceResult};
use std::collections::BTreeSet;

mod blocks;
mod describe;
mod sources;

pub(crate) struct UnionInput<'a> {
    pub(crate) baseline: &'a SliceResult,
    pub(crate) expanded: &'a SliceResult,
    pub(crate) attribution: &'a EmissionAttribution,
    pub(crate) original_files: &'a BTreeSet<String>,
    pub(crate) context_files: &'a BTreeSet<String>,
}

#[derive(Debug)]
pub(crate) struct UnionOutcome {
    pub(crate) result: SliceResult,
    pub(crate) added_block_ids: Vec<usize>,
    pub(crate) stats: UnionStats,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct UnionStats {
    pub(crate) additional_sinks: usize,
    pub(crate) additional_sources: usize,
    pub(crate) dropped_sinks: usize,
    pub(crate) dropped_sources: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UnionError {
    BlockIdOverflow,
}

fn key(finding: &SliceFinding) -> (Option<String>, String, usize) {
    (finding.category.clone(), finding.file.clone(), finding.line)
}

/// Inputs are the producer's matching, index-aligned RAW result and attribution.
/// Baseline values are cloned verbatim; no alignment, graph work or finalization occurs here.
pub(crate) fn union_results(input: UnionInput<'_>) -> Result<UnionOutcome, UnionError> {
    let mut result = input.baseline.clone();
    let mut stats = UnionStats::default();
    let baseline_keys: BTreeSet<_> = input.baseline.findings.iter().map(key).collect();
    let mut included = BTreeSet::new();
    for (index, finding) in input.expanded.findings.iter().enumerate() {
        if baseline_keys.contains(&key(finding)) {
            continue;
        }
        if !matches!(
            finding.category.as_deref(),
            Some("taint_sink" | "unquoted_expansion")
        ) {
            continue;
        }
        let evidence = input.expanded.evidence[index].as_ref();
        if !describe::touches_context(finding, evidence, input.context_files) {
            stats.dropped_sinks += 1;
            continue;
        }
        included.insert(index);
    }
    let relations = sources::relations(
        input.attribution,
        &included,
        input.original_files,
        input.context_files,
    );
    for (index, finding) in input.expanded.findings.iter().enumerate() {
        if baseline_keys.contains(&key(finding)) {
            continue;
        }
        let mut added = finding.clone();
        let evidence = if included.contains(&index) {
            let sink = input
                .attribution
                .sinks
                .iter()
                .find(|s| s.finding_index == index)
                .expect("matching producer sink attribution");
            describe::decorate(
                &mut added,
                input.expanded.evidence[index].as_ref(),
                sink,
                input.original_files,
                input.context_files,
            );
            stats.additional_sinks += 1;
            input.expanded.evidence[index].clone()
        } else if finding.category.as_deref() == Some("taint_source") {
            let source = input
                .attribution
                .sources
                .iter()
                .find(|s| s.finding_index == index)
                .expect("matching producer source attribution");
            let at = (source.file.clone(), source.line);
            if !relations.evidence.contains_key(&at) {
                stats.dropped_sources += 1;
                continue;
            }
            added.related_files = relations.backlinks.get(&at).cloned().unwrap_or_default();
            stats.additional_sources += 1;
            Some(relations.evidence[&at].clone())
        } else {
            continue;
        };
        result.findings.push(added);
        result.evidence.push(evidence);
    }
    let (new_blocks, added_block_ids) = blocks::build(
        &input.baseline.blocks,
        &result.findings[input.baseline.findings.len()..],
        &result.evidence[input.baseline.evidence.len()..],
        input.original_files,
        input.context_files,
    )?;
    result.blocks.extend(new_blocks);
    Ok(UnionOutcome {
        result,
        added_block_ids,
        stats,
    })
}

#[cfg(test)]
mod tests;
