//! SY01-SY05: the crossing ceiling on every route of the combined entry (#602 S3b-R).
//!
//! The combined entry (`slice_in_context`) takes an explicit ceiling over a context set: the loaded
//! files that are not original. SY01-SY03 are CONTROLLED: files, roots, seeds and sinks are parsed
//! by the real producer, but ONE DataFlow edge into the context file is injected (SY01, SY02) or
//! the real call edge is relabeled (SY03), so the label under test is chosen by the test and not by
//! the parser. SY05 is CONTROLLED the other way: it cuts the seed's own flow so only same-file
//! direct-reference synthesis can reach the sink. SY04 is REAL: it re-runs the S3a.1s route table.
//! Every fixture asserts its own preconditions first ("inadmissible precondition:").
use super::cases::*;
use super::*;
use crate::algorithms::taint::run::slice_scoped_impl;
use crate::algorithms::taint::source_scope::admission::{
    slice_in_context, slice_scoped, RunScope, ScopedTaintOutcome,
};
use petgraph::visit::EdgeRef;
use prism::cpg::{CrossingCeiling, TaintCrossingPolicy};

const EXACT: FlowConfidence = FlowConfidence::Exact;
const WEAK: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::SameLine);

/// (label of the one context entry, ceiling, the entry is admitted).
const ROWS: [(FlowConfidence, CrossingCeiling, bool); 4] = [
    (EXACT, CrossingCeiling::Exact, true),
    (EXACT, CrossingCeiling::NameOnly, true),
    (WEAK, CrossingCeiling::Exact, false),
    (WEAK, CrossingCeiling::NameOnly, true),
];

// A Django view: Python target seeds carry no line scope, so a context line is never filtered by
// `js_ts_seed_scope_contains`. Seeds: `request` (line 2) and `q` (line 3); the sink is line 4.
const VIEWS: &str =
    "import os\ndef lookup_view(request):\n    q = request.GET.get(\"q\")\n    os.system(q)\n";
// The context file of SY01 and SY02: the sink is line 3, its parameter Def line 2.
const SINK_LIB: &str = "import os\ndef handle(cmd):\n    os.system(cmd)\n";
const APP_PY: &str = "def g(d):\n    p = d\n    return p\n";
// The real call pair of the core return-flow tests; the callee holds the sink (line 4).
const MAIN_PY: &str =
    "from lib import helper\ndef run(user):\n    value = helper(user)\n    sink(value)\n";
const HELPER_LIB: &str = "import os\ndef helper(p):\n    q = p\n    os.system(q)\n    return q\n";

/// One program set. `wire` is the CONTROLLED edit of the built graph; the label it receives is
/// the row's, and `None` leaves the parsed graph as the producer built it.
struct Fixture {
    files: Vec<(&'static str, &'static str)>,
    original: Vec<&'static str>,
    input: DiffInput,
    config: TaintConfig,
    wire: fn(&mut CpgContext<'_>, Option<FlowConfidence>),
}

/// Both routes of one run: the traced combined entry and the untraced internal path.
struct Outcome {
    traced: ScopedTaintOutcome,
    plain: SliceResult,
}

impl Fixture {
    fn original_set(&self) -> BTreeSet<String> {
        originals(&self.original)
    }

    /// The context set: loaded files minus the original ones.
    fn context_set(&self) -> BTreeSet<String> {
        let original = self.original_set();
        let loaded = self.files.iter().map(|(path, _)| path.to_string());
        loaded.filter(|path| !original.contains(path)).collect()
    }

