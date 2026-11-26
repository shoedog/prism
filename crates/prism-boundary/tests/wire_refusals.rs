#[path = "support/admission.rs"]
mod admission;

#[path = "support/fold1_refusals.rs"]
mod fold1;

use prism_boundary::revision::{DependencyReceipt, Format, ProducerMediaEvidence};
use prism_boundary::{index_schema, Coverage, SchemaIndex};
use serde_json::{json, Value};

fn response(schema: Value) -> Value {
    json!({"openapi":"3.1.0","paths":{"/p":{"post":{"responses":{
        "200":{"content":{"application/json":{"schema":schema}}}
    }}}}})
}

fn index(document: Value) -> SchemaIndex {
    let bytes = serde_json::to_vec(&document).unwrap();
    index_raw(&bytes)
}

fn index_raw(bytes: &[u8]) -> SchemaIndex {
    index_schema(&admission::admitted(
        bytes,
        Format::Json,
        "test-api",
        None,
        &[],
    ))
    .unwrap()
}

fn fields(index: &SchemaIndex) -> Vec<String> {
    index
        .fields
        .iter()
        .map(|field| field.id.to_string())
        .collect()
}

fn assert_limit(index: &SchemaIndex, expected: &str) {
    let message = &index.operations[0].messages[0];
    assert_ne!(message.coverage, Coverage::Checked);
    assert!(
        message.limits.iter().any(|limit| limit.reason == expected),
        "{message:?}"
    );
}

#[test]
fn unknown_required_value_limits_its_object() {
    let with_required = |required| {
        response(json!({
            "type":"object",
            "properties":{
                "stable":{"type":"string"},
                "gate":{"$ref":"#/$defs/Unavailable"}
            },
            "required":required
        }))
    };
    let optional = index(with_required(json!([])));
    assert_eq!(
        fields(&optional),
        [
            "field:resp:POST /p:200#/gate",
            "field:resp:POST /p:200#/stable"
        ]
    );
    assert_limit(&optional, "reference_unresolved");
    let required = index(with_required(json!(["gate"])));
    assert_limit(&required, "reference_unresolved");
    assert!(
        fields(&required).is_empty(),
        "the absent value may make this object impossible"
    );
    assert_eq!(required.observed_fields.len(), 2);
}

#[test]
fn closed_minimum_needs_enough_established_names() {
    let with_minimum = |minimum| {
        response(json!({
            "type":"object", "additionalProperties":false, "minProperties":minimum,
            "properties":{"x":{"type":"integer"},"gate":{"futureAssertion":true}}
        }))
    };
    let supported = index(with_minimum(1));
    assert_eq!(fields(&supported), ["field:resp:POST /p:200#/x"]);
    let blocked = index(with_minimum(2));
    assert_limit(&blocked, "unsupported_schema_keyword");
    assert!(fields(&blocked).is_empty());
    assert_eq!(blocked.observed_fields.len(), 2);
}

#[test]
fn composition_refusals_keep_an_independent_field() {
    let choices = vec![json!({"type":"string"}); 129];
    let cases = [
        (json!({"anyOf": choices}), "composition_budget"),
        (
            json!({"allOf":[{"anyOf":vec![json!({}); 12]},{"anyOf":vec![json!({}); 12]}]}),
            "composition_budget",
        ),
        (
            json!({"oneOf":[42, {"type":"string"}]}),
            "composition_unreadable",
        ),
        (
            json!({"futureAssertion": true}),
            "unsupported_schema_keyword",
        ),
    ];
    let document = |schema| {
        response(json!({
            "type":"object",
            "properties":{"maybe":schema,"stable":{"type":"string"}}
        }))
    };
    for (schema, reason) in cases {
        let union_guard = schema.get("anyOf").is_some();
        let limited = index(document(schema));
        assert_limit(&limited, reason);
        assert_eq!(fields(&limited), ["field:resp:POST /p:200#/stable"]);
        if union_guard {
            assert_eq!(
                limited
                    .observed_fields
                    .iter()
                    .map(|field| field.id.to_string())
                    .collect::<Vec<_>>(),
                ["field:resp:POST /p:200#/maybe"],
                "the per-union guard preserves the observed field"
            );
        }
    }
    let supported = index(document(
        json!({"anyOf":vec![json!({"type":"string"}); 128]}),
    ));
    assert_eq!(fields(&supported).len(), 2);
}

