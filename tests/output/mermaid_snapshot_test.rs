use prism::output::format_mermaid_report;
use prism::slice::{
    EdgeStyle, GraphEdge, GraphNode, GraphShape, MultiSliceResult, NodeKind, SliceFinding,
    SliceGraph, SliceResult, SlicingAlgorithm,
};
use std::collections::BTreeMap;

#[test]
fn mermaid_report_snapshot_taint_finding() {
    let mut taint = SliceResult::new(SlicingAlgorithm::Taint);
    taint.findings.push(SliceFinding {
        algorithm: "Taint".to_string(),
        file: "foo.c".to_string(),
        line: 67,
        severity: "concern".to_string(),
        description: "tainted strcpy".to_string(),
        function_name: None,
        related_lines: vec![],
        related_files: vec![],
        category: Some("taint_sink".to_string()),
        parse_quality: None,
        diagrams: vec![SliceGraph {
            title: Some("Data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![
                GraphNode {
                    id: "src".to_string(),
                    label: "foo.c:42 read".to_string(),
                    kind: NodeKind::Source,
                    file: Some("foo.c".to_string()),
                    line: Some(42),
                },
                GraphNode {
                    id: "snk".to_string(),
                    label: "foo.c:67 strcpy".to_string(),
                    kind: NodeKind::Sink,
                    file: Some("foo.c".to_string()),
                    line: Some(67),
                },
            ],
            edges: vec![GraphEdge {
                from: "src".to_string(),
                to: "snk".to_string(),
                label: Some("tainted".to_string()),
                style: EdgeStyle::Solid,
            }],
            clusters: vec![],
            // Hand-rendered mermaid string for snapshot stability — changes in
            // the renderer surface as failures in per-template unit tests, not
            // here. This snapshot pins the formatter's markdown layout only.
            mermaid: "flowchart TD\n    src[\"foo.c:42 read\"]:::source\n    snk[\"foo.c:67 strcpy\"]:::sink\n    src -->|tainted| snk\n".to_string(),
        }],
    });
    let multi = MultiSliceResult {
        version: "snapshot".to_string(),
        algorithms_run: vec!["Taint".to_string()],
        results: vec![taint],
        findings: vec![],
        errors: vec![],
        warnings: vec![],
        parse_quality: BTreeMap::new(),
        diagram_warnings: vec![],
    };
    let actual = format_mermaid_report(&multi);
    let expected = include_str!("../fixtures/diagram_snapshot/snapshot.md");
    assert_eq!(
        actual, expected,
        "Snapshot mismatch — review changes and update fixture if intended"
    );
}
