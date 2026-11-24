//! R01-R15: CONTROLLED union-map/result assembly over REAL parsed source files.
//! No expansion engine runs here; source/witness/backlink shapes are explicit fixture inputs.
use super::*;
use crate::api::map_view::MapView;
use crate::output::mermaid::format_mermaid_report;
use crate::output::review_compact::{to_compact_review_output, to_compact_review_output_in};
use prism::ast::{parse_quality::check_parse_quality, ParsedFile};
use prism::call_graph::FunctionId;
use prism::diff::ModifyType;
use prism::finding_confidence::{EvidenceHop, EvidencePath};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge};
use prism::slice::{DiagramWarningKind, GraphShape, MultiSliceResult, SlicingAlgorithm};
use std::collections::BTreeSet;

const MAIN: &str = "def original(value):\n    source = value\n    return source\n";
const CONTEXT: &str =
    "def contextual(value):\n    result = value\n    sink(result)\n    return result\n";
const DEGRADED: &str =
    "def contextual(value):\n    first = value\n    broken =\n    return first\n";

fn graph(name: &str) -> SliceGraph {
    SliceGraph {
        title: Some(format!("{name} diagram")),
        shape: GraphShape::Chain,
        nodes: vec![],
        edges: vec![],
        clusters: vec![],
        mermaid: format!("flowchart TD\n  a[{name}]\n"),
    }
}
fn finding(file: &str, line: usize, category: &str, severity: &str) -> SliceFinding {
    SliceFinding {
        algorithm: "taint".into(),
        file: file.into(),
        line,
        severity: severity.into(),
        description: format!("controlled {category} at {file}:{line}"),
        function_name: Some("f".into()),
        related_lines: vec![],
        related_files: vec![],
        category: Some(category.into()),
        parse_quality: None,
        diagrams: vec![],
    }
}
fn block(id: usize, file: &str, lines: &[(usize, bool)]) -> DiffBlock {
    let mut block = DiffBlock::new(id, file.into(), ModifyType::Modified);
    for (line, diff) in lines {
        block.add_line(file, *line, *diff);
    }
    block
}
fn evidence(file: &str, line: usize) -> EvidencePath {
    EvidencePath::from_hops(vec![EvidenceHop::Call {
        edge: ResolvedCallEdge {
            caller: FunctionId {
                file: file.into(),
                name: "f".into(),
                start_line: 1,
                end_line: 4,
            },
            call_site_line: line,
            confidence: ResolutionConfidence::Exact,
            kind: ResolutionKind::FreeSingle,
        },
        confidence: ResolutionConfidence::Exact,
    }])
}
fn bytes<T: serde::Serialize + ?Sized>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn multi(result: SliceResult) -> MultiSliceResult {
    MultiSliceResult {
        version: "controlled".into(),
        algorithms_run: vec!["Taint".into()],
        results: vec![result],
        findings: vec![],
        errors: vec![],
        warnings: vec![],
        parse_quality: BTreeMap::new(),
        diagram_warnings: vec![],
    }
}
struct Fixture {
    files: BTreeMap<String, ParsedFile>,
    original_files: BTreeMap<String, ParsedFile>,
    original_quality: BTreeMap<String, FileParseQuality>,
    original_sources: BTreeMap<String, String>,
    sources: BTreeMap<String, String>,
    quality: BTreeMap<String, FileParseQuality>,
    context: BTreeSet<String>,
    baseline: SliceResult,
    result: SliceResult,
}
impl Fixture {
    fn new() -> Self {
        let original_sources: BTreeMap<String, String> =
            BTreeMap::from([("main.py".into(), MAIN.into())]);
        let sources: BTreeMap<String, String> = BTreeMap::from([
            ("main.py".into(), MAIN.into()),
            ("context.py".into(), CONTEXT.into()),
        ]);
        let files = sources
            .iter()
            .map(|(file, source)| {
                (
                    file.clone(),
                    ParsedFile::parse(file, source, Language::Python).unwrap(),
                )
            })
            .collect();
        let (_, quality) = check_parse_quality(&files);
        let original_files = BTreeMap::from([("main.py".to_string(), files["main.py"].clone())]);
        let original_quality = check_parse_quality(&original_files).1;
        let mut baseline = SliceResult::new(SlicingAlgorithm::Taint);
        baseline.blocks = vec![
            block(0, "main.py", &[(2, true), (3, false)]),
            block(3, "main.py", &[(1, false)]),
        ];
        baseline
            .findings
            .push(finding("main.py", 3, "taint_sink", "warning"));
        baseline.evidence.push(Some(evidence("main.py", 3)));
        baseline.warnings.push("ordinary warning".into());
        baseline.diagrams.push(graph("baseline"));
        baseline.diagram_warnings.push(DiagramWarning {
            algorithm: "Taint".into(),
            graph_title: Some("baseline diagram".into()),
            kind: DiagramWarningKind::LabelTruncated,
            detail: "baseline warning".into(),
        });
        let mut result = baseline.clone();
        result
            .blocks
            .push(block(4, "context.py", &[(2, false), (3, true), (4, false)]));
        let mut source = finding("main.py", 2, "taint_source", "info");
        source.related_files.push("context.py".into());
        let mut sink = finding("context.py", 3, "taint_sink", "warning");
        sink.description = "tainted value from main.py:2 (sink file outside the analysed file set; loaded as taint context)".into();
        sink.related_files.push("main.py".into());
        sink.diagrams.push(graph("context"));
        result.findings.extend([source, sink]);
        result.evidence.extend([
            Some(evidence("main.py", 2)),
            Some(evidence("context.py", 3)),
        ]);
        Self {
            files,
            original_files,
            original_quality,
            original_sources,
            sources,
            quality,
            context: BTreeSet::from(["context.py".into()]),
            baseline,
            result,
        }
    }
    fn view(&self) -> MapView<'_> {
        MapView::new(&self.files, &self.sources, &self.quality)
    }
    fn original(&self) -> MapView<'_> {
        MapView::new(
            &self.original_files,
            &self.original_sources,
            &self.original_quality,
        )
    }
    fn json(&self) -> ReviewOutput {
        to_review_output_in(&self.result, self.view(), Some(&self.context))
    }
    fn compact(
        &self,
        floor: u8,
        full: bool,
        no_diagrams: bool,
    ) -> crate::output::review_compact::CompactReviewOutput {
        to_compact_review_output_in(
            &self.result,
            self.view(),
            Some(&self.context),
            floor,
            full,
            no_diagrams,
        )
    }
    fn guard_sources(&self) {
        assert_eq!(
            self.files["main.py"].source, MAIN,
            "inadmissible precondition: original parsed bytes"
        );
        assert_eq!(
            self.sources["context.py"], CONTEXT,
            "inadmissible precondition: admitted context bytes"
        );
        assert_eq!(
            self.result
                .blocks
                .iter()
                .map(|b| b.block_id)
                .collect::<Vec<_>>(),
            vec![0, 3, 4],
            "inadmissible precondition: expansion-shaped blocks exist"
        );
    }
}

