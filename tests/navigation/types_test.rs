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
