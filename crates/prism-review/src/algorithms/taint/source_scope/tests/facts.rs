//! RS09, the RS10 consumer combination and RS14 (#602 S3a.1s).
//!
//! RS09 and RS10 are CONTROLLED: files, roots, detector facts and sinks are parsed by the
//! real producer, but ONE cross-file NameOnly DataFlow edge is injected, so they prove
//! scoping and projection over a given graph, not parser reach. RS14 is REAL.
use super::cases::*;
use super::*;
use crate::algorithms::taint::run::slice_scoped_impl;
use crate::algorithms::taint::slice;
use crate::algorithms::taint::source_scope::admission::{
    slice_scoped, RunScope, ScopedTaintOutcome,
};

fn with_extra(case: &RouteCase, extra: &[&str]) -> RouteCase {
    let extra_sinks = extra.iter().map(|s| s.to_string()).collect();
    let config = TaintConfig {
        extra_sinks,
        ..case.config.clone()
    };
    RouteCase {
        config,
        ..case.clone()
    }
}

/// CONTROLLED run: one NameOnly edge from the entry file's Def into the context file's
/// source-line Def. Every later flow (def to sink use) is the parsed producer's own edge.
fn run_wired(
    case: &RouteCase,
    entry: (&'static str, usize, &'static str),
    context: (usize, &'static str),
    original: &BTreeSet<String>,
) -> ScopedTaintOutcome {
    let target = case.context_path();
    case.run(original, |ctx| {
        let (file, line, base) = entry;
        let from = unique_var_node(ctx, file, line, base, VarAccessKind::Def);
        let to = unique_var_node(ctx, target, context.0, context.1, VarAccessKind::Def);
        add_flow(ctx, from, to, FlowConfidence::NameOnly(FlowDoubt::SameLine));
    })
}

/// The context copy has detector facts (a Framework root) on `source_line` and, with every
/// file original, reports `sink_line`: otherwise the scoped assertions below prove nothing.
fn assert_context_facts(
    all: &ScopedTaintOutcome,
    case: &RouteCase,
    source_line: usize,
    sink: usize,
) {
    let context = case.context_path();
    let fact = |r: &&TaintRoot| r.origin == Framework && r.seed.line == source_line;
    let msg = "inadmissible precondition: no detector fact on the context source line";
    assert!(roots_of(all, context).iter().any(fact), "{msg}");
    let msg = "inadmissible precondition: all-original run misses the context sink";
    assert!(sink_lines(&all.result, context).contains(&sink), "{msg}");
}

#[test]
fn source_scope_keeps_full_source_recognition_and_global_sinks_go() {
    let case = route("facts_go");
    let text = case.files[1].1;
    let (source, sink) = (line_of(text, "c.Query"), line_of(text, "c.File(q)"));
    let entry = ("a.go", 4, "p");
    let all = run_wired(&case, entry, (source, "q"), &case.loaded_set());
    assert_context_facts(&all, &case, source, sink);

    let original = originals(&case.original);
    let outcome = run_wired(&case, entry, (source, "q"), &original);
    let context = case.context_path();
    assert!(roots_of(&outcome, context).is_empty(), "no context root");
    let reported = sink_lines(&outcome.result, context);
    assert!(
        reported.contains(&sink),
        "the global sink is reached from an original root"
    );

    // Source-set suppression: `Query` is a name sink only through `extra_sinks`; the full
    // framework fact list keeps the context source line out of it.
    let named = with_extra(&case, &["=Query"]);
    let reported = sink_lines(
        &run_wired(&named, entry, (source, "q"), &original).result,
        context,
    );
    assert!(
        !reported.contains(&source),
        "context source line reported as a flat sink"
    );
    assert!(reported.contains(&sink));

    // Capability control: the same reached line with no recognised source does fire.
    let plain = with_extra(&route("plain_go"), &["=Query"]);
    let plain_source = line_of(plain.files[1].1, "c.Query");
    let control = run_wired(
        &plain,
        entry,
        (plain_source, "q"),
        &originals(&plain.original),
    );
    let msg = "inadmissible precondition: an unrecognised reached Query line must fire";
    assert!(
        sink_lines(&control.result, context).contains(&plain_source),
        "{msg}"
    );
}

#[test]
fn source_scope_keeps_full_source_recognition_and_global_sinks_js() {
    let case = route("facts_js");
    let text = case.files[1].1;
    let (source, sink) = (line_of(text, "const Query"), line_of(text, "eval(Query)"));
    let entry = ("a.js", 2, "p");
    let all = run_wired(&case, entry, (source, "Query"), &case.loaded_set());
    assert_context_facts(&all, &case, source, sink);

    let original = originals(&case.original);
    let outcome = run_wired(&case, entry, (source, "Query"), &original);
    let context = case.context_path();
    assert!(roots_of(&outcome, context).is_empty(), "no context root");
    assert!(
        sink_lines(&outcome.result, context).contains(&sink),
        "later eval is reported"
    );

    // Target-range suppression: `Query` is the seed's target. The `=exec` row is a masked
    // control, not a second kill seam: `exec` is a bare property of the request access, and
    // the later request-source check refuses it even when the access-range map is empty, so
    // this row does not discriminate access-range suppression on its own.
    for name in ["=Query", "=exec"] {
        let named = with_extra(&case, &[name]);
        let run = run_wired(&named, entry, (source, "Query"), &original);
        let reported = sink_lines(&run.result, context);
        assert!(
            !reported.contains(&source),
            "{name}: context source line reported"
        );
    }

    // Capability control: without the express facts the same line fires for both names.
    let plain = route("plain_js");
    let plain_source = line_of(plain.files[1].1, "const Query");
    for name in ["=Query", "=exec"] {
        let named = with_extra(&plain, &[name]);
        let run = run_wired(
            &named,
            entry,
            (plain_source, "Query"),
            &originals(&plain.original),
        );
        let msg = format!("inadmissible precondition: unrecognised line must fire for {name}");
        assert!(
            sink_lines(&run.result, context).contains(&plain_source),
            "{msg}"
        );
    }
}

/// RS10: the consumer combination the projection tests cannot build in one file. The
/// framework root sorts first, the selected sink witness is framework-origin, and the
/// eligible Diff frontier (a NameOnly edge into the same sink) stays in `outcome.loading`.
#[test]
fn source_scope_loading_keeps_diff_frontier_behind_framework_witness() {
    let case = route("consumer_nameonly");
    let source = line_of(case.source(), "c.Query");
    let sink = line_of(case.source(), "c.File(q)");
    let outcome = case.run(&originals(&case.original), |ctx| {
        let from = unique_var_node(ctx, "b.go", 4, "p", VarAccessKind::Def);
        let to = unique_var_node(ctx, "a.go", sink, "q", VarAccessKind::Use);
        add_flow(ctx, from, to, FlowConfidence::NameOnly(FlowDoubt::SameLine));
    });
    let roots = &outcome.trace.roots;

    let first = roots.first().expect("inadmissible precondition: no roots");
    let msg = "inadmissible precondition: framework root must sort first";
    assert!(
        first.origin == Framework && first.seed.file == "a.go",
        "{msg}: {first:?}"
    );
    let (index, _) = (outcome.result.findings.iter().enumerate())
        .find(|(_, f)| {
            f.category.as_deref() == Some("taint_sink") && f.file == "a.go" && f.line == sink
        })
        .expect("inadmissible precondition: no sink finding on the framework file");
    let hops = &outcome.result.evidence[index]
        .as_ref()
        .expect("aligned evidence")
        .hops;
    let starts = matches!(hops.first(), Some(EvidenceHop::DataFlow { from, .. }) if from.file == "a.go" && from.line == source);
    let msg = "inadmissible precondition: selected witness is not the framework root's path";
    assert!(
        starts && hops.iter().all(EvidenceHop::is_exact),
        "{msg}: {hops:?}"
    );
    let weak = |e: &EvidencePath| !e.hops.is_empty() && e.hops.iter().all(|h| !h.is_exact());
    let frontier = |r: &&TaintRoot| {
        let into_sink = |t: &&TaintReached| t.location.file == "a.go" && t.location.line == sink;
        r.origin == Diff
            && r.seed.file == "b.go"
            && r.reached
                .iter()
                .filter(into_sink)
                .any(|t| weak(&t.evidence))
    };
    let msg = "inadmissible precondition: no Diff root reaching the sink over a NameOnly edge";
    assert!(roots.iter().any(|r| frontier(&r)), "{msg}");

    let eligible: Vec<_> = roots
        .iter()
        .filter(|r| matches!(r.origin, Diff | Explicit))
        .collect();
    assert_eq!(
        snaps(eligible),
        snaps(&outcome.loading.roots),
        "whole eligible roots, in order"
    );
    assert!(
        outcome.loading.roots.iter().any(|r| frontier(&r)),
        "the weak frontier survives"
    );
    assert!(outcome.loading.roots.iter().all(|r| r.origin != Framework));
    assert!(outcome.excluded_explicit.is_empty());
}

fn same(a: &SliceResult, b: &SliceResult) {
    assert_eq!(
        serde_json::to_vec(a).unwrap(),
        serde_json::to_vec(b).unwrap()
    );
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
}

#[test]
fn source_scope_preserves_absent_and_all_original_results() {
    for (name, explicit_line, positive) in [
        ("py_positive", 3, true),
        ("py_negative", 3, false),
        ("express_targeted", 4, true),
    ] {
        for (return_flow, explicit) in [(false, false), (false, true), (true, false), (true, true)]
        {
            let mut case = route(name);
            case.config.return_flow = return_flow;
            if explicit {
                case.config.sources = vec![(case.original_path().to_string(), explicit_line)];
            }
            let label = format!("{name} return_flow={return_flow} explicit={explicit}");
            let files = parse(&case.files);
            let ctx = CpgContext::build(&files, None);
            let (input, config) = (case.input(), &case.config);

            // (a) absent scope: public entries versus the internal path.
            let legacy = slice(&ctx, &input, config).unwrap();
            let (traced, trace) = slice_with_trace(&ctx, &input, config).unwrap();
            let none = slice_scoped_impl(&ctx, &input, config, None, RunScope::default()).unwrap();
            let mut internal_trace = TaintTrace::default();
            let default = RunScope::default();
            let internal =
                slice_scoped_impl(&ctx, &input, config, Some(&mut internal_trace), default);
            let internal = internal.unwrap();
            same(&legacy, &none);
            same(&traced, &internal);
            same(&legacy, &traced);
            assert_eq!(
                format!("{trace:?}"),
                format!("{internal_trace:?}"),
                "{label}"
            );
            let found =
                (traced.findings.iter()).any(|f| f.category.as_deref() == Some("taint_sink"));
            assert_eq!(found, positive, "inadmissible precondition: {label}");

            // (b) every loaded file original: same oracle, nothing excluded.
            let outcome = slice_scoped(&ctx, &input, config, &case.loaded_set()).unwrap();
            same(&outcome.result, &traced);
            assert_eq!(
                format!("{:?}", outcome.trace),
                format!("{trace:?}"),
                "{label}"
            );
            assert!(outcome.excluded_explicit.is_empty(), "{label}");

            // (c) composition: the independently written expectation of `loading`.
            let eligible =
                (outcome.trace.roots.iter()).filter(|r| matches!(r.origin, Diff | Explicit));
            let expected = format!("{:?}", eligible.collect::<Vec<_>>());
            assert_eq!(
                format!("{:?}", outcome.loading.roots.iter().collect::<Vec<_>>()),
                expected
            );
            assert!(
                !outcome.loading.roots.is_empty(),
                "inadmissible precondition: {label}"
            );
        }
    }
}
