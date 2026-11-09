use super::*;
use crate::algorithms::taint::TaintReached;
use prism::{
    call_graph::{CallSiteOrigin, JsLocalBinding},
    data_flow::VarAccessKind,
    finding_confidence::EvidencePath,
    resolution::ResolutionConfidence,
};

#[test]
fn complete_population_and_none_memo_survive_duplicate_reversed_roots() {
    let f = js(&["send(req);"; 12].join("\n"));
    let ctx = f.context();
    let (_, mut trace) = f.trace(&ctx, &TaintConfig::default());
    assert_eq!(
        ctx.cpg
            .call_graph
            .calls
            .values()
            .flatten()
            .filter(|s| root_match(&ctx, &trace, s))
            .count(),
        12
    );
    let original = checked(&f, &ctx, &trace);
    trace.roots.extend(trace.roots.clone());
    trace.roots.reverse();
    // The captured caller parse, not later filesystem contents, owns matching.
    std::fs::write(f.dir.path().join("a.js"), "").unwrap();
    assert_eq!(original, checked(&f, &ctx, &trace));
    let mut observed = Vec::new();
    for emit in [false, true] {
        let mut calls = 0;
        let out = collect_with(
            &ctx,
            &trace,
            Ok(|_: &CallSite| {
                calls += 1;
                emit.then(hint)
            }),
        );
        observed.push((calls, out.records.len()));
    }
    let empty = collect_with(
        &ctx,
        &TaintTrace::default(),
        Ok(|_: &CallSite| panic!("no qualified sites")),
    );
    assert!(empty.records.is_empty());
    assert_eq!(
        (observed, original.records.len()),
        (vec![(12, 0), (12, 12)], 12)
    );
}

#[test]
fn typed_failure_and_hints_preserve_successful_result_and_evidence() {
    let f = js("const copy = req;\neval(copy);\nsend(copy);");
    let ctx = f.context();
    let (result, trace) = f.trace(&ctx, &TaintConfig::default());
    assert!(!result.findings.is_empty());
    assert!(result.evidence.iter().flatten().any(|e| !e.hops.is_empty()));
    let before = (
        serde_json::to_value(&result).unwrap(),
        format!("{:?}", result.evidence),
        format!("{trace:?}"),
    );
    let mut expected = vec![hint()];
    expected.extend(
        [
            ReviewUnresolvedReason::NoFile,
            ReviewUnresolvedReason::TooMany,
            ReviewUnresolvedReason::ReplaceUnproven,
        ]
        .map(|why| ReviewImportHint::Unresolvable {
            spec: "./lost".into(),
            why,
        }),
    );
    let observed: Vec<_> = expected
        .iter()
        .map(|h| {
            let out = collect_with(
                &ctx,
                &trace,
                Ok(|s: &CallSite| (s.callee_name == "send").then(|| h.clone())),
            );
            out.records.into_iter().map(|r| r.hint).collect::<Vec<_>>()
        })
        .collect();
    // Deliberately injected factory failure; core's captured-listing test covers its origin.
    let unavailable = collect_with(
        &ctx,
        &trace,
        Err::<fn(&CallSite) -> Option<ReviewImportHint>, _>("cannot list blocked: denied".into()),
    );
    assert_eq!(
        before,
        (
            serde_json::to_value(&result).unwrap(),
            format!("{:?}", result.evidence),
            format!("{trace:?}")
        )
    );
    assert_eq!(
        (
            observed,
            unavailable.availability,
            unavailable.records.len()
        ),
        (
            expected.into_iter().map(|h| vec![h]).collect::<Vec<_>>(),
            Availability::ListingUnavailable("cannot list blocked: denied".into()),
            0
        )
    );
}

#[test]
fn injected_cross_file_reach_keeps_destination_exact_when_source_proof_fails() {
    let f = js("send(req);");
    let ctx = f.context();
    let (_, trace) = f.trace(&ctx, &TaintConfig::default());
    let call = site(&ctx, "send");
    let recovery = RecoveryContext::new(&ctx.files["a.js"]);
    let actual = trace
        .roots
        .iter()
        .find(|r| recovery.matches_argument(&call, &r.location))
        .unwrap();
    let mut counts = Vec::new();
    for source in ["missing.js", "a.js"] {
        let mut root = actual.clone();
        root.seed.file = "invalid-source.js".into();
        root.location.file = source.into();
        root.location.kind = VarAccessKind::Def;
        root.reached = vec![TaintReached {
            location: actual.location.clone(),
            evidence: EvidencePath::default(),
        }];
        counts.push(
            checked(&f, &ctx, &TaintTrace { roots: vec![root] })
                .records
                .len(),
        );
    }
    assert_eq!(counts, [1, 1]);
}

