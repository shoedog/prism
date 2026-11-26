use crate::schema::Inventory;
use crate::support::{index, json, rows};
use prism_boundary::element::route::{project_route, Limit};
use prism_boundary::revision::{Blob, Format, ReferenceBlob};
use serde_json::json;

fn ids(inventory: &Inventory) -> Vec<String> {
    inventory
        .operations
        .keys()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn referring_path_servers_survive_reference() {
    let inventory = json(
        br##"{"openapi":"3.1.0","servers":[{"url":"/v1"}],
        "paths":{"/reed":{"$ref":"#/components/pathItems/Reed","servers":[{"url":"/v2"}]}},
        "components":{"pathItems":{"Reed":{"get":{"responses":{"200":{}}}}}}}"##,
    );
    assert_eq!(ids(&inventory), ["op:GET /v2/reed"]);
    let declaration = &inventory.operations.values().next().unwrap().declarations[0];
    assert_eq!(
        declaration.site.pointer,
        ["components", "pathItems", "Reed", "get"]
    );
    assert_eq!(
        declaration.deployments[0].site.pointer,
        ["paths", "/reed", "servers", "0"]
    );
}

#[test]
fn external_path_item_keeps_each_fields_declaring_document() {
    let parts = br#"{"reed":{"get":{"parameters":[{"in":"path","name":"label",
        "style":"label","schema":{"type":"string"}}],
        "servers":[{"url":"api"}],"responses":{"200":{}}}}}"#;
    let reference = ReferenceBlob {
        retrieval_uri: "https://example.test/root/shared/parts.json",
        blob: Blob {
            path: "shared/parts.json",
            tree: "test-tree",
            oid: "shared-blob",
            bytes: parts,
            format: Format::Json,
        },
    };
    let inventory = index(
        br#"{"openapi":"3.1.0","paths":{"/reed/{label}":{
            "$ref":"shared/parts.json#/reed","servers":[{"url":"api"}],
            "post":{"responses":{"201":{}}}}}}"#,
        Format::Json,
        "test-api",
        Some("https://example.test/root/openapi.json"),
        &[reference],
    )
    .unwrap();
    assert_eq!(
        ids(&inventory),
        [
            "op:GET /root/shared/api/reed/.{}",
            "op:POST /root/api/reed/{}"
        ]
    );
    let mut operations = inventory.operations.values();
    for blob in ["shared-blob", "entry-blob"] {
        let declaration = &operations.next().unwrap().declarations[0];
        assert_eq!(declaration.site.blob, blob);
        assert_eq!(declaration.deployments[0].site.blob, blob);
    }
}

#[test]
fn overlapping_path_item_fields_only_limit_dependent_operations() {
    let inventory = json(br##"{"openapi":"3.1.0","paths":{"/reed":{
        "$ref":"#/items/reed","servers":[{"url":"/local"}],"get":{},
        "post":{"servers":[{"url":"/override"}]} }},
        "items":{"reed":{"servers":[{"url":"/target"}],"get":{},"put":{"responses":{"202":{}}}}}}"##);
    assert_eq!(ids(&inventory), ["op:POST /override/reed"]);
    assert_eq!(inventory.unprojected[0].source_method, "put");
    let overlaps: Vec<_> = inventory
        .diagnostics
        .iter()
        .filter(|d| d.code == "path_item_field_unchecked")
        .map(|d| d.site.pointer.last().unwrap().as_str())
        .collect();
    assert_eq!(overlaps, ["get", "servers"]);
}

#[test]
fn unprojected_operation_retains_structured_responses() {
    let inventory = json(
        br#"{"openapi":"3.1.0","paths":{"/reed":{"get":{
        "servers":[{"url":"api"}],"responses":{"201":{"description":"Created"}}}}}}"#,
    );
    assert!(inventory.operations.is_empty());
    let declaration = &inventory.unprojected[0];
    assert_eq!(
        declaration.value["responses"]["201"]["description"],
        "Created"
    );
    assert_eq!(
        declaration.success_keys,
        [prism_boundary::element::SuccessKey::Exact(201)]
    );
    let response = &declaration.responses["201"];
    assert_eq!(
        response.site.pointer,
        ["paths", "/reed", "get", "responses", "201"]
    );
    assert_eq!(response.value["description"], "Created");
}

