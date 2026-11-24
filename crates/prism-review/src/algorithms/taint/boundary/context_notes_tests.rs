//! CONTROLLED completed states; D18 and the missing-root row live in the API runner tests.
use super::{context_notes::reconcile, display, records::*};
use crate::algorithms::taint::{
    context_budget::{
        AdmissionBound, AttemptCounters, ContextLimits, Published, SkipReason, StopCause,
    },
    frontier::{CallProvenance, Disposition, FrontierState},
    result_union::{UnionError, UnionStats},
    rounds::{RoundStats, Stop},
};
use crate::api::context_run::{ContextRequest, ContextState, ReviewContext, UnionState};
use prism::{
    api::{ContextUnavailable, ReviewHintBasis, ReviewImportHint},
    call_graph::FunctionId,
    cpg::CrossingCeiling,
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
fn request() -> ContextRequest {
    ContextRequest {
        limits: ContextLimits {
            depth: 2,
            max_files: 7,
            source_bytes: 42,
            time: Duration::from_secs(3),
        },
        ceiling: CrossingCeiling::NameOnly,
    }
}
fn record(i: usize) -> BoundaryRecord {
    BoundaryRecord {
        site: PhysicalSite {
            caller: FunctionId {
                file: format!("f{i:02}.js"),
                name: "f".into(),
                start_line: 1,
                end_line: 4,
            },
            start_byte: i * 10,
            end_byte: i * 10 + 4,
        },
        line: 2,
        callee: "g".into(),
        language: Language::JavaScript,
        hint: ReviewImportHint::InRepo {
            files: vec!["target.js".into()],
            declaring: vec![],
            spec: "./target".into(),
            basis: ReviewHintBasis::Bound,
            member: Some("g".into()),
        },
        kind: BoundaryKind::InRepo,
    }
}
fn collection(n: usize) -> BoundaryCollection {
    BoundaryCollection {
        availability: Availability::Available,
        records: (0..n).map(record).collect(),
    }
}
fn prov(r: &BoundaryRecord, round: u8) -> CallProvenance {
    CallProvenance {
        site: r.site.clone(),
        line: r.line,
        callee: r.callee.clone(),
        language: r.language,
        hint: r.hint.clone(),
        first_round: round,
    }
}
fn state(frontier: FrontierState, stop: Stop) -> ContextState {
    ContextState::Expanded(ReviewContext {
        files: BTreeMap::new(),
        sources: BTreeMap::new(),
        parse_quality: BTreeMap::new(),
        parse_warnings: vec![],
        context_files: BTreeSet::from(["target.js".into()]),
        round: 1,
        published: Published::default(),
        stats: RoundStats::default(),
        stop,
        attempts: AttemptCounters::default(),
        frontier,
        union: UnionState::Applied {
            stats: UnionStats::default(),
            added_block_ids: vec![],
        },
    })
}
fn run(c: &BoundaryCollection, s: &ContextState) -> (BoundaryCollection, Vec<ContextRunNote>) {
    reconcile(c, s, &request(), &[])
}
fn resolved(ds: Vec<Disposition>) -> ContextState {
    let r = record(0);
    let frontier = FrontierState {
        provenance: vec![prov(&r, 0)],
        dispositions: ds
            .into_iter()
            .enumerate()
            .map(|(i, d)| (0, format!("t{i}"), d))
            .collect(),
        ..Default::default()
    };
    state(frontier, Stop::FrontierExhausted)
}
#[test]
fn d01_not_requested_parity() {
    let c = collection(1);
    let (out, n) = run(&c, &ContextState::NotRequested);
    assert_eq!(display::project(&c), display::project_with_notes(&out, n));
}
#[test]
fn d02_resolved_removed() {
    let (c, _) = run(&collection(1), &resolved(vec![Disposition::Resolved]));
    assert!(c.records.is_empty());
}
#[test]
fn d03_loaded_unresolved_wording() {
    let (c, _) = run(
        &collection(1),
        &resolved(vec![Disposition::LoadedUnresolved]),
    );
    let d = display::project(&c);
    assert_eq!(d.located()[0].kind(), BoundaryKind::LoadedUnresolved);
    assert!(d.located()[0].message().contains("loaded as taint context"));
    assert!(!d.located()[0].message().contains("not analysed"));
}
#[test]
fn d04_unfollowed_layer_a_bytes() {
    for d in [
        Disposition::DepthBound,
        Disposition::TargetRefused,
        Disposition::Unresolvable,
        Disposition::GoDeclarationUnknown,
        Disposition::BarrelCycle,
        Disposition::Requested,
    ] {
        let c = collection(1);
        let (out, _) = run(&c, &resolved(vec![d]));
        assert_eq!(display::project(&c), display::project(&out));
    }
}
#[test]
fn d05_ineligible_survives() {
    let c = collection(2);
    let (out, _) = run(&c, &resolved(vec![Disposition::Resolved]));
    assert_eq!(out.records, vec![record(1)]);
}
#[test]
fn d06_resolved_wins() {
    let (c, _) = run(
        &collection(1),
        &resolved(vec![Disposition::LoadedUnresolved, Disposition::Resolved]),
    );
    assert!(c.records.is_empty());
    let (c, _) = run(
        &collection(1),
        &resolved(vec![Disposition::LoadedUnresolved, Disposition::DepthBound]),
    );
    assert_eq!(c.records[0].kind, BoundaryKind::LoadedUnresolved);
    // Two independently indexed targets share exactly one physical call site.
    for resolved_first in [false, true] {
        let r = record(0);
        let mut x = prov(&r, 0);
        let mut y = prov(&r, 0);
        if let ReviewImportHint::InRepo { files, .. } = &mut x.hint {
            *files = vec!["x.js".into()];
        }
        if let ReviewImportHint::InRepo { files, .. } = &mut y.hint {
            *files = vec!["y.js".into()];
        }
        let (provenance, dispositions) = if resolved_first {
            (
                vec![y, x],
                vec![
                    (0, "y.js".into(), Disposition::Resolved),
                    (1, "x.js".into(), Disposition::LoadedUnresolved),
                ],
            )
        } else {
            (
                vec![x, y],
                vec![
                    (0, "x.js".into(), Disposition::LoadedUnresolved),
                    (1, "y.js".into(), Disposition::Resolved),
                ],
            )
        };
        let frontier = FrontierState {
            provenance,
            dispositions,
            ..Default::default()
        };
        let (out, _) = run(&collection(1), &state(frontier, Stop::FrontierExhausted));
        assert!(
            out.records.is_empty(),
            "any resolved index wins for the same physical site"
        );
    }
}
#[test]
fn d07_context_call_added() {
    let mut r = record(2);
    r.site.caller.file = "a.go".into();
    r.language = Language::Go;
    if let ReviewImportHint::InRepo { declaring, .. } = &mut r.hint {
        declaring.clear();
    }
    let fr = FrontierState {
        provenance: vec![prov(&r, 1)],
        dispositions: vec![(0, "x.go".into(), Disposition::DepthBound)],
        ..Default::default()
    };
    let (out, _) = run(&collection(0), &state(fr, Stop::FrontierExhausted));
    let d = display::project(&out);
    assert_eq!(d.located()[0].file(), "a.go");
    assert!(d.located()[0].message().contains("declaring file"));
}
#[test]
fn d08_reconcile_before_cap() {
    let c = collection(14);
    let fr = FrontierState {
        provenance: vec![prov(&record(0), 0), prov(&record(1), 0)],
        dispositions: vec![
            (0, "a".into(), Disposition::Resolved),
            (1, "b".into(), Disposition::Resolved),
        ],
        ..Default::default()
    };
    let (out, _) = run(&c, &state(fr, Stop::FrontierExhausted));
    let d = display::project(&out);
    assert_eq!(d.located().len(), 10);
    assert_eq!(d.omitted_count(), 2);
}
#[test]
fn d09_stop_notes_and_limits() {
    let rows = [
        (
            Stop::DepthReached,
            ContextRunNoteKind::DepthBound,
            "depth bound 3 reached; 0 boundary calls not followed",
        ),
        (
            Stop::Bound(AdmissionBound::Files {
                first_skipped: 9,
                skipped: 3,
            }),
            ContextRunNoteKind::FilesBound,
            "max-files bound 7 reached; 3 context files not loaded",
        ),
        (
            Stop::Bound(AdmissionBound::SourceBytes {
                first_skipped: 9,
                skipped: 2,
            }),
            ContextRunNoteKind::BytesBound,
            "context source bound 42 bytes reached; 2 context files not loaded",
        ),
        (
            Stop::Guard(StopCause::Time),
            ContextRunNoteKind::TimeGuard,
            "3",
        ),
        (
            Stop::Guard(StopCause::Cancelled),
            ContextRunNoteKind::Cancelled,
            "cancel",
        ),
        (
            Stop::StageFailed("boom\n".into()),
            ContextRunNoteKind::StageFailed,
            "boom\\n",
        ),
    ];
    for (stop, kind, part) in rows {
        let mut nondefault = request();
        nondefault.limits.depth = 3;
        let (_, notes) = reconcile(
            &collection(0),
            &state(FrontierState::default(), stop),
            &nondefault,
            &[],
        );
        assert_eq!(notes[0].kind(), kind);
        assert!(notes[0].message().contains(part));
    }
    let (_, n) = run(
        &collection(0),
        &state(FrontierState::default(), Stop::FrontierExhausted),
    );
    assert!(n.is_empty());
}
#[test]
fn d10_depth_count_permutation() {
    let first = record(0);
    let second = record(1);
    let fr = FrontierState {
        provenance: vec![prov(&first, 0), prov(&second, 0)],
        dispositions: vec![
            (0, "x".into(), Disposition::DepthBound),
            (1, "y".into(), Disposition::DepthBound),
        ],
        ..Default::default()
    };
    let a = run(&collection(2), &state(fr, Stop::DepthReached)).1;
    let reversed = FrontierState {
        provenance: vec![prov(&second, 0), prov(&first, 0)],
        dispositions: vec![
            (0, "y".into(), Disposition::DepthBound),
            (1, "x".into(), Disposition::DepthBound),
        ],
        ..Default::default()
    };
    let b = run(&collection(2), &state(reversed, Stop::DepthReached)).1;
    assert_eq!(a, b);
    assert!(a[0].message().contains("2 boundary calls"));
}
#[test]
fn d11_all_skip_reasons_and_cap() {
    let reasons = vec![
        SkipReason::InvalidPath,
        SkipReason::NotListed,
        SkipReason::Unsupported,
        SkipReason::OriginalFile,
        SkipReason::UnsafePath,
        SkipReason::NotRegular,
        SkipReason::Unreadable {
            kind: std::io::ErrorKind::Other,
            message: "bad".into(),
        },
        SkipReason::NotUtf8,
        SkipReason::TooLarge {
            observed_at_least: 2,
        },
        SkipReason::ParseFailed {
            message: "bad".into(),
        },
        SkipReason::SevereParse {
            error_count: 1,
            node_count: 2,
        },
        SkipReason::Unknown,
    ];
    let mut rendered = BTreeSet::new();
    for reason in &reasons {
        let mut s = state(FrontierState::default(), Stop::FrontierExhausted);
        if let ContextState::Expanded(c) = &mut s {
            c.attempts.skips.push(("x".into(), reason.clone()));
        }
        let n = run(&collection(0), &s).1;
        assert_eq!(n[0].kind(), ContextRunNoteKind::FileSkipped);
        rendered.insert(n[0].message().to_string());
    }
    assert_eq!(rendered.len(), 12);
    let mut s = state(FrontierState::default(), Stop::FrontierExhausted);
    if let ContextState::Expanded(c) = &mut s {
        c.attempts.skips = reasons
            .into_iter()
            .enumerate()
            .rev()
            .map(|(i, r)| (format!("p{i:02}"), r))
            .collect();
        c.attempts.skips.push(("p12".into(), SkipReason::Unknown));
    }
    let n = run(&collection(0), &s).1;
    assert_eq!(n.len(), 11);
    assert!(n[0].message().contains("p00"));
    assert_eq!(n[10].kind(), ContextRunNoteKind::SkippedSummary);
    assert!(n[10].message().contains('3'));
}
#[test]
fn d12_warning_prefix_and_escape_once() {
    let raw = "same\n\u{202e}\"\\literal".to_string();
    let mut s = state(FrontierState::default(), Stop::FrontierExhausted);
    if let ContextState::Expanded(c) = &mut s {
        c.parse_warnings = vec![raw.clone(), raw.clone()];
    }
    let (_, n) = reconcile(&collection(0), &s, &request(), std::slice::from_ref(&raw));
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].kind(), ContextRunNoteKind::ContextParse);
    assert_eq!(n[0].message(), display::escape_content(&raw));
    if let ContextState::Expanded(c) = &s {
        assert_eq!(c.parse_warnings, vec![raw.clone(), raw]);
    }
}
#[test]
fn d13_unavailable_preserves() {
    let c = collection(1);
    let s = ContextState::Unavailable(ContextUnavailable::RootUnavailable {
        message: "gone".into(),
    });
    let (out, n) = run(&c, &s);
    assert_eq!(out, c);
    assert_eq!(n[0].kind(), ContextRunNoteKind::Unavailable);
}
#[test]
fn d14_existing_display_order() {
    let mut c = collection(11);
    c.availability = Availability::ListingUnavailable("bad".into());
    let old = display::project(&c);
    let note = ContextRunNote {
        kind: ContextRunNoteKind::Ignored,
        message: "tail".into(),
        location: None,
    };
    let d = display::project_with_notes(&c, vec![note]);
    assert_eq!(d.located(), old.located());
    assert_eq!(d.summary(), old.summary());
    assert_eq!(d.availability(), old.availability());
    assert_eq!(d.run_notes()[0].message(), "tail");
    let mut expected = old.messages();
    expected.push("tail".into());
    assert_eq!(d.messages(), expected);
}
#[test]
fn d15_escaped_dynamic_skip() {
    let mut s = state(FrontierState::default(), Stop::FrontierExhausted);
    if let ContextState::Expanded(c) = &mut s {
        c.attempts
            .skips
            .push(("x\n\u{202e}\"\\n".into(), SkipReason::InvalidPath));
    }
    let n = run(&collection(0), &s).1;
    assert!(n[0].message().contains("x\\n\\u{202e}\\\"\\\\n"));
}
#[test]
fn refusal_preserves_baseline() {
    let mut s = state(FrontierState::default(), Stop::DepthReached);
    if let ContextState::Expanded(c) = &mut s {
        c.union = UnionState::Refused(UnionError::BlockIdOverflow);
        c.frontier = if let ContextState::Expanded(x) = resolved(vec![Disposition::Resolved]) {
            x.frontier
        } else {
            unreachable!()
        };
    }
    let c = collection(1);
    let (out, n) = run(&c, &s);
    assert_eq!(out, c);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].kind(), ContextRunNoteKind::UnionRefused);
    assert_eq!(n[0].message(),"taint-context: expanded results could not be combined with baseline (block ID overflow); baseline taint only");
}
