use super::{evidence, tests::file, Family};
use crate::targets::{inventory_document, TargetsMeta};
use serde_json::{json, Value};
use std::{fs, path::Path};
use tempfile::tempdir;

fn fixture(id: &str) -> String {
    let dir = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/targets_inventory/py/http_receiver"
    ));
    fs::read_to_string(dir.join(format!("{id}.py"))).unwrap()
}

#[test]
fn receiver_evidence_joins_both_occurrences_of_esc22() {
    let repo = tempdir().unwrap();
    let out = file(&fixture("esc22"), repo.path(), &[Family::Http]);
    let doc = inventory_document(&TargetsMeta::default(), &out);
    let [group] = doc.targets.as_slice() else {
        panic!("expected one grouped unsupported row");
    };
    let traces = evidence::sidecar(&doc.targets, &out.occurrences);
    let [first, second] = traces.as_slice() else {
        panic!("expected one evidence entry per call");
    };
    assert_eq!((&first.id, &second.id), (&group.id, &group.id));
    assert_ne!(first.occurrence, second.occurrence);
    assert_eq!(first.proof.route, evidence::Route::R2);
    assert_eq!(second.proof.route, evidence::Route::R5);
    assert_eq!(first.proof.call.range, [7, 15, 7, 28]);
    assert_eq!(second.proof.call.range, [7, 30, 7, 43]);
    assert_eq!(first.proof.call.text, "direct.get(u)");
    assert_eq!(second.proof.call.text, "helper.get(u)");
    let traces = serde_json::to_value(traces).unwrap();
    assert_eq!(
        traces[0]["proof"]["receiver"]["construction"]["range"][0],
        5
    );
    assert_eq!(
        traces[1]["proof"]["receiver"]["construction"]["range"][0],
        3
    );
}

#[test]
fn receiver_evidence_records_the_eight_constructed_class_promotions() {
    let repo = tempdir().unwrap();
    for (id, route, construction, declaration) in [
        ("sl2-22", "r4", 4, 6),
        ("as2-24", "r5", 4, 2),
        ("wf61", "r4", 5, 7),
        ("wf63", "r4", 4, 6),
        ("wf67", "r4", 4, 6),
        ("wf69", "r5", 5, 4),
        ("wf70", "r5", 4, 3),
        ("wf73", "r5", 5, 4),
    ] {
        let out = file(&fixture(id), repo.path(), &[Family::Http]);
        assert!(out.unproven.is_empty(), "{id}");
        assert_eq!(out.sites.len(), 1, "{id}");
        assert_eq!(out.sites[0].resolved_callee, "requests.Session.get", "{id}");
        let doc = inventory_document(&TargetsMeta::default(), &out);
        let evidence =
            serde_json::to_value(evidence::sidecar(&doc.targets, &out.occurrences)).unwrap();
        let proof = &evidence[0]["proof"];
        assert_eq!(proof["route"], route, "{id}");
        assert_eq!(
            proof["receiver"]["construction"]["class"], "requests.Session",
            "{id}"
        );
        assert_eq!(
            proof["receiver"]["construction"]["range"][0], construction,
            "{id}"
        );
        let conflicts = &proof["receiver"]["conflicting_declarations"];
        assert_eq!(conflicts.as_array().map(Vec::len), Some(1), "{id}");
        assert_eq!(conflicts[0]["class"], "httpx.Client", "{id}");
        assert_eq!(conflicts[0]["range"][0], declaration, "{id}");
        let public = serde_json::to_value(&doc.targets[0]).unwrap();
        assert_eq!(public.get("proof"), None::<&Value>, "{id}");
        assert_eq!(public["confidence"], json!("nameonly"), "{id}");
    }
}
