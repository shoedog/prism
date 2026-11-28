use super::{evidence, tests::file, Family};
use crate::targets::{inventory_document, TargetsMeta};
use serde_json::{json, Value};
use std::{fs, path::Path};
use tempfile::tempdir;

fn fixture(id: &str) -> String {
    fixture_in("http_receiver", id)
}

fn fixture_in(group: &str, id: &str) -> String {
    let dir = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/targets_inventory/py"
    ));
    fs::read_to_string(dir.join(group).join(format!("{id}.py"))).unwrap()
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

#[test]
fn stdlib_declaration_sidecar_replays_source_without_construction() {
    let repo = tempdir().unwrap();
    // The byte ranges and call text are from the pinned 3.11.11 AST.
    for (id, route, line, decl, call, callee_end, text) in [
        (
            "pin03",
            evidence::Route::R4,
            5,
            [4, 39, 4, 43],
            [5, 14, 5, 37],
            35,
            "config_path.read_text()",
        ),
        (
            "pin04",
            evidence::Route::R4,
            5,
            [4, 20, 4, 24],
            [5, 7, 5, 24],
            22,
            "pot_path.exists()",
        ),
        (
            "pin04",
            evidence::Route::R4,
            6,
            [4, 20, 4, 24],
            [6, 8, 6, 33],
            27,
            "pot_path.write_text(text)",
        ),
        (
            "pin05",
            evidence::Route::R4,
            6,
            [5, 19, 5, 23],
            [6, 4, 6, 25],
            23,
            "frontend_dist.mkdir()",
        ),
        (
            "pin06",
            evidence::Route::R4,
            7,
            [3, 16, 3, 20],
            [7, 4, 7, 26],
            24,
            "RELOAD_TRIGGER.touch()",
        ),
        (
            "as1-28",
            evidence::Route::R5,
            8,
            [2, 25, 2, 29],
            [8, 15, 8, 28],
            26,
            "path.exists()",
        ),
        (
            "sl4-14",
            evidence::Route::R4,
            3,
            [2, 11, 2, 15],
            [3, 15, 3, 25],
            23,
            "src.stat()",
        ),
    ] {
        let source = fixture_in("stdlib", id);
        let out = file(&source, repo.path(), &[Family::Filesystem]);
        assert!(out.unproven.is_empty(), "{id}");
        let doc = inventory_document(&TargetsMeta::default(), &out);
        let target = doc
            .targets
            .iter()
            .find(|target| target.site.line == line)
            .unwrap();
        let traces = evidence::sidecar(&doc.targets, &out.occurrences);
        let witnesses: Vec<_> = traces
            .iter()
            .filter(|trace| trace.id == target.id)
            .collect();
        let [trace] = witnesses.as_slice() else {
            panic!("expected one trace per target for {id}:{line}");
        };
        let proof = &trace.proof;
        assert_eq!(proof.route, route, "{id}");
        assert_eq!(proof.call.range, call, "{id}");
        assert_eq!(proof.call.text, text, "{id}");
        assert_eq!(
            proof.callee.range,
            [call[0], call[1], call[2], callee_end],
            "{id}"
        );
        assert_eq!(proof.callee.text, text.split_once('(').unwrap().0, "{id}");
        let wire = serde_json::to_value(proof).unwrap();
        assert_eq!(
            wire["declaration"],
            json!({"class":"pathlib.Path","range":decl}),
            "{id}"
        );
        assert!(wire.get("receiver").is_none(), "{id}");
        assert!(serde_json::to_value(target).unwrap().get("proof").is_none());
        assert_eq!(
            target.kind,
            if id == "sl4-14" {
                "other"
            } else {
                "external_call"
            }
        );
    }
}

