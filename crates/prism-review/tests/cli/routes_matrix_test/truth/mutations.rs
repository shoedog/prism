use super::super::{fixture_root, mutations::edits, read_rows, rows::tree_sha256};
use super::{compare_receipt, items, runtime_only_count};
use serde_json::Value;
use std::collections::BTreeSet;

// These facts require the source oracle or native replay beyond this comparator.
// Keep the exceptions contextual where another receipt independently supplies the fact.
fn static_only(row: &Value, truth: &Value, pointer: &str, field: &str) -> Option<&'static str> {
    if matches!(
        field,
        "served[].effective_methods" | "served[].effective_methods[]"
    ) {
        return Some("effective methods");
    }
    if matches!(
        field,
        "served[].endpoint" | "calls[].args.endpoint" | "calls[].args.name"
    ) {
        return Some("native endpoint names");
    }
    let mut location = pointer.split('/').skip(1);
    let collection = location.next()?;
    if collection == "served" && field.starts_with("served[].runtime_callback") {
        let index: usize = location.next()?.parse().ok()?;
        let id = &truth["served"][index]["call_ids"][0];
        let linked = items(&truth["calls"])
            .iter()
            .find(|call| call["id"] == *id)?;
        if items(&linked["callbacks"]).is_empty() {
            return Some("endpoint-only runtime callback");
        }
    }
    if collection != "calls" {
        return None;
    }
    let index: usize = location.next()?.parse().ok()?;
    let call = &truth["calls"][index];
    let id = &call["id"];
    let projection = items(&truth["projection"])
        .iter()
        .find(|record| record["call_id"] == *id);
    let served = items(&truth["served"]);
    let mounted = served
        .iter()
        .any(|record| items(&record["mount_call_ids"]).contains(id));
    let paired_copy = call["mounted"] == true
        || items(&truth["calls"])
            .iter()
            .any(|other| other["origin_call_id"] == *id);
    let installed = served
        .iter()
        .any(|record| items(&record["call_ids"]).last() == Some(id));
    match field {
        "calls[].site.column"
        | "calls[].site.column_byte"
        | "calls[].site.end_column_byte"
        | "calls[].site.end_line"
            if runtime_only_count(row).is_none() =>
        {
            Some("unpinned call-frame columns and end line")
        }
        "calls[].site.file" | "calls[].site.line" if projection.is_none() && !mounted => {
            Some("unpaired call-frame file and line")
        }
        "calls[].args.router_prefix"
            if !installed
                && !items(&truth["calls"])
                    .iter()
                    .any(|other| other["origin_call_id"] == *id) =>
        {
            Some("unconsumed router prefix")
        }
        "calls[].args.has_path_arg" if !mounted && call.get("path").is_none() => {
            Some("untraced middleware path presence")
        }
        "calls[].path" | "calls[].path.raw" | "calls[].path.literal"
            if projection.is_none()
                && !items(&row["expect_mounts"]).iter().any(|pin| {
                    pin["prefix"]["status"] == "unevaluated"
                        && pin["site"]["file"] == call["site"]["file"]
                        && pin["site"]["line"] == call["site"]["line"]
                }) =>
        {
            Some("unprojected source-path helper")
        }
        "calls[].callbacks[].column" | "calls[].callbacks[].source" => {
            Some("raw callback column and source")
        }
        _ if field.starts_with("calls[].callbacks[].")
            && projection.is_none_or(|record| record.get("callbacks").is_none())
            && !paired_copy
            && !(truth["framework"] == "flask" && installed) =>
        {
            Some("unprojected callback identity")
        }
        _ => None,
    }
}

#[test]
fn every_checked_saved_truth_field_rejects_generated_corruption() {
    let root = fixture_root();
    let mut errors = Vec::new();
    let rows = read_rows(&root.join("rows.jsonl"), &mut errors);
    let receipts = read_rows(&root.join("truth/truth.jsonl"), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    let mut executed = 0;
    let mut fields = 0;
    let mut tested = 0;
    let mut rejected = 0;
    let mut exceptions = BTreeSet::new();
    let mut survivors = Vec::new();
    for (id, row) in &rows {
        if row["truth_source"] != "executed" {
            continue;
        }
        let truth = &receipts[id];
        let hash = tree_sha256(&root.join(row["dir"].as_str().unwrap()));
        compare_receipt(row, truth, &hash, &mut errors);
        assert!(errors.is_empty(), "{id}: {errors:?}");
        executed += 1;
        let mut row_fields = BTreeSet::new();
        for edit in edits(truth) {
            if let Some(reason) = static_only(row, truth, &edit.pointer, &edit.pattern) {
                exceptions.insert(reason);
                continue;
            }
            tested += 1;
            row_fields.insert(edit.pointer.clone());
            compare_receipt(row, &edit.apply(truth), &hash, &mut errors);
            if errors.is_empty() {
                survivors.push(format!("{id} {}", edit.pointer));
            } else {
                rejected += 1;
                errors.clear();
            }
        }
        fields += row_fields.len();
    }
    println!("{executed} executed rows, {fields} field instances, {rejected}/{tested} corruptions rejected; static-only: {exceptions:?}");
    assert!(
        survivors.is_empty(),
        "{} accepted corruptions: {survivors:?}",
        survivors.len()
    );
}
