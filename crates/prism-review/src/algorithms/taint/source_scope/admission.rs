//! Source-file admission and the crate-private scoped taint outcome (#602 S3a.1s).
//!
//! A scoped run lets context files stay loaded (they still hold reachable sinks and
//! keep their source-recognition facts) while only original files start roots or
//! independent inline source=sink findings. Nothing here is public.
use super::loading::project_loading_trace;
use crate::algorithms::taint::{TaintConfig, TaintTrace};
use anyhow::Result;
use prism::cpg::CpgContext;
use prism::diff::DiffInput;
use prism::slice::SliceResult;
use std::collections::{BTreeMap, BTreeSet};

/// Per-run scope, passed as ONE parameter so a later slice adds a field, not an
/// argument. `source: None` is the legacy run: every loaded file may start roots.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RunScope<'a> {
    source: Option<&'a BTreeSet<String>>,
}

impl<'a> RunScope<'a> {
    pub(crate) fn original(original_files: &'a BTreeSet<String>) -> Self {
        Self {
            source: Some(original_files),
        }
    }

    /// The ONE admission predicate: may `file` start a root or an inline finding?
    /// File identity only: no line validation, canonicalization or name matching.
    pub(crate) fn admits_source_file<V>(&self, loaded: &BTreeMap<String, V>, file: &str) -> bool {
        match self.source {
            None => true,
            Some(original) => loaded.contains_key(file) && original.contains(file),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExclusionReason {
    NotLoaded,
    OutsideOriginalFiles,
}

/// An explicit source request the scope refused, in request order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExcludedExplicit {
    pub(crate) file: String,
    pub(crate) line: usize,
    pub(crate) reason: ExclusionReason,
}

#[derive(Debug)]
pub(crate) struct ScopedTaintOutcome {
    pub(crate) result: SliceResult,
    /// The full trace of the scoped run (admitted roots only, every origin).
    pub(crate) trace: TaintTrace,
    /// The roots allowed to cause loading, projected from the COMPLETED `trace`.
    pub(crate) loading: TaintTrace,
    pub(crate) excluded_explicit: Vec<ExcludedExplicit>,
}

/// Explicit requests the scope refuses: loaded is checked first, repeats are kept.
fn excluded_explicit(
    ctx: &CpgContext,
    config: &TaintConfig,
    original_files: &BTreeSet<String>,
) -> Vec<ExcludedExplicit> {
    let mut excluded = Vec::new();
    for (file, line) in &config.sources {
        let reason = if !ctx.files.contains_key(file) {
            ExclusionReason::NotLoaded
        } else if !original_files.contains(file) {
            ExclusionReason::OutsideOriginalFiles
        } else {
            continue;
        };
        excluded.push(ExcludedExplicit {
            file: file.clone(),
            line: *line,
            reason,
        });
    }
    excluded
}

pub(crate) fn slice_scoped(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &TaintConfig,
    original_files: &BTreeSet<String>,
) -> Result<ScopedTaintOutcome> {
    let excluded_explicit = excluded_explicit(ctx, config, original_files);
    let mut trace = TaintTrace::default();
    let scope = RunScope::original(original_files);
    let result = crate::algorithms::taint::run::slice_scoped_impl(
        ctx,
        diff,
        config,
        Some(&mut trace),
        scope,
    )?;
    // Project only from the completed full trace, never from findings or a capped list.
    let loading = project_loading_trace(&trace, original_files, ctx.files);
    Ok(ScopedTaintOutcome {
        result,
        trace,
        loading,
        excluded_explicit,
    })
}