#[test]
fn stdlib_construction_and_starred_annotation_sidecar_spans_match_source() {
    let repo = tempdir().unwrap();
    for (id, family, route, line, construction, conflict, range, end, text) in [
        (
            "as1-27",
            Family::Parse,
            evidence::Route::R5,
            6,
            Some(("configparser.ConfigParser", [3, 11, 3, 25])),
            Some([2, 14, 2, 29]),
            [6, 11, 6, 28],
            22,
            "parser.read(path)",
        ),
        (
            "wf72",
            Family::Parse,
            evidence::Route::R5,
            7,
            Some(("configparser.ConfigParser", [4, 11, 4, 25])),
            Some([3, 14, 3, 39]),
            [7, 11, 7, 30],
            27,
            "parser.read_file(p)",
        ),
        (
            "tr3-26",
            Family::Filesystem,
            evidence::Route::R5,
            6,
            Some(("pathlib.Path", [3, 11, 3, 20])),
            None,
            [6, 11, 6, 24],
            22,
            "p.read_text()",
        ),
        (
            "pin07",
            Family::Filesystem,
            evidence::Route::R1,
            6,
            Some(("pathlib.Path", [6, 24, 6, 42])),
            None,
            [6, 24, 6, 54],
            52,
            "Path(\"Dockerfile\").read_text()",
        ),
        (
            "pin08",
            Family::Filesystem,
            evidence::Route::R2,
            7,
            Some(("pathlib.Path", [5, 18, 5, 28])),
            None,
            [7, 7, 7, 28],
            26,
            "path_object.is_file()",
        ),
        (
            "pin08",
            Family::Filesystem,
            evidence::Route::R2,
            9,
            Some(("pathlib.Path", [5, 18, 5, 28])),
            None,
            [9, 9, 9, 29],
            27,
            "path_object.exists()",
        ),
        (
            "as3-32",
            Family::Parse,
            evidence::Route::Root,
            3,
            None,
            None,
            [3, 23, 3, 42],
            33,
            r#"json.loads('["T"]')"#,
        ),
    ] {
        let out = file(&fixture_in("stdlib", id), repo.path(), &[family]);
        assert!(out.unproven.is_empty(), "{id}");
        let doc = inventory_document(&TargetsMeta::default(), &out);
        let target = doc
            .targets
            .iter()
            .find(|target| target.site.line == line)
            .unwrap();
        let traces = evidence::sidecar(&doc.targets, &out.occurrences);
        let proof = &traces
            .iter()
            .find(|trace| trace.id == target.id)
            .unwrap()
            .proof;
        assert_eq!(proof.route, route, "{id}");
        assert_eq!(proof.call.range, range, "{id}");
        assert_eq!(proof.call.text, text, "{id}");
        assert_eq!(proof.callee.range, [line, range[1], line, end], "{id}");
        assert_eq!(proof.callee.text, &text[..end - range[1]], "{id}");
        assert!(proof.declaration.is_none(), "{id}");
        assert_eq!(
            proof.receiver.as_ref().map(|receiver| (
                receiver.construction.class.as_str(),
                receiver.construction.range
            )),
            construction,
            "{id}"
        );
        if let Some(range) = conflict {
            let conflicts = &proof.receiver.as_ref().unwrap().conflicting_declarations;
            assert_eq!(conflicts.len(), 1, "{id}");
            assert_eq!(conflicts[0].class, "configparser.RawConfigParser", "{id}");
            assert_eq!(conflicts[0].range, range, "{id}");
        }
    }
}

#[test]
fn field_sidecar_has_occurrence_spans_by_default() {
    let repo = tempdir().unwrap();
    let key = "app/main.py";
    let source = fixture_in("stdlib/pfield", "pf01");
    let parsed =
        prism::ast::ParsedFile::parse(key, &source, prism::languages::Language::Python).unwrap();
    let out = super::collect(
        &std::collections::BTreeMap::from([(key.into(), parsed)]),
        repo.path(),
        &super::InventoryOptions::default(),
    );
    let doc = inventory_document(&TargetsMeta::default(), &out);
    let traces = serde_json::to_value(evidence::sidecar(&doc.targets, &out.occurrences)).unwrap();
    let traces = traces.as_array().unwrap();
    assert_eq!(traces.len(), 3, "{traces:#?}");
    assert_eq!(
        traces
            .iter()
            .filter(|t| t["proof"]["route"] == "r2")
            .count(),
        1
    );
    let expected = [
        ("self.path.exists()", [8, 15, 8, 24]),
        ("self.path.write_text(text)", [11, 15, 11, 24]),
    ];
    for (text, receiver_range) in expected {
        let trace = traces
            .iter()
            .find(|t| t["proof"]["call"]["text"] == text)
            .unwrap();
        let proof = &trace["proof"];
        assert_eq!(proof["route"], "r3");
        assert!(proof.get("receiver").is_none());
        assert!(proof.get("declaration").is_none());
        assert_eq!(proof["field"]["class"], "pathlib.Path");
        let source_lines: Vec<_> = source.split_inclusive('\n').collect();
        for (key, range) in [
            ("owner", [2, 0, 11, 41]),
            ("initializer", [3, 4, 5, 24]),
            ("parameter", [3, 23, 3, 33]),
            ("annotation", [3, 29, 3, 33]),
            ("store", [5, 8, 5, 24]),
            ("receiver_use", receiver_range),
        ] {
            let span = &proof["field"][key];
            assert_eq!(span["range"], json!(range), "{key}");
            let [start_line, start_col, end_line, end_col] = range;
            let start = source_lines[..start_line - 1]
                .iter()
                .map(|s| s.len())
                .sum::<usize>()
                + start_col;
            let end = source_lines[..end_line - 1]
                .iter()
                .map(|s| s.len())
                .sum::<usize>()
                + end_col;
            assert_eq!(span["text"], &source[start..end], "{key}");
        }
        let target = doc
            .targets
            .iter()
            .find(|row| trace["id"] == row.id)
            .unwrap();
        assert!(serde_json::to_value(target).unwrap().get("proof").is_none());
    }
}
