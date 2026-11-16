//! E02-E07: what the scoped run records at each decision site (#602 S4d1 part E).
//!
//! Support, fixtures and the REAL / CONTROLLED labels are in the parent `tests.rs`. Every
//! "inadmissible precondition" is read from the run's public result (`finding_at`, evidence of a
//! finding) or from the fixture, never from the sidecar under test, so a sidecar that records
//! nothing fails a behavioral assertion and cannot be mistaken for a fixture failure.
use super::*;

fn all_fixtures() -> Vec<Fixture> {
    let mut all = fixtures();
    all.push(bash_script(SCRIPT_SH));
    all.push(weights());
    all
}

fn sink_style(category: Option<&str>) -> bool {
    matches!(category, Some("taint_sink" | "unquoted_expansion"))
}

fn positions(result: &SliceResult, keep: impl Fn(Option<&str>) -> bool) -> Vec<usize> {
    let found = result.findings.iter().enumerate();
    let found = found.filter(|(_, f)| keep(f.category.as_deref()));
    found.map(|(index, _)| index).collect()
}

fn debug(evidence: &EvidencePath) -> String {
    format!("{evidence:?}")
}

/// The source part of the description, rendered from the sidecar's own selected source.
fn rendered(sink: &SinkEmission) -> String {
    match &sink.selected_source {
        Some((file, line)) if *file == sink.file => format!("line {line}"),
        Some((file, line)) => format!("{file}:{line}"),
        None => "diff lines".to_string(),
    }
}

/// E02 REAL (the `weights` row is CONTROLLED). Every emitted finding is indexed, once, at the
/// position it holds; every emitted source is licensed by at least one sink.
#[test]
fn every_emitted_finding_is_indexed() {
    let mut sources = 0;
    for fixture in all_fixtures() {
        let name = fixture.name;
        let run = fixture.run(CrossingCeiling::NameOnly);
        let (result, sidecar) = (&run.result, &run.emission);
        let expected = positions(result, sink_style);
        let msg = format!("inadmissible precondition: {name} emitted no sink-style finding");
        assert!(!expected.is_empty(), "{msg}");
        let emitted = positions(result, |category| category == Some("taint_source"));
        for sink in &sidecar.sinks {
            let finding = result.findings.get(sink.finding_index);
            let finding = finding
                .unwrap_or_else(|| panic!("{name}: index {} out of range", sink.finding_index));
            let got = (
                finding.category.as_deref(),
                finding.file.as_str(),
                finding.line,
            );
            let want = (Some(sink.category.as_str()), sink.file.as_str(), sink.line);
            assert_eq!(got, want, "{name}: sink emission points at another finding");
        }
        let mut indexed: Vec<usize> = sidecar.sinks.iter().map(|s| s.finding_index).collect();
        indexed.sort_unstable();
        assert_eq!(indexed, expected, "{name}: every sink-style finding, once");

        let mut recorded: Vec<usize> = sidecar.sources.iter().map(|s| s.finding_index).collect();
        recorded.sort_unstable();
        assert_eq!(
            recorded, emitted,
            "{name}: every taint_source finding, once"
        );
        for source in &sidecar.sources {
            let finding = &result.findings[source.finding_index];
            let got = (finding.file.as_str(), finding.line);
            assert_eq!(
                got,
                (source.file.as_str(), source.line),
                "{name}: source finding"
            );
            let names = |l: &LicensedSource| (l.file.as_str(), l.line) == got;
            let licensing = sidecar.sinks.iter().any(|s| s.licensed.iter().any(names));
            assert!(
                licensing,
                "{name}: {}:{} is licensed by no sink",
                source.file, source.line
            );
        }
        sources += emitted.len();
    }
    let msg = "inadmissible precondition: no fixture emitted a taint_source finding";
    assert!(sources > 0, "{msg}");
}