    /// `ceiling: None` is the source-only entry (S3a.1s); `Some` is the combined entry. Both the
    /// traced entry and the untraced internal path run on the same wired graph.
    fn run(&self, ceiling: Option<CrossingCeiling>, label: Option<FlowConfidence>) -> Outcome {
        let files = parse(&self.files);
        let mut ctx = CpgContext::build(&files, None);
        (self.wire)(&mut ctx, label);
        let (original, context) = (self.original_set(), self.context_set());
        let policy = ceiling.map(|ceiling| TaintCrossingPolicy::new(&context, ceiling));
        let (input, config) = (&self.input, &self.config);
        let entry = match &policy {
            Some(policy) => slice_in_context(&ctx, input, config, &original, policy),
            None => slice_scoped(&ctx, input, config, &original),
        };
        let scope = match &policy {
            Some(policy) => RunScope::in_context(&original, policy),
            None => RunScope::original(&original),
        };
        let internal = slice_scoped_impl(&ctx, input, config, None, scope);
        Outcome {
            traced: entry.expect("traced scoped taint"),
            plain: internal.expect("untraced scoped taint"),
        }
    }
}

fn views() -> Fixture {
    Fixture {
        files: vec![("views.py", VIEWS), ("ctx/lib.py", SINK_LIB)],
        original: vec!["views.py"],
        input: DiffInput { files: vec![] },
        config: TaintConfig {
            taint_from_diff: false,
            ..TaintConfig::default()
        },
        wire: wire_views,
    }
}

fn app() -> Fixture {
    Fixture {
        files: vec![("app.py", APP_PY), ("ctx/lib.py", SINK_LIB)],
        original: vec!["app.py"],
        input: diff("app.py", &[2]),
        config: TaintConfig::default(),
        wire: wire_app,
    }
}

fn call_pair() -> Fixture {
    Fixture {
        files: vec![("main.py", MAIN_PY), ("lib.py", HELPER_LIB)],
        original: vec!["main.py"],
        input: diff("main.py", &[3]),
        config: TaintConfig {
            return_flow: true,
            ..TaintConfig::default()
        },
        wire: wire_call,
    }
}

type Site = (&'static str, usize, &'static str);

/// CONTROLLED: one DataFlow edge between two Defs of the parsed graph.
fn inject(ctx: &mut CpgContext<'_>, from: Site, to: Site, label: FlowConfidence) {
    let from = unique_var_node(ctx, from.0, from.1, from.2, VarAccessKind::Def);
    let to = unique_var_node(ctx, to.0, to.1, to.2, VarAccessKind::Def);
    add_flow(ctx, from, to, label);
}

fn wire_views(ctx: &mut CpgContext<'_>, label: Option<FlowConfidence>) {
    if let Some(label) = label {
        inject(ctx, ("views.py", 3, "q"), ("ctx/lib.py", 2, "cmd"), label);
    }
}

fn wire_app(ctx: &mut CpgContext<'_>, label: Option<FlowConfidence>) {
    if let Some(label) = label {
        inject(ctx, ("app.py", 2, "p"), ("ctx/lib.py", 2, "cmd"), label);
    }
}

/// CONTROLLED: drop every DataFlow edge that leaves the seed's own Def, so the parsed flow from
/// the seed reaches nothing and only direct-reference synthesis can.
fn wire_views_cut(ctx: &mut CpgContext<'_>, _label: Option<FlowConfidence>) {
    let node = unique_var_node(ctx, "views.py", 3, "q", VarAccessKind::Def);
    let before = ctx.cpg.graph.edge_count();
    ctx.cpg.graph.retain_edges(|graph, edge| {
        let from_seed = graph
            .edge_endpoints(edge)
            .is_some_and(|(source, _)| source == node);
        !(from_seed && matches!(graph[edge], CpgEdge::DataFlow(_)))
    });
    let msg = "inadmissible precondition: the seed Def has a DataFlow edge to cut";
    assert!(ctx.cpg.graph.edge_count() < before, "{msg}");
}

fn flow_labels(ctx: &CpgContext<'_>, from: NodeIndex, to: NodeIndex) -> Vec<FlowConfidence> {
    let edges = ctx.cpg.graph.edges(from).filter(|e| e.target() == to);
    edges
        .filter_map(|edge| match edge.weight() {
            CpgEdge::DataFlow(label) => Some(*label),
            _ => None,
        })
        .collect()
}

/// The real argument -> parameter edge of the call pair must be ONE Exact DataFlow edge; a row's
/// label then relabels it (CONTROLLED).
fn wire_call(ctx: &mut CpgContext<'_>, label: Option<FlowConfidence>) {
    let arg = unique_var_node(ctx, "main.py", 3, "user", VarAccessKind::Use);
    let param = unique_var_node(ctx, "lib.py", 2, "p", VarAccessKind::Def);
    let msg =
        "inadmissible precondition: the argument -> parameter edge is one Exact DataFlow edge";
    assert_eq!(flow_labels(ctx, arg, param), vec![EXACT], "{msg}");
    let Some(label) = label else { return };
    let ids: Vec<_> = (ctx.cpg.graph.edges(arg))
        .filter(|e| e.target() == param && matches!(e.weight(), CpgEdge::DataFlow(_)))
        .map(|e| e.id())
        .collect();
    for id in ids {
        ctx.cpg.graph[id] = CpgEdge::DataFlow(label);
    }
}

/// Files reached by the frontiers of every root of the full trace.
fn trace_reaches(run: &Outcome, file: &str) -> bool {
    let roots = run.traced.trace.roots.iter();
    roots
        .flat_map(|r| r.reached.iter())
        .any(|t| t.location.file == file)
}

/// Files reached by the roots allowed to cause loading.
fn loading_reaches(run: &Outcome, file: &str) -> bool {
    let roots = run.traced.loading.roots.iter();
    roots
        .flat_map(|r| r.reached.iter())
        .any(|t| t.location.file == file)
}

fn sink_evidence<'a>(outcome: &'a ScopedTaintOutcome, file: &str, line: usize) -> &'a EvidencePath {
    let at = outcome.result.findings.iter().position(|f| {
        f.category.as_deref() == Some("taint_sink") && f.file == file && f.line == line
    });
    let at = at.unwrap_or_else(|| panic!("inadmissible precondition: no sink {file}:{line}"));
    outcome.result.evidence[at]
        .as_ref()
        .expect("aligned evidence")
}

