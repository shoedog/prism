//! E01 and E08, and the support the decision-site tests of `sites.rs` share (#602 S4d1 part E).
//!
//! Kinds. REAL: files, roots, sinks and every edge come from the parser and the real producer
//! (all of E01, E02, E04, E05, E06, E08). CONTROLLED: `weights` (E02 row, E03, E06 row, E07 row 1) and
//! `through_def` (E07 row 2) parse their files but inject or relabel the DataFlow edges named in
//! their `wire_*` function, so the label under test is the test's and not the parser's. Every
//! fixture asserts its own preconditions first ("inadmissible precondition:").
use super::*;
use crate::algorithms::taint::run::{slice_attributed_impl, slice_scoped_impl};
use crate::algorithms::taint::source_scope::admission::{
    slice_in_context, slice_scoped, RunScope, ScopedTaintOutcome,
};
use crate::algorithms::taint::{slice, slice_with_trace, TaintConfig, TaintTrace};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use prism::ast::ParsedFile;
use prism::cpg::{
    CpgContext, CpgEdge, CrossingCeiling, FlowConfidence, FlowDoubt, TaintCrossingPolicy,
};
use prism::data_flow::VarAccessKind;
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::finding_confidence::{EvidenceHop, EvidencePath};
use prism::languages::Language;
use prism::slice::SliceResult;
use std::collections::{BTreeMap, BTreeSet};

mod sites;

const EXACT: FlowConfidence = FlowConfidence::Exact;
const WEAK: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::SameLine);

// JS: an inline source=sink `load(req.body.payload)`; JS framework seeds are targeted, so no
// targetless source exists and the sink names none.
const YAML_JS: &str = "import express from \"express\";\n\nconst app = express();\n\napp.post(\"/yaml\", (req, res) => {\n  return load(req.body.payload);\n});\n\nconst { load } = require(\"js-yaml\");\n";
// A Django view: the sink is line 4, the targeted seeds are `request` (2) and `q` (3).
const VIEWS: &str =
    "import os\ndef lookup_view(request):\n    q = request.GET.get(\"q\")\n    os.system(q)\n";
// A gin handler whose source and sink share line 6: no flow path links them.
const GO_PARAM: &str = "package main\n\nimport \"github.com/gin-gonic/gin\"\n\nfunc handler(c *gin.Context) {\n\tc.File(c.Param(\"f\"))\n}\n";
// The two-file call pair of the core return-flow tests; the callee holds the sink (line 4).
const MAIN_PY: &str =
    "from lib import helper\ndef run(user):\n    value = helper(user)\n    sink(value)\n";
const HELPER_LIB: &str = "import os\ndef helper(p):\n    q = p\n    os.system(q)\n    return q\n";
// Bash: line 4 is the diff-seeded source, line 6 the unquoted `cat $file`.
const SCRIPT_SH: &str = "#!/bin/bash\n\nprocess() {\n    local file=\"$1\"\n    echo \"processing\"\n    cat $file\n}\n\nprocess \"$@\"\n";
const QUOTED_SH: &str = "#!/bin/bash\n\nprocess() {\n    local file=\"$1\"\n    echo \"processing\"\n    cat \"$file\"\n}\n\nprocess \"$@\"\n";
// `weights`: source S is line 3 of app.py; D is its own line 4, C is the context file's line 3.
const WEIGHTS_PY: &str = "import os\ndef g(d):\n    p = input()\n    os.system(p)\n";
const SINK_LIB: &str = "import os\ndef handle(cmd):\n    os.system(cmd)\n";
const DEF_FLOW_PY: &str = "def g(d):\n    p = d\n    return p\n";

type Wire = fn(&mut CpgContext<'_>);

struct Fixture {
    name: &'static str,
    files: Vec<(&'static str, &'static str)>,
    original: Vec<&'static str>,
    input: DiffInput,
    config: TaintConfig,
    wire: Wire,
}

impl Fixture {
    fn new(
        name: &'static str,
        files: &[(&'static str, &'static str)],
        original: &[&'static str],
    ) -> Self {
        Self {
            name,
            files: files.to_vec(),
            original: original.to_vec(),
            input: DiffInput { files: vec![] },
            config: framework_only(),
            wire: |_| {},
        }
    }

    fn loaded(&self) -> BTreeSet<String> {
        self.files
            .iter()
            .map(|(path, _)| path.to_string())
            .collect()
    }

    fn original_set(&self) -> BTreeSet<String> {
        self.original.iter().map(|path| path.to_string()).collect()
    }

    /// The loaded files that are not original.
    fn context_set(&self) -> BTreeSet<String> {
        let original = self.original_set();
        self.loaded().difference(&original).cloned().collect()
    }

    /// Parse, build, wire, then hand the graph to `check`.
    fn with_ctx<R>(&self, check: impl FnOnce(&CpgContext<'_>) -> R) -> R {
        let files = parse(&self.files);
        let mut ctx = CpgContext::build(&files, None);
        (self.wire)(&mut ctx);
        check(&ctx)
    }

    /// The scoped combined entry (the route S4 uses): `slice_in_context` under `ceiling`.
    fn run(&self, ceiling: CrossingCeiling) -> ScopedTaintOutcome {
        self.with_ctx(|ctx| {
            let (original, context) = (self.original_set(), self.context_set());
            let policy = TaintCrossingPolicy::new(&context, ceiling);
            slice_in_context(ctx, &self.input, &self.config, &original, &policy)
                .expect("scoped taint")
        })
    }
}

fn parse(files: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let parse = |path: &str, source: &str| {
        ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
    };
    files
        .iter()
        .map(|&(path, source)| (path.to_string(), parse(path, source)))
        .collect()
}

fn diff(path: &str, lines: &[usize]) -> DiffInput {
    DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: lines.iter().copied().collect(),
        }],
    }
}