/// E03 CONTROLLED (the `weights` graph: one injected NameOnly edge, the def-use edge forced
/// Exact). S reaches the context sink C by NameOnly and its own sink D by Exact.
#[test]
fn per_sink_evidence_survives_a_better_global_witness() {
    let run = weights().run(CrossingCeiling::NameOnly);
    let result = &run.result;
    let d_at = finding_at(result, "taint_sink", "app.py", 4);
    let c_at = finding_at(result, "taint_sink", "ctx/lib.py", 3);
    let s_at = finding_at(result, "taint_source", "app.py", 3);
    let msg = "inadmissible precondition: the result's evidence for S is not the Exact one";
    let global = evidence_at(result, s_at);
    assert!(is_exact(global), "{msg}: {global:?}");

    let sidecar = &run.emission;
    let (d, c) = (
        sink_at(sidecar, "app.py", 4),
        sink_at(sidecar, "ctx/lib.py", 3),
    );
    assert_eq!((d.finding_index, c.finding_index), (d_at, c_at));
    let (to_d, to_c) = (license_of(d, "app.py", 3), license_of(c, "app.py", 3));
    assert!(is_exact(&to_d.evidence), "S->D: {:?}", to_d.evidence);
    assert!(is_name_only(&to_c.evidence), "S->C: {:?}", to_c.evidence);
    assert_eq!((d.licensed.len(), c.licensed.len()), (1, 1));
    assert_eq!(
        (to_d.route, to_c.route),
        (LicenseRoute::Path, LicenseRoute::Path)
    );

    // Negative: the Exact ceiling refuses the NameOnly entry into the context file. C is gone,
    // and S keeps D's Exact relation: the refusal does not touch the original-file pair.
    let strict = weights().run(CrossingCeiling::Exact);
    finding_at(&strict.result, "taint_sink", "app.py", 4); // fixture precondition
    let msg = "inadmissible precondition: the Exact ceiling still emits the context sink";
    let kept = strict
        .result
        .findings
        .iter()
        .any(|f| f.file == "ctx/lib.py");
    assert!(!kept, "{msg}");
    let d = sink_at(&strict.emission, "app.py", 4);
    assert!(is_exact(&license_of(d, "app.py", 3).evidence));
    let context = strict
        .emission
        .sinks
        .iter()
        .filter(|s| s.file == "ctx/lib.py");
    assert_eq!(context.count(), 0, "a context sink was recorded");
}

/// E04 REAL positive. `c.File(c.Param("f"))`: source and sink share a line, no flow path links
/// them, so the nearest-source fallback licenses the source with unlabeled evidence. Negatives
/// use CONTROLLED graphs (`weights` and `inline_yaml_without_paths`).
///
/// The packet's negative "a fallback source outside `taint_sources` records nothing" has no
/// reachable input: every fallback candidate is drawn from `ipc_source_set` or `taint_sources`, and
/// the IPC lines are seeded into `taint_sources` first. The reachable negatives follow instead.
#[test]
fn fallback_license_is_recorded_unlabeled() {
    let run = gin_handler().run(CrossingCeiling::NameOnly);
    let at = finding_at(&run.result, "taint_sink", "main.go", 6);
    let frontier = run.trace.roots.iter().flat_map(|r| r.reached.iter());
    let reached = frontier.filter(|t| t.location.file == "main.go" && t.location.line == 6);
    let msg = "inadmissible precondition: a flow path reaches the source=sink line";
    assert_eq!(reached.count(), 0, "{msg}");
    finding_at(&run.result, "taint_source", "main.go", 6); // fixture precondition
    let sink = sink_at(&run.emission, "main.go", 6);
    assert_eq!(sink.finding_index, at);
    assert_eq!(sink.selected_source, Some(("main.go".to_string(), 6)));
    assert_eq!(sink.licensed.len(), 1);
    let licensed = license_of(sink, "main.go", 6);
    assert_eq!(licensed.route, LicenseRoute::Fallback);
    assert!(is_unlabeled(&licensed.evidence), "{:?}", licensed.evidence);
    let emitted =
        (run.emission.sources.iter()).any(|s| (s.file.as_str(), s.line) == ("main.go", 6));
    assert!(emitted, "the fallback-licensed source is emitted");

    // Negative: sinks whose source came from a flow path record no Fallback entry.
    let weighted = weights().run(CrossingCeiling::NameOnly);
    let sinks = weighted.emission.sinks.iter();
    let routes: Vec<_> = sinks.flat_map(|s| &s.licensed).collect();
    assert!(
        !routes.is_empty(),
        "the path-derived sinks licensed nothing"
    );
    assert!(routes.iter().all(|l| l.route == LicenseRoute::Path));
    // CONTROLLED negative: targeted roots cannot reach the independently discovered inline sink,
    // and there are no targetless fallback seeds. It licenses nothing and selects nothing.
    let js = inline_yaml_without_paths().run(CrossingCeiling::NameOnly);
    let at = finding_at(&js.result, "taint_sink", "app.js", 6);
    assert!(
        js.trace.roots.iter().all(|root| root.reached.is_empty()),
        "inadmissible precondition: the controlled graph has a reached target"
    );
    let msg = "inadmissible precondition: the JS sink names a source";
    assert!(
        js.result.findings[at]
            .description
            .contains("from diff lines"),
        "{msg}"
    );
    let sink = sink_at(&js.emission, "app.js", 6);
    assert!(sink.selected_source.is_none() && sink.licensed.is_empty());
    assert!(js.emission.sources.is_empty());
}

