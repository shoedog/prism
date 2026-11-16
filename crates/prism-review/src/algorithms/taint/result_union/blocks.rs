//! New block custody: no baseline mutation, admitted coordinates, lazy checked IDs.
use super::describe::{allowed, locations};
use super::UnionError;
use prism::diff::{DiffBlock, ModifyType};
use prism::finding_confidence::EvidencePath;
use prism::slice::SliceFinding;
use std::collections::{BTreeMap, BTreeSet};

type Lines = BTreeMap<String, BTreeMap<usize, bool>>;
fn add(
    lines: &mut Lines,
    file: &str,
    line: usize,
    is_anchor: bool,
    original: &BTreeSet<String>,
    context: &BTreeSet<String>,
) {
    if !allowed(file, original, context) {
        return;
    }
    let flag = lines
        .entry(file.to_string())
        .or_default()
        .entry(line)
        .or_default();
    // DiffBlock::add_line overwrites flags, so combine roles before calling it.
    *flag |= is_anchor;
}

pub(super) fn build(
    baseline: &[DiffBlock],
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    original: &BTreeSet<String>,
    context: &BTreeSet<String>,
) -> Result<(Vec<DiffBlock>, Vec<usize>), UnionError> {
    let mut lines = Lines::new();
    for (finding, evidence) in findings.iter().zip(evidence) {
        add(
            &mut lines,
            &finding.file,
            finding.line,
            true,
            original,
            context,
        );
        if let Some(evidence) = evidence {
            for (file, line) in locations(evidence) {
                add(&mut lines, &file, line, false, original, context);
            }
        }
    }
    let mut last_id = baseline.iter().map(|b| b.block_id).max();
    let mut new_blocks = Vec::new();
    let mut ids = Vec::new();
    for (file, lines) in lines {
        let id = match last_id {
            Some(id) => id.checked_add(1).ok_or(UnionError::BlockIdOverflow)?,
            None => 0,
        };
        let mut block = DiffBlock::new(id, file.clone(), ModifyType::Modified);
        for (line, flag) in lines {
            block.add_line(&file, line, flag);
        }
        last_id = Some(id);
        ids.push(id);
        new_blocks.push(block);
    }
    Ok((new_blocks, ids))
}
