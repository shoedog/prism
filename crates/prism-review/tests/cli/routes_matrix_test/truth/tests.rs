use super::*;

fn example() -> (Value, Value) {
    let site = json!({"file": "app.js", "line": 7});
    let path = json!({"raw": "'/:id'", "literal": "/:id"});
    let exposure = json!({"status": "full", "value": "/api/:id", "normalized": "/api/{}"});
    let callback = json!({"position": 0, "file": "app.js", "name": "handler", "start_line": 3});
    let mut projected = json!({
        "framework": "express", "site": site, "path": path, "paths": [exposure],
        "methods": {"known": ["GET"]}, "callbacks": [callback]
    });
    let row = json!({"truth_source": "executed", "expect": [projected]});
    projected["call_id"] = json!(1);
    let truth = json!({
        "status": "executed", "projection": [projected],
        "calls": [{"id": 1, "site": {"file": "app.js", "line": 7, "ordinal": 2},
            "callbacks": [{"file": "app.js", "name": "handler", "first_line": 3}]}],
        "served": [{"call_ids": [1], "path": "/api/:id", "effective_methods": ["GET", "HEAD"]}]
    });
    (row, truth)
}

fn assert_fields(row: &Value, truth: Option<&Value>, expected: &[&str]) {
    let mut errors = Vec::new();
    compare_row(row, truth, &mut errors);
    let fields: Vec<_> = errors
        .iter()
        .map(|error| error.split_once(':').unwrap().0)
        .collect();
    assert_eq!(fields, expected, "{errors:?}");
}

#[test]
fn rejects_changed_callback_full_path_declared_method_and_site_line() {
    let (row, truth) = example();
    assert_fields(&row, Some(&truth), &[]);
    let mut changed = row.clone();
    changed["expect"][0]["callbacks"][0]["name"] = json!("other_handler");
    assert_fields(&changed, Some(&truth), &["expect[0].callbacks[0].name"]);
    let mut changed = row.clone();
    changed["expect"][0]["paths"][0]["value"] = json!("/:id");
    assert_fields(&changed, Some(&truth), &["expect[0].paths[0].value"]);
    let mut changed = row.clone();
    changed["expect"][0]["methods"] = json!({"known": ["GET", "HEAD"]});
    assert_fields(&changed, Some(&truth), &["expect[0].methods"]);
    let mut changed = row;
    changed["expect"][0]["site"]["line"] = json!(8);
    assert_fields(&changed, Some(&truth), &["expect[0].site.line"]);
}

#[test]
fn pairs_reordered_same_line_calls_by_raw_ordinal() {
    let (mut row, mut truth) = example();
    let mut second = truth["projection"][0].clone();
    second["call_id"] = json!(2);
    second["methods"] = json!({"known": ["POST"]});
    truth["projection"].as_array_mut().unwrap().push(second);
    let mut call = truth["calls"][0].clone();
    call["id"] = json!(2);
    call["site"]["ordinal"] = json!(3);
    truth["calls"].as_array_mut().unwrap().push(call);
    truth["served"][0]["call_ids"] = json!([1, 2]);
    let mut second_pin = row["expect"][0].clone();
    second_pin["methods"] = json!({"known": ["POST"]});
    row["expect"].as_array_mut().unwrap().insert(0, second_pin);

    assert_fields(
        &row,
        Some(&truth),
        &["expect[0].site.ordinal", "expect[1].site.ordinal"],
    );
    row["expect"][0]["site"]["ordinal"] = json!(4);
    row["expect"][1]["site"]["ordinal"] = json!(2);
    assert_fields(&row, Some(&truth), &["expect[0].site.ordinal"]);
    row["expect"][0]["site"]["ordinal"] = json!(3);
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0]["methods"] = json!({"known": ["GET"]});
    row["expect"][1]["methods"] = json!({"known": ["POST"]});
    assert_fields(
        &row,
        Some(&truth),
        &["expect[0].methods", "expect[1].methods"],
    );
}

#[test]
fn checks_linked_served_paths_and_normalized_suffixes() {
    let (mut row, mut truth) = example();
    row["expect"][0]["paths"] = json!([{"status": "suffix", "value": "/:id", "normalized": "/{}"}]);
    truth["served"]
        .as_array_mut()
        .unwrap()
        .push(json!({"call_ids": [1], "path": "/also"}));
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0]["paths"][0]["normalized"] = json!("/wrong");
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].normalized"]);
    truth["served"][0]["path"] = json!("/api/:id/");
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].value"]);
    row["expect"][0]["paths"] = json!([{"status": "full", "value": "/also"}]);
    assert_fields(&row, Some(&truth), &["expect[0].paths"]);
    row["expect"][0]["paths"] = json!([
        {"status": "full", "value": "/also"},
        {"status": "full", "value": "/api/:id/"}
    ]);
    assert_fields(&row, Some(&truth), &[]);
}

