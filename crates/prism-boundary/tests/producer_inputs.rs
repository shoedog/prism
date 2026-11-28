//! P2-1 admitted-input rows: receipt-bound sources, per-file coverage and adapter refusals.
//! DESIGN 5.2 (coverage), 5.6 (custody); enumeration rows D01-D04, D12-D13, X11, A15, A16.

#[allow(dead_code)]
#[path = "support/producer.rs"]
mod support;

use prism_boundary::model::{AnnotationDisposition as D, EdgeGrade, ExtractionStatus as S};
use prism_boundary::producer::{
    bind_producer, ExpectedFile, ProducerAdapter, ProducerError, ProducerInputs, ProducerOptions,
    SourceBlob,
};
use prism_boundary::SchemaIndex;
use serde_json::json;
use support::*;

const H: &[u8] = b"package p\n\n// @Router /x [get]\nfunc H() {}\n";
const K: &[u8] = b"package q\n\n// @Router /y [get]\nfunc K() {}\n";

fn status(doc: &prism_boundary::model::BindingsDocument, path: &str) -> (S, Option<String>) {
    let region = file_region(doc, path);
    (region.status, region.reason.clone())
}

#[test]
fn d12_a_file_whose_blob_or_tree_differs_is_refused_for_that_file_only() {
    // X11 / D12: matching text earns no custody credit; the sibling is untouched.
    let expected = [
        ExpectedFile {
            path: "a.go",
            blob: "blobA",
        },
        ExpectedFile {
            path: "b.go",
            blob: "blobB",
        },
    ];
    let input = input();
    let schema = index(&[("GET", "/x"), ("GET", "/y")]);
    let cases = [
        SourceBlob {
            tree: TREE,
            path: "a.go",
            blob: "WRONG",
            bytes: H,
        },
        SourceBlob {
            tree: "other-tree",
            path: "a.go",
            blob: "blobA",
            bytes: H,
        },
    ];
    for (a, reason) in cases.into_iter().zip(["blob_mismatch", "tree_mismatch"]) {
        let b = SourceBlob {
            tree: TREE,
            path: "b.go",
            blob: "blobB",
            bytes: K,
        };
        let doc = bind_producer(
            &ProducerInputs {
                input: &input,
                expected: &expected,
                files: &[a, b],
            },
            &schema,
            &ProducerOptions::default(),
        )
        .unwrap();
        assert_eq!(status(&doc, "a.go"), (S::Unknown, Some(reason.into())));
        assert_eq!(status(&doc, "b.go"), (S::Complete, None));
        assert_eq!(
            edges(&doc),
            [("op:GET /y".into(), Some(EdgeGrade::Bound), "K".into())]
        );
        assert!(
            records(&doc).iter().all(|r| r.site.file == "b.go"),
            "no record from a refused file"
        );
    }
}

#[test]
fn d01_missing_unexpected_duplicate_and_unreadable_files_have_their_own_status() {
    let bad_utf8: &[u8] = b"package p\n// \xff\xfe\n";
    let expected = [
        ExpectedFile {
            path: "ok.go",
            blob: "b1",
        },
        ExpectedFile {
            path: "gone.go",
            blob: "b2",
        },
        ExpectedFile {
            path: "bin.go",
            blob: "b3",
        },
        ExpectedFile {
            path: "dup.go",
            blob: "b4",
        },
    ];
    let files = [
        SourceBlob {
            tree: TREE,
            path: "ok.go",
            blob: "b1",
            bytes: H,
        },
        SourceBlob {
            tree: TREE,
            path: "bin.go",
            blob: "b3",
            bytes: bad_utf8,
        },
        SourceBlob {
            tree: TREE,
            path: "dup.go",
            blob: "b4",
            bytes: K,
        },
        SourceBlob {
            tree: TREE,
            path: "dup.go",
            blob: "b4",
            bytes: K,
        },
        SourceBlob {
            tree: TREE,
            path: "extra.go",
            blob: "b5",
            bytes: K,
        },
    ];
    let input = input();
    let doc = bind_producer(
        &ProducerInputs {
            input: &input,
            expected: &expected,
            files: &files,
        },
        &index(&[("GET", "/x"), ("GET", "/y")]),
        &ProducerOptions::default(),
    )
    .unwrap();
    assert_eq!(status(&doc, "ok.go"), (S::Complete, None));
    assert_eq!(
        status(&doc, "gone.go"),
        (S::Unknown, Some("file_not_supplied".into()))
    );
    assert_eq!(
        status(&doc, "bin.go"),
        (S::Unreadable, Some("invalid_utf8".into()))
    );
    assert_eq!(
        status(&doc, "dup.go"),
        (S::Unknown, Some("duplicate_file_input".into()))
    );
    assert_eq!(
        status(&doc, "extra.go"),
        (S::Skipped, Some("not_in_expected_inventory".into()))
    );
    assert_eq!(doc.expected_files, ["bin.go", "dup.go", "gone.go", "ok.go"]);
    assert_eq!(
        edges(&doc),
        [("op:GET /x".into(), Some(EdgeGrade::Bound), "H".into())]
    );
    assert_wire_round_trip(&doc);
}

