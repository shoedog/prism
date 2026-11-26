use crate::support::{fixture, index, json, row_index, rows};
use prism_boundary::revision::Format;

#[test]
fn deployment_origins_and_withheld_inputs() {
    let row = rows()
        .find(|r| r.id == "t1-relative-server-document-base")
        .unwrap();
    let without = json(&fixture(&row, None));
    assert!(without.operations.is_empty());
    assert_eq!(without.unprojected.len(), 1);
    assert_eq!(without.unprojected[0].source_method, "get");
    assert!(without
        .diagnostics
        .iter()
        .any(|d| d.code == "retrieval_base_unchecked"));
    let row = rows()
        .find(|r| r.id == "s1-oas31-external-path-item-server")
        .unwrap();
    let without = index(
        &fixture(&row, None),
        Format::Json,
        "test-api",
        Some("https://moonspindle.invalid/plans/openapi.json"),
        &[],
    )
    .unwrap();
    assert!(without.operations.is_empty());
    assert!(without
        .diagnostics
        .iter()
        .any(|d| d.code == "reference_blob_unchecked"));
    let index = row_index(&row);
    let declaration = &index.operations.values().next().unwrap().declarations[0];
    assert_eq!(declaration.referring_site.blob, "entry-blob");
    assert_eq!(
        declaration.referring_site.pointer,
        ["paths", "/reed", "$ref"]
    );
    assert_eq!(
        declaration.site.pointer,
        ["components", "pathItems", "Wisp", "get"]
    );
    assert_eq!(
        declaration.reference_chain,
        [declaration.referring_site.clone()]
    );
    assert_eq!(declaration.site.blob, "path-item-blob");
    assert_eq!(
        declaration.deployments[0].origin.authority.as_deref(),
        Some("moonspindle.invalid")
    );
    let index = json(
        br#"{"swagger":"2.0","basePath":"/studio","host":"example.test:8443",
        "schemes":["http"],"paths":{"/":{"get":{"schemes":["https"]},"post":{}}}}"#,
    );
    let deployments: Vec<_> = index
        .operations
        .values()
        .map(|op| &op.declarations[0].deployments[0])
        .collect();
    assert_eq!(deployments[0].origin.scheme.as_deref(), Some("https"));
    assert_eq!(deployments[1].origin.scheme.as_deref(), Some("http"));
    assert_eq!(
        deployments[0].origin.authority.as_deref(),
        Some("example.test:8443")
    );
    let bytes = br#"{"openapi":"3.1.0","servers":[{"url":"../api//v{version}/","variables":{"version":{"default":"2"}}}],"paths":{"/a":{"get":{}}}}"#;
    let admitted = crate::support::index(
        bytes,
        Format::Json,
        "test-api",
        Some("https://example.test/catalog/spec/openapi.json"),
        &[],
    )
    .unwrap();
    assert_eq!(
        admitted.operations.keys().next().unwrap().to_string(),
        "op:GET /catalog/api//v2//a"
    );
    assert!(admitted
        .diagnostics
        .iter()
        .any(|d| d.code == "server_variable_unchecked"));
    assert!(json(bytes).operations.is_empty());
}

#[test]
fn authority_and_server_alternatives_preserve_provenance() {
    let row = rows()
        .find(|r| r.id == "s1-oas31-authority-dispatch")
        .unwrap();
    let mut ids = Vec::new();
    for (file, api) in [
        ("openapi.json", "wax.moonspindle.invalid"),
        ("pith.openapi.json", "pith.moonspindle.invalid"),
    ] {
        let bytes = fixture(&row, Some(file));
        let inventory = index(&bytes, Format::Json, api, None, &[]).unwrap();
        assert_eq!(inventory.api, api);
        let op = inventory.operations.values().next().unwrap();
        assert_eq!(
            op.declarations[0].deployments[0]
                .origin
                .authority
                .as_deref(),
            Some(api)
        );
        assert_eq!(
            op.success_keys,
            [prism_boundary::element::SuccessKey::Exact(200)]
        );
        ids.push(op.id.to_string());
    }
    assert_eq!(ids, ["op:GET /reed", "op:GET /reed"]);

    let inventory = json(br#"{"openapi":"3.1.0","servers":[
        {"url":"//network.example.test:8443/base"},
        {"url":"https://first.example.test/base"},
        {"url":"https://second.example.test/{name}","variables":{"name":{"default":"base","enum":["base","other"]}}}
    ],"paths":{"/a":{"get":{"responses":{"200":{}}}}}}"#);
    assert_eq!(
        inventory
            .operations
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["op:GET /base/a", "op:GET /other/a"]
    );
    let op = inventory.operations.values().next().unwrap();
    assert_eq!(op.declarations.len(), 1);
    let origins = op.declarations[0]
        .deployments
        .iter()
        .map(|d| (d.origin.scheme.as_deref(), d.origin.authority.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        origins,
        [
            (None, Some("network.example.test:8443")),
            (Some("https"), Some("first.example.test")),
            (Some("https"), Some("second.example.test"))
        ]
    );
    assert!(inventory.diagnostics.iter().any(|d| d.scope == "origin"));
    assert!(inventory
        .diagnostics
        .iter()
        .all(|d| d.code != "duplicate_operation"));
}