#[test]
fn skips_only_static_paths_and_static_metadata() {
    let (mut row, mut truth) = example();
    let pin = &mut row["expect"][0];
    pin["receiver"] = json!({"status": "unproven"});
    pin["site"]["text"] = json!("different source text");
    pin["site"]["occurrence"] = json!(5);
    pin["path"]["text"] = json!("static metadata");
    pin["callbacks"][0]["status"] = json!("unresolved");
    pin["callbacks"][0]["end_line"] = json!(100);
    pin["callbacks"][0]["delegates"] = json!(["somewhere_else"]);
    pin["paths"] = json!([{"status": "dynamic", "value": "/wrong"}]);
    row["expect_mounts"] = json!([{"site": {"line": 23}}]);
    row["expect_unlisted"] = json!(true);
    assert_fields(&row, Some(&truth), &[]);

    truth["served"][0]["call_ids"] = json!([99]);
    for reason in ["no_known_mount", "mount_cycle", "expansion_truncated"] {
        row["expect"][0]["paths"][0] =
            json!({"status": "suffix", "value": "/wrong", "reason": reason});
        assert_fields(&row, Some(&truth), &[]);
    }
    row["expect"][0]["paths"][0]["reason"] = json!("variable_prefix");
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].value"]);
    row["expect"][0]["paths"][0]["reason"] = json!("no_known_mount");
    truth["served"][0]["call_ids"] = json!([1]);
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].value"]);
}

#[test]
fn checks_raw_callback_count_and_falls_back_to_raw_identity() {
    let (mut row, mut truth) = example();
    truth["calls"][0]["callbacks"][0]["name"] = json!("raw_name");
    assert_fields(&row, Some(&truth), &[]);
    truth["projection"][0]
        .as_object_mut()
        .unwrap()
        .remove("callbacks");
    assert_fields(&row, Some(&truth), &["expect[0].callbacks[0].name"]);
    row["expect"][0]["callbacks"][0]["name"] = json!("raw_name");
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0]["callbacks"] = json!([]);
    assert_fields(&row, Some(&truth), &["expect[0].callbacks.length"]);
}

#[test]
fn accepts_only_compatible_declared_method_pins() {
    let (mut row, mut truth) = example();
    truth["projection"][0]
        .as_object_mut()
        .unwrap()
        .remove("methods");
    assert_fields(&row, Some(&truth), &["expect[0].methods"]);
    for pin in [json!("any"), json!({"any": true}), json!({"status": "any"})] {
        row["expect"][0]["methods"] = pin;
        truth["projection"][0]["methods"] = Value::Null;
        assert_fields(&row, Some(&truth), &[]);
        truth["projection"][0]["methods"] = json!({"any": true});
        assert_fields(&row, Some(&truth), &[]);
        truth["projection"][0]["methods"] = json!({"known": ["GET"]});
        assert_fields(&row, Some(&truth), &["expect[0].methods"]);
    }
    row["expect"][0]["methods"] = json!({"status": "unknown", "text": "M"});
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0].as_object_mut().unwrap().remove("methods");
    assert_fields(&row, Some(&truth), &[]);
}

#[test]
fn checks_missing_and_extra_registrations_with_static_exception() {
    let (mut row, mut truth) = example();
    let pin = row["expect"][0].clone();
    row["expect"] = json!([]);
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    row["expect"] = json!([pin]);
    truth["projection"] = json!([]);
    assert_fields(&row, Some(&truth), &["expect[0].registrations"]);
    row["expect"][0]["receiver"] = json!({"status": "unproven"});
    assert_fields(&row, Some(&truth), &["expect[0].registrations"]);
    row["expect"][0]["paths"] = json!([]);
    assert_fields(&row, Some(&truth), &[]);
}

#[test]
fn runtime_only_registrations_require_approved_diagnostic_sites() {
    let (mut row, mut truth) = example();
    row["expect"] = json!([]);
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    row["accepted_loss"] = json!("runtime_generated_registration");
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    row["id"] = json!("et1-10");
    row["expect_diagnostics"] =
        json!([{"site": {"file":"app.js","line":5},"reason":"runtime_generated_registration"}]);
    truth["calls"][0]["kind"] = json!("verb");
    truth["calls"][0]["site"] = json!({"file":"app.js","line":5,"column":14,"ordinal":1});
    assert_fields(&row, Some(&truth), &[]);
    truth["calls"][0]["site"]["line"] = json!(8);
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
}