fn is_cross(hop: &EvidenceHop, from: &str, to: &str) -> bool {
    matches!(hop, EvidenceHop::DataFlow { from: f, to: t, .. } if f.file == from && t.file == to)
}

/// Exactness of every DataFlow hop of `evidence` that leaves `from` for `to`.
fn cross_hops(evidence: &EvidencePath, from: &str, to: &str) -> Vec<bool> {
    let hops = evidence.hops.iter();
    hops.filter(|hop| is_cross(hop, from, to))
        .map(EvidenceHop::is_exact)
        .collect()
}

fn assert_same_outcome(a: &ScopedTaintOutcome, b: &ScopedTaintOutcome, label: &str) {
    let bytes = |o: &ScopedTaintOutcome| serde_json::to_vec(&o.result).unwrap();
    assert!(bytes(a) == bytes(b), "{label}: serialized result");
    let evidence = |o: &ScopedTaintOutcome| format!("{:?}", o.result.evidence);
    assert_eq!(evidence(a), evidence(b), "{label}: evidence");
    assert_eq!(
        format!("{:?}", a.trace),
        format!("{:?}", b.trace),
        "{label}: trace"
    );
    assert_eq!(
        format!("{:?}", a.loading),
        format!("{:?}", b.loading),
        "{label}: loading"
    );
    assert_eq!(
        a.excluded_explicit, b.excluded_explicit,
        "{label}: exclusions"
    );
}

/// Seed, origin and location of every root, in order.
fn identities(trace: &TaintTrace) -> Vec<String> {
    let roots = trace.roots.iter();
    roots
        .map(|r| format!("{:?}", (&r.seed, r.origin, &r.location)))
        .collect()
}

/// SY01 CONTROLLED: a real targeted Framework seed (Django view); one injected edge from the
/// seed's Def into a context value that feeds a context sink.
#[test]
fn crossing_refuses_nameonly_synthetic_entry_in_both_paths() {
    let fixture = views();
    let control = fixture.run(None, Some(WEAK));
    let msg = "inadmissible precondition";
    let roots = &control.traced.trace.roots;
    let targeted = roots
        .iter()
        .all(|r| r.origin == Framework && r.seed.target.is_some());
    assert!(
        !roots.is_empty() && targeted,
        "{msg}: only targeted Framework roots"
    );
    assert!(
        roots.iter().all(|r| r.seed.scope.is_none()),
        "{msg}: seeds carry no line scope"
    );
    for (run, name) in [
        (&control.traced.result, "traced"),
        (&control.plain, "untraced"),
    ] {
        assert_eq!(
            sink_lines(run, "ctx/lib.py"),
            [3],
            "{msg}: {name} synthetic route"
        );
        assert_eq!(sink_lines(run, "views.py"), [4], "{msg}: {name} own sink");
    }
    assert!(
        trace_reaches(&control, "ctx/lib.py"),
        "{msg}: synthetic frontier"
    );

    for (label, ceiling, admitted) in ROWS {
        let row = format!("{label:?} {ceiling:?}");
        let run = fixture.run(Some(ceiling), Some(label));
        let context: &[usize] = if admitted { &[3] } else { &[] };
        assert_eq!(
            sink_lines(&run.traced.result, "ctx/lib.py"),
            context,
            "{row}: traced"
        );
        assert_eq!(
            sink_lines(&run.plain, "ctx/lib.py"),
            context,
            "{row}: untraced"
        );
        assert_eq!(
            trace_reaches(&run, "ctx/lib.py"),
            admitted,
            "{row}: frontier"
        );
        // The refusal is the crossing's: the original file's own finding stays.
        assert_eq!(
            sink_lines(&run.traced.result, "views.py"),
            [4],
            "{row}: traced own"
        );
        assert_eq!(
            sink_lines(&run.plain, "views.py"),
            [4],
            "{row}: untraced own"
        );
    }
    let lenient = fixture.run(Some(CrossingCeiling::NameOnly), Some(WEAK));
    assert_same_outcome(
        &lenient.traced,
        &control.traced,
        "NameOnly equals source-only",
    );
}

