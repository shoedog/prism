//! Deterministic listing of per-file prototype-write facts and related value reads.
use super::NavigationSession;
use crate::ast::js_ts::proto_writes::{GuardForm, GuardState, NameRead, ProtoWrite};
use crate::repo_loader::{load_repo, LoadedRepo};
use std::collections::{BTreeMap, BTreeSet};

const FILE_HISTORY_BUDGET: usize = 500_000;
use std::path::Path;

#[derive(serde::Serialize)]
pub struct ProtoWriteRow {
    pub file: String,
    #[serde(flatten)]
    pub write: ProtoWrite,
}

#[derive(serde::Serialize)]
pub struct LineReads {
    pub file: String,
    pub line: usize,
    pub reads: Vec<NameRead>,
}

#[derive(serde::Serialize)]
pub struct ProtoWriteListing {
    pub facts: Vec<ProtoWriteRow>,
    pub name_reads: Vec<LineReads>,
    pub guard_counts: BTreeMap<GuardState, usize>,
    pub discharged_by_form: BTreeMap<GuardForm, usize>,
    pub discharged_with_multiple_forms: usize,
    pub guard_refusals: BTreeMap<String, usize>,
}

pub fn proto_writes(session: &NavigationSession) -> ProtoWriteListing {
    list(&session.repo, true)
}

/// The standalone listing only needs the repository's parsed files.
pub fn repo_proto_writes(root: &Path) -> anyhow::Result<ProtoWriteListing> {
    Ok(list(&load_repo(root)?, true))
}

/// Text needs facts and refusal reasons. Candidate histories are never expanded.
pub fn repo_proto_writes_text(root: &Path) -> anyhow::Result<ProtoWriteListing> {
    Ok(list(&load_repo(root)?, false))
}

fn list(repo: &LoadedRepo, histories: bool) -> ProtoWriteListing {
    use crate::ast::js_ts::proto_writes::KeyRole;

    let mut listing = ProtoWriteListing {
        facts: Vec::new(),
        name_reads: Vec::new(),
        guard_counts: BTreeMap::from([
            (GuardState::None, 0),
            (GuardState::Partial, 0),
            (GuardState::Sufficient, 0),
        ]),
        discharged_by_form: BTreeMap::new(),
        discharged_with_multiple_forms: 0,
        guard_refusals: BTreeMap::new(),
    };
    for (file, parsed) in &repo.files {
        let mut lines = BTreeSet::new();
        let mut remaining = FILE_HISTORY_BUDGET;
        for write in parsed.js_ts_proto_writes() {
            *listing.guard_counts.entry(write.guard).or_default() += 1;
            if write.guard == GuardState::Sufficient {
                listing.discharged_with_multiple_forms += usize::from(write.guard_forms.len() > 1);
                for form in &write.guard_forms {
                    *listing.discharged_by_form.entry(*form).or_default() += 1;
                }
            }
            lines.extend(write.line..=write.end_line);
            lines.extend(
                write
                    .key_operands
                    .iter()
                    .filter(|key| key.role == KeyRole::Selector)
                    .map(|key| key.line),
            );
            listing.facts.push(ProtoWriteRow {
                file: file.clone(),
                write,
            });
        }
        for (reason, count) in parsed.js_ts_proto_write_guard_refusals() {
            *listing
                .guard_refusals
                .entry(format!("guard_unrecognised:{reason}"))
                .or_default() += count;
        }
        for line in lines {
            let reads = if histories {
                parsed.js_ts_listed_name_reads(line, &mut remaining)
            } else {
                parsed
                    .js_ts_name_read_reasons(line)
                    .map(|(read, _)| NameRead {
                        path: read.path.clone(),
                        base: read.base.clone(),
                        span: read.span.clone(),
                        owner: read.owner.clone(),
                        binding: read.binding.clone(),
                        reaching_versions: Vec::new(),
                        resolution_complete: false,
                        incomplete_reason: read.incomplete_reason,
                        candidate_history_refused: false,
                    })
                    .collect()
            };
            if histories || !reads.is_empty() {
                listing.name_reads.push(LineReads {
                    file: file.clone(),
                    line,
                    reads,
                });
            }
        }
    }
    listing
}
