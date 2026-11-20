//! CONTROLLED typed note/display fixtures through the real SARIF serializer.
use super::*;
use crate::algorithms::taint::boundary::{
    display,
    records::{
        Availability, BoundaryCollection, BoundaryKind, BoundaryRecord, ContextRunNote,
        ContextRunNoteKind, PhysicalSite,
    },
};
use prism::{
    api::{ReviewHintBasis, ReviewImportHint},
    call_graph::FunctionId,
    finding_confidence::{EvidencePath, MinConfidence, ResolutionMode},
    languages::Language,
};

fn note(kind: ContextRunNoteKind, text: &str, location: Option<(&str, usize)>) -> ContextRunNote {
    ContextRunNote {
        kind,
        message: text.into(),
        location: location.map(|(p, l)| (p.into(), l)),
    }
}
fn display(notes: Vec<ContextRunNote>) -> ReviewBoundaryDiagnostics {
    ReviewBoundaryDiagnostics {
        run_notes: notes,
        ..Default::default()
    }
}
fn record(file: &str, i: usize, kind: BoundaryKind) -> BoundaryRecord {
    BoundaryRecord {
        site: PhysicalSite {
            caller: FunctionId {
                file: file.into(),
                name: "f".into(),
                start_line: 1,
                end_line: 20,
            },
            start_byte: i * 10,
            end_byte: i * 10 + 5,
        },
        line: i + 1,
        callee: "g".into(),
        language: Language::JavaScript,
        hint: ReviewImportHint::InRepo {
            files: vec!["target.js".into()],
            declaring: vec![],
            basis: ReviewHintBasis::Bound,
            spec: "./target".into(),
            member: Some("g".into()),
        },
        kind,
    }
}
fn boundary(file: &str, n: usize, availability: Availability) -> ReviewBoundaryDiagnostics {
    display::project(&BoundaryCollection {
        availability,
        records: (0..n)
            .map(|i| record(file, i, BoundaryKind::InRepo))
            .collect(),
    })
}
fn notices(doc: &Value) -> Vec<Value> {
    doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}
