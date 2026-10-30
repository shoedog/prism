//! S3a1's additive surfaces, exercised without executing fixture source.
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::navigation::{inventory::functions_inventory, queries::call_site_dump};
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
#[ignore = "S3a1b"] // Requires graph attachment and the additive dump wire join.
fn anonymous_dump_adds_metadata_with_named_and_factory_controls() {
    for ext in ["js", "ts", "tsx"] {
        let file = format!("a.{ext}");
        let source = "function outer() { sink(() => hit()); }\n\
                      function named() { hit(); }\n\
                      const memo = useMemo(() => hit(), []);\n\
                      const callback = useCallback(() => hit(), []);";
        let files = BTreeMap::from([(
            file.clone(),
            ParsedFile::parse(&file, source, Language::from_path(&file).unwrap()).unwrap(),
        )]);
        let graph = CallGraph::build(&files);
        let dump = call_site_dump(&graph);
        let anonymous: Vec<_> = dump
            .iter()
            .filter(|row| {
                row["caller"]["name"]
                    .as_str()
                    .is_some_and(prism::anon_owner::is_anonymous_owner_name)
            })
            .collect();
        assert_eq!(anonymous.len(), 1, "fixture must have an anonymous caller");
        for row in &dump {
            if !row["caller"]["name"]
                .as_str()
                .is_some_and(prism::anon_owner::is_anonymous_owner_name)
            {
                assert!(row["caller"].get("anon").is_none());
            }
        }
        assert!(
            anonymous[0]["caller"]["anon"].is_object(),
            "{ext}: {anonymous:?}"
        );
    }
}

#[test]
#[ignore = "S3a1b"] // Requires inventory enrichment after legacy sibling dedup.
fn anonymous_inventory_adds_identity_after_legacy_sibling_dedup() {
    let dir = tempfile::tempdir().unwrap();
    for ext in ["js", "ts", "tsx"] {
        std::fs::write(
            dir.path().join(format!("a.{ext}")),
            "sink(() => hit(), () => hit());\nfunction named() {}\n\
             const memo = useMemo(() => hit(), []);\n\
             const callback = useCallback(() => hit(), []);",
        )
        .unwrap();
    }
    let records = functions_inventory(dir.path()).unwrap();
    // Same-line anonymous arrows collapse under the existing public key.
    assert_eq!(records.len(), 12);
    let rows = serde_json::to_value(records).unwrap();
    let rows = rows.as_array().unwrap();
    let anonymous: Vec<_> = rows
        .iter()
        .filter(|row| row["name"] == Value::Null)
        .collect();
    assert_eq!(anonymous.len(), 3);
    for row in rows.iter().filter(|row| row["name"] != Value::Null) {
        assert!(row.get("anon").is_none());
        assert!(row.get("function_id").is_none());
    }
    for row in anonymous {
        assert_eq!(row["kind"], "arrow_function");
        assert_eq!(row["start_line"], 1);
        assert_eq!(row["end_line"], 1);
        assert!(row["function_id"].is_object(), "{row}");
        assert!(row["anon"].is_object(), "{row}");
    }
}