#[test]
fn real_nameonly_full_presence_suppresses_even_when_wrapper_is_empty() {
    let source = "package main\ntype Command struct { Run func(string) }\nfunc h1(x string) {}\nfunc h2(x string) {}\nfunc a() *Command { return &Command{Run: h1} }\nfunc b() *Command { return &Command{Run: h2} }\nfunc invoke(cmd *Command, x string) {\n cmd.Run(x)\n}\n";
    let f = Fixture::new(&[("main.go", source)], &["main.go"], &[("main.go", 8)]);
    let ctx = f.context();
    let (_, trace) = f.trace(&ctx, &TaintConfig::default());
    let call = site(&ctx, "Run");
    assert!(root_match(&ctx, &trace, &call));
    let full = ctx.cpg.call_graph.resolve_call_site_full(&call).resolved;
    assert_eq!(full.len(), 2);
    assert!(full
        .iter()
        .all(|r| r.confidence == ResolutionConfidence::NameOnly));
    assert!(ctx.cpg.call_graph.resolve_call_site(&call).is_empty());
    let mut count = 0;
    let out = collect_with(
        &ctx,
        &trace,
        Ok(|_: &CallSite| {
            count += 1;
            Some(hint())
        }),
    );
    assert_eq!((count, out.records.len()), (0, 0));
}

#[test]
fn controlled_alias_groups_use_all_records_and_agreeing_ordinary_tuples() {
    let f = js("send(req); send(req);");
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    let mut admissible = Vec::new();
    for mode in [
        "loaded",
        "unloaded",
        "synthetic",
        "qualifier",
        "callee",
        "line",
        "metadata",
    ] {
        for alias in ["aa", "zz"] {
            for emit in [false, true] {
                let mut ctx = f.context();
                let (_, trace) = f.trace(&ctx, &TaintConfig::default());
                let ordinary = site(&ctx, "send");
                let mut other = ordinary.clone();
                match mode {
                    "loaded" | "unloaded" | "synthetic" => {
                        other.origin = CallSiteOrigin::IndirectResolution;
                        other.callee_name = alias.into();
                        other.local_binding = JsLocalBinding::Unchecked;
                        if mode == "loaded" {
                            other.pre_resolved_target =
                                Some(ctx.cpg.call_graph.functions["loaded"][0].clone());
                        }
                        if mode == "synthetic" {
                            other.source_callee_name = Some("send".into());
                        }
                    }
                    "qualifier" => other.qualifier = Some("different".into()),
                    "callee" => {
                        other.callee_name = alias.into();
                        other.local_binding = JsLocalBinding::Unchecked;
                    }
                    "line" => other.line += 1,
                    "metadata" => other.receiver_type = Some(alias.into()),
                    _ => unreachable!(),
                }
                let full_loaded = !ctx
                    .cpg
                    .call_graph
                    .resolve_call_site_full(&other)
                    .resolved
                    .is_empty();
                let other_matches = root_match(&ctx, &trace, &other);
                let sites = ctx.cpg.call_graph.calls.get_mut(&ordinary.caller).unwrap();
                if mode == "synthetic" {
                    sites.remove(&ordinary);
                }
                sites.insert(other);
                let group: Vec<_> = sites
                    .iter()
                    .filter(|s| s.start_byte == ordinary.start_byte)
                    .collect();
                let order_correct = !matches!(mode, "loaded" | "unloaded")
                    || (group[0].origin == CallSiteOrigin::IndirectResolution) == (alias == "aa");
                admissible.push(
                    order_correct
                        && (mode != "loaded" || (full_loaded && !other_matches))
                        && (mode != "synthetic" || other_matches),
                );
                let mut first_calls = 0;
                let mut total = 0;
                let mut source_ok = true;
                let out = collect_with(
                    &ctx,
                    &trace,
                    Ok(|s: &CallSite| {
                        total += 1;
                        first_calls += usize::from(s.start_byte == ordinary.start_byte);
                        source_ok &= s.origin == CallSiteOrigin::Source
                            && s.source_callee_name.is_none()
                            && s.callee_name == "send"
                            && s.qualifier.is_none();
                        emit.then(hint)
                    }),
                );
                let wanted = usize::from(matches!(mode, "unloaded" | "metadata"));
                eprintln!("ALIAS {mode} {alias} emit={emit} preconditions={} first={first_calls} total={total}", admissible.last().unwrap());
                observed.push((first_calls, total, out.records.len(), source_ok));
                expected.push((wanted, wanted + 1, if emit { wanted + 1 } else { 0 }, true));
            }
        }
    }
    assert!(
        admissible.iter().all(|v| *v),
        "inadmissible alias fixture: {admissible:?}"
    );
    assert_eq!(observed, expected);
}
