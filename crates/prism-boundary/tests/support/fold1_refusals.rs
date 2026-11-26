use super::{admission, assert_limit, fields, index, index_raw, response};
use prism_boundary::revision::{
    Blob, DependencyReceipt, Format, ProducerMediaEvidence, ReferenceBlob,
};
use prism_boundary::{index_schema, signature, Coverage, Property, RootKey};
use serde_json::{json, Value};

#[test]
fn duplicate_responses_withholds_the_collection() {
    let control = r#"{"openapi":"3.1.0","paths":{"/p":{"post":{
        "responses":{"200":{"content":{"application/json":{"schema":{
            "properties":{"x":{"type":"string"}}
        }}}}}
    }}}}"#;
    assert_eq!(
        fields(&index_raw(control.as_bytes())),
        ["field:resp:POST /p:200#/x"]
    );
    let duplicate = control.replace(r#""responses":"#, r#""responses":{"204":{}},"responses":"#);
    let result = index_raw(duplicate.as_bytes());
    let op = &result.operations[0];
    assert_eq!(
        op.messages[0].coverage,
        Coverage::Withheld,
        "duplicate responses must withhold collection"
    );
    assert_eq!(op.messages[0].key, RootKey::Fallback);
    assert!(op.success_keys.is_empty() && op.response_keys.is_empty());
    assert!(fields(&result).is_empty());
    assert_limit(&result, "duplicate_member");
}

#[test]
fn duplicate_request_and_parameter_declarations_withhold_the_request() {
    for control in [
        r#"{"openapi":"3.1.0","paths":{"/p":{"post":{"requestBody":{
            "content":{"application/json":{"schema":{"properties":{"x":{"type":"string"}}}}}
        }}}}}"#,
        r#"{"swagger":"2.0","consumes":["application/json"],"paths":{"/p":{"post":{"parameters":[{
            "in":"body","name":"data","schema":{"properties":{"x":{"type":"string"}}}
        }]}}}}"#,
    ] {
        assert_eq!(
            fields(&index_raw(control.as_bytes())),
            ["field:req:POST /p#/x"]
        );
        let duplicate = control
            .replace(r#""requestBody":"#, r#""requestBody":{},"requestBody":"#)
            .replace(r#""parameters":"#, r#""parameters":[],"parameters":"#)
            .replace(r#""name":"data""#, r#""name":"old","name":"data""#);
        let result = index_raw(duplicate.as_bytes());
        assert_eq!(
            result.operations[0].messages[0].coverage,
            Coverage::Withheld,
            "duplicate request declaration must withhold the request"
        );
        assert_limit(&result, "duplicate_member");
        assert!(fields(&result).is_empty());
    }
    let control = r#"{"swagger":"2.0","consumes":["application/json"],"paths":{"/p":{"post":{"parameters":[{
        "in":"body","name":"data","schema":{"properties":{"x":{"type":"string"}}}
    }]}}}}"#;
    assert_eq!(
        fields(&index_raw(control.as_bytes())),
        ["field:req:POST /p#/x"]
    );
    let parameter = control.replace(r#""name":"data""#, r#""name":"old","name":"data""#);
    assert_limit(&index_raw(parameter.as_bytes()), "duplicate_member");
}