/// SY02 CONTROLLED: an original diff root and one injected edge to a context sink line.
#[test]
fn crossing_gates_ordinary_context_entry_through_combined_entry() {
    let fixture = app();
    let control = fixture.run(None, Some(WEAK));
    let msg = "inadmissible precondition";
    assert_eq!(
        sink_lines(&control.traced.result, "ctx/lib.py"),
        [3],
        "{msg}: context sink"
    );
    assert!(
        loading_reaches(&control, "ctx/lib.py"),
        "{msg}: loading frontier"
    );
    assert!(
        loading_reaches(&control, "app.py"),
        "{msg}: original-side flow"
    );
    let evidence = sink_evidence(&control.traced, "ctx/lib.py", 3);
    let cross = cross_hops(evidence, "app.py", "ctx/lib.py");
    assert!(
        !cross.is_empty() && cross.iter().all(|exact| !exact),
        "{msg}: NameOnly hop"
    );

    for (label, ceiling, admitted) in ROWS {
        let row = format!("{label:?} {ceiling:?}");
        let run = fixture.run(Some(ceiling), Some(label));
        let context: &[usize] = if admitted { &[3] } else { &[] };
        assert_eq!(
            sink_lines(&run.traced.result, "ctx/lib.py"),
            context,
            "{row}: traced"
        );
        assert_eq!(
            sink_lines(&run.plain, "ctx/lib.py"),
            context,
            "{row}: untraced"
        );
        assert_eq!(
            loading_reaches(&run, "ctx/lib.py"),
            admitted,
            "{row}: loading frontier"
        );
        assert!(
            loading_reaches(&run, "app.py"),
            "{row}: original-side flow kept"
        );
        if admitted {
            let evidence = sink_evidence(&run.traced, "ctx/lib.py", 3);
            let cross = cross_hops(evidence, "app.py", "ctx/lib.py");
            // The injected hop keeps its own label: a ceiling never relabels an edge.
            let same = cross.iter().all(|&exact| exact == label.is_exact());
            assert!(!cross.is_empty() && same, "{row}: {cross:?}");
        }
    }
}

/// SY03: `return_flow = true` on the real two-file call pair; the argument -> parameter entry is
/// relabeled (CONTROLLED). The descent stays an unlabeled bridge whatever the entry label.
#[test]
fn crossing_gates_return_flow_route_through_combined_entry() {
    let fixture = call_pair();
    assert!(fixture.config.return_flow);
    let control = fixture.run(None, None);
    let msg = "inadmissible precondition";
    assert_eq!(
        sink_lines(&control.traced.result, "lib.py"),
        [4],
        "{msg}: callee sink"
    );
    assert!(
        loading_reaches(&control, "lib.py"),
        "{msg}: descent reaches the callee"
    );
    assert!(
        loading_reaches(&control, "main.py"),
        "{msg}: caller-side flow"
    );
    let evidence = sink_evidence(&control.traced, "lib.py", 4);
    assert!(
        evidence.crossed_unlabeled,
        "{msg}: the descent is an unlabeled bridge"
    );

    for (label, ceiling, admitted) in ROWS {
        let row = format!("{label:?} {ceiling:?}");
        let run = fixture.run(Some(ceiling), Some(label));
        let context: &[usize] = if admitted { &[4] } else { &[] };
        assert_eq!(
            sink_lines(&run.traced.result, "lib.py"),
            context,
            "{row}: traced"
        );
        assert_eq!(sink_lines(&run.plain, "lib.py"), context, "{row}: untraced");
        assert_eq!(
            loading_reaches(&run, "lib.py"),
            admitted,
            "{row}: loading frontier"
        );
        assert!(
            loading_reaches(&run, "main.py"),
            "{row}: caller-side flow kept"
        );
        if admitted {
            let evidence = sink_evidence(&run.traced, "lib.py", 4);
            assert!(evidence.crossed_unlabeled, "{row}: unlabeled bridge");
            let cross = cross_hops(evidence, "main.py", "lib.py");
            assert!(
                cross.is_empty(),
                "{row}: the descent is never a labeled hop: {cross:?}"
            );
        }
    }
    let relabeled = fixture.run(None, Some(WEAK));
    let lenient = fixture.run(Some(CrossingCeiling::NameOnly), Some(WEAK));
    assert_same_outcome(
        &lenient.traced,
        &relabeled.traced,
        "NameOnly equals source-only",
    );
}

