use super::matrix::rows;
use super::wire_cases::run_catalog;
use prism_boundary::element::SuccessKey;
use prism_boundary::{Coverage, JsonMediaSource, RootKey};
use serde_json::{json, Value};

#[test]
fn pinned_swagger_omission_proofs_are_root_scoped() {
    let mut failures = Vec::new();
    for (row_id, op, roots, fallback) in [
        (
            "s1-swagger-response-ref-two-statuses",
            "POST /reed",
            &[(200, "/wax"), (201, "/wax")][..],
            false,
        ),
        (
            "swagger-default-exclusion",
            "GET /cone",
            &[(200, "/fired")][..],
            true,
        ),
    ] {
        let row = rows().find(|row| row.id == row_id).unwrap();
        for (route, template) in [
            (
                "server_codec",
                include_str!("../fixtures/p1b_rows/fixtures/m03-codec/codec.media.json"),
            ),
            (
                "producer_source",
                include_str!("../fixtures/p1b_rows/fixtures/m03-source/source.media.json"),
            ),
        ] {
            let proof: Value = serde_json::from_str(template).unwrap();
            for variant in [
                "admitted",
                "missing",
                "mis-scoped",
                "conflicting-media",
                "incomplete",
            ] {
                let mut records = Vec::new();
                for (status, _) in roots {
                    let mut entry = proof[1].clone();
                    entry["operation"] = json!(format!("op:{op}"));
                    entry["status"] = json!(status.to_string());
                    if *status == 200 {
                        match variant {
                            "mis-scoped" => entry["status"] = json!("202"),
                            "conflicting-media" => entry["media_type"] = json!("text/plain"),
                            "incomplete" => {
                                entry["coverage"]["complete"] = json!(false);
                                entry["generator"]["final_dispatch_json"] = json!(false);
                            }
                            _ => {}
                        }
                    }
                    records.push(entry);
                }
                let bytes = serde_json::to_vec(&records).unwrap();
                let catalog = (variant != "missing").then_some(bytes.as_slice());
                let index = run_catalog(&row, &Value::Null, catalog).unwrap();
                let mut comparable: Vec<_> = index
                    .fields
                    .iter()
                    .map(|field| field.id.to_string())
                    .collect();
                comparable.sort();
                let mut observed: Vec<_> = index
                    .observed_fields
                    .iter()
                    .map(|field| field.id.to_string())
                    .collect();
                observed.sort();
                let promoted =
                    |status| variant == "admitted" || (variant != "missing" && status != 200);
                let mut expected_fields = Vec::new();
                let mut expected_observed = Vec::new();
                let Some(operation) = index.operations.first() else {
                    failures.push(format!("{row_id}/{route}/{variant}: expected operation op:{op} with scoped JSON root evidence"));
                    continue;
                };
                for (i, (status, ptr)) in roots.iter().enumerate() {
                    let key = RootKey::Response(SuccessKey::Exact(*status));
                    let id = format!("field:resp:{op}:{status}#{ptr}");
                    let actual = operation.messages.iter().find(|message| message.key == key);
                    let valid = if promoted(*status) {
                        expected_fields.push(id);
                        actual.is_some_and(|message| {
                            message.coverage == Coverage::Checked
                                && message.json_source
                                    == Some(JsonMediaSource::Producer {
                                        input_id: "media-catalog".into(),
                                        entry: i + 1,
                                    })
                        })
                    } else {
                        expected_observed.push(id);
                        let expected = if variant == "missing" {
                            Coverage::ObservedOnly
                        } else {
                            Coverage::Withheld
                        };
                        actual.is_some_and(|message| {
                            message.coverage == expected
                                && message
                                    .limits
                                    .iter()
                                    .filter(|limit| {
                                        limit.reason == "media_unverified"
                                            && limit.pointer.is_empty()
                                    })
                                    .count()
                                    == 1
                        })
                    };
                    if !valid {
                        failures.push(format!("{row_id}/{route}/{variant}: invalid scope or JSON proof on {op}:{status}"));
                    }
                }
                if comparable != expected_fields || observed != expected_observed {
                    failures.push(format!("{row_id}/{route}/{variant}: expected fields {expected_fields:?} and observations {expected_observed:?}, got {comparable:?} and {observed:?}"));
                }
                if fallback
                    && !operation.messages.iter().any(|message| {
                        message.key == RootKey::Fallback
                            && message.coverage == Coverage::Withheld
                            && message
                                .limits
                                .iter()
                                .any(|limit| limit.reason == "fallback_success")
                    })
                {
                    failures.push(format!(
                        "{row_id}/{route}/{variant}: fallback must keep its success coverage limit"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Swagger producer media proof controls\n{}",
        failures.join("\n")
    );
}
