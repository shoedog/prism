use super::*;

#[test]
fn n02_expansion_adds_a_context_finding_with_its_maps() {
    let f = Fixture::new(B);
    let (baseline, _) = f.control();
    let before_sources = f.inputs.sources.clone();
    let off = f.review(&[SlicingAlgorithm::Taint], None);
    let on = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    let c = context(&on.context);
    assert!(matches!(c.union, UnionState::Applied { .. }));
    assert!(c.context_files.contains("b.js"));
    assert!(c.files.contains_key("b.js") && c.sources.contains_key("b.js"));
    assert!(!c.parse_quality.contains_key("b.js")); // Clean context is sparse, not Unknown.
    assert!(c
        .parse_quality
        .keys()
        .all(|file| c.files.contains_key(file)));
    assert_eq!(c.published.files, c.context_files.len());
    assert!(c.round > 0 && c.stats.rounds_completed > 0);
    assert!(c.union_stats().unwrap().additional_sinks > 0);
    assert!(!c.added_block_ids().is_empty());
    let result = &on.results[0];
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.file == "b.js"
                && finding.category.as_deref() == Some("taint_sink"))
    );
    assert!(!baseline.findings.is_empty());
    let n = off.results[0].findings.len();
    assert_eq!(
        bytes(&result.findings[..n]),
        bytes(&off.results[0].findings)
    );
    assert_eq!(
        format!("{:?}", &result.evidence[..n]),
        format!("{:?}", off.results[0].evidence)
    );
    assert_eq!(
        bytes(&result.blocks[..off.results[0].blocks.len()]),
        bytes(&off.results[0].blocks)
    );
    assert_eq!(f.inputs.sources, before_sources);
    assert!(!f.inputs.files.contains_key("b.js"));
}

#[test]
fn n06_captured_root_wins_over_the_repo_argument() {
    let f = Fixture::new(B);
    f.control();
    let other = tempfile::tempdir().unwrap();
    std::fs::write(
        other.path().join("b.js"),
        "export function g(x) { eval(x); eval(x); }\n",
    )
    .unwrap();
    let (actual, state) = f.single(Some(&request(64)), other.path());
    assert_eq!(context(&state).sources["b.js"], B);
    let (expected, _) = f.single(Some(&request(64)), f.dir.path());
    same_result(&actual.result, &expected.result);
}

#[test]
fn n08_context_edit_is_seen_on_a_round_zero_cache_hit() {
    let mut f = Fixture::new(B);
    let cache = tempfile::tempdir().unwrap();
    f.options.no_cache = false;
    f.options.cache_dir = Some(cache.path().into());
    f.review(&[SlicingAlgorithm::Taint], None); // Populate only the original CPG cache.
    let warm = cache_files(cache.path());
    assert!(
        !warm.is_empty(),
        "inadmissible precondition: original cache persisted"
    );
    let first = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    assert_eq!(
        cache_files(cache.path()),
        warm,
        "round-zero cache hit leaves bytes unchanged"
    );
    assert_eq!(context(&first.context).sources["b.js"], B);
    let changed = "export function g(x) {\n  eval(x);\n}\n";
    std::fs::write(f.dir.path().join("b.js"), changed).unwrap();
    let second = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    assert_eq!(cache_files(cache.path()), warm);
    assert_eq!(context(&second.context).sources["b.js"], changed);
    assert!(second.results[0]
        .findings
        .iter()
        .any(|f| f.file == "b.js" && f.line == 2));
    assert_ne!(bytes(&first.results[0]), bytes(&second.results[0]));
}

#[test]
fn n09_scoped_round_zero_does_not_scope_the_context() {
    let f = Fixture::new(B);
    f.control();
    let plain = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    let mut scoped = Fixture::new(B);
    scoped.options.scoped_cpg = true;
    let scoped = scoped.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    assert!(context(&plain.context).context_files.contains("b.js"));
    assert!(context(&scoped.context).context_files.contains("b.js"));
    same_result(&plain.results[0], &scoped.results[0]);
}

#[test]
fn n10_filtering_and_annotations_use_the_selected_maps() {
    let body = "export function g(x) { eval(x); }\nfunction unrelated() { const broken = ; }\n";
    let f = Fixture::new(body);
    f.control();
    let on = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    let c = context(&on.context);
    assert!(
        c.parse_quality.contains_key("b.js"),
        "prepared context retains degraded target parse quality"
    );
    let quality = c.parse_quality["b.js"].quality.clone();
    assert!(on
        .findings
        .iter()
        .any(|f| f.file == "b.js" && f.parse_quality.as_deref() == Some(quality.as_str())));
    assert!(on.results[0]
        .findings
        .iter()
        .filter(|f| f.file == "b.js")
        .all(|f| f.parse_quality.is_none()));
    // CONTROLLED evidence projection over REAL prepared maps; producer labels are not upgraded.
    use prism::call_graph::FunctionId;
    use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge};
    let finding = on.results[0]
        .findings
        .iter()
        .find(|f| f.file == "b.js")
        .unwrap()
        .clone();
    for (confidence, crossed, kept) in [
        (ResolutionConfidence::Exact, false, true),
        (ResolutionConfidence::NameOnly, false, false),
        (ResolutionConfidence::Exact, true, false),
    ] {
        let mut path = EvidencePath::from_hops(vec![EvidenceHop::Call {
            edge: ResolvedCallEdge {
                caller: FunctionId {
                    file: "b.js".into(),
                    name: "g".into(),
                    start_line: 1,
                    end_line: 1,
                },
                call_site_line: 1,
                confidence,
                kind: ResolutionKind::FreeSingle,
            },
            confidence,
        }]);
        path.crossed_unlabeled = crossed;
        let mut result = SliceResult::new(SlicingAlgorithm::Taint);
        result.findings.push(finding.clone());
        result.evidence.push(Some(path));
        let mut original = result.clone();
        crate::api::run::filter_result_findings_in(
            &mut result,
            on.context.view(&f.inputs, SlicingAlgorithm::Taint),
            MinConfidence::Exact,
            ResolutionMode::Scoped,
        );
        crate::api::run::filter_result_findings(
            &mut original,
            &f.inputs,
            MinConfidence::Exact,
            ResolutionMode::Scoped,
        );
        assert_eq!(result.findings.len(), usize::from(kept));
        assert_eq!(original.findings.len(), result.findings.len()); // Quality affects tier, not confidence floor.
    }
    assert!(!on
        .context
        .view(&f.inputs, SlicingAlgorithm::EchoSlice)
        .files
        .contains_key("b.js"));
}