#[test]
fn non_object_response_and_malformed_content_are_withheld() {
    let mut document = response(json!({"properties":{"x":{"type":"string"}}}));
    let expected = "field:resp:POST /p:200#/x";
    assert_eq!(fields(&index(document.clone())), [expected]);
    for (change, reason) in [
        (json!(null), "response_unreadable"),
        (json!({"content":[]}), "media_unreadable"),
    ] {
        document["paths"]["/p"]["post"]["responses"]["200"] = change;
        let limited = index(document.clone());
        assert_limit(&limited, reason);
        assert!(fields(&limited).is_empty());
    }
}

#[test]
fn producer_coverage_needs_at_least_one_json_path() {
    let bytes = br#"{"swagger":"2.0","paths":{"/p":{"get":{"responses":{"200":{
        "schema":{"type":"object","properties":{"x":{"type":"string"}}}
    }}}}}}"#;
    for (outcome, count) in [("json", 1), ("excluded", 0)] {
        let catalog = serde_json::to_vec(&json!([{
            "api":"test-api","producer_tree":"test-tree","schema_blob":"entry-blob",
            "operation":"op:GET /p","direction":"response","status":"200","media_type":"application/json",
            "cutoff_admitted":true,"route":"producer_source","coverage":{"complete":true,"paths":[{
                "file":"handler.go","blob":"handler-blob","span":[10,14],"outcome":outcome,
                "final_content_type":"application/json","reason":"a different status"
            }]}
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
        let index = index_schema(&input).unwrap();
        assert_eq!(index.fields.len(), count, "{outcome}");
        if count == 0 {
            assert_limit(&index, "media_unverified");
        }
    }
}

#[test]
fn overlong_reference_chain_is_partial_with_resolving_control() {
    for (count, checked) in [(2, true), (130, false)] {
        let mut document = response(json!({"$ref":"#/$defs/d0"}));
        let definitions = (0..count)
            .map(|i| {
                let schema = if i + 1 == count {
                    json!({"properties":{"x":{"type":"string"}}})
                } else {
                    json!({"$ref":format!("#/$defs/d{}", i + 1)})
                };
                (format!("d{i}"), schema)
            })
            .collect();
        document["$defs"] = Value::Object(definitions);
        let indexed = index(document);
        if checked {
            assert_eq!(fields(&indexed), ["field:resp:POST /p:200#/x"]);
        } else {
            assert_limit(&indexed, "composition_budget");
            assert!(fields(&indexed).is_empty());
        }
    }
}

#[test]
fn an_unknown_seventeenth_edge_retains_its_reason_at_the_boundary() {
    let wrapped = |leaf| {
        (0..16).fold(
            leaf,
            |child, _| json!({"type":"object","properties":{"p":child}}),
        )
    };
    let limited = index(response(wrapped(json!({
        "type":"object","properties":{"leaf":{"$ref":"#/$defs/Unavailable"}}
    }))));
    assert_limit(&limited, "reference_unresolved");
    assert_eq!(
        limited.operations[0].messages[0].limits[0].pointer.len(),
        16
    );
    assert_eq!(fields(&limited).len(), 16);
    let control = index(response(wrapped(json!({
        "type":"object","properties":{"leaf":false}
    }))));
    assert_eq!(
        control.operations[0].messages[0].coverage,
        Coverage::Checked
    );
    assert_eq!(fields(&control).len(), 16);
}

#[test]
fn loose_object_typing_does_not_certify_a_required_unknown_field() {
    let schema = |required| {
        response(json!({
            "properties":{
                "stable":{"type":"string"},
                "gate":{"$ref":"#/$defs/Unavailable"}
            },
            "required":required
        }))
    };
    assert_eq!(fields(&index(schema(json!([])))).len(), 2);
    let limited = index(schema(json!(["gate"])));
    assert_limit(&limited, "reference_unresolved");
    assert!(fields(&limited).is_empty());
}

#[test]
fn impossible_oneof_arm_does_not_block_its_feasible_sibling() {
    let document = |gate| {
        response(json!({
            "oneOf":[
                {"type":"object","properties":{"x":{"type":"string"}}},
                {"type":"object","required":["gate"],"properties":{"gate":gate}}
            ]
        }))
    };
    let impossible = index(document(json!(false)));
    assert_eq!(fields(&impossible), ["field:resp:POST /p:200#/x"]);
    let overlapping = index(document(json!(true)));
    assert_limit(&overlapping, "unsupported_schema_keyword");
    assert!(fields(&overlapping).is_empty());
}

#[test]
fn malformed_json_encoding_is_scoped_to_its_request() {
    let document = |encoding| {
        json!({"openapi":"3.1.0","paths":{"/p":{"post":{
            "requestBody":{"content":{"application/json":{
                "schema":{"properties":{"value":{"type":"string"}}},
                "encoding":encoding
            }}}
        }}}})
    };
    let control = index(document(json!({})));
    assert_eq!(fields(&control), ["field:req:POST /p#/value"]);
    let malformed = index(document(json!([])));
    assert_limit(&malformed, "media_encoding_unchecked");
    assert!(fields(&malformed).is_empty());
}

#[test]
fn unresolved_conjunct_keeps_the_local_sibling_declaration() {
    let schema = |reference| {
        json!({"type":"object","properties":{"box":{
            "$ref":reference,
            "properties":{"local":{"type":"string"}}
        }}})
    };
    let mut document = response(schema("#/$defs/Base"));
    document["$defs"] = json!({"Base":{"type":"object"}});
    assert_eq!(
        fields(&index(document)),
        [
            "field:resp:POST /p:200#/box",
            "field:resp:POST /p:200#/box/local"
        ]
    );
    let limited = index(response(schema("#/$defs/Unavailable")));
    assert_limit(&limited, "reference_unresolved");
    assert_eq!(fields(&limited), ["field:resp:POST /p:200#/box"]);
    assert_eq!(
        limited
            .observed_fields
            .iter()
            .map(|field| field.id.to_string())
            .collect::<Vec<_>>(),
        ["field:resp:POST /p:200#/box/local"]
    );
}

#[test]
fn a_property_named_dollar_id_does_not_create_a_resource() {
    let document = |id| {
        let mut schema = json!({"type":"object","properties":{
            "$id":{"type":"string"},
            "box":{"$ref":"#/$defs/Base"}
        }});
        if let Some(id) = id {
            schema["$id"] = json!(id);
        }
        let mut document = response(schema);
        document["$defs"] = json!({"Base":{"properties":{"x":{"type":"string"}}}});
        document
    };
    let supported = index(document(None));
    assert_eq!(
        fields(&supported),
        [
            "field:resp:POST /p:200#/%24id",
            "field:resp:POST /p:200#/box",
            "field:resp:POST /p:200#/box/x"
        ]
    );
    let new_resource = index(document(Some("elsewhere.json")));
    assert_limit(&new_resource, "reference_scope");
    assert!(fields(&new_resource).is_empty());
}

#[test]
fn malformed_nested_schema_shapes_keep_an_independent_field() {
    for (malformed, reason) in [
        (json!(42), "schema_form_unchecked"),
        (json!({"type":"future-kind"}), "schema_keyword_unreadable"),
        (json!({"minLength":0.5}), "schema_keyword_unreadable"),
        (
            json!({"type":"string","enum":["a"],"pattern":"(?=a)a"}),
            "scalar_feasibility_unchecked",
        ),
        (
            json!({"type":"number","enum":[1],"multipleOf":1e-100}),
            "scalar_feasibility_unchecked",
        ),
    ] {
        let document = |value| {
            response(json!({
                "type":"object","properties":{"x":value,"stable":{"type":"string"}}
            }))
        };
        let limited = index(document(malformed));
        assert_limit(&limited, reason);
        assert_eq!(fields(&limited), ["field:resp:POST /p:200#/stable"]);
        assert_eq!(limited.observed_fields.len(), 1);
        assert_eq!(fields(&index(document(json!({"type":"string"})))).len(), 2);
    }
}

#[test]
fn integer_intervals_need_an_integral_witness() {
    let schema = |(minimum, maximum)| {
        json!({
            "type":"object","properties":{"x":{"type":"integer","minimum":minimum,"maximum":maximum}}
        })
    };
    for ((minimum, maximum), count) in [
        ((0.1, 0.9), 0),
        ((-0.9, -0.1), 0),
        ((-1.1, -0.1), 1),
        ((0.9, 1.1), 1),
    ] {
        let index = index(response(schema((minimum, maximum))));
        assert_eq!(fields(&index).len(), count);
        assert_eq!(index.operations[0].messages[0].coverage, Coverage::Checked);
    }
}

#[test]
fn a_reference_into_a_nested_resource_needs_its_own_base() {
    let document = |resource| {
        let mut document = response(json!({"$ref":"#/$defs/Outer/properties/inner"}));
        let mut outer = json!({"properties":{"inner":{"properties":{"x":{"type":"string"}}}}});
        if resource {
            outer["$id"] = json!("elsewhere.json");
        }
        document["$defs"] = json!({"Outer":outer});
        document
    };
    assert_eq!(
        fields(&index(document(false))),
        ["field:resp:POST /p:200#/x"]
    );
    let limited = index(document(true));
    assert_limit(&limited, "reference_scope");
    assert!(fields(&limited).is_empty());
}

#[test]
fn swagger_json_media_can_coexist_with_an_excluded_representation() {
    let document = |produces| {
        json!({
            "swagger":"2.0","produces":produces,
            "paths":{"/p":{"get":{"responses":{"200":{"schema":{
                "type":"object","properties":{"x":{"type":"string"}}
            }}}}}}
        })
    };
    let json_only = index(document(json!(["application/json"])));
    let both = index(document(json!(["application/json", "text/xml"])));
    assert_eq!(fields(&both), fields(&json_only));
    assert_eq!(
        both.operations[0].messages[0].declared_media,
        ["application/json", "text/xml"]
    );
    assert_limit(&both, "non_json_media");
}

#[test]
fn omission_receipt_candidates_are_checked_before_media_can_be_promoted() {
    let data = serde_json::to_vec(&json!([{
        "api":"test-api","producer_tree":"test-tree","schema_blob":"entry-blob",
        "operation":"op:GET /p","direction":"response","status":"200","media_type":"application/json",
        "cutoff_admitted":true,"route":"producer_source","coverage":{"complete":true,"paths":[
            {"file":"handler.go","blob":"handler-blob","span":[10,14],"outcome":"json",
             "final_content_type":"application/json"},
            {"file":"handler.go","blob":"handler-blob","span":[18,21],"outcome":"excluded",
             "reason":"returns a different status"}
        ]}
    }])).unwrap();
    let bytes = br#"{"swagger":"2.0","paths":{"/p":{"get":{"responses":{"200":{
        "schema":{"properties":{"x":{"type":"string"}}}
    }}}}}}"#;
    let receipt = |input_id, bytes, entry| ProducerMediaEvidence {
        receipt: DependencyReceipt {
            input_id,
            tree: "test-tree",
            path: "media.json",
            blob: "media-blob",
            entry,
        },
        bytes,
    };
    let valid = receipt("verified", &data, 1);
    let mut unknown: Value = serde_json::from_slice(&data).unwrap();
    unknown[0]["coverage"]["paths"][1]["outcome"] = json!("unrecognized");
    let invalid = serde_json::to_vec(&unknown).unwrap();
    for (evidence, count) in [
        (vec![valid], 1),
        (vec![receipt("unchecked", &data, 0)], 0),
        (vec![receipt("unchecked", &data, 2)], 0),
        (vec![receipt("unchecked", b"invalid", 1)], 0),
        (vec![receipt("unchecked", &invalid, 1)], 0),
        (vec![valid, receipt("ambiguous", &data, 1)], 0),
    ] {
        let mut input = admission::admitted(bytes, Format::Json, "test-api", None, &[]);
        input.media_receipts = &evidence;
        let indexed = index_schema(&input).unwrap();
        assert_eq!(fields(&indexed).len(), count, "{evidence:?}");
        if count == 0 {
            assert_limit(&indexed, "media_unverified");
        }
    }
}

