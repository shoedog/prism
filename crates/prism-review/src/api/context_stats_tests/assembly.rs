use super::*;

#[test]
fn x01_attempted_and_admitted_are_distinct() {
    let stats = ContextStats::from_context(&context());
    assert_eq!(
        (stats.attempted_files, stats.attempted_read_bytes),
        (5, 9000)
    );
    assert_eq!(
        (
            stats.admitted_files,
            stats.admitted_bytes,
            stats.admitted_depth
        ),
        (2, 3000, 2)
    );
    assert_eq!((stats.rounds_started, stats.rounds_completed), (3, 2));
}
#[test]
fn x02_rejected_sentinel_bytes_remain_attempted() {
    let mut c = context();
    c.attempts.attempted_read_bytes = 2_097_153;
    c.attempts.skips.push((
        "too-large.js".into(),
        SkipReason::TooLarge {
            observed_at_least: 2_097_153,
        },
    ));
    let s = ContextStats::from_context(&c);
    assert_eq!(s.attempted_read_bytes, 2_097_153);
    assert_eq!(s.admitted_bytes, 3000);
    assert_eq!(s.skips_too_large, 1);
}
#[test]
fn x03_discard_retains_attempts_prior_bundle_and_durations() {
    let mut c = context();
    c.stop = Stop::Guard(StopCause::Cancelled);
    c.stats.elapsed_read_parse_ns = 31;
    c.stats.elapsed_analysis_ns = 17;
    c.stats.elapsed_total_ns = 100;
    let s = ContextStats::from_context(&c);
    assert_eq!(
        (
            s.attempted_files,
            s.admitted_files,
            s.rounds_started,
            s.rounds_completed
        ),
        (5, 2, 3, 2)
    );
    assert_eq!(
        (
            s.elapsed_read_parse_ns,
            s.elapsed_analysis_ns,
            s.elapsed_total_ns
        ),
        (31, 17, 100)
    );
    assert_eq!((s.cancelled, s.bound_time), (1, 0));
}
#[test]
fn x04_all_skip_variants() {
    let mut c = context();
    let variants = vec![
        SkipReason::InvalidPath,
        SkipReason::NotListed,
        SkipReason::Unsupported,
        SkipReason::OriginalFile,
        SkipReason::UnsafePath,
        SkipReason::NotRegular,
        SkipReason::Unreadable {
            kind: std::io::ErrorKind::PermissionDenied,
            message: "denied".into(),
        },
        SkipReason::NotUtf8,
        SkipReason::TooLarge {
            observed_at_least: 7,
        },
        SkipReason::ParseFailed {
            message: "bad".into(),
        },
        SkipReason::SevereParse {
            error_count: 1,
            node_count: 4,
        },
        SkipReason::Unknown,
    ];
    c.attempts.skips = variants
        .into_iter()
        .enumerate()
        .map(|(i, r)| (format!("{i}"), r))
        .collect();
    c.attempts
        .skips
        .push(("repeat".into(), SkipReason::Unknown));
    let s = ContextStats::from_context(&c);
    assert_eq!(
        [
            s.skips_invalid_path,
            s.skips_not_listed,
            s.skips_unsupported,
            s.skips_original_file,
            s.skips_unsafe_path,
            s.skips_not_regular,
            s.skips_unreadable,
            s.skips_not_utf8,
            s.skips_too_large,
            s.skips_parse_failed,
            s.skips_severe_parse,
            s.skips_unknown
        ],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2]
    );
}
#[test]
fn x05_stop_flags_and_exhaustion() {
    let cases = [
        Stop::DepthReached,
        Stop::Bound(AdmissionBound::Files {
            first_skipped: 0,
            skipped: 2,
        }),
        Stop::Bound(AdmissionBound::SourceBytes {
            first_skipped: 0,
            skipped: 2,
        }),
        Stop::Guard(StopCause::Time),
        Stop::Guard(StopCause::Cancelled),
        Stop::StageFailed("failed".into()),
        Stop::FrontierExhausted,
    ];
    for (i, stop) in cases.into_iter().enumerate() {
        let mut c = context();
        c.stop = stop;
        let s = ContextStats::from_context(&c);
        let flags = [
            s.bound_depth,
            s.bound_files,
            s.bound_bytes,
            s.bound_time,
            s.cancelled,
            s.stage_failed,
        ];
        for (j, v) in flags.into_iter().enumerate() {
            assert_eq!(v, u64::from(i == j), "stop {i}, flag {j}");
        }
    }
}
#[test]
fn x06_resolved_wins_grouped_before_cap_even_on_refusal() {
    for reverse in [false, true] {
        let mut c = context();
        c.frontier.provenance = (0..14).map(provenance).collect();
        c.frontier.provenance.push(provenance(0));
        c.frontier.dispositions = (0..14)
            .map(|i| (i, "b.js".into(), Disposition::LoadedUnresolved))
            .collect();
        c.frontier
            .dispositions
            .push((14, "c.js".into(), Disposition::Resolved));
        if reverse {
            c.frontier.dispositions.reverse();
        }
        let s = ContextStats::from_context(&c);
        assert_eq!(
            (s.eligible_boundary_calls, s.loaded_unresolved_calls),
            (15, 13)
        );
        c.union = UnionState::Refused(UnionError::BlockIdOverflow);
        assert_eq!(ContextStats::from_context(&c).loaded_unresolved_calls, 13);
    }
}
#[test]
fn x07_union_availability_and_retained_facts() {
    let mut c = context();
    c.union = UnionState::Applied {
        stats: UnionStats {
            additional_sinks: 2,
            additional_sources: 3,
            dropped_sinks: 5,
            dropped_sources: 7,
        },
        added_block_ids: vec![],
    };
    let s = ContextStats::from_context(&c);
    assert_eq!(
        (
            s.union_stats_available,
            s.additional_sinks,
            s.additional_sources,
            s.perturbation_dropped_sinks,
            s.perturbation_dropped_sources
        ),
        (1, 2, 3, 5, 7)
    );
    c.union = UnionState::Applied {
        stats: UnionStats::default(),
        added_block_ids: vec![],
    };
    assert_eq!(
        (
            ContextStats::from_context(&c).union_stats_available,
            ContextStats::from_context(&c).additional_sinks
        ),
        (1, 0)
    );
    for state in [
        UnionState::NotNeeded,
        UnionState::Refused(UnionError::BlockIdOverflow),
    ] {
        c.union = state;
        let s = ContextStats::from_context(&c);
        assert_eq!(
            (
                s.union_stats_available,
                s.additional_sinks,
                s.additional_sources,
                s.perturbation_dropped_sinks,
                s.perturbation_dropped_sources
            ),
            (0, 0, 0, 0, 0)
        );
        assert_eq!(
            (
                s.attempted_files,
                s.admitted_files,
                s.driver_stats_available
            ),
            (5, 2, 1)
        );
    }
}