#[test]
fn r01_in_form_equals_the_legacy_form_without_context() {
    let f = Fixture::new();
    f.guard_sources();
    assert_eq!(
        bytes(&to_review_output_in(&f.baseline, f.original(), None)),
        bytes(&to_review_output(&f.baseline, &f.original_sources))
    );
    for floor in [0, 2, 3] {
        for full in [false, true] {
            for no_diagrams in [false, true] {
                assert_eq!(
                    bytes(&to_compact_review_output_in(
                        &f.baseline,
                        f.original(),
                        None,
                        floor,
                        full,
                        no_diagrams
                    )),
                    bytes(&to_compact_review_output(
                        &f.baseline,
                        &f.original_sources,
                        floor,
                        full,
                        no_diagrams
                    ))
                );
            }
        }
    }
}

#[test]
fn r02_context_block_text_is_the_context_source() {
    let f = Fixture::new();
    f.guard_sources();
    let wrong = to_review_output(&f.result, &f.original_sources);
    assert!(
        wrong.slices[2].slice_text.is_empty() && wrong.slices[2].slice_lines.is_empty(),
        "inadmissible precondition: original sources have no context block text"
    );
    let out = f.json();
    assert_eq!(
        out.slices[2].slice_text,
        "    2|    result = value\n+   3|    sink(result)\n    4|    return result\n"
    );
    assert_eq!(out.slices[2].slice_lines, vec![2, 3, 4]);
    assert_eq!(
        f.compact(2, true, false).slices[2].slice_text,
        out.slices[2].slice_text
    );
}

