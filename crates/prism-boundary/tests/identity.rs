use element::{schema_path, FieldId, MessageRoot, OperationId, PointerToken, SuccessKey};
use prism_boundary::{element, revision};
use revision::Format;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

#[path = "support/deployments.rs"]
mod deployments;
#[path = "support/routes.rs"]
mod routes;
#[allow(dead_code)]
#[path = "../src/schema/mod.rs"]
mod schema;
mod support;

use support::{fixture, index, json, row_index, rows};

#[test]
fn path_and_typed_pointer_identity() {
    for (raw, base, expected) in [
        ("/kiln", "/studio/v4", "/studio/v4/kiln"),
        ("/", "/studio", "/studio/"),
        ("/a//b", "", "/a//b"),
        ("/a/", "", "/a/"),
        ("/", "/", "/"),
        ("item/{id}", "/api/v1", "/api/v1/item/{}"),
        ("/f/{id}.json", "", "/f/{}.json"),
        ("/a/{x}{x}/b", "", "/a/{}{}/b"),
        (
            "/Entry/:id.json/%2F",
            "//Api//",
            "//Api///Entry/:id.json/%2F",
        ),
        ("/{id}/%7Bid%7D", "", "/{}/%7Bid%7D"),
    ] {
        assert_eq!(schema_path(raw, base, &[]).to_string(), expected);
    }
    let operation = OperationId::new("PUT", &schema_path("/answer/admin/api/report/", "", &[]));
    assert_eq!(operation.to_string(), "op:PUT /answer/admin/api/report/");
    let operation = OperationId::new("GET", &schema_path("/api/tokens", "", &[]));
    let root = MessageRoot::Response(operation, SuccessKey::Exact(200));
    assert_eq!(
        FieldId::new(
            root,
            vec![
                PointerToken::property("data"),
                PointerToken::ListItem,
                PointerToken::property("key")
            ]
        )
        .to_string(),
        "field:resp:GET /api/tokens:200#/data/[]/key"
    );
    let op = OperationId::new("POST", &schema_path("/a", "", &[]));
    let request = MessageRoot::Request(op);
    let mut encoded = BTreeSet::new();
    for (name, spelling) in [
        ("", ""),
        ("[]", "%5B%5D"),
        ("*", "%2A"),
        ("~1", "~01"),
        ("/", "~1"),
        ("%23", "%2523"),
        ("#", "%23"),
        ("a? b", "a%3F%20b"),
        ("é", "%C3%A9"),
        ("e\u{301}", "e%CC%81"),
        ("\0\t\n", "%00%09%0A"),
        ("a/b~#*[]", "a~1b~0%23%2A%5B%5D"),
        ("x", "x"),
    ] {
        let pointer = PointerToken::property(name);
        assert_eq!(pointer.to_string(), spelling);
        assert_eq!(PointerToken::decode(spelling).unwrap(), pointer);
        assert!(encoded.insert(spelling));
        assert_eq!(
            FieldId::new(request.clone(), vec![pointer]).to_string(),
            format!("field:req:POST /a#/{spelling}")
        );
    }
    assert_eq!(PointerToken::decode("[]").unwrap(), PointerToken::ListItem);
    assert_eq!(PointerToken::decode("*").unwrap(), PointerToken::MapValue);
    assert!(PointerToken::decode("%ZZ").is_err());
    assert!(PointerToken::decode("~9").is_err());
    assert_eq!(
        element::decode_fragment_pointer("/a~1b~0c/%2523/~01").unwrap(),
        ["a/b~c", "%23", "~1"]
    );
    assert!(element::decode_fragment_pointer("anchor").is_err());
    assert!(PointerToken::decode("%FF").is_err());
}

#[test]
fn json_matrix_operation_and_status_census() {
    let mut status: BTreeMap<_, _> = include_str!("projections/schema-status.tsv")
        .lines()
        .map(|line| line.split_once('\t').unwrap())
        .collect();
    assert_eq!(status.len(), 164);
    let mut count = 0;
    for row in rows().filter(|r| !r.id.starts_with("control-") && r.fixture.ends_with(".json")) {
        let first = row_index(&row);
        let mut inventory: Vec<_> = first.operations.values().cloned().collect();
        if row.id == "oas-server-variable-distinct-operations" {
            inventory.extend(
                json(&fixture(&row, Some("salt.openapi.json")))
                    .operations
                    .into_values(),
            );
        }
        inventory.sort_by_key(|operation| operation.id.clone());
        assert_eq!(
            inventory
                .iter()
                .map(|op| op.id.to_string())
                .collect::<Vec<_>>(),
            row.operations,
            "{}",
            row.id
        );
        let expected = status.remove(row.id).unwrap();
        let expected = if expected.is_empty() {
            vec![]
        } else {
            expected
                .split(',')
                .map(|keys| {
                    if keys == "-" {
                        vec![]
                    } else {
                        keys.split('/').collect()
                    }
                })
                .collect()
        };
        assert_eq!(inventory.len(), expected.len(), "{}", row.id);
        for (op, keys) in inventory.iter().zip(expected) {
            let observed: Vec<_> = op.responses.keys().map(String::as_str).collect();
            assert_eq!(observed, keys, "{}", row.id);
            let success: Vec<_> = keys
                .iter()
                .copied()
                .filter(|key| {
                    *key == "2XX" && !row.fixture.ends_with("swagger.json")
                        || key.len() == 3
                            && key.starts_with('2')
                            && key.bytes().all(|b| b.is_ascii_digit())
                })
                .collect();
            assert_eq!(
                op.success_keys
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                success,
                "{}",
                row.id
            );
            assert_eq!(
                op.default_response().is_some(),
                keys.contains(&"default"),
                "{}",
                row.id
            );
        }
        if row.id == "s1-oas31-default-success-witness" {
            assert!(first
                .diagnostics
                .iter()
                .any(|d| d.code == "fallback_success"));
        }
        if row.id == "swagger-duplicate-collision" {
            let op = &inventory[0];
            assert_eq!(
                op.declarations
                    .iter()
                    .map(|d| d.source_path.as_str())
                    .collect::<Vec<_>>(),
                ["/mold/{cast}", "/mold/{pattern}"]
            );
            assert!(first
                .diagnostics
                .iter()
                .any(|d| d.code == "duplicate_operation"));
        }
        count += 1;
    }
    assert_eq!(count, 164);
    assert!(status.is_empty());
}