/// E05 REAL. The bash `cat $file`: the unquoted expansion is a sink-style finding that licenses
/// the nearest preceding same-file source (the diff line). Its description names no source.
#[test]
fn unquoted_expansion_is_attributed() {
    let run = bash_script(SCRIPT_SH).run(CrossingCeiling::NameOnly);
    let at = finding_at(&run.result, "unquoted_expansion", "script.sh", 6);
    let source_at = finding_at(&run.result, "taint_source", "script.sh", 4);
    let sink = sink_at(&run.emission, "script.sh", 6);
    assert_eq!(sink.category, "unquoted_expansion");
    assert_eq!(sink.finding_index, at);
    assert!(
        sink.selected_source.is_none(),
        "its description names no source"
    );
    assert_eq!(sink.licensed.len(), 1);
    let licensed = license_of(sink, "script.sh", 4);
    assert_eq!(licensed.route, LicenseRoute::Unquoted);
    // The finding's evidence and the license come from the same pair lookup and fallback.
    let own = evidence_at(&run.result, at);
    assert_eq!(debug(&licensed.evidence), debug(own));
    let source =
        (run.emission.sources.iter()).find(|s| (s.file.as_str(), s.line) == ("script.sh", 4));
    let source = source.unwrap_or_else(|| panic!("the licensed source is not recorded"));
    assert_eq!(source.finding_index, source_at);
    assert!(
        source.finding_index > sink.finding_index,
        "sources follow sinks"
    );

    // Negative: a quoted expansion is no sink-style finding, so nothing is recorded.
    let quoted = bash_script(QUOTED_SH).run(CrossingCeiling::NameOnly);
    let msg = "inadmissible precondition: the quoted script emitted a sink-style finding";
    assert!(positions(&quoted.result, sink_style).is_empty(), "{msg}");
    assert!(quoted.emission.sinks.is_empty());
    assert!(quoted.emission.sources.is_empty());
}

/// E06 REAL for gin, CONTROLLED for `weights` and the source-free inline YAML row. The selection is
/// the one the run printed: `line N` for a same-file source, `file:N` for a cross-file one,
/// `diff lines` when none was selected.
#[test]
fn selected_source_matches_the_description() {
    let (mut same, mut cross, mut none) = (0, 0, 0);
    for fixture in [weights(), gin_handler(), inline_yaml_without_paths()] {
        let run = fixture.run(CrossingCeiling::NameOnly);
        let sinks = run.result.findings.iter();
        let sinks = sinks.filter(|f| f.category.as_deref() == Some("taint_sink"));
        let msg = format!(
            "inadmissible precondition: {} emitted no taint_sink",
            fixture.name
        );
        assert!(sinks.count() > 0, "{msg}");
        for sink in run
            .emission
            .sinks
            .iter()
            .filter(|s| s.category == "taint_sink")
        {
            let finding = &run.result.findings[sink.finding_index];
            let want = format!(
                "tainted value from {} reaches sink at line {}",
                rendered(sink),
                sink.line
            );
            assert_eq!(finding.description, want, "{}: {}", fixture.name, sink.file);
            match &sink.selected_source {
                Some((file, _)) if *file == sink.file => same += 1,
                Some(_) => cross += 1,
                None => none += 1,
            }
        }
    }
    let msg = format!("rows recorded: same={same} cross={cross} none={none}");
    assert!(same > 0 && cross > 0 && none > 0, "{msg}");
    let run = weights().run(CrossingCeiling::NameOnly);
    let c = sink_at(&run.emission, "ctx/lib.py", 3);
    assert_eq!(c.selected_source, Some(("app.py".to_string(), 3)));
}

