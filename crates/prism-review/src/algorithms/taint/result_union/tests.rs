//! U01-U10: CONTROLLED results, sidecars and evidence; no graph-recall claim.
use super::*;
use crate::algorithms::taint::attribution::{
    LicenseRoute, LicensedSource, SinkEmission, SourceEmission,
};
use prism::access_path::AccessPath;
use prism::call_graph::FunctionId;
use prism::cpg::{FlowConfidence, FlowDoubt};
use prism::data_flow::{VarAccessKind, VarLocation};
use prism::diff::{DiffBlock, ModifyType};
use prism::finding_confidence::{EvidenceHop, EvidencePath};
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge};
use prism::slice::{DiagramWarning, DiagramWarningKind, SliceFinding, SlicingAlgorithm};

mod blocks;
mod real;

const EXACT: FlowConfidence = FlowConfidence::Exact;
const WEAK: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::SameLine);
const OUTSIDE: &str = " (sink file outside the analysed file set; loaded as taint context)";

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}
fn location(file: &str, line: usize) -> VarLocation {
    VarLocation {
        file: file.into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::from("x"),
        start_byte: line * 3,
        end_byte: line * 3 + 1,
        kind: VarAccessKind::Def,
    }
}
fn path(from: (&str, usize), to: (&str, usize), confidence: FlowConfidence) -> EvidencePath {
    EvidencePath::from_hops(vec![EvidenceHop::DataFlow {
        from: location(from.0, from.1),
        to: location(to.0, to.1),
        confidence,
    }])
}
fn call(file: &str, line: usize) -> EvidencePath {
    EvidencePath::from_hops(vec![EvidenceHop::Call {
        edge: ResolvedCallEdge {
            caller: FunctionId {
                file: file.into(),
                name: "callee_is_not_a_file".into(),
                start_line: 1,
                end_line: 99,
            },
            call_site_line: line,
            confidence: ResolutionConfidence::Exact,
            kind: ResolutionKind::FreeSingle,
        },
        confidence: ResolutionConfidence::Exact,
    }])
}
fn finding(category: &str, file: &str, line: usize) -> SliceFinding {
    SliceFinding {
        algorithm: "taint".into(),
        file: file.into(),
        line,
        severity: "warning".into(),
        description: format!("emitted {file}:{line} from source original.py:2"),
        function_name: Some("f".into()),
        related_lines: vec![101],
        related_files: vec![],
        category: Some(category.into()),
        parse_quality: Some("degraded".into()),
        diagrams: vec![],
        rules: Vec::new(),
    }
}
fn block(id: usize, file: &str, line: usize) -> DiffBlock {
    let mut b = DiffBlock::new(id, file.into(), ModifyType::Modified);
    b.add_line(file, line, true);
    b
}
fn bytes<T: serde::Serialize + ?Sized>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn witness(value: &Option<EvidencePath>) -> String {
    format!("{value:?}")
}