#[test]
fn d13_a_default_or_foreign_schema_index_is_not_checked_never_zero_bindings() {
    // The reader must not turn an unadmitted inventory into a claim of absence.
    let foreign = index_at(&swagger(None, &[("GET", "/x")]), "T2", "C2");
    for schema in [SchemaIndex::default(), foreign] {
        let doc = run(&[("i.go", H)], &schema);
        assert!(edges(&doc).is_empty());
        let record = record(&doc, "/x");
        assert_eq!(
            (record.disposition, record.reason.as_deref()),
            (D::NotChecked, Some("schema_not_admitted"))
        );
        assert_eq!(
            status(&doc, "i.go"),
            (S::Partial, Some("schema_not_admitted".into()))
        );
        assert!(file_region(&doc, "i.go").possible_keys.is_empty());
    }
    let doc = run(&[("i.go", H)], &index(&[("GET", "/x")]));
    assert_eq!(
        status(&doc, "i.go"),
        (S::Complete, None),
        "control: the admitted index binds"
    );
}

#[test]
fn a15_same_named_handlers_stay_distinct_and_a_skipped_competitor_blocks_the_negative() {
    let q: &[u8] = b"package q\n\n// @Router /x [get]\nfunc H() {}\n";
    let schema = index(&[("GET", "/x")]);
    let doc = run(&[("p.go", H), ("q.go", q)], &schema);
    assert_eq!(edges(&doc).len(), 2);
    assert!(rows(&doc)
        .iter()
        .all(|r| r.grade == Some(EdgeGrade::Candidate)));
    let files: Vec<_> = rows(&doc)
        .iter()
        .map(|r| r.handler.as_ref().unwrap().site.file.clone())
        .collect();
    assert_eq!(files, ["p.go", "q.go"]);
    let doc = run(&[("p.go", H), ("q.go", b"package q\n\xff")], &schema);
    assert_eq!(
        edges(&doc),
        [("op:GET /x".into(), Some(EdgeGrade::Bound), "H".into())]
    );
    assert_eq!(
        status(&doc, "q.go").0,
        S::Unreadable,
        "the competitor is unaccounted, uniqueness is not claimed"
    );
}

#[test]
fn parse_errors_make_the_file_partial_and_leave_siblings_complete() {
    let broken: &[u8] = b"package p\nfunc (\n";
    let doc = run(&[("a.go", broken), ("b.go", K)], &index(&[("GET", "/y")]));
    assert_eq!(
        status(&doc, "a.go"),
        (S::Partial, Some("parse_error".into()))
    );
    assert_eq!(status(&doc, "b.go"), (S::Complete, None));
    assert_eq!(edges(&doc).len(), 1);
}

#[test]
fn output_is_independent_of_input_order_and_survives_the_wire() {
    let schema = index(&[("GET", "/x"), ("GET", "/y")]);
    let one = run(&[("a.go", H), ("b.go", K)], &schema);
    let two = run(&[("b.go", K), ("a.go", H)], &schema);
    assert_eq!(
        serde_json::to_value(&one).unwrap(),
        serde_json::to_value(&two).unwrap()
    );
    assert_wire_round_trip(&one);
}