#[test]
fn scoped_yaml_format_refusal() {
    let row = rows()
        .find(|r| r.id == "s1-oas31-yaml-status-keys")
        .unwrap();
    let bytes = fixture(&row, None);
    let error = index(&bytes, Format::Yaml, "test-api", None, &[]).unwrap_err();
    assert_eq!(error.scope(), "schema_format");
    assert_eq!(Some(error.code()), row.reason);
    assert_eq!(
        index(&bytes, Format::Json, "test-api", None, &[])
            .unwrap_err()
            .code(),
        "invalid_json"
    );
}

#[test]
fn schema_proto_operation_controls() {
    let slash = json(
        br#"{"swagger":"2.0","basePath":"/answer",
        "paths":{"/admin/api/report/":{"put":{"responses":{"204":{}}}}}}"#,
    );
    assert_eq!(
        slash.operations.keys().next().unwrap().to_string(),
        "op:PUT /answer/admin/api/report/"
    );
    for (source, ids) in [
        (
            "mini.swagger.json",
            &[
                "op:GET /api/v1/item",
                "op:GET /api/v1/item/{}",
                "op:HEAD /api/v1/status",
                "op:POST /api/v1/item",
            ][..],
        ),
        ("collision.json", &["op:GET /a/{}", "op:POST /a/{}"][..]),
        ("duplicate.json", &["op:GET /a/{}"][..]),
    ] {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(
            root.join("../../eval/schema_impact/tests/fixtures")
                .join(source),
        )
        .unwrap();
        let index = json(&bytes);
        assert_eq!(
            index
                .operations
                .keys()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ids,
            "{source}"
        );
        if source == "mini.swagger.json" {
            assert_eq!(index.info_version.as_deref(), Some("1.2.3"));
            let op = index
                .operations
                .values()
                .find(|op| op.operation_id.as_deref() == Some("getItem"))
                .unwrap();
            assert_eq!(op.declarations[0].source_path, "/item/{id}");
            assert_eq!(op.success_keys, [SuccessKey::Exact(200)]);
        }
        if source == "duplicate.json" {
            let op = index.operations.values().next().unwrap();
            assert_eq!(op.declarations[0].source_path, "/a/{id}");
            assert!(op.responses["200"].value.to_string().contains("first"));
            assert!(!op.responses["200"].value.to_string().contains("second"));
        }
    }
}

#[test]
fn status_precedence_and_bodyless_declarations() {
    let index = json(
        br#"{"openapi":"3.1.0","paths":{"/a":{
        "head":{"responses":{"204":{},"205":{},"2XX":{},"default":{}}},
        "get":{"responses":{"201":{},"204":{},"2XX":{},"default":{}}}}}}"#,
    );
    let get = index.operations.values().next().unwrap();
    assert_eq!(
        get.success_keys,
        [
            SuccessKey::Exact(201),
            SuccessKey::Exact(204),
            SuccessKey::Range
        ]
    );
    assert_eq!(get.response_key(201), Some("201"));
    assert_eq!(get.response_key(202), Some("2XX"));
    assert_eq!(get.response_key(302), Some("default"));
    assert!(!get.can_have_body(204));
    assert!(!get.can_have_body(205));
    assert!(get.can_have_body(201));
    let head = index.operations.values().last().unwrap();
    assert_eq!(
        head.success_keys,
        [
            SuccessKey::Exact(204),
            SuccessKey::Exact(205),
            SuccessKey::Range
        ]
    );
    assert!(!head.can_have_body(201));
    assert!(head.default_response().is_some());
}

#[test]
fn binary_help_and_version() {
    for (flag, expected) in [
        ("--help", "Usage: prism-boundary"),
        (
            "--version",
            concat!("prism-boundary ", env!("CARGO_PKG_VERSION")),
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_prism-boundary"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(output.status.success(), "{flag}");
        assert!(
            String::from_utf8(output.stdout).unwrap().contains(expected),
            "{flag}"
        );
        assert!(output.stderr.is_empty(), "{flag}");
    }
}
