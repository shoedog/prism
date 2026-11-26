use crate::diff_input::{fixture, schema, Input};
use prism_boundary::element::schema_path;
use prism_boundary::{
    diff_boundary, diff_schemas, ChangeKind, ComparisonClaim, ComparisonScope, ComparisonSide,
    Coverage, DiffBasis, DiffSide, Fact, LimitScope, ProducerSlot, SchemaDiff, StatusInventory,
    Validator,
};
use serde_json::{json, Value};

fn kinds(diff: &SchemaDiff<'_>) -> Vec<ChangeKind> {
    diff.changes
        .iter()
        .flat_map(|change| change.kinds.iter().copied())
        .collect()
}

#[test]
fn empty_request_excluded_by_dialect_keeps_a_root_limit() {
    let with_body = br#"{"openapi":"3.1.0","paths":{"/a":{"get":{"requestBody":{"content":{"application/json":{"schema":{"type":"string"}}}},"responses":{}}}}}"#;
    let without_body = br#"{"openapi":"3.1.0","paths":{"/a":{"get":{"responses":{}}}}}"#;
    let excluded = br#"{"openapi":"3.0.3","paths":{"/a":{"get":{"responses":{}}}}}"#;
    let before = schema(with_body, "test-api", &Value::Null, 0);
    let after = schema(excluded, "test-api", &Value::Null, 1);
    let diff = diff_schemas(&before, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert_eq!(diff.limits.len(), 1);
    assert_eq!(diff.limits[0].reason, "request_dialect_excluded");
    assert_eq!(diff.limits[0].side, ComparisonSide::After);
    assert!(diff.limits[0].pointer.is_empty());
    let control = schema(without_body, "test-api", &Value::Null, 1);
    let diff = diff_schemas(&before, &control).unwrap();
    assert!(diff.changes.is_empty() && diff.limits.is_empty());
}

#[test]
fn proved_non_json_competitor_leaves_a_unique_json_pair() {
    let before = fixture("p1c_rows", "wire/string.json");
    let two_json = fixture(
        "p1c_rows",
        "control-ambiguous-status-root-replacement/openapi.json",
    );
    let mut media: Value = serde_json::from_slice(&two_json).unwrap();
    media["paths"]["/loom"]["get"]["responses"]["202"]["content"] =
        json!({"text/plain": {"schema": {"type": "string"}}});
    let before = schema(&before, "test-api", &Value::Null, 0);
    let after = schema(
        &serde_json::to_vec(&media).unwrap(),
        "test-api",
        &Value::Null,
        1,
    );
    let diff = diff_schemas(&before, &after).unwrap();
    assert_eq!(diff.response_root_pairs.len(), 1);
    assert_eq!(diff.response_root_pairs[0].after.to_string(), "201");
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "non_json_media"));
    let control = schema(&two_json, "test-api", &Value::Null, 1);
    let diff = diff_schemas(&before, &control).unwrap();
    assert!(diff.response_root_pairs.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "ambiguous_root_pairing"));
}

#[test]
fn common_prefix_restricts_checked_negatives_to_typed_segments() {
    let before = br#"{"openapi":"3.1.0","paths":{}}"#;
    let after = br#"{"openapi":"3.1.0","paths":{"/loom": {"get":{}}, "/loom/child":{"get":{}}, "/looming":{"get":{}}}}"#;
    let before = schema(before, "test-api", &Value::Null, 0);
    let after = schema(after, "test-api", &Value::Null, 1);
    let bound = |prefix: &str, tree, schema| DiffSide {
        basis: Some(DiffBasis {
            api: "test-api",
            authority: "schema",
            tree,
            scope: ComparisonScope::PathPrefix(schema_path(prefix, "", &[])),
        }),
        schema: Some(schema),
        producer: None,
    };
    for prefixes in [["/loom", "/loom/child"], ["/loom/child", "/loom"]] {
        let diff = diff_boundary(
            &bound(prefixes[0], "before-tree", &before),
            &bound(prefixes[1], "after-tree", &after),
        )
        .unwrap();
        assert_eq!(kinds(&diff), [ChangeKind::OpAdded]);
        assert_eq!(diff.changes[0].operation.path(), "/loom/child");
    }
    assert_eq!(
        kinds(&diff_schemas(&before, &after).unwrap()),
        [ChangeKind::OpAdded; 3]
    );
}

