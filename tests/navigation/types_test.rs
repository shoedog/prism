use prism::navigation::types::*;

#[test]
fn evidence_serializes_to_expected_shape() {
    let ev = Evidence {
        query: "nodes-at:a.py:2".into(),
        items: vec![EvidenceItem {
            symbol: Some(SymbolRef::Function {
                file: "a.py".into(),
                name: "f".into(),
                start_line: 1,
                end_line: 3,
                ordinal: 0,
            }),
            location: Location {
                file: "a.py".into(),
                start_line: 1,
                end_line: 3,
            },
            score: 1.0,
            source: Source::PrismCpg,
            fallback: false,
            why: vec![Reason::EnclosingFunction {
                function: SymbolRef::Function {
                    file: "a.py".into(),
                    name: "f".into(),
                    start_line: 1,
                    end_line: 3,
                    ordinal: 0,
                },
            }],
            snippet: None,
        }],
        truncated: false,
        warnings: vec![],
        graph: None,
        reasoning: None,
    };
    let v: serde_json::Value = serde_json::to_value(&ev).unwrap();
    assert_eq!(v["query"], "nodes-at:a.py:2");
    assert_eq!(v["items"][0]["score"], 1.0);
    assert_eq!(v["items"][0]["source"], "PrismCpg");
    assert_eq!(
        v["items"][0]["why"][0]["EnclosingFunction"]["function"]["Function"]["name"],
        "f"
    );
}

#[test]
fn evidence_without_graph_omits_key() {
    let ev = Evidence {
        query: "nodes-at:a.rs:1".into(),
        items: vec![],
        truncated: false,
        warnings: vec![],
        graph: None,
        reasoning: None,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(
        !json.contains("\"graph\""),
        "graph:None must be omitted so existing nav goldens stay byte-identical: {json}"
    );
}

#[test]
fn evidence_with_graph_serializes_payload() {
    let ev = Evidence {
        query: "repo-map".into(),
        items: vec![],
        truncated: false,
        warnings: vec![],
        graph: Some(GraphPayload {
            nodes: vec![GraphNode {
                symbol: None,
                location: Location {
                    file: "a.rs".into(),
                    start_line: 1,
                    end_line: 1,
                },
            }],
            edges: vec![GraphEdge {
                from: 0,
                to: 0,
                kind: "ModuleDep".into(),
            }],
        }),
        reasoning: None,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("\"graph\""));
    assert!(json.contains("\"ModuleDep\""));
}