#[test]
fn stopped_reference_shapes_are_unknown_and_signed_distinctly() {
    let schema = |field| response(json!({"type":"object","properties":{"x":field}}));
    let control = index(schema(json!({})));
    assert_eq!(
        signature(&control.fields[0].shape),
        r#"[["missing"],["missing"],["known",false],["missing"],["missing"]]"#
    );
    let assert_stopped = |stopped: prism_boundary::SchemaIndex, reason| {
        assert_limit(&stopped, reason);
        assert_eq!(
            signature(&stopped.fields[0].shape),
            r#"[["unknown"],["unknown"],["known",false],["unknown"],["unknown"]]"#,
            "{reason}: a stopped reference owns unknown signature slots"
        );
        assert_ne!(stopped.content_hash, control.content_hash, "{reason}");
    };
    for (reference, reason) in [
        ("#/$defs/Missing", "reference_unresolved"),
        ("missing.json", "retrieval_base_unchecked"),
        (
            "https://example.test/missing.json",
            "reference_blob_unavailable",
        ),
        ("#unsupported-anchor", "reference_scope"),
        (
            "#/paths/~1p/post/responses/200/content/application~1json/schema/properties/x",
            "recursive",
        ),
    ] {
        assert_stopped(index(schema(json!({"$ref":reference}))), reason);
    }
    let uri = "https://example.test/value.json";
    let bytes = serde_json::to_vec(&schema(json!({"$ref":uri}))).unwrap();
    let references = [
        ("first", br#"{"type":"string"}"#.as_slice()),
        ("second", br#"{"type":"number"}"#.as_slice()),
    ]
    .map(|(oid, bytes)| ReferenceBlob {
        retrieval_uri: uri,
        blob: Blob {
            path: "value.json",
            tree: "test-tree",
            oid,
            bytes,
            format: Format::Json,
        },
    });
    let indexed = |references| {
        index_schema(&admission::admitted(
            &bytes,
            Format::Json,
            "test-api",
            None,
            references,
        ))
        .unwrap()
    };
    assert_eq!(
        indexed(&references[..1]).fields[0].shape.field_type,
        Property::Known("string".into())
    );
    assert_stopped(indexed(&references), "reference_blob_ambiguous");

    let required = index(response(json!({
        "type":"object","required":["x"],"properties":{"x":{"$ref":"#/$defs/Missing"}}
    })));
    assert_eq!(
        required.observed_fields[0].shape.required,
        Property::Known(true)
    );
}

#[test]
fn infeasible_seventeenth_object_edge_does_not_count_a_cut() {
    let wrap = |leaf| {
        (0..16).fold(
            leaf,
            |child, _| json!({"type":"object","properties":{"p":child}}),
        )
    };
    let deep = |value| {
        index(response(wrap(json!({
            "type":"object","properties":{"child":{
                "type":"object","required":["z"],"properties":{"z":value}
            }},"required":["child"]
        }))))
    };
    let viable = deep(json!({"type":"string"}));
    assert_limit(&viable, "depth_cut");
    assert_eq!(fields(&viable).len(), 16);
    let immediate = index(response(wrap(
        json!({"type":"object","properties":{"child":false}}),
    )));
    assert_eq!(fields(&immediate).len(), 16);
    assert!(immediate.operations[0].messages[0].limits.is_empty());

    let impossible = deep(json!(false));
    assert!(
        impossible.operations[0].messages[0].limits.is_empty(),
        "an infeasible seventeenth object edge must not report depth_cut"
    );
    assert_eq!(fields(&impossible).len(), 15);
}

#[test]
fn swagger_request_proof_requires_content_type_selection() {
    let bytes = br#"{"swagger":"2.0","paths":{"/p":{"post":{"parameters":[{
        "name":"data","in":"body","schema":{"properties":{"x":{"type":"string"}}}
    }]}}}}"#;
    for (selected, count) in [(Some(true), 1), (Some(false), 0), (None, 0)] {
        let mut path = json!({
            "file":"handler.go","blob":"handler-blob","span":[10,14],
            "outcome":"json","final_content_type":"application/json"
        });
        if let Some(selected) = selected {
            path["selects_request_content_type"] = Value::Bool(selected);
        }
        let catalog = serde_json::to_vec(&json!([{
            "api":"test-api","producer_tree":"test-tree","schema_blob":"entry-blob",
            "operation":"op:POST /p","direction":"request","status":null,"media_type":"application/json",
            "cutoff_admitted":true,"route":"producer_source","coverage":{"complete":true,"paths":[path]}
        }])).unwrap();
        let evidence = [ProducerMediaEvidence {
            receipt: DependencyReceipt {
                input_id: "media",
                tree: "test-tree",
                path: "media.json",
                blob: "media-blob",
                entry: 1,
            },
            bytes: &catalog,
        }];
        let mut input = admission::admitted(bytes, Format::Json, "test-api", None, &[]);
        input.media_receipts = &evidence;
        let result = index_schema(&input).unwrap();
        assert_eq!(
            fields(&result).len(),
            count,
            "request proof selection {selected:?}"
        );
        if count == 0 {
            assert_limit(&result, "media_unverified");
        }
    }
}