#[test]
fn missing_root_evidence_cannot_sign_a_field_change() {
    let before = fixture("p1c_rows", "wire/string.json");
    let before = schema(&before, "test-api", &Value::Null, 0);
    let after = br#"{"openapi":"3.1.0","paths":{"/loom":{"get":{"responses":{"200":{"content":{"application/json":{"schema":{"properties":{"x":{"type":"integer"}}}}}}}}}}}"#;
    let mut after = schema(after, "test-api", &Value::Null, 1);
    assert_eq!(
        kinds(&diff_schemas(&before, &after).unwrap()),
        [ChangeKind::FieldTypeChanged]
    );
    after.operations[0].messages.clear();
    let diff = diff_schemas(&before, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.side == ComparisonSide::After
            && limit.reason == "root_inventory_unavailable"
            && limit.claim == ComparisonClaim::Coverage));
}

#[test]
fn producer_census_must_cover_a_missing_operation() {
    let left = json!({"fixture": "producer/baseline.json", "producer": true});
    let right = json!({"fixture": "producer/baseline.json", "producer": true});
    let mut before = Input::load(&left, 0);
    let after = Input::load(&right, 1);
    before.producer.as_mut().unwrap().operations.clear();
    assert_eq!(
        kinds(&diff_boundary(&before.side(), &after.side()).unwrap()),
        [ChangeKind::OpAdded]
    );
    before.producer.as_mut().unwrap().coverage = Coverage::Partial;
    let diff = diff_boundary(&before.side(), &after.side()).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.side == ComparisonSide::Before
            && limit.reason == "producer_operation_unchecked"
            && limit.claim == ComparisonClaim::OperationPresence));

    let mut duplicate = Input::load(&left, 0);
    let mut additional = Input::load(&left, 0);
    duplicate
        .producer
        .as_mut()
        .unwrap()
        .operations
        .append(&mut additional.producer.as_mut().unwrap().operations);
    let diff = diff_boundary(&duplicate.side(), &after.side()).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "ambiguous_producer_operation"));
}

#[test]
fn producer_slots_require_corresponding_evidence() {
    let left = json!({"fixture": "producer/baseline.json", "producer": true});
    let right = json!({"fixture": "producer/multi-edit.json", "producer": true});
    let before = Input::load(&left, 0);
    let mut after = Input::load(&right, 1);
    let diff = diff_boundary(&before.side(), &after.side()).unwrap();
    assert_eq!(
        kinds(&diff),
        [
            ChangeKind::RouteDeclarationChanged,
            ChangeKind::ValidationChanged,
            ChangeKind::HandlerCodeChanged
        ]
    );
    let op = &mut after.producer.as_mut().unwrap().operations[0];
    op.code.sites[0].tree = "unbound-tree".into();
    op.validation.fact = Fact::Checked(Some(Validator {
        identity: Some("local".into()),
        digest: None,
    }));
    let diff = diff_boundary(&before.side(), &after.side()).unwrap();
    assert_eq!(kinds(&diff), [ChangeKind::RouteDeclarationChanged]);
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.side == ComparisonSide::After
            && limit.claim == ComparisonClaim::Producer(ProducerSlot::Code)
            && limit.reason == "producer_evidence_unbound"));
    assert!(diff.limits.iter().any(|limit| limit.claim
        == ComparisonClaim::Producer(ProducerSlot::Validation)
        && limit.reason == "validator_incomparable"));
}