struct Fixture {
    baseline: SliceResult,
    expanded: SliceResult,
    attribution: EmissionAttribution,
    originals: BTreeSet<String>,
    context: BTreeSet<String>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            baseline: SliceResult::new(SlicingAlgorithm::Taint),
            expanded: SliceResult::new(SlicingAlgorithm::Taint),
            attribution: EmissionAttribution::default(),
            originals: set(&["original.py", "other.py"]),
            context: set(&["ctx.py", "zctx.py"]),
        }
    }
    fn sink(
        &mut self,
        file: &str,
        line: usize,
        evidence: Option<EvidencePath>,
        licenses: Vec<LicensedSource>,
    ) -> usize {
        let index = self.expanded.findings.len();
        self.expanded
            .findings
            .push(finding("taint_sink", file, line));
        self.expanded.evidence.push(evidence);
        self.attribution.sinks.push(SinkEmission {
            finding_index: index,
            category: "taint_sink".into(),
            file: file.into(),
            line,
            selected_source: licenses.first().map(|s| (s.file.clone(), s.line)),
            licensed: licenses,
        });
        index
    }
    fn source(&mut self, file: &str, line: usize, evidence: Option<EvidencePath>) {
        let index = self.expanded.findings.len();
        self.expanded
            .findings
            .push(finding("taint_source", file, line));
        self.expanded.evidence.push(evidence);
        self.attribution.sources.push(SourceEmission {
            finding_index: index,
            file: file.into(),
            line,
        });
    }
    fn run(&self) -> Result<UnionOutcome, UnionError> {
        union_results(UnionInput {
            baseline: &self.baseline,
            expanded: &self.expanded,
            attribution: &self.attribution,
            original_files: &self.originals,
            context_files: &self.context,
        })
    }
}
fn license(file: &str, line: usize, evidence: EvidencePath) -> LicensedSource {
    LicensedSource {
        file: file.into(),
        line,
        evidence,
        route: LicenseRoute::Path,
    }
}
fn assert_prefix(baseline: &SliceResult, result: &SliceResult) {
    let n = baseline.findings.len();
    assert_eq!(bytes(&result.findings[..n]), bytes(&baseline.findings));
    assert_eq!(
        format!("{:?}", &result.evidence[..baseline.evidence.len()]),
        format!("{:?}", baseline.evidence)
    );
    assert_eq!(
        bytes(&result.blocks[..baseline.blocks.len()]),
        bytes(&baseline.blocks)
    );
    assert_eq!(bytes(&result.warnings), bytes(&baseline.warnings));
    assert_eq!(bytes(&result.diagrams), bytes(&baseline.diagrams));
    assert_eq!(
        bytes(&result.diagram_warnings),
        bytes(&baseline.diagram_warnings)
    );
    assert_eq!(result.algorithm, baseline.algorithm);
}

#[test]
fn u01_baseline_pairs_are_unchanged_and_first() {
    let mut f = Fixture::new();
    f.baseline.findings = vec![finding("taint_sink", "original.py", 2); 2];
    let old = path(("original.py", 1), ("original.py", 2), WEAK);
    f.baseline.evidence = vec![Some(old.clone()), Some(old)];
    f.baseline.blocks = vec![block(7, "original.py", 2)];
    f.baseline.warnings = vec!["preserve warning".into()];
    f.baseline
        .diagrams
        .push(crate::algorithms::taint::report::build_taint_chain_diagram(
            "original.py",
            1,
            "x",
            "original.py",
            2,
            "sink",
            &[],
        ));
    f.baseline.findings[0].diagrams = f.baseline.diagrams.clone();
    f.baseline.diagram_warnings.push(DiagramWarning {
        algorithm: "taint".into(),
        graph_title: Some("original".into()),
        kind: DiagramWarningKind::LabelTruncated,
        detail: "preserve".into(),
    });
    f.sink("ctx.py", 3, None, vec![]);
    let out = f.run().unwrap();
    assert_prefix(&f.baseline, &out.result);
    assert_eq!(out.result.findings.len(), 3);
    f.context.clear();
    let empty = f.run().unwrap();
    assert_eq!(bytes(&empty.result), bytes(&f.baseline));
    assert_eq!(
        format!("{:?}", empty.result.evidence),
        format!("{:?}", f.baseline.evidence)
    );
}