#[test]
fn checks_truth_availability_by_row_source() {
    let (mut row, truth) = example();
    let refused = json!({"status": "hand_required"});
    assert_fields(&row, None, &["truth"]);
    assert_fields(&row, Some(&refused), &["truth.status"]);
    row["truth_source"] = json!("hand");
    assert_fields(&row, None, &[]);
    assert_fields(&row, Some(&refused), &[]);
    assert_fields(&row, Some(&truth), &["truth.status"]);
}

fn saved_case(id: &str) -> (Value, Value) {
    let root = super::super::fixture_root();
    let rows = super::super::read_rows(&root.join("rows.jsonl"), &mut Vec::new());
    let truth = super::super::read_rows(&root.join("truth/truth.jsonl"), &mut Vec::new());
    (rows[id].clone(), truth[id].clone())
}

#[test]
fn corrupted_websocket_protocol_fails_raw_and_served_checks() {
    for id in ["fa06", "es2-10"] {
        let (row, truth) = saved_case(id);
        assert_fields(&row, Some(&truth), &[]);
        for field in ["calls", "served"] {
            let mut corrupted = truth.clone();
            let websocket = corrupted[field]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|record| record["kind"] == "websocket")
                .unwrap();
            websocket["kind"] = json!("http");
            let mut errors = Vec::new();
            compare_row(&row, Some(&corrupted), &mut errors);
            assert!(
                errors.iter().any(|e| e.contains("protocol")),
                "{id}.{field}: {errors:?}"
            );
        }
    }
}

#[test]
fn swapped_native_mount_links_fail() {
    let (row, mut truth) = saved_case("ex24");
    assert_fields(&row, Some(&truth), &[]);
    truth["served"][0]["mount_call_ids"] = json!([3]);
    truth["served"][1]["mount_call_ids"] = json!([2]);
    let mut errors = Vec::new();
    compare_row(&row, Some(&truth), &mut errors);
    assert!(
        errors.iter().any(|e| e.contains("mount_call_ids")),
        "{errors:?}"
    );
}

#[test]
fn empty_prefix_mount_requires_exactly_one_call_id() {
    let (row, truth) = saved_case("es2-04");
    assert_fields(&row, Some(&truth), &[]);
    let mut undetected = Vec::new();
    for ids in [json!([]), json!([3, 3])] {
        let mut corrupted = truth.clone();
        let inventory = corrupted["served"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|route| route["path"] == "/inventory")
            .unwrap();
        assert_eq!(inventory["mount_call_ids"], json!([3]));
        inventory["mount_call_ids"] = ids.clone();
        let mut errors = Vec::new();
        compare_row(&row, Some(&corrupted), &mut errors);
        if !errors.iter().any(|error| error.contains("mount_call_ids")) {
            undetected.push(ids);
        }
    }
    assert!(undetected.is_empty(), "accepted {undetected:?}");
}

#[test]
fn equal_prefix_mounts_follow_endpoint_order() {
    let (mut row, mut truth) = saved_case("ex25");
    row["expect_mounts"][1]["prefix"]["value"] = json!("/sub");
    row["expect"][0]["paths"][0]["value"] = json!("/sub/sub/status");
    truth["calls"][2]["mount_prefix"] = json!("/sub");
    truth["served"][0]["path"] = json!("/sub/sub/status");
    assert_eq!(truth["served"][0]["mount_call_ids"], json!([3, 2]));
    assert_fields(&row, Some(&truth), &[]);
    truth["served"][0]["mount_call_ids"] = json!([2, 3]);
    assert_fields(&row, Some(&truth), &["truth.served[0].mount_call_ids"]);
}

#[test]
fn runtime_loss_does_not_cover_another_column_on_the_same_line() {
    for (id, column) in [
        ("et1-10", "column"),
        ("es1-20", "column_byte"),
        ("es3-07", "column_byte"),
    ] {
        let (row, mut truth) = saved_case(id);
        assert_fields(&row, Some(&truth), &[]);
        let site = &mut truth["calls"][0]["site"];
        site[column] = json!(site[column].as_u64().unwrap() + 1);
        assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    }
}

#[test]
fn native_path_keys_match_shared_grammar_vectors() {
    let vectors: Vec<(String, String, Option<Vec<Value>>)> = serde_json::from_str(include_str!(
        "../../../../../../tests/routes_matrix/truth/path_key_vectors.json"
    ))
    .unwrap();
    for (framework, path, tokens) in vectors {
        assert_eq!(
            native_tokens(&framework, &path),
            tokens,
            "{framework} {path}"
        );
    }
}