#[test]
fn matching_printed_paths_need_typed_correspondence() {
    let before = fixture("p1c_rows", "collisions/single.json");
    let before = schema(&before, "test-api", &Value::Null, 0);
    let fixture = fixture("p1c_rows", "collisions/literal-and-capture.json");
    let mut after: Value = serde_json::from_slice(&fixture).unwrap();
    after["paths"].as_object_mut().unwrap().remove("/loom/{a}");
    let run = |after: &Value| {
        schema(
            &serde_json::to_vec(after).unwrap(),
            "test-api",
            &Value::Null,
            1,
        )
    };
    let literal = run(&after);
    let diff = diff_schemas(&before, &literal).unwrap();
    assert!(diff.changes.is_empty());
    assert_eq!(
        diff.limits
            .iter()
            .filter(|limit| limit.reason == "typed_operation_mismatch")
            .count(),
        2
    );
    let route = after["paths"]
        .as_object_mut()
        .unwrap()
        .remove("/loom/{}")
        .unwrap();
    after["paths"]["/loom/{renamed}"] = route;
    assert_eq!(
        kinds(&diff_schemas(&before, &run(&after)).unwrap()),
        [ChangeKind::FieldTypeChanged]
    );
}

#[test]
fn unobserved_operation_and_field_claims_require_a_checked_census() {
    let malformed = schema(
        br#"{"openapi":"3.1.0","paths":null}"#,
        "test-api",
        &Value::Null,
        0,
    );
    let empty = schema(
        br#"{"openapi":"3.1.0","paths":{}}"#,
        "test-api",
        &Value::Null,
        0,
    );
    let bytes = fixture("p1c_rows", "wire/string.json");
    let before = schema(&bytes, "test-api", &Value::Null, 0);
    let mut after = schema(&bytes, "test-api", &Value::Null, 1);
    assert_eq!(
        kinds(&diff_schemas(&empty, &after).unwrap()),
        [ChangeKind::OpAdded]
    );
    let diff = diff_schemas(&malformed, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "paths_unreadable"));

    let field = after.fields.pop().unwrap();
    assert_eq!(
        kinds(&diff_schemas(&before, &after).unwrap()),
        [ChangeKind::FieldRemoved]
    );
    after.observed_fields.push(field);
    let diff = diff_schemas(&before, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "field_presence_unchecked"));
}

#[test]
fn a_requested_producer_slot_preserves_independent_schema_changes() {
    let old = json!({"fixture": "producer/baseline.json", "producer": true});
    let new = json!({"fixture": "producer/retarget.json", "producer": true});
    let before_producer = Input::load(&old, 0);
    let mut after_producer = Input::load(&new, 1);
    let diff = diff_boundary(&before_producer.side(), &after_producer.side()).unwrap();
    assert_eq!(kinds(&diff), [ChangeKind::HandlerReferenceChanged]);
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "endpoint_body_incomparable"));

    let config = json!({"authority": "route_table"});
    let old_schema = br#"{"openapi":"3.1.0","paths":{"/kiln":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let new_schema = br#"{"openapi":"3.1.0","paths":{"/kiln":{"get":{"responses":{"200":{"description":"ok"},"204":{"description":"no body"}}}}}}"#;
    let before = schema(old_schema, "test-api", &config, 0);
    let after = schema(new_schema, "test-api", &config, 1);
    let before = DiffSide {
        schema: Some(&before),
        producer: before_producer.producer.as_ref(),
        basis: None,
    };
    let without_schema = DiffSide {
        schema: None,
        producer: None,
        basis: after_producer.basis.clone(),
    };
    let diff = diff_boundary(&before, &without_schema).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.reason == "operation_inventory_unavailable"));

    let after_schema = DiffSide {
        schema: Some(&after),
        ..DiffSide::default()
    };
    let diff = diff_boundary(&before, &after_schema).unwrap();
    assert_eq!(kinds(&diff), [ChangeKind::SuccessStatusAdded]);
    assert_eq!(
        diff.limits
            .iter()
            .filter(|limit| limit.reason == "producer_unavailable")
            .count(),
        4
    );
    after_producer.producer.as_mut().unwrap().operations.clear();
    let no_record = DiffSide {
        producer: after_producer.producer.as_ref(),
        ..after_schema
    };
    let diff = diff_boundary(&before, &no_record).unwrap();
    assert_eq!(kinds(&diff), [ChangeKind::SuccessStatusAdded]);
    assert_eq!(
        diff.limits
            .iter()
            .filter(|limit| limit.reason == "producer_fact_unavailable")
            .count(),
        4
    );
}