#[test]
fn u02_only_context_touching_sinks_are_added() {
    for (name, anchor, evidence, related, included) in [
        ("anchor", "ctx.py", None, vec![], true),
        (
            "from",
            "original.py",
            Some(path(("ctx.py", 4), ("original.py", 9), EXACT)),
            vec![],
            true,
        ),
        (
            "to",
            "original.py",
            Some(path(("original.py", 9), ("ctx.py", 4), EXACT)),
            vec![],
            true,
        ),
        (
            "call",
            "original.py",
            Some(call("ctx.py", 14)),
            vec![],
            true,
        ),
        ("related", "original.py", None, vec!["ctx.py".into()], true),
        (
            "diff-only",
            "original.py",
            Some(path(("original.py", 1), ("original.py", 9), EXACT)),
            vec![],
            false,
        ),
        ("absent", "original.py", None, vec![], false),
    ] {
        for category in ["taint_sink", "unquoted_expansion"] {
            let mut f = Fixture::new();
            let i = f.sink(anchor, 9, evidence.clone(), vec![]);
            f.expanded.findings[i].category = Some(category.into());
            f.expanded.findings[i].related_files = related.clone();
            f.attribution.sinks[0].category = category.into();
            f.attribution.sinks[0].selected_source = None;
            let out = f.run().unwrap();
            assert_eq!(
                out.result.findings.len(),
                usize::from(included),
                "{name}/{category}"
            );
            assert_eq!(out.stats.additional_sinks, usize::from(included), "{name}");
            assert_eq!(out.stats.dropped_sinks, usize::from(!included), "{name}");
        }
    }
}

#[test]
fn u03_baseline_key_is_never_added_twice() {
    let mut f = Fixture::new();
    let i = f.sink("ctx.py", 3, Some(call("ctx.py", 3)), vec![]);
    let mut original = f.expanded.findings[i].clone();
    original.description = "baseline wording".into();
    f.baseline.findings = vec![original.clone(), original];
    f.baseline.evidence = vec![None, None];
    f.baseline
        .findings
        .push(finding("taint_source", "original.py", 2));
    f.baseline.evidence.push(None);
    f.source("original.py", 2, Some(call("ctx.py", 10)));
    let no_add = f.run().unwrap();
    assert_eq!(bytes(&no_add.result), bytes(&f.baseline));
    assert_eq!(
        format!("{:?}", no_add.result.evidence),
        format!("{:?}", f.baseline.evidence)
    );
    f.sink("ctx.py", 3, None, vec![]);
    f.expanded.findings.last_mut().unwrap().category = Some("unquoted_expansion".into());
    f.attribution.sinks.last_mut().unwrap().category = "unquoted_expansion".into();
    let out = f.run().unwrap();
    assert_prefix(&f.baseline, &out.result);
    assert_eq!(out.result.findings.len(), 4);
    assert_eq!(
        out.result.findings[3].category.as_deref(),
        Some("unquoted_expansion")
    );
}

#[test]
fn u04_source_needs_an_included_new_sink() {
    for kind in [
        "included",
        "diff-only",
        "baseline",
        "unlicensed",
        "not-emitted",
    ] {
        let mut f = Fixture::new();
        let relation = path(("original.py", 2), ("ctx.py", 5), WEAK);
        let anchor = if kind == "diff-only" {
            "original.py"
        } else {
            "ctx.py"
        };
        let licenses = if kind == "unlicensed" {
            vec![]
        } else {
            vec![license("original.py", 2, relation)]
        };
        f.sink(anchor, 5, None, licenses);
        if kind == "baseline" {
            f.baseline.findings.push(f.expanded.findings[0].clone());
            f.baseline.evidence.push(None);
        }
        if kind != "not-emitted" {
            f.source("original.py", 2, None);
        }
        let out = f.run().unwrap();
        let expected = usize::from(kind == "included");
        assert_eq!(out.stats.additional_sources, expected, "{kind}");
        assert_eq!(
            out.stats.dropped_sources,
            usize::from(kind != "included" && kind != "not-emitted"),
            "{kind}"
        );
        assert_eq!(
            out.result
                .findings
                .iter()
                .filter(|x| x.category.as_deref() == Some("taint_source"))
                .count(),
            expected,
            "{kind}"
        );
    }
}