#[test]
fn exact_integer_bounds_withhold_unresolved_large_intervals() {
    let compare = |low: &str, upper: &str| {
        let value: Value = serde_json::from_str(&format!(
            r#"{{"type":"integer","exclusiveMinimum":{low},"maximum":{upper}}}"#
        ))
        .unwrap();
        index(response(json!({"type":"object","properties":{"x":value}})))
    };
    let checked = compare("1", "2");
    assert_eq!(fields(&checked), ["field:resp:POST /p:200#/x"]);
    let limited = compare("1e100", "2e100");
    assert_limit(&limited, "scalar_feasibility_unchecked");
    assert!(fields(&limited).is_empty());
    let contradictory = compare(
        "1e1000000000000000000000000000000000000000",
        "9e999999999999999999999999999999999999999",
    );
    assert_eq!(
        contradictory.operations[0].messages[0].coverage,
        Coverage::Checked
    );
    assert!(fields(&contradictory).is_empty());
}

#[test]
fn oneof_proof_requires_a_directional_finite_discriminant() {
    let union = |left, right| {
        let arm = |tag| json!({"type":"object","required":["kind"],"properties":{"kind":tag}});
        response(json!({"oneOf":[arm(left),arm(right)]}))
    };
    let tag = json!({"type":"string","enum":["a"]});
    let proven = index(union(tag.clone(), json!({"type":"string","enum":["b"]})));
    assert_eq!(fields(&proven), ["field:resp:POST /p:200#/kind"]);
    for right in [
        json!({"type":"string"}),
        json!({"type":"string","enum":["b"],"writeOnly":true}),
    ] {
        let limited = index(union(tag.clone(), right));
        assert_limit(&limited, "unsupported_schema_keyword");
        assert!(fields(&limited).is_empty());
    }

    let scalar_union = |right| {
        response(json!({"properties":{"x":{
            "oneOf":[{"enum":[1]},{"enum":[right]}]
        }}}))
    };
    assert_eq!(
        fields(&index(scalar_union(2))),
        ["field:resp:POST /p:200#/x"]
    );
    assert_limit(&index(scalar_union(1)), "unsupported_schema_keyword");
}

#[test]
fn partial_declaration_observations_respect_the_same_pointer_bound() {
    for (depth, count) in [(5, 5), (17, 16)] {
        let mut declarations = json!({"type":"string"});
        let mut instance = json!("end");
        for _ in 0..depth {
            declarations = json!({"properties":{"p":declarations}});
            instance = json!({"p":instance});
        }
        declarations["futureAssertion"] = json!(true);
        for schema in [declarations, json!({"enum":[instance]})] {
            let limited = index(response(schema));
            assert_limit(&limited, "unsupported_schema_keyword");
            assert!(fields(&limited).is_empty());
            assert_eq!(limited.observed_fields.len(), count);
        }
    }
}