#[test]
fn fold2_scoped_schema_status_survives_a_producer_gap() {
    let config = json!({"authority": "route_table"});
    let old_schema = br#"{"openapi":"3.1.0","paths":{"/kiln":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let new_schema = br#"{"openapi":"3.1.0","paths":{"/kiln":{"get":{"responses":{"200":{"description":"ok"},"204":{"description":"no body"}}}}}}"#;
    let before = schema(old_schema, "test-api", &config, 0);
    let after = schema(new_schema, "test-api", &config, 1);
    let checked = json!({"fixture": "producer/baseline.json", "producer": true, "scope": "/kiln"});
    let withheld = json!({"fixture": "producer/baseline.json", "producer": true, "scope": "/kiln", "declaration_withheld": "route_unreadable"});
    let after_producer = Input::load(&checked, 1);
    let compare = |config| {
        let before_producer = Input::load(config, 0);
        let before = DiffSide {
            schema: Some(&before),
            ..before_producer.side()
        };
        let after = DiffSide {
            schema: Some(&after),
            ..after_producer.side()
        };
        kinds(&diff_boundary(&before, &after).unwrap())
    };
    assert_eq!(compare(&checked), [ChangeKind::SuccessStatusAdded]);
    assert_eq!(
        compare(&withheld),
        [ChangeKind::SuccessStatusAdded],
        "a withheld producer declaration leaves checked schema status evidence intact"
    );
}

#[test]
fn producer_facts_require_the_schema_operations_typed_path() {
    let old = json!({"fixture": "producer/baseline.json", "producer": true});
    let new = json!({"fixture": "producer/retarget.json", "producer": true});
    let mut before = Input::load(&old, 0);
    let mut after = Input::load(&new, 1);
    let config = json!({"authority": "route_table"});
    for (i, side) in [&mut before, &mut after].into_iter().enumerate() {
        let status = if i == 0 { "204" } else { "205" };
        let schema_bytes = serde_json::to_vec(&json!({
            "openapi": "3.1.0", "paths": {"/kiln/{capture}": {"get": {"responses": {
                (status): {"description": "no body"}
            }}}}
        }))
        .unwrap();
        let index = schema(&schema_bytes, "test-api", &config, i);
        let op = &mut side.producer.as_mut().unwrap().operations[0];
        op.operation = index.operations[0].id.clone();
        let Fact::Checked(Some(route)) = &mut op.declaration.fact else {
            panic!("checked fixture")
        };
        route.canonical_path = schema_path("/kiln/{capture}", "", &[]);
        side.schema = Some(index);
    }
    let diff = diff_boundary(&before.side(), &after.side()).unwrap();
    assert_eq!(
        kinds(&diff),
        [
            ChangeKind::SuccessStatusRemoved,
            ChangeKind::SuccessStatusAdded,
            ChangeKind::HandlerReferenceChanged
        ]
    );
    let Fact::Checked(Some(route)) = &mut after.producer.as_mut().unwrap().operations[0]
        .declaration
        .fact
    else {
        panic!("checked fixture");
    };
    route.canonical_path = schema_path("/kiln/{}", "", &[]);
    let diff = diff_boundary(&before.side(), &after.side()).unwrap();
    assert_eq!(
        kinds(&diff),
        [
            ChangeKind::SuccessStatusRemoved,
            ChangeKind::SuccessStatusAdded
        ]
    );
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.side == ComparisonSide::After
            && limit.reason == "producer_operation_mismatch"));
}

#[test]
fn whole_operation_addition_preserves_standing_status_disagreement() {
    let before = schema(
        br#"{"openapi":"3.1.0","paths":{}}"#,
        "test-api",
        &Value::Null,
        0,
    );
    for (fixture_name, known, limits) in [
        ("different-statuses.json", vec![], 2),
        ("common-status.json", vec!["200"], 0),
    ] {
        let bytes = fixture("p1c_rows", &format!("collisions/{fixture_name}"));
        let after = schema(&bytes, "test-api", &Value::Null, 1);
        let diff = diff_schemas(&before, &after).unwrap();
        assert_eq!(kinds(&diff), [ChangeKind::OpAdded]);
        assert_eq!(
            diff.changes[0]
                .after
                .success_keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            known
        );
        assert_eq!(
            diff.limits
                .iter()
                .filter(|limit| limit.reason == "status_membership_unknown")
                .count(),
            limits
        );
    }
}

