//! Source-only entry through the repository loader's walk and admissibility
//! policy. Clients supply the view used to check the shared parse cutoff.

use super::{
    collect_walk_items, parse_candidates_parallel, MergeItem, SkipReason, SkippedFile,
    SEVERE_PARSE_ERROR_RATE,
};
use crate::ast::ParsedFile;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) struct Selection<'a> {
    pub includes: &'a (dyn Fn(&str) -> bool + Sync),
    pub excluded_directories: &'a [&'a str],
}

/// Load selected parsed sources without building repository topology.
/// `error_rate` can grade a normalized view while retaining the original
/// bytes and parse in the returned files.
pub fn load_ast_sources(
    root: &Path,
    includes: &(dyn Fn(&str) -> bool + Sync),
    excluded_directories: &[&str],
    error_rate: impl Fn(&ParsedFile) -> f64 + Send,
) -> Result<(BTreeMap<String, ParsedFile>, Vec<SkippedFile>)> {
    let (items, candidates, _) = collect_walk_items(
        root,
        Some(Selection {
            includes,
            excluded_directories,
        }),
    )?;
    let mut outcomes = parse_candidates_parallel(candidates).into_iter();
    crate::build_pool::install(move || {
        let mut files = BTreeMap::new();
        let mut skipped = Vec::new();
        for item in items {
            match item {
                MergeItem::Skip(skip) if includes(&skip.path) => skipped.push(skip),
                MergeItem::Skip(_) => {}
                MergeItem::Candidate { rel } => {
                    let outcome = outcomes.next().expect("one parse outcome per source");
                    debug_assert_eq!(outcome.rel, rel);
                    match outcome.parsed {
                        Ok(parsed) if error_rate(&parsed) <= SEVERE_PARSE_ERROR_RATE => {
                            files.insert(rel, parsed);
                        }
                        _ => skipped.push(SkippedFile {
                            path: rel,
                            reason: SkipReason::ParseFailed,
                        }),
                    }
                }
            }
        }
        debug_assert!(outcomes.next().is_none());
        Ok((files, skipped))
    })
}