/// E07. For a sink whose finding evidence came from the selected pair, the licensed value is
/// that evidence. `weights` and `through_def` are CONTROLLED. The latter checks a two-hop value
/// against the independent producer: FlowPath pairs encode transitive reachability, not just
/// direct graph edges. REAL negative: Django's synthesized targeted pair has no producer evidence
/// entry and stays unlabeled. Neither path length nor absence of a direct edge implies unlabeled.
#[test]
fn sidecar_evidence_is_the_runs_own() {
    let run = weights().run(CrossingCeiling::NameOnly);
    for (file, line) in [("app.py", 4), ("ctx/lib.py", 3)] {
        let at = finding_at(&run.result, "taint_sink", file, line);
        let msg = "inadmissible precondition: the finding evidence is not from a labeled pair";
        assert!(
            !evidence_at(&run.result, at).hops.is_empty(),
            "{msg}: {file}:{line}"
        );
    }
    let mut compared = 0;
    for sink in &run.emission.sinks {
        let Some((file, line)) = &sink.selected_source else {
            continue;
        };
        let licensed = license_of(sink, file, *line);
        let own = evidence_at(&run.result, sink.finding_index);
        assert_eq!(
            debug(&licensed.evidence),
            debug(own),
            "{}:{}",
            sink.file,
            sink.line
        );
        compared += 1;
    }
    assert_eq!(compared, 2, "both sinks name a selected source");

    let fixture = through_def();
    let produced = fixture.with_ctx(|ctx| {
        let context = fixture.context_set();
        let policy = TaintCrossingPolicy::new(&context, CrossingCeiling::NameOnly);
        let (_, evidence) = ctx
            .cpg
            .taint_forward_cfg_labeled_with_policy(&[("app.py".to_string(), 2)], &policy);
        let from = unique_var_node(ctx, "app.py", 2, "p", VarAccessKind::Def);
        let to = unique_var_node(ctx, "ctx/lib.py", 3, "cmd", VarAccessKind::Use);
        let pair = (
            ctx.cpg.to_var_location(from).unwrap(),
            ctx.cpg.to_var_location(to).unwrap(),
        );
        evidence
            .get(&pair)
            .expect("inadmissible precondition: producer has the selected transitive pair")
            .clone()
    });
    assert_eq!(
        produced.hops.len(),
        2,
        "inadmissible precondition: two hops"
    );
    assert!(
        matches!(&produced.hops[0], EvidenceHop::DataFlow { from, to, confidence }
            if from.file == "app.py" && from.line == 2
                && to.file == "ctx/lib.py" && to.line == 2 && *confidence == WEAK),
        "inadmissible precondition: first hop is the injected edge: {produced:?}"
    );
    let run = fixture.run(CrossingCeiling::NameOnly);
    let at = finding_at(&run.result, "taint_sink", "ctx/lib.py", 3);
    let own = evidence_at(&run.result, at);
    assert_eq!(
        debug(own),
        debug(&produced),
        "inadmissible precondition: the public sink uses the producer pair"
    );
    let sink = sink_at(&run.emission, "ctx/lib.py", 3);
    let licensed = license_of(sink, "app.py", 2);
    assert_eq!(licensed.route, LicenseRoute::Path);
    assert_eq!(debug(&licensed.evidence), debug(&produced));

    // Negative: target synthesis supplies a pair but no producer evidence; preserve unlabeled.
    let run = django_view().run(CrossingCeiling::NameOnly);
    let at = finding_at(&run.result, "taint_sink", "views.py", 4);
    let own = evidence_at(&run.result, at);
    assert!(
        is_unlabeled(own),
        "inadmissible precondition: synthetic Django pair is unlabeled: {own:?}"
    );
    let sink = sink_at(&run.emission, "views.py", 4);
    let licensed = license_of(sink, "views.py", 3);
    assert_eq!(licensed.route, LicenseRoute::Path);
    assert!(is_unlabeled(&licensed.evidence), "{:?}", licensed.evidence);
    assert_eq!(debug(&licensed.evidence), debug(own));
}