#[test]
fn x08_literal_name_table_is_complete_unique_and_zero_inclusive() {
    let names: BTreeSet<_> = ContextStats::default()
        .rows()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let expected = BTreeSet::from([
        "review.taint_context.rounds_started",
        "review.taint_context.rounds_completed",
        "review.taint_context.attempted_files",
        "review.taint_context.attempted_read_bytes",
        "review.taint_context.admitted_files",
        "review.taint_context.admitted_bytes",
        "review.taint_context.admitted_depth",
        "review.taint_context.eligible_boundary_calls",
        "review.taint_context.loaded_unresolved_calls",
        "review.taint_context.skips.invalid_path",
        "review.taint_context.skips.not_listed",
        "review.taint_context.skips.unsupported",
        "review.taint_context.skips.original_file",
        "review.taint_context.skips.unsafe_path",
        "review.taint_context.skips.not_regular",
        "review.taint_context.skips.unreadable",
        "review.taint_context.skips.not_utf8",
        "review.taint_context.skips.too_large",
        "review.taint_context.skips.parse_failed",
        "review.taint_context.skips.severe_parse",
        "review.taint_context.skips.unknown",
        "review.taint_context.bound_depth",
        "review.taint_context.bound_files",
        "review.taint_context.bound_bytes",
        "review.taint_context.bound_time",
        "review.taint_context.stage_failed",
        "review.taint_context.cancelled",
        "review.taint_context.additional_sinks",
        "review.taint_context.additional_sources",
        "review.taint_context.perturbation_dropped_sinks",
        "review.taint_context.perturbation_dropped_sources",
        "review.taint_context.elapsed_discovery_ns",
        "review.taint_context.elapsed_read_parse_ns",
        "review.taint_context.elapsed_build_ns",
        "review.taint_context.elapsed_analysis_ns",
        "review.taint_context.elapsed_total_ns",
        "review.taint_context.union_stats_available",
        "review.taint_context.driver_stats_available",
        "review.taint_context.context_unavailable",
    ]);
    assert_eq!(names.len(), 39);
    assert_eq!(names, expected);
    assert!(ContextStats::default().rows().iter().all(|(_, v)| *v == 0));
}