#[test]
fn shared_unresolved_alias_is_scoped_to_each_referring_path() {
    let bytes = br##"{"openapi":"3.1.0","paths":{
        "/a":{"$ref":"#/items/alias"},"/b":{"$ref":"#/items/alias"},
        "/control":{"get":{}}},"items":{"alias":{"$ref":"missing.json#/reed"}}}"##;
    let inventory = index(
        bytes,
        Format::Json,
        "test-api",
        Some("https://example.test/openapi.json"),
        &[],
    )
    .unwrap();
    assert_eq!(ids(&inventory), ["op:GET /control"]);
    let missing: Vec<_> = inventory
        .diagnostics
        .iter()
        .filter(|d| d.code == "reference_blob_unchecked")
        .map(|d| {
            d.site
                .pointer
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(missing, [["paths", "/a"], ["paths", "/b"]]);
    for (path, diagnostic) in ["/a", "/b"].into_iter().zip(
        inventory
            .diagnostics
            .iter()
            .filter(|d| d.code == "reference_blob_unchecked"),
    ) {
        assert_eq!(diagnostic.scope, "path_item");
        assert_eq!(diagnostic.reference_chain.len(), 2);
        assert_eq!(
            diagnostic.reference_chain[0].pointer,
            ["paths", path, "$ref"]
        );
        assert_eq!(
            diagnostic.reference_chain[1].pointer,
            ["items", "alias", "$ref"]
        );
    }
}

fn gin(suffix: &str, prefixes: &[&str]) -> prism_boundary::element::route::Projection {
    let row = rows().find(|row| row.id == "gin-catchall").unwrap();
    let cases = crate::routes::projections();
    let mut declaration = crate::routes::projection(&row, &cases[row.id][0]);
    declaration.paths = vec![suffix.into()];
    declaration.prefixes = vec![prefixes.iter().map(|p| (*p).into()).collect()];
    project_route(&declaration)
}

#[test]
fn gin_empty_suffix_preserves_joined_group_path() {
    for (prefixes, expected) in [
        (&["/mugs/"][..], "op:GET /mugs/"),
        (&["/v1/"][..], "op:GET /v1/"),
        (&["/v1/", ""][..], "op:GET /v1/"),
        (&["/v1/", "pots"][..], "op:GET /v1/pots"),
    ] {
        let projection = gin("", prefixes);
        assert_eq!(projection.routes[0].id.to_string(), expected);
        assert!(projection.routes[0].public);
    }
    assert_eq!(
        gin("pots/", &["/v1/"]).routes[0].id.to_string(),
        "op:GET /v1/pots/"
    );
}

#[test]
fn gin_refuses_nonterminal_catch_all() {
    for suffix in ["/a/*tail/b", "/a/*tail/"] {
        let projection = gin(suffix, &[]);
        assert!(projection.routes.is_empty(), "{projection:?}");
        assert!(projection.limits.contains(&Limit::PathUnchecked));
    }
    assert_eq!(
        gin("/a/*tail", &[]).routes[0].id.to_string(),
        "op:GET /a/{*0+}"
    );
}

#[test]
fn empty_operation_servers_inherit_path_item_servers() {
    let inventory = json(
        br#"{"openapi":"3.1.0","servers":[{"url":"/root"}],
        "paths":{"/a":{"servers":[{"url":"/path"}],"get":{"servers":[],"responses":{"200":{}}}}}}"#,
    );
    assert_eq!(ids(&inventory), ["op:GET /path/a"]);
    let inventory = json(
        br#"{"openapi":"3.1.0","paths":{"/a":{"get":{"servers":[],"responses":{"200":{}}}}}}"#,
    );
    assert_eq!(ids(&inventory), ["op:GET /a"]);
}

#[test]
fn empty_path_item_servers_inherit_document_servers() {
    let inventory = json(
        br#"{"openapi":"3.1.0","servers":[{"url":"/root"}],
        "paths":{"/a":{"servers":[],"get":{"responses":{"200":{}}}}}}"#,
    );
    assert_eq!(ids(&inventory), ["op:GET /root/a"]);
}

#[test]
fn nonempty_server_list_without_deployments_is_unchecked() {
    let inventory = json(
        br#"{"openapi":"3.1.0","servers":[{"url":"/root"}],"paths":{
        "/a":{"get":{"servers":[{"url":"/v{missing}"}]}},
        "/b":{"servers":[{"url":"relative"}],"get":{}},
        "/control":{"get":{"servers":[{"url":"/good"},{"url":"relative"}]}}}}"#,
    );
    assert_eq!(ids(&inventory), ["op:GET /good/control"]);
    let sites: Vec<_> = inventory
        .diagnostics
        .iter()
        .filter(|d| d.code == "server_deployments_unchecked")
        .map(|d| {
            d.site
                .pointer
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        sites,
        [
            vec!["paths", "/a", "get", "servers"],
            vec!["paths", "/b", "servers"]
        ]
    );
}

#[test]
fn default_response_only_limits_uncovered_success_codes() {
    for limit in [298, 299] {
        let mut responses = serde_json::Map::from_iter(
            (200..=limit).map(|code: u16| (code.to_string(), json!({}))),
        );
        responses.insert("default".into(), json!({}));
        let inventory = json(
            serde_json::to_string(&json!({"openapi":"3.1.0",
            "paths":{"/a":{"get":{"responses":responses}}}}))
            .unwrap()
            .as_bytes(),
        );
        assert_eq!(
            inventory
                .diagnostics
                .iter()
                .any(|d| d.code == "fallback_success"),
            limit == 298
        );
    }
}
