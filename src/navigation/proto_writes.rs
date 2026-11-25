//! Deterministic listing of per-file prototype-write facts and related value reads.
use super::NavigationSession;
use crate::ast::js_ts::proto_writes::{NameRead, ProtoWrite};
use crate::repo_loader::{load_repo, LoadedRepo};
use std::collections::BTreeSet;
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
    list(&session.repo)
}

/// The standalone listing only needs the repository's parsed files.
pub fn repo_proto_writes(root: &Path) -> anyhow::Result<ProtoWriteListing> {
    Ok(list(&load_repo(root)?))
}

fn list(repo: &LoadedRepo) -> ProtoWriteListing {
    use crate::ast::js_ts::proto_writes::KeyRole;

    let mut listing = ProtoWriteListing {
        facts: Vec::new(),
        name_reads: Vec::new(),
    };
    for (file, parsed) in &repo.files {
        let mut lines = BTreeSet::new();
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
        listing
            .name_reads
            .extend(lines.into_iter().map(|line| LineReads {
                file: file.clone(),
                line,
                reads: parsed.js_ts_name_reads(line),
            }));
    }
    listing
}