fn framework_only() -> TaintConfig {
    TaintConfig {
        taint_from_diff: false,
        ..TaintConfig::default()
    }
}

fn js_same_file() -> Fixture {
    Fixture::new("js_same_file", &[("app.js", YAML_JS)], &["app.js"])
}

fn django_view() -> Fixture {
    Fixture::new("django_view", &[("views.py", VIEWS)], &["views.py"])
}

fn gin_handler() -> Fixture {
    Fixture::new("gin_handler", &[("main.go", GO_PARAM)], &["main.go"])
}

fn call_pair() -> Fixture {
    let files = [("main.py", MAIN_PY), ("lib.py", HELPER_LIB)];
    Fixture {
        input: diff("main.py", &[3]),
        config: TaintConfig {
            return_flow: true,
            ..TaintConfig::default()
        },
        ..Fixture::new("call_pair", &files, &["main.py"])
    }
}

fn bash_script(source: &'static str) -> Fixture {
    Fixture {
        input: diff("script.sh", &[4]),
        config: TaintConfig::default(),
        ..Fixture::new("bash_script", &[("script.sh", source)], &["script.sh"])
    }
}

/// CONTROLLED. S (app.py:3) reaches D (app.py:4) by the parsed def-use edge forced Exact, and C
/// (ctx/lib.py:3, a context sink) by ONE injected NameOnly edge into the sink's argument.
fn weights() -> Fixture {
    let files = [("app.py", WEIGHTS_PY), ("ctx/lib.py", SINK_LIB)];
    Fixture {
        input: diff("app.py", &[3]),
        config: TaintConfig::default(),
        wire: wire_weights,
        ..Fixture::new("weights", &files, &["app.py"])
    }
}

/// CONTROLLED. SY02's shape: one injected NameOnly edge from the diff Def into the context
/// parameter Def, so no direct edge links the source line to the context sink line.
fn through_def() -> Fixture {
    let files = [("app.py", DEF_FLOW_PY), ("ctx/lib.py", SINK_LIB)];
    Fixture {
        input: diff("app.py", &[2]),
        config: TaintConfig::default(),
        wire: wire_through_def,
        ..Fixture::new("through_def", &files, &["app.py"])
    }
}

/// The four parsed fixtures of E01.
fn fixtures() -> Vec<Fixture> {
    vec![js_same_file(), django_view(), gin_handler(), call_pair()]
}

fn wire_weights(ctx: &mut CpgContext<'_>) {
    let def = unique_var_node(ctx, "app.py", 3, "p", VarAccessKind::Def);
    let used = unique_var_node(ctx, "app.py", 4, "p", VarAccessKind::Use);
    relabel(ctx, def, used, EXACT);
    let arg = unique_var_node(ctx, "ctx/lib.py", 3, "cmd", VarAccessKind::Use);
    ctx.cpg.graph.add_edge(def, arg, CpgEdge::DataFlow(WEAK));
}

fn wire_through_def(ctx: &mut CpgContext<'_>) {
    let from = unique_var_node(ctx, "app.py", 2, "p", VarAccessKind::Def);
    let to = unique_var_node(ctx, "ctx/lib.py", 2, "cmd", VarAccessKind::Def);
    ctx.cpg.graph.add_edge(from, to, CpgEdge::DataFlow(WEAK));
}

fn unique_var_node(
    ctx: &CpgContext,
    file: &str,
    line: usize,
    base: &str,
    kind: VarAccessKind,
) -> NodeIndex {
    let hits: Vec<_> = (ctx.cpg.nodes_at(file, line).into_iter())
        .filter_map(|node| Some((node, ctx.cpg.to_var_location(node)?)))
        .filter(|(_, loc)| loc.path.base == base && loc.kind == kind)
        .map(|(node, _)| node)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "inadmissible precondition: {base}@{line} {kind:?}"
    );
    hits[0]
}