/// SY04 REAL: the S3a.1s route table through the combined entry.
#[test]
fn combined_nameonly_equals_source_only_on_all_routes() {
    let cases = route_cases();
    assert_eq!(
        cases.len(),
        ROUTE_NAMES.len(),
        "inadmissible precondition: the route table"
    );
    let (mut roots, mut findings) = (0, 0);
    for case in cases {
        // The all-original control asserts the case's own `needs`.
        let _control = case.all_original();
        for return_flow in [false, true] {
            let label = format!("{} return_flow={return_flow}", case.name);
            let config = TaintConfig {
                return_flow,
                ..case.config.clone()
            };
            let files = parse(&case.files);
            let ctx = CpgContext::build(&files, None);
            let (input, original) = (case.input(), originals(&case.original));
            let loaded = case.loaded_set();
            let context: BTreeSet<String> = loaded.difference(&original).cloned().collect();
            let source_only = slice_scoped(&ctx, &input, &config, &original).unwrap();
            let lenient = TaintCrossingPolicy::new(&context, CrossingCeiling::NameOnly);
            let nameonly = slice_in_context(&ctx, &input, &config, &original, &lenient).unwrap();
            assert_same_outcome(&nameonly, &source_only, &label);
            let strict = TaintCrossingPolicy::new(&context, CrossingCeiling::Exact);
            let exact = slice_in_context(&ctx, &input, &config, &original, &strict).unwrap();
            assert_eq!(
                identities(&exact.trace),
                identities(&nameonly.trace),
                "{label}: roots"
            );
            roots += source_only.trace.roots.len();
            findings += source_only.result.findings.len();
        }
    }
    let msg = "inadmissible precondition: the route table holds no roots or findings";
    assert!(roots > 0 && findings > 0, "{msg}");
}

/// SY05 CONTROLLED: with the seed's own parsed flow cut, only same-file direct-reference
/// synthesis reaches the sink, and no ceiling touches it.
#[test]
fn same_file_synthesis_survives_exact_ceiling() {
    let mut fixture = views();
    fixture.wire = wire_views_cut;
    let control = fixture.run(None, None);
    let msg = "inadmissible precondition";
    let roots = &control.traced.trace.roots;
    let targeted = roots
        .iter()
        .all(|r| r.origin == Framework && r.seed.target.is_some());
    assert!(
        !roots.is_empty() && targeted,
        "{msg}: only targeted Framework roots"
    );
    for (run, name) in [
        (&control.traced.result, "traced"),
        (&control.plain, "untraced"),
    ] {
        assert_eq!(
            sink_lines(run, "views.py"),
            [4],
            "{msg}: {name} direct-reference route"
        );
    }
    assert!(
        trace_reaches(&control, "views.py"),
        "{msg}: same-file frontier"
    );

    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let run = fixture.run(Some(ceiling), None);
        assert_eq!(
            sink_lines(&run.traced.result, "views.py"),
            [4],
            "{ceiling:?}: traced"
        );
        assert_eq!(
            sink_lines(&run.plain, "views.py"),
            [4],
            "{ceiling:?}: untraced"
        );
        assert!(
            trace_reaches(&run, "views.py"),
            "{ceiling:?}: same-file frontier"
        );
        assert!(
            !trace_reaches(&run, "ctx/lib.py"),
            "{ceiling:?}: no context line is reached"
        );
        assert_same_outcome(&run.traced, &control.traced, &format!("{ceiling:?}"));
    }
}
