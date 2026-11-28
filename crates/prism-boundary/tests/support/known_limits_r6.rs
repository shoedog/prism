// Known-limit rows for the round-6 findings of PR #958. Each row pins what the
// current code does on an input outside the certified request shape, beside a
// control that behaves correctly. EXPECTED UNTIL THE REDESIGN: when the single
// settlement path lands these assertions flip, and the rows are deleted.
use crate::doc_cases::json_fixture;
use crate::fold2::{admission, admission_with_bytes, impact, live_pin, media_request};
use crate::fold3::historical_computed;
use serde_json::{json, Value};

const CUTOFF: &str = "2026-11-25T12:00:00Z";

fn row(id: &str) -> Value {
    json_fixture("known-limits.json")[&format!("known-limit-{id}")].clone()
}

fn historical_pin(rev: &str) -> Value {
    json!({"kind":"historical_pin","branch":"main","rev":rev,
        "available_at":CUTOFF,"evidence":"archive"})
}

fn status<'a>(report: &'a prism_boundary::revision::AdmissionReport, id: &str) -> &'a str {
    if report.claims[id].status == "admitted" {
        "admitted"
    } else {
        "unchecked"
    }
}

#[test]
fn known_limit_12_live_exemption_reads_the_raw_subject_head() {
    let row = row("12");
    let mut input = json_fixture("admission.json");
    input["inputs"][1]["selection"] = live_pin("client-before-rev");
    input["inputs"][2]["selection"] = live_pin("service-after-rev");
    input["claims"].as_array_mut().unwrap().push(json!({
        "id":"head","input_id":"service-after","slot":"service-after",
        "purpose":"metadata","receipt_ids":[]
    }));
    let control = admission(input.clone());
    assert_eq!(status(&control, "head"), "admitted", "{control:?}");
    assert_eq!(status(&control, "schema-claim"), "admitted");
    assert!(control.counts.is_empty());

    input["inputs"][2]["selection"] = live_pin("wrong-rev");
    let report = admission(input);
    assert_eq!(status(&report, "head"), "unchecked", "{report:?}");
    // Known limit: the same-role exemption reads the raw head, so the claim stays
    // admitted although the head's own selection is not.
    assert_eq!(
        status(&report, "schema-claim"),
        row["observed"]["schema_claim"].as_str().unwrap()
    );
}

#[test]
fn known_limit_13_historical_media_claim_skips_the_topic_head_edge() {
    let row = row("13");
    let (mut input, bytes) = media_request();
    input["mode"] = json!("historical_as_of");
    input["cutoff_at"] = json!(CUTOFF);
    input["subject_head_input_id"] = json!("service-after");
    input["inputs"][2]["snapshot"]["committed_at"] = json!(CUTOFF);
    input["inputs"][1]["selection"] = historical_pin("client-before-rev");
    input["inputs"][2]["selection"] = historical_pin("service-after-rev");
    let control = admission_with_bytes(input.clone(), &bytes);
    assert!(control.counts.is_empty(), "{control:?}");
    let media = control
        .claims
        .keys()
        .find(|id| id.starts_with("media:"))
        .expect("a media decision claim")
        .clone();
    assert_eq!(status(&control, &media), "admitted");

    input["inputs"][2]["selection"] = historical_pin("wrong-rev");
    let report = admission_with_bytes(input, &bytes);
    assert_eq!(status(&report, "schema-claim"), "unchecked", "{report:?}");
    // Known limit: media decisions are settled before the selection dependencies.
    assert_eq!(
        status(&report, &media),
        row["observed"]["media_claim"].as_str().unwrap()
    );
}

fn frozen_input(wire: &Value, id: &str) -> Value {
    wire["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|input| input["input_id"] == id)
        .unwrap()
        .clone()
}

#[test]
fn known_limit_14_frozen_cutoff_loses_duplicate_merge_holders() {
    let row = row("14");
    let (_, wire) = historical_computed();
    let control = impact(wire.clone()).validate();
    assert_eq!(control.claims["report-cutoff"].status, "admitted");

    let mut holder = frozen_input(&wire, "service-after");
    holder["input_id"] = json!("duplicate");
    let mut plain = frozen_input(&wire, "topic-head");
    plain["input_id"] = json!("duplicate");
    let outcome = |first: &Value, second: &Value| {
        let mut wire = wire.clone();
        let inputs = wire["inputs"].as_array_mut().unwrap();
        inputs.push(first.clone());
        inputs.push(second.clone());
        let report = impact(wire).validate();
        (
            report.claims["input:duplicate"].status == "admitted",
            report.claims["report-cutoff"].status == "admitted",
        )
    };
    // Control order: the duplicate commit comes first, the cutoff refuses.
    let (_, cutoff) = outcome(&plain, &holder);
    assert!(!cutoff);
    // Known limit: with the holder first the duplicate is refused but the cutoff is not.
    let (input, cutoff) = outcome(&holder, &plain);
    assert!(!input);
    assert_eq!(cutoff, row["observed"]["report_cutoff_admitted"]);
}

#[test]
fn known_limit_15_frozen_cutoff_waives_proof_on_an_unchecked_before_input() {
    let row = row("15");
    let (_, wire) = historical_computed();
    let control = impact(wire.clone()).validate();
    assert_eq!(control.claims["input:service-before"].status, "admitted");
    assert_eq!(control.claims["report-cutoff"].status, "admitted");

    let mut broken = wire;
    let before = broken["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|input| input["input_id"] == "service-before")
        .unwrap();
    before["dependencies"] = json!(["missing"]);
    let report = impact(broken).validate();
    assert_eq!(report.claims["input:service-before"].status, "unchecked");
    // Known limit: the waiver reads the raw Before role, repo and snapshot.
    assert_eq!(
        report.claims["report-cutoff"].status == "admitted",
        row["observed"]["report_cutoff_admitted"]
    );
}