/// CONTROLLED: every DataFlow edge `from` -> `to` gets `label`; there must be one to relabel.
fn relabel(ctx: &mut CpgContext<'_>, from: NodeIndex, to: NodeIndex, label: FlowConfidence) {
    let edges = ctx.cpg.graph.edges(from);
    let edges = edges.filter(|e| e.target() == to && matches!(e.weight(), CpgEdge::DataFlow(_)));
    let ids: Vec<_> = edges.map(|e| e.id()).collect();
    let msg = "inadmissible precondition: the parsed def-use edge exists";
    assert!(!ids.is_empty(), "{msg}");
    for id in ids {
        ctx.cpg.graph[id] = CpgEdge::DataFlow(label);
    }
}

fn bytes(result: &SliceResult) -> Vec<u8> {
    serde_json::to_vec(result).unwrap()
}

fn assert_same(a: &SliceResult, b: &SliceResult, label: &str) {
    assert!(bytes(a) == bytes(b), "{label}: serialized result");
    let evidence = |r: &SliceResult| format!("{:?}", r.evidence);
    assert_eq!(evidence(a), evidence(b), "{label}: evidence");
}

fn is_exact(evidence: &EvidencePath) -> bool {
    let all = evidence.hops.iter().all(EvidenceHop::is_exact);
    !evidence.crossed_unlabeled && !evidence.hops.is_empty() && all
}

fn is_name_only(evidence: &EvidencePath) -> bool {
    let weak = evidence.hops.iter().any(|hop| !hop.is_exact());
    !evidence.crossed_unlabeled && weak
}

fn is_unlabeled(evidence: &EvidencePath) -> bool {
    evidence.crossed_unlabeled && evidence.hops.is_empty()
}

fn sink_at<'a>(sidecar: &'a EmissionAttribution, file: &str, line: usize) -> &'a SinkEmission {
    let found = sidecar
        .sinks
        .iter()
        .find(|s| s.file == file && s.line == line);
    // A behavioral failure of the sidecar. Fixture preconditions use `finding_at` on the result.
    found.unwrap_or_else(|| panic!("no sink-style emission {file}:{line}"))
}

/// Position of the `category` finding at `file:line` in the public result; a fixture failure when
/// the run emitted none, so it never reads the sidecar under test.
fn finding_at(result: &SliceResult, category: &str, file: &str, line: usize) -> usize {
    let found = result
        .findings
        .iter()
        .position(|f| f.category.as_deref() == Some(category) && f.file == file && f.line == line);
    found
        .unwrap_or_else(|| panic!("inadmissible precondition: no {category} finding {file}:{line}"))
}

fn license_of<'a>(sink: &'a SinkEmission, file: &str, line: usize) -> &'a LicensedSource {
    let found = sink
        .licensed
        .iter()
        .find(|l| l.file == file && l.line == line);
    found.unwrap_or_else(|| {
        // A behavioral failure of the sidecar, not a fixture failure.
        panic!("{}:{} licensed no {file}:{line}", sink.file, sink.line)
    })
}

fn evidence_at(result: &SliceResult, index: usize) -> &EvidencePath {
    result.evidence[index].as_ref().expect("aligned evidence")
}

/// Stores made by this thread's recording helpers so far.
fn stored() -> usize {
    super::STORED.with(|stored| stored.get())
}