#[test]
fn r03_baseline_block_bytes_do_not_move() {
    let f = Fixture::new();
    f.guard_sources(); // Independent actual input guard, NOT a rendered-text precondition.
    let original = to_review_output_in(&f.result, f.original(), None);
    let union = f.json();
    let expected = "+   2|    source = value\n    3|    return source\n";
    assert_eq!(
        original.slices[0].slice_text, expected,
        "baseline rendered text behavioral oracle"
    );
    assert_eq!(bytes(&original.slices[..2]), bytes(&union.slices[..2]));
    assert_eq!(original.slices[0].block_id, 0);
    let original = to_compact_review_output_in(&f.result, f.original(), None, 0, true, false);
    let union = f.compact(0, true, false);
    assert_eq!(original.slices[0].slice_text, expected);
    assert_eq!(bytes(&original.slices[..2]), bytes(&union.slices[..2]));
}

#[test]
fn r04_block_identity_and_order() {
    let f = Fixture::new();
    f.guard_sources();
    assert_eq!(
        f.json()
            .slices
            .iter()
            .map(|b| (b.block_id, b.file.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "main.py"), (3, "main.py"), (4, "context.py")]
    );
    assert_eq!(
        f.compact(0, true, false)
            .slices
            .iter()
            .map(|b| b.block_id)
            .collect::<Vec<_>>(),
        vec![0, 3, 4]
    );
}

#[test]
fn r05_multi_file_block_lists_cross_file_refs() {
    let mut f = Fixture::new();
    f.result.blocks[2].add_line("main.py", 2, false);
    let out = f.json();
    assert!(out.slices[2]
        .slice_text
        .contains("--- context.py\n    2|    result = value\n"));
    assert!(out.slices[2]
        .slice_text
        .contains("--- main.py\n    2|    source = value\n"));
    assert_eq!(out.slices[2].cross_file_refs, vec!["main.py"]);
    let compact = f.compact(0, true, false);
    assert_eq!(compact.slices[2].slice_text, out.slices[2].slice_text);
    assert_eq!(compact.slices[2].cross_file_refs, vec!["main.py"]);
}

#[test]
fn r06_source_and_sink_line_annotations() {
    let f = Fixture::new();
    let out = f.json();
    assert!(out.slices[0].slice_text.contains("source = value"));
    assert!(out.slices[2].slice_text.contains("sink(result)"));
    assert_eq!(out.slices[0].source_lines, vec![2]);
    assert_eq!(out.slices[2].sink_lines, vec![3]);
    let compact = f.compact(2, true, false); // info source is hidden, its annotation remains.
    assert!(compact.slices[2].slice_text.contains("sink(result)"));
    assert_eq!(compact.slices[0].source_lines, vec![2]);
    assert_eq!(compact.slices[2].sink_lines, vec![3]);
    assert!(compact
        .findings
        .iter()
        .all(|f| f.category.as_deref() != Some("taint_source")));
}