#[test]
fn u05_source_uses_the_included_pair_not_global_best() {
    let mut f = Fixture::new();
    let mut weak = path(("original.py", 2), ("ctx.py", 4), WEAK);
    weak.hops
        .extend(path(("ctx.py", 4), ("ctx.py", 8), EXACT).hops);
    let exact = path(("original.py", 2), ("original.py", 9), EXACT);
    assert_ne!(
        format!("{weak:?}"),
        format!("{exact:?}"),
        "inadmissible precondition: pair witnesses differ"
    );
    f.sink(
        "ctx.py",
        8,
        Some(weak.clone()),
        vec![license("original.py", 2, weak.clone())],
    );
    f.sink(
        "original.py",
        9,
        Some(exact.clone()),
        vec![license("original.py", 2, exact.clone())],
    );
    f.source("original.py", 2, Some(exact));
    let out = f.run().unwrap();
    assert_eq!(out.result.findings.len(), 2);
    assert_eq!(witness(&out.result.evidence[1]), witness(&Some(weak)));
    assert_eq!(out.result.findings[1].related_files, vec!["ctx.py"]);
    assert_eq!(out.stats.dropped_sinks, 1);
}

#[test]
fn u06_rank_and_first_emission_ties_preserve_actual_multihop() {
    let empty = EvidencePath::default();
    let weak = path(("original.py", 2), ("ctx.py", 5), WEAK);
    let mut exact = path(("original.py", 2), ("ctx.py", 6), EXACT);
    exact
        .hops
        .extend(path(("ctx.py", 6), ("zctx.py", 7), EXACT).hops);
    for (first, second, expected) in [
        (empty.clone(), weak.clone(), weak.clone()),
        (weak.clone(), exact.clone(), exact.clone()),
        (exact.clone(), weak.clone(), exact.clone()),
        (
            exact.clone(),
            path(("original.py", 2), ("ctx.py", 80), EXACT),
            exact.clone(),
        ),
        (
            weak.clone(),
            path(("original.py", 2), ("ctx.py", 81), WEAK),
            weak.clone(),
        ),
        (
            empty.clone(),
            EvidencePath::unlabeled_for("taint"),
            empty.clone(),
        ),
    ] {
        let mut f = Fixture::new();
        f.sink(
            "ctx.py",
            5,
            Some(second.clone()),
            vec![license("original.py", 2, second)],
        );
        f.sink(
            "zctx.py",
            7,
            Some(first.clone()),
            vec![
                license("original.py", 2, first.clone()),
                license("original.py", 2, first),
            ],
        );
        f.attribution.sinks.swap(0, 1); // emission order is deliberately opposite to result/key order.
        f.source("original.py", 2, None);
        let out = f.run().unwrap();
        assert_eq!(
            witness(out.result.evidence.last().unwrap()),
            witness(&Some(expected))
        );
        assert_eq!(
            out.result.findings.last().unwrap().related_files,
            vec!["zctx.py", "ctx.py"]
        );
    }
}

#[test]
fn u07_empty_unlabeled_and_route_evidence_are_never_upgraded() {
    let mut crossed = path(("original.py", 2), ("ctx.py", 7), EXACT);
    crossed.crossed_unlabeled = true;
    for evidence in [
        EvidencePath::default(),
        EvidencePath::unlabeled_for("taint"),
        crossed,
    ] {
        for route in [
            LicenseRoute::Path,
            LicenseRoute::Fallback,
            LicenseRoute::Unquoted,
        ] {
            let mut f = Fixture::new();
            let mut pair = license("original.py", 2, evidence.clone());
            pair.route = route;
            f.sink("ctx.py", 7, Some(evidence.clone()), vec![pair]);
            f.source("original.py", 2, Some(call("ctx.py", 20)));
            let out = f.run().unwrap();
            assert_eq!(
                witness(&out.result.evidence[0]),
                witness(&Some(evidence.clone()))
            );
            assert_eq!(
                witness(&out.result.evidence[1]),
                witness(&Some(evidence.clone()))
            );
        }
    }
}