/// E01 REAL. The sidecar changes no public output. Equivalent source policies only: with every
/// loaded file original, scoped admission equals the legacy run, so `slice`, `slice_with_trace`,
/// `slice_scoped`, the wrapper and the sidecar run must agree. A restricted policy (context files
/// that cannot start roots) is NOT comparable to `slice` and is compared with the wrapper only.
#[test]
fn attribution_changes_no_public_output() {
    let mut context_sinks = 0;
    for fixture in fixtures() {
        let name = fixture.name;
        let all = fixture.loaded();
        fixture.with_ctx(|ctx| {
            let (input, config) = (&fixture.input, &fixture.config);
            let legacy = slice(ctx, input, config).unwrap();
            let msg = format!("inadmissible precondition: {name} has no taint_sink");
            let sink = |f: &prism::slice::SliceFinding| f.category.as_deref() == Some("taint_sink");
            assert!(legacy.findings.iter().any(sink), "{msg}");
            let (traced, trace) = slice_with_trace(ctx, input, config).unwrap();
            let scoped = slice_scoped(ctx, input, config, &all).unwrap();
            let scope = RunScope::original(&all);
            let wrapper = slice_scoped_impl(ctx, input, config, None, scope).unwrap();
            let mut sidecar = EmissionAttribution::default();
            let attributed =
                slice_attributed_impl(ctx, input, config, None, scope, Some(&mut sidecar));
            let attributed = attributed.unwrap();
            assert_same(&legacy, &traced, &format!("{name}: slice_with_trace"));
            assert_same(&legacy, &scoped.result, &format!("{name}: slice_scoped"));
            assert_same(&legacy, &wrapper, &format!("{name}: slice_scoped_impl"));
            assert_same(&legacy, &attributed, &format!("{name}: sidecar run"));

            // The same default scope, traced: result, evidence and trace are the legacy ones.
            let mut kept = TaintTrace::default();
            let mut again = EmissionAttribution::default();
            let plain = slice_attributed_impl(
                ctx,
                input,
                config,
                Some(&mut kept),
                RunScope::default(),
                Some(&mut again),
            );
            assert_same(
                &traced,
                &plain.unwrap(),
                &format!("{name}: traced sidecar run"),
            );
            assert_eq!(format!("{trace:?}"), format!("{kept:?}"), "{name}: trace");
        });
        // A restricted policy under both ceilings: the sidecar run equals the wrapper.
        let (original, context) = (fixture.original_set(), fixture.context_set());
        for ceiling in [CrossingCeiling::NameOnly, CrossingCeiling::Exact] {
            let policy = TaintCrossingPolicy::new(&context, ceiling);
            let scope = RunScope::in_context(&original, &policy);
            fixture.with_ctx(|ctx| {
                let (input, config) = (&fixture.input, &fixture.config);
                let wrapper = slice_scoped_impl(ctx, input, config, None, scope).unwrap();
                let entry = slice_in_context(ctx, input, config, &original, &policy).unwrap();
                assert_same(
                    &wrapper,
                    &entry.result,
                    &format!("{name} {ceiling:?}: scoped"),
                );
                // Counted from the public result, never from the sidecar under test.
                let findings = entry.result.findings.iter();
                let sink = |f: &&prism::slice::SliceFinding| {
                    f.category.as_deref() == Some("taint_sink") && context.contains(&f.file)
                };
                context_sinks += findings.filter(sink).count();
            });
        }
    }
    let msg = "inadmissible precondition: no context sink under the restricted policy";
    assert!(context_sinks > 0, "{msg}");
}

/// E08 REAL. A run with no sidecar records nothing. Two observations, neither of which is "a
/// default object nobody passed stays default" (that proves nothing): the thread-local count of
/// stores made by the helpers does not move across every sidecar-free entry, and `Recorder::new(None)`
/// holds no buffer after being driven through all three helpers. The positive control that this
/// counter does move is `the_counter_sees_every_recorded_entry`.
#[test]
fn none_records_nothing() {
    let before = stored();
    let mut recorder = Recorder::new(None);
    let evidence = EvidencePath::unlabeled_for("taint");
    recorder.license(("a.py", 1), &evidence, LicenseRoute::Path);
    recorder.sink(0, "taint_sink", ("a.py", 2), Some(("a.py", 1)));
    recorder.source(1, ("a.py", 1));
    assert_eq!(recorder.licensed.capacity(), 0, "a license was buffered");
    assert_eq!(stored(), before, "a helper stored with no sidecar");

    for fixture in fixtures() {
        let all = fixture.loaded();
        fixture.with_ctx(|ctx| {
            let (input, config) = (&fixture.input, &fixture.config);
            let scope = RunScope::original(&all);
            let before = stored();
            slice_scoped_impl(ctx, input, config, None, scope).unwrap();
            slice_attributed_impl(ctx, input, config, None, scope, None).unwrap();
            slice(ctx, input, config).unwrap();
            slice_with_trace(ctx, input, config).unwrap();
            let msg = format!("{}: a run without a sidecar stored", fixture.name);
            assert_eq!(stored(), before, "{msg}");
        });
    }
}

/// E08 control: the counter moves by exactly the entries a sidecar run recorded.
#[test]
fn the_counter_sees_every_recorded_entry() {
    let fixture = django_view();
    let all = fixture.loaded();
    fixture.with_ctx(|ctx| {
        let (input, config) = (&fixture.input, &fixture.config);
        let before = stored();
        let mut sidecar = EmissionAttribution::default();
        let scope = RunScope::original(&all);
        let result = slice_attributed_impl(ctx, input, config, None, scope, Some(&mut sidecar));
        finding_at(&result.unwrap(), "taint_sink", "views.py", 4); // fixture precondition
        let licensed: usize = sidecar.sinks.iter().map(|s| s.licensed.len()).sum();
        let recorded = licensed + sidecar.sinks.len() + sidecar.sources.len();
        assert!(recorded > 0, "the sidecar run recorded nothing");
        assert_eq!(stored() - before, recorded);
    });
}