#[test]
fn adapter_selection_refuses_what_is_not_implemented() {
    assert_eq!(
        ProducerOptions::default().adapters,
        [ProducerAdapter::SwaggoAnnotation]
    );
    let input = input();
    let schema = index(&[("GET", "/x")]);
    let call = |adapters: Vec<ProducerAdapter>| {
        bind_producer(
            &ProducerInputs {
                input: &input,
                expected: &[],
                files: &[],
            },
            &schema,
            &ProducerOptions { adapters },
        )
        .err()
    };
    assert_eq!(call(vec![]), Some(ProducerError::NoAdapters));
    assert_eq!(
        call(vec![ProducerAdapter::GoStructField]),
        Some(ProducerError::NotImplemented(
            ProducerAdapter::GoStructField
        ))
    );
    assert_eq!(
        call(vec![
            ProducerAdapter::SwaggoAnnotation,
            ProducerAdapter::GoStructField
        ]),
        Some(ProducerError::NotImplemented(
            ProducerAdapter::GoStructField
        ))
    );
    assert_eq!(
        call(vec![ProducerAdapter::SwaggoAnnotation]),
        None,
        "control: the annotation adapter runs"
    );
}

#[test]
fn the_input_must_be_a_producer_annotation_api_commit_or_tree() {
    let schema = index(&[("GET", "/x")]);
    let run_with = |patch: serde_json::Value| {
        let input = input_with(patch);
        bind_producer(
            &ProducerInputs {
                input: &input,
                expected: &[],
                files: &[],
            },
            &schema,
            &ProducerOptions::default(),
        )
        .err()
    };
    assert_eq!(
        run_with(json!({"role": "consumer"})),
        Some(ProducerError::WrongRole)
    );
    assert_eq!(
        run_with(json!({"operation_authority": "schema"})),
        Some(ProducerError::WrongAuthority)
    );
    assert_eq!(
        run_with(json!({"scope": {"kind": "path_prefix", "segments": []}})),
        Some(ProducerError::UnsupportedScope)
    );
    assert_eq!(
        run_with(json!({"snapshot_kind": "tag"})),
        Some(ProducerError::UnsupportedSnapshot)
    );
    assert_eq!(run_with(json!({})), None);
}

#[test]
fn an_empty_expected_inventory_claims_nothing() {
    let doc = run(&[], &index(&[("GET", "/x")]));
    assert!(doc.expected_files.is_empty() && doc.file_status.is_empty());
    assert!(edges(&doc).is_empty());
}

#[test]
fn the_extended_wire_contract_is_closed() {
    // The producer fields are optional, but the values they take are closed.
    let doc = run(&[("i.go", H)], &index(&[("GET", "/x")]));
    let wire = serde_json::to_value(&doc).unwrap();
    let decode = |wire: &serde_json::Value| {
        prism_boundary::BindingsDocument::from_json(&serde_json::to_vec(wire).unwrap()).is_ok()
    };
    assert!(decode(&wire), "control: the produced document decodes");
    let mutations: [(&str, serde_json::Value); 5] = [
        ("/operations/0/grade", json!("maybe")),
        ("/operations/0/evidence", json!(["a", "a"])),
        ("/operations/0/handler/site/line", json!(0)),
        ("/annotations/0/disposition", json!("guessed")),
        ("/annotations/0/surprise", json!(true)),
    ];
    for (pointer, value) in mutations {
        let mut bad = wire.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        bad.pointer_mut(parent).unwrap()[key] = value;
        assert!(!decode(&bad), "{pointer} must be refused");
    }
    let mut old = wire.clone();
    old["operations"][0]
        .as_object_mut()
        .unwrap()
        .retain(|k, _| !["grade", "handler", "evidence"].contains(&k.as_str()));
    old.as_object_mut().unwrap().remove("annotations");
    assert!(
        decode(&old),
        "a P1d document without the P2 fields still decodes"
    );
}