#[test]
fn u08_wording_follows_anchor_and_retains_source_text() {
    for anchor in ["ctx.py", "original.py"] {
        let mut f = Fixture::new();
        let i = f.sink(
            anchor,
            8,
            Some(path(("zctx.py", 5), ("ctx.py", 7), EXACT)),
            vec![],
        );
        f.expanded.findings[i].related_files =
            vec!["ctx.py".into(), "zctx.py".into(), "ctx.py".into()];
        let before = f.expanded.findings[i].description.clone();
        let out = f.run().unwrap();
        let suffix = if anchor == "ctx.py" {
            OUTSIDE
        } else {
            " (witness passes through taint context: ctx.py, zctx.py)"
        };
        assert_eq!(
            out.result.findings[0].description,
            format!("{before}{suffix}")
        );
        assert!(!out.result.findings[0]
            .description
            .contains("outside the diff"));
        assert_eq!(
            out.result.findings[0].description.matches(suffix).count(),
            1
        );
    }
    let mut related = Fixture::new();
    related.sink("original.py", 8, None, vec![]);
    related.expanded.findings[0].related_files = vec!["zctx.py".into()];
    assert!(related.run().unwrap().result.findings[0]
        .description
        .ends_with(" (witness passes through taint context: zctx.py)"));
}

#[test]
fn u09_related_files_and_backlinks_agree_with_admitted_maps() {
    let mut f = Fixture::new();
    let mut evidence = path(("other.py", 2), ("ctx.py", 5), EXACT);
    evidence
        .hops
        .extend(path(("foreign.py", 4), ("original.py", 6), EXACT).hops);
    evidence.hops.extend(call("zctx.py", 12).hops);
    evidence.hops.extend(call("other.py", 13).hops);
    f.sink(
        "ctx.py",
        9,
        Some(evidence.clone()),
        vec![license("original.py", 2, evidence.clone())],
    );
    f.sink(
        "zctx.py",
        10,
        None,
        vec![license("original.py", 2, evidence)],
    );
    f.sink(
        "other.py",
        11,
        None,
        vec![license("original.py", 2, EvidencePath::default())],
    );
    f.source("original.py", 2, None);
    let out = f.run().unwrap();
    assert_eq!(
        out.result.findings[0].related_files,
        vec!["original.py", "other.py", "zctx.py"]
    );
    assert_eq!(
        out.result.findings[2].related_files,
        vec!["ctx.py", "zctx.py"]
    );
    assert!(out.result.blocks.iter().all(|b| b.file != "foreign.py"));
    let z = out
        .result
        .blocks
        .iter()
        .find(|b| b.file == "zctx.py")
        .unwrap();
    assert_eq!(z.file_line_map["zctx.py"][&12], false);
    assert!(!z.file_line_map["zctx.py"].contains_key(&1)); // no fabricated call target/start line.
    let mut foreign = Fixture::new();
    foreign.sink(
        "ctx.py",
        5,
        Some(path(("foreign.py", 2), ("ctx.py", 5), EXACT)),
        vec![license("foreign.py", 2, EvidencePath::default())],
    );
    assert!(foreign.run().unwrap().result.findings[0]
        .related_files
        .is_empty());
}

#[test]
fn u10_every_licensed_emitted_source_is_added_once() {
    for selected in [true, false] {
        let mut f = Fixture::new();
        let licenses = vec![
            license("original.py", 2, EvidencePath::default()),
            license("original.py", 3, EvidencePath::default()),
            license("ctx.py", 4, EvidencePath::default()),
        ];
        f.sink("ctx.py", 9, None, licenses.clone());
        f.sink("ctx.py", 10, None, licenses);
        if !selected {
            for s in &mut f.attribution.sinks {
                s.selected_source = None;
            }
        }
        for (file, line) in [("original.py", 2), ("original.py", 3), ("ctx.py", 4)] {
            f.source(file, line, None);
        }
        let out = f.run().unwrap();
        assert_eq!(out.stats.additional_sources, 3);
        assert_eq!(out.result.findings.len(), 5);
        assert_eq!(out.result.findings[2].related_files, vec!["ctx.py"]);
        assert_eq!(out.result.findings[3].related_files, vec!["ctx.py"]);
        assert!(out.result.findings[4].related_files.is_empty());
    }
}