#[test]
fn unprojected_capture_withholds_only_its_possible_method() {
    let before_input = |style: &str| {
        json!({
            "openapi":"3.1.0","paths":{"/loom/{a}":{"get":{"parameters":[
                {"name":"a","in":"path","style":style,"schema":{"type":"string"}}
            ]}}}
        })
    };
    let after = schema(
        br#"{"openapi":"3.1.0","paths":{"/loom/{a}":{"get":{},"post":{}}}}"#,
        "test-api",
        &Value::Null,
        1,
    );
    for style in ["simple", "matrix"] {
        let before = schema(
            &serde_json::to_vec(&before_input(style)).unwrap(),
            "test-api",
            &Value::Null,
            0,
        );
        let diff = diff_schemas(&before, &after).unwrap();
        assert_eq!(kinds(&diff), [ChangeKind::OpAdded]);
        assert_eq!(diff.changes[0].operation.method(), "POST");
        assert_eq!(
            diff.limits
                .iter()
                .any(|limit| limit.reason == "path_style_unsupported"
                    && limit.claim == ComparisonClaim::OperationPresence),
            style == "matrix"
        );
        assert_eq!(before.operations.is_empty(), style == "matrix");
    }
}

#[test]
fn overlapping_server_declarations_leave_operation_presence_unknown() {
    let mut input = json!({
        "openapi": "3.1.0",
        "paths": {"/loom": {
            "$ref": "#/components/pathItems/Shared",
            "get": {"responses": {"204": {"description": "no body"}}}
        }},
        "components": {"pathItems": {"Shared": {"servers": [{"url": "/mount"}]}}}
    });
    let after = schema(
        br#"{"openapi":"3.1.0","paths":{"/mount/loom":{"get":{"responses":{"205":{"description":"no body"}}}}}}"#,
        "test-api", &Value::Null, 1,
    );
    let before = |input: &Value| {
        schema(
            &serde_json::to_vec(input).unwrap(),
            "test-api",
            &Value::Null,
            0,
        )
    };
    let control = before(&input);
    assert_eq!(
        kinds(&diff_schemas(&control, &after).unwrap()),
        [
            ChangeKind::SuccessStatusRemoved,
            ChangeKind::SuccessStatusAdded
        ]
    );
    input["paths"]["/loom"]["servers"] = json!([{"url": "/competing"}]);
    let blocked = before(&input);
    assert!(blocked.operations.is_empty());
    let diff = diff_schemas(&blocked, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.side == ComparisonSide::Before
            && limit.claim == ComparisonClaim::OperationPresence
            && limit.reason == "operation_unprojected"));
}

#[test]
fn withheld_status_without_a_diagnostic_keeps_a_membership_limit() {
    let before = fixture("p1c_rows", "statuses/204.json");
    let after = fixture("p1c_rows", "statuses/205.json");
    let before = schema(&before, "test-api", &Value::Null, 0);
    let mut after = schema(&after, "test-api", &Value::Null, 1);
    assert_eq!(
        kinds(&diff_schemas(&before, &after).unwrap()),
        [
            ChangeKind::SuccessStatusRemoved,
            ChangeKind::SuccessStatusAdded
        ]
    );
    after.operations[0].declarations[0].status_inventory = StatusInventory::Withheld;
    let diff = diff_schemas(&before, &after).unwrap();
    assert!(diff.changes.is_empty());
    assert!(diff
        .limits
        .iter()
        .any(|limit| limit.side == ComparisonSide::After
            && limit.claim == ComparisonClaim::StatusMembership
            && limit.scope == LimitScope::Status(after.operations[0].id.clone(), None)
            && limit.reason == "status_membership_unknown"));
}