#[test]
fn r07_per_result_parse_quality_is_context_only() {
    let mut f = Fixture::new();
    for file in ["original_bad.py", "context_bad.py"] {
        f.files.insert(
            file.into(),
            ParsedFile::parse(file, DEGRADED, Language::Python).unwrap(),
        );
        f.sources.insert(file.into(), DEGRADED.into());
    }
    f.original_files
        .insert("original_bad.py".into(), f.files["original_bad.py"].clone());
    f.original_sources
        .insert("original_bad.py".into(), DEGRADED.into());
    f.original_quality = check_parse_quality(&f.original_files).1;
    f.quality = check_parse_quality(&f.files).1;
    assert!(
        f.quality.contains_key("original_bad.py")
            && f.quality.contains_key("context_bad.py")
            && !f.quality.contains_key("context.py"),
        "inadmissible precondition: sparse actual degraded/clean parse entries"
    );
    for file in ["original_bad.py", "context_bad.py"] {
        assert!(
            f.files[file].error_rate() > 0.01 && f.files[file].error_rate() <= 0.3,
            "inadmissible precondition: admitted degraded parse"
        );
    }
    f.context.insert("context_bad.py".into());
    let expected = BTreeMap::from([(
        "context_bad.py".to_string(),
        f.quality["context_bad.py"].clone(),
    )]);
    let original_quality = bytes(&f.original_quality);
    assert_eq!(bytes(&f.json().parse_quality), bytes(&expected));
    assert_eq!(
        bytes(&f.compact(2, true, false).parse_quality),
        bytes(&expected)
    );
    assert!(
        !serde_json::to_value(to_review_output_in(&f.result, f.view(), None))
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("parse_quality")
    );
    assert!(!serde_json::to_value(to_compact_review_output_in(
        &f.result,
        f.view(),
        None,
        2,
        true,
        false
    ))
    .unwrap()
    .as_object()
    .unwrap()
    .contains_key("parse_quality"));
    assert_eq!(bytes(&f.original_quality), original_quality);
    let clean_context = BTreeSet::from(["context.py".into()]);
    assert!(
        to_review_output_in(&f.result, f.view(), Some(&clean_context))
            .parse_quality
            .is_empty()
    );
    assert!(
        to_compact_review_output_in(&f.result, f.view(), Some(&clean_context), 2, true, false)
            .parse_quality
            .is_empty()
    );
}

#[test]
fn r08_findings_pass_through_unchanged() {
    let f = Fixture::new();
    f.guard_sources();
    let evidence_before = format!("{:?}", f.result.evidence);
    assert_eq!(bytes(&f.json().findings), bytes(&f.result.findings));
    assert_eq!(
        bytes(&f.compact(0, true, false).findings),
        bytes(&f.result.findings)
    );
    assert_eq!(format!("{:?}", f.result.evidence), evidence_before);
    assert_eq!(f.result.findings[1].related_files, vec!["context.py"]);
    assert_eq!(f.result.findings[2].related_files, vec!["main.py"]);
    assert!(f.result.findings[2]
        .description
        .contains("main.py:2 (sink file outside the analysed file set; loaded as taint context)"));
    assert_eq!(f.result.evidence[2].as_ref().unwrap().hops.len(), 1);
    assert!(
        matches!(&f.result.evidence[2].as_ref().unwrap().hops[0], EvidenceHop::Call { edge, .. } if edge.caller.file == "context.py" && edge.call_site_line == 3)
    );
}

#[test]
fn r09_compact_floor_and_retention_see_context_blocks() {
    let mut f = Fixture::new();
    let retained = f.compact(2, false, false);
    assert_eq!(retained.findings.len(), 2);
    assert_eq!(
        retained
            .slices
            .iter()
            .map(|b| b.block_id)
            .collect::<Vec<_>>(),
        vec![0, 4]
    );
    assert!(retained.slices[1].slice_text.contains("sink(result)"));
    f.result.findings[2].severity = "suggestion".into();
    let hidden = f.compact(2, false, false);
    assert_eq!(hidden.findings.len(), 1);
    assert_eq!(hidden.findings_below_floor, 1);
    assert_eq!(
        hidden.slices.iter().map(|b| b.block_id).collect::<Vec<_>>(),
        vec![0]
    );
    let full = f.compact(2, true, false);
    assert_eq!(
        full.slices.iter().map(|b| b.block_id).collect::<Vec<_>>(),
        vec![0, 3, 4]
    );
    assert!(full.slices[2].slice_text.contains("sink(result)"));
    assert_eq!(full.findings_below_floor, 1);
}

