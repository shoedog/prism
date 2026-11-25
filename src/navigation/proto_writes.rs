//! Deterministic listing of per-file prototype-write facts and related value reads.
use super::NavigationSession;
use crate::ast::js_ts::proto_writes::{NameRead, ProtoWrite};
use crate::repo_loader::{load_repo, LoadedRepo};
use std::collections::BTreeSet;

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
    };
    for (file, parsed) in &repo.files {
        let mut lines = BTreeSet::new();
        let mut remaining = FILE_HISTORY_BUDGET;
        for write in parsed.js_ts_proto_writes() {
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
