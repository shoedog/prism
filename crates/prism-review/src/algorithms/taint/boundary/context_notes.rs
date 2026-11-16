//! Reconcile retained round-zero diagnostics with one coherent completed review.
use super::{display::escape_content, records::*};
use crate::{
    algorithms::taint::{
        context_budget::{AdmissionBound, SkipReason, StopCause},
        frontier::Disposition,
        result_union::UnionError,
        rounds::Stop,
    },
    api::context_run::{ContextRequest, ContextState, ReviewContext, UnionState},
};
use prism::{
    api::{ContextUnavailable, ReviewImportHint},
    languages::Language,
};
use std::collections::BTreeSet;

fn note(kind: ContextRunNoteKind, message: String) -> ContextRunNote {
    ContextRunNote {
        kind,
        message,
        location: None,
    }
}
fn unavailable(value: &ContextUnavailable) -> ContextRunNote {
    let reason = match value {
        ContextUnavailable::ListingUnavailable { message } => escape_content(message),
        ContextUnavailable::RootUnavailable { message } => escape_content(message),
        ContextUnavailable::RootChangedDuringCapture => {
            "repository root changed during capture".into()
        }
        _ => "loading authority unavailable".into(),
    };
    note(
        ContextRunNoteKind::Unavailable,
        format!("taint-context: context loading unavailable: {reason}; baseline taint only"),
    )
}
pub(crate) fn notes_only(
    state: &ContextState,
    request: &ContextRequest,
    original_warnings: &[String],
) -> Vec<ContextRunNote> {
    match state {
        ContextState::Unavailable(reason) => vec![unavailable(reason)],
        ContextState::Expanded(context) => context_run_notes(context, request, original_warnings),
        ContextState::NotRequested => Vec::new(),
    }
}
fn skip_reason(reason: &SkipReason) -> String {
    match reason {
        SkipReason::InvalidPath => "invalid path".into(),
        SkipReason::NotListed => "not listed".into(),
        SkipReason::Unsupported => "unsupported language".into(),
        SkipReason::OriginalFile => "original file".into(),
        SkipReason::UnsafePath => "unsafe path".into(),
        SkipReason::NotRegular => "not a regular file".into(),
        SkipReason::Unreadable { kind, message } => {
            format!("unreadable {kind:?}: {}", escape_content(message))
        }
        SkipReason::NotUtf8 => "not UTF-8".into(),
        SkipReason::TooLarge { observed_at_least } => {
            format!("too large (at least {observed_at_least} bytes)")
        }
        SkipReason::ParseFailed { message } => format!("parse failed: {}", escape_content(message)),
        SkipReason::SevereParse {
            error_count,
            node_count,
        } => format!("severe parse ({error_count}/{node_count} nodes)"),
        SkipReason::Unknown => "unknown reason".into(),
    }
}
fn hint_kind(hint: &ReviewImportHint, language: Language) -> BoundaryKind {
    match hint {
        ReviewImportHint::InRepo { declaring, .. }
            if language == Language::Go && declaring.is_empty() =>
        {
            BoundaryKind::GoDeclarationUnknown
        }
        ReviewImportHint::InRepo { .. } => BoundaryKind::InRepo,
        _ => BoundaryKind::Unresolved,
    }
}
/// `original_warnings` is the exact prefix cloned by the driver and carried by
/// atomic stages into N; only its appended suffix belongs to admitted context.
pub(crate) fn reconcile(
    round_zero: &BoundaryCollection,
    state: &ContextState,
    request: &ContextRequest,
    original_warnings: &[String],
) -> (BoundaryCollection, Vec<ContextRunNote>) {
    let ContextState::Expanded(context) = state else {
        return (
            round_zero.clone(),
            notes_only(state, request, original_warnings),
        );
    };
    if matches!(&context.union, UnionState::Refused(_)) {
        return (
            round_zero.clone(),
            context_run_notes(context, request, original_warnings),
        );
    }
    let mut collection = round_zero.clone();
    if matches!(&context.union, UnionState::Applied { .. }) && !context.context_files.is_empty() {
        let status = |site: &PhysicalSite| {
            let indices: BTreeSet<_> = context
                .frontier
                .provenance
                .iter()
                .enumerate()
                .filter_map(|(i, p)| (&p.site == site).then_some(i))
                .collect();
            let mut resolved = false;
            let mut unresolved = false;
            for (origin, _, disposition) in &context.frontier.dispositions {
                if indices.contains(origin) {
                    resolved |= *disposition == Disposition::Resolved;
                    unresolved |= *disposition == Disposition::LoadedUnresolved;
                }
            }
            (resolved, unresolved)
        };
        collection.records.retain(|r| !status(&r.site).0);
        for r in &mut collection.records {
            if status(&r.site).1 {
                r.kind = BoundaryKind::LoadedUnresolved;
            }
        }
        for p in &context.frontier.provenance {
            if p.first_round == 0 || collection.records.iter().any(|r| r.site == p.site) {
                continue;
            }
            let (resolved, unresolved) = status(&p.site);
            if resolved {
                continue;
            }
            collection.records.push(BoundaryRecord {
                site: p.site.clone(),
                line: p.line,
                callee: p.callee.clone(),
                language: p.language,
                hint: p.hint.clone(),
                kind: if unresolved {
                    BoundaryKind::LoadedUnresolved
                } else {
                    hint_kind(&p.hint, p.language)
                },
            });
        }
    }
    (
        collection,
        context_run_notes(context, request, original_warnings),
    )
}
fn context_run_notes(
    context: &ReviewContext,
    request: &ContextRequest,
    original_warnings: &[String],
) -> Vec<ContextRunNote> {
    if matches!(
        &context.union,
        UnionState::Refused(UnionError::BlockIdOverflow)
    ) {
        return vec![note(ContextRunNoteKind::UnionRefused,"taint-context: expanded results could not be combined with baseline (block ID overflow); baseline taint only".into())];
    }
    let mut notes = Vec::new();
    match &context.stop {
        Stop::DepthReached => {
            let mut sites=BTreeSet::new();
            for (i,_,d) in &context.frontier.dispositions {
                if *d == Disposition::DepthBound {
                    if let Some(p)=context.frontier.provenance.get(*i) {
                        let resolved=context.frontier.dispositions.iter().any(|(j,_,v)| *v==Disposition::Resolved && context.frontier.provenance.get(*j).is_some_and(|q|q.site==p.site));
                        if !resolved {sites.insert(p.site.clone());}
                    }
                }
            }
            notes.push(note(ContextRunNoteKind::DepthBound,format!("taint-context: depth bound {} reached; {} boundary calls not followed",request.limits.depth,sites.len())));
        }
        Stop::Bound(AdmissionBound::Files{skipped,..}) => notes.push(note(ContextRunNoteKind::FilesBound,format!("taint-context: max-files bound {} reached; {skipped} context files not loaded",request.limits.max_files))),
        Stop::Bound(AdmissionBound::SourceBytes{skipped,..}) => notes.push(note(ContextRunNoteKind::BytesBound,format!("taint-context: context source bound {} bytes reached; {skipped} context files not loaded",request.limits.source_bytes))),
        Stop::Guard(StopCause::Time) => notes.push(note(ContextRunNoteKind::TimeGuard,format!("taint-context: time guard ({:?}) stopped expansion; results are from the last completed round {}",request.limits.time,context.round))),
        Stop::Guard(StopCause::Cancelled) => notes.push(note(ContextRunNoteKind::Cancelled,format!("taint-context: expansion cancelled; results are from the last completed round {}",context.round))),
        Stop::StageFailed(message) => notes.push(note(ContextRunNoteKind::StageFailed,format!("taint-context: expansion round {} failed: {}; results are from the last completed round",context.round+1,escape_content(message)))),
        Stop::FrontierExhausted => {},
    }
    let mut skips: Vec<_> = context.attempts.skips.iter().collect();
    skips.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, reason) in skips.iter().take(10) {
        notes.push(note(
            ContextRunNoteKind::FileSkipped,
            format!(
                "taint-context: context file skipped: {} ({})",
                escape_content(path),
                skip_reason(reason)
            ),
        ));
    }
    if skips.len() > 10 {
        notes.push(note(
            ContextRunNoteKind::SkippedSummary,
            format!(
                "taint-context: {} more context files skipped",
                skips.len() - 10
            ),
        ));
    }
    for warning in context.parse_warnings.iter().skip(original_warnings.len()) {
        notes.push(note(
            ContextRunNoteKind::ContextParse,
            escape_content(warning),
        ));
    }
    notes
}