#[test]
fn r10_compact_no_diagrams_strips_context_findings_too() {
    let f = Fixture::new();
    assert!(
        !f.result.findings[2].diagrams.is_empty(),
        "inadmissible precondition: context diagram exists before consumer"
    );
    let stripped = f.compact(0, true, true);
    assert!(stripped.diagrams.is_empty());
    assert!(stripped.findings.iter().all(|f| f.diagrams.is_empty()));
    assert_eq!(
        bytes(&stripped.diagram_warnings),
        bytes(&f.result.diagram_warnings)
    );
    let retained = f.compact(0, true, false);
    assert_eq!(bytes(&retained.diagrams), bytes(&f.result.diagrams));
    assert_eq!(
        bytes(&retained.findings[2].diagrams),
        bytes(&f.result.findings[2].diagrams)
    );
}

#[test]
fn r11_text_renders_context_lines() {
    let f = Fixture::new();
    f.guard_sources();
    assert_eq!(
        format_slice_result(&f.result.blocks[2..], &f.original_sources),
        "# Block 4 [M] context.py\n",
        "inadmissible precondition: original view gives header only"
    );
    assert_eq!(format_slice_result(&f.result.blocks[2..], f.view().sources), "# Block 4 [M] context.py\n    2|    result = value\n+   3|    sink(result)\n    4|    return result\n");
}

#[test]
fn r12_text_baseline_bytes_equal() {
    let f = Fixture::new();
    f.guard_sources();
    let baseline = format_slice_result(&f.baseline.blocks, &f.original_sources);
    assert!(
        baseline.contains("source = value"),
        "inadmissible precondition: baseline text is meaningful without new adapter"
    );
    assert_eq!(
        format_slice_result(&f.baseline.blocks, f.view().sources),
        baseline
    );
    assert!(format_slice_result(&f.result.blocks, f.view().sources).starts_with(&baseline));
    assert!(format_slice_result(&f.result.blocks, &f.original_sources).starts_with(&baseline));
}

#[test]
fn r13_paper_carries_context_blocks() {
    let f = Fixture::new();
    f.guard_sources();
    let baseline = to_paper_format(&f.baseline.blocks);
    let union = to_paper_format(&f.result.blocks);
    assert_eq!(
        union.as_array().unwrap()[..2],
        baseline.as_array().unwrap()[..]
    );
    assert_eq!(
        union[2],
        serde_json::json!({"block_id":4,"file":"context.py","modify_type":"M","diff_lines":[3],"diff_list":{"context.py":{"2":0,"3":1,"4":0}}})
    );
}

#[test]
fn r14_mermaid_report_includes_a_context_finding_diagram() {
    let f = Fixture::new();
    assert!(
        !f.result.findings[2].diagrams.is_empty(),
        "inadmissible precondition: supplied added finding has diagram"
    );
    let baseline = format_mermaid_report(&multi(f.baseline.clone()));
    let union = format_mermaid_report(&multi(f.result.clone()));
    let prefix = baseline.split("## Diagnostics").next().unwrap();
    assert!(prefix.contains("a[baseline]"));
    assert!(union.starts_with(prefix));
    assert!(union.contains("### Finding 3: tainted value from main.py:2 (sink file outside the analysed file set; loaded as taint context) at context.py\n\n```mermaid\nflowchart TD\n  a[context]"));
    assert_eq!(
        union.split("## Diagnostics").nth(1),
        baseline.split("## Diagnostics").nth(1)
    );
}

#[test]
fn r15_mermaid_report_without_context_is_unchanged() {
    let f = Fixture::new();
    let report = format_mermaid_report(&multi(f.baseline.clone()));
    assert_eq!(report, "# Prism diagram report\n\n## Taint\n\n### baseline diagram\n\n```mermaid\nflowchart TD\n  a[baseline]\n\n```\n\n## Diagnostics\n\n- Taint/baseline diagram: LabelTruncated: baseline warning\n");
    assert_eq!(format_mermaid_report(&multi(f.baseline)), report);
}