fn messages(doc: &Value) -> Vec<String> {
    notices(doc)
        .iter()
        .map(|n| n["message"]["text"].as_str().unwrap().to_string())
        .collect()
}
#[test]
fn n01_no_run_notes_is_byte_identical() {
    let f = [finding("taint", "a.js", 1)];
    let evidence = [Some(EvidencePath::unlabeled_for("taint"))];
    let empty = display(Vec::new());
    let plain = to_sarif(&SarifInputs::new(&f).evidence(&evidence));
    let with_empty = to_sarif(
        &SarifInputs::new(&f)
            .evidence(&evidence)
            .boundary_diagnostics(&empty),
    );
    assert_eq!(
        serde_json::to_vec(&plain).unwrap(),
        serde_json::to_vec(&with_empty).unwrap()
    );
    let existing = boundary("a.js", 1, Availability::Available);
    let doc = to_sarif(
        &SarifInputs::new(&f)
            .evidence(&evidence)
            .boundary_diagnostics(&existing),
    );
    let rows = notices(&doc);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["level"], "note");
    assert_eq!(rows[0]["message"]["text"], existing.located()[0].message());
}
#[test]
fn n02_each_kind_has_level_text_and_order() {
    use ContextRunNoteKind::*;
    let kinds = [
        (DepthBound, "note"),
        (FilesBound, "note"),
        (BytesBound, "note"),
        (TimeGuard, "note"),
        (Cancelled, "note"),
        (Ignored, "warning"),
        (Unauthorized, "warning"),
        (Unavailable, "warning"),
        (StageFailed, "warning"),
        (UnionRefused, "warning"),
        (FileSkipped, "warning"),
        (SkippedSummary, "warning"),
        (ContextParse, "warning"),
    ];
    let text = "already\\n\\u{202e}\\\"\\\\literal";
    let mut notes: Vec<_> = kinds.iter().map(|(k, _)| note(*k, text, None)).collect();
    notes.push(note(Ignored, text, None));
    let d = display(notes);
    let doc = to_sarif(&SarifInputs::new(&[]).boundary_diagnostics(&d));
    let rows = notices(&doc);
    assert_eq!(rows.len(), 14);
    for (i, (_, level)) in kinds.iter().enumerate() {
        assert_eq!(rows[i]["level"], *level);
        assert_eq!(rows[i]["message"]["text"], text);
    }
    assert_eq!(rows[13]["message"]["text"], text);
    assert_eq!(
        doc["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
    let errors = [AlgorithmError {
        algorithm: "Chop".into(),
        error: "failed".into(),
    }];
    let failed = to_sarif(
        &SarifInputs::new(&[])
            .errors(&errors)
            .boundary_diagnostics(&d),
    );
    assert_eq!(
        failed["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
}
#[test]
fn n03_explicit_optional_location_has_a_region() {
    let d = display(vec![
        note(
            ContextRunNoteKind::FilesBound,
            "located",
            Some(("dir/a b.js", 7)),
        ),
        note(ContextRunNoteKind::FilesBound, "unlocated", None),
    ]);
    let rows = notices(&to_sarif(&SarifInputs::new(&[]).boundary_diagnostics(&d)));
    assert_eq!(rows.len(), 2);
    let p = &rows[0]["locations"][0]["physicalLocation"];
    assert_eq!(p["artifactLocation"]["uri"], "dir/a%20b.js");
    assert_eq!(p["artifactLocation"]["uriBaseId"], "%SRCROOT%");
    assert_eq!(p["region"]["startLine"], 7);
    assert!(rows[1].get("locations").is_none());
}
#[test]
fn n04_actual_channels_precede_run_notes() {
    let mut d = boundary(
        "a.js",
        11,
        Availability::ListingUnavailable("missing".into()),
    );
    let f = [finding("taint", "a.js", 1)];
    let errors = [AlgorithmError {
        algorithm: "Chop".into(),
        error: "bad".into(),
    }];
    let parse = ["parse".into()];
    let load = ["load".into()];
    let build = ["build".into()];
    let before = to_sarif(
        &SarifInputs::new(&f)
            .errors(&errors)
            .parse_warnings(&parse)
            .load_warnings(&load)
            .build_warnings(&build)
            .boundary_diagnostics(&d),
    );
    let old = notices(&before);
    assert_eq!(old[0]["message"]["text"], "Chop: bad");
    assert_eq!(old[1]["message"]["text"], "parse");
    assert_eq!(old[2]["message"]["text"], "load");
    assert!(old[3]["message"]["text"]
        .as_str()
        .unwrap()
        .contains("unavailable"));
    assert_eq!(old[4]["message"]["text"], "build");
    assert!(old[5]["message"]["text"]
        .as_str()
        .unwrap()
        .contains("evidence alignment mismatch"));
    d.run_notes.push(note(
        ContextRunNoteKind::StageFailed,
        "new",
        Some(("plain.js", 4)),
    ));
    let mut after = to_sarif(
        &SarifInputs::new(&f)
            .errors(&errors)
            .parse_warnings(&parse)
            .load_warnings(&load)
            .build_warnings(&build)
            .boundary_diagnostics(&d),
    );
    let rows = notices(&after);
    assert_eq!(&rows[..old.len()], &old);
    assert_eq!(rows.len(), old.len() + 1);
    assert_eq!(rows.last().unwrap()["message"]["text"], "new");
    after["runs"][0]["invocations"][0]["toolExecutionNotifications"] = Value::Array(old);
    assert_eq!(after, before);
}
#[test]
fn n05_notes_without_any_finding() {
    let d = display(vec![note(ContextRunNoteKind::Ignored, "ignored", None)]);
    let doc = to_sarif(&SarifInputs::new(&[]).boundary_diagnostics(&d));
    let n = notices(&doc);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0]["level"], "warning");
    assert_eq!(n[0]["message"]["text"], "ignored");
    assert_eq!(doc["runs"][0]["results"].as_array().unwrap().len(), 0);
    assert_eq!(
        doc["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
}
#[test]
fn n06_loaded_unresolved_remains_located_boundary_note() {
    let c = BoundaryCollection {
        availability: Availability::Available,
        records: vec![record("a.js", 1, BoundaryKind::LoadedUnresolved)],
    };
    let d = display::project(&c);
    let expected = d.located()[0].message().to_string();
    let doc = to_sarif(&SarifInputs::new(&[]).boundary_diagnostics(&d));
    let rows = notices(&doc);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["level"], "note");
    assert_eq!(rows[0]["message"]["text"], expected);
    assert_eq!(
        rows[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "a.js"
    );
    assert_eq!(
        rows[0]["locations"][0]["physicalLocation"]["region"]["startLine"],
        2
    );
}
#[test]
fn n07_escaping_paths_share_one_early_warning() {
    let mut d = boundary("../shared.js", 1, Availability::Available);
    d.run_notes = vec![
        note(
            ContextRunNoteKind::FileSkipped,
            "new-only",
            Some(("../new-only.js", 3)),
        ),
        note(
            ContextRunNoteKind::FileSkipped,
            "shared-one",
            Some(("../shared.js", 4)),
        ),
        note(
            ContextRunNoteKind::FileSkipped,
            "shared-two",
            Some(("../shared.js", 5)),
        ),
    ];
    let doc = to_sarif(&SarifInputs::new(&[]).boundary_diagnostics(&d));
    let m = messages(&doc);
    assert_eq!(m.len(), 6);
    assert_eq!(m[0], "path escapes repo root: ../new-only.js");
    assert_eq!(m[1], "path escapes repo root: ../shared.js");
    assert_eq!(m[2], d.located()[0].message());
    assert_eq!(&m[3..], &["new-only", "shared-one", "shared-two"]);
    let rows = notices(&doc);
    for row in &rows[2..] {
        assert!(row.get("locations").is_some());
    }
}
#[test]
fn n08_run_notes_survive_exact_floor() {
    let f = [finding("taint", "a.js", 1)];
    let evidence = [Some(EvidencePath::unlabeled_for("taint"))];
    let d = display(vec![note(ContextRunNoteKind::Ignored, "kept", None)]);
    let doc = to_sarif(
        &SarifInputs::new(&f)
            .evidence(&evidence)
            .min_confidence(MinConfidence::Exact)
            .resolution(ResolutionMode::Scoped)
            .boundary_diagnostics(&d),
    );
    assert!(doc["runs"][0]["results"].as_array().unwrap().is_empty());
    let rows = notices(&doc);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["message"]["text"], "kept");
}
