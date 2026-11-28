//! P2-1 fold 3 rows (PR #1006 round 3): one shared check owns every declared identity field of a
//! schema source (rev, committed_at, subject, sha256, content_hash, info_version), shared with
//! the P1d input validator; plus the row-07 consequence pinned as a known limit. Each negative
//! sits beside a control that still binds. All rows are CONSTRUCTED.

#[allow(dead_code)]
#[path = "support/producer.rs"]
mod support;

use prism_boundary::model::{EdgeGrade, ExtractionStatus as S};
use prism_boundary::revision::{AdmittedSchemaInput, Blob, Format, Snapshot};
use prism_boundary::{index_schema, SchemaIndex};
use serde_json::{json, Value};
use support::*;

const H: &[u8] = b"package p\n\n// @Router /x [get]\nfunc H() {}\n";
const WHEN: &str = "2026-03-01T10:00:00Z";
const SUBJECT: &str = "fix the thing";

/// Every receipt field spelled out; `None` values are JSON null.
struct Receipt {
    rev: Option<&'static str>,
    committed_at: Option<&'static str>,
    subject: Option<&'static str>,
    content_hash: Option<String>,
    info_version: Option<&'static str>,
}

fn receipt(schema: &SchemaIndex, r: Receipt) -> Value {
    json!({"schema": {"source": {"kind": "file", "repo": "pilot-producer", "tree": TREE,
        "rev": r.rev, "committed_at": r.committed_at, "subject": r.subject,
        "path": "swagger.json", "blob": "schema-blob", "format": "json",
        "sha256": schema.byte_sha256}, "content_hash": r.content_hash,
        "info_version": r.info_version}})
}

/// A receipt that equals the commit snapshot and the index in every field.
fn exact(schema: &SchemaIndex) -> Receipt {
    Receipt {
        rev: Some(REV),
        committed_at: Some(WHEN),
        subject: Some(SUBJECT),
        content_hash: Some(schema.content_hash.clone()),
        info_version: Some("1"),
    }
}

fn commit_patch(receipt: Value) -> Value {
    let mut patch = receipt;
    patch["subject"] = json!(SUBJECT);
    patch["committed_at"] = json!(WHEN);
    patch
}

fn binds(doc: &prism_boundary::model::BindingsDocument, id: &str) -> bool {
    edges(doc) == [(id.to_string(), Some(EdgeGrade::Bound), "H".to_string())]
        && file_region(doc, "i.go").status == S::Complete
}

fn refused(doc: &prism_boundary::model::BindingsDocument) -> bool {
    let region = file_region(doc, "i.go");
    edges(doc).is_empty()
        && (region.status, region.reason.as_deref()) == (S::Partial, Some("schema_not_admitted"))
}

fn run_commit(schema: &SchemaIndex, patch: Value) -> prism_boundary::model::BindingsDocument {
    run_with_input(&[("i.go", H)], schema, &input_with(commit_patch(patch)))
}

#[test]
fn f3_01_a_contradicting_committed_at_refuses_admission() {
    let schema = index(&[("GET", "/x")]);
    let mut wrong = exact(&schema);
    wrong.committed_at = Some("2026-03-02T10:00:00Z");
    assert!(
        refused(&run_commit(&schema, receipt(&schema, wrong))),
        "a receipt that contradicts the snapshot's commit date must not be admitted"
    );
    // Controls: equal, and the same instant spelled with another offset.
    for at in [WHEN, "2026-03-01T11:00:00+01:00"] {
        let mut same = exact(&schema);
        same.committed_at = Some(at);
        assert!(
            binds(&run_commit(&schema, receipt(&schema, same)), "op:GET /x"),
            "{at}"
        );
    }
}

#[test]
fn f3_02_a_contradicting_subject_refuses_admission() {
    let schema = index(&[("GET", "/x")]);
    let mut wrong = exact(&schema);
    wrong.subject = Some("another commit");
    assert!(
        refused(&run_commit(&schema, receipt(&schema, wrong))),
        "a receipt that contradicts the snapshot's subject must not be admitted"
    );
    assert!(binds(
        &run_commit(&schema, receipt(&schema, exact(&schema))),
        "op:GET /x"
    ));
}

#[test]
fn f3_03_a_commit_receipt_must_declare_the_snapshot_revision() {
    // The impact validator already requires the declared revision to equal the snapshot's; a
    // missing one is a mismatch there, so the binder must not admit it either.
    let schema = index(&[("GET", "/x")]);
    let mut none = exact(&schema);
    none.rev = None;
    assert!(
        refused(&run_commit(&schema, receipt(&schema, none))),
        "a commit snapshot with a receipt that declares no revision"
    );
    assert!(binds(
        &run_commit(&schema, receipt(&schema, exact(&schema))),
        "op:GET /x"
    ));
}

fn computed_index() -> SchemaIndex {
    let bytes = swagger(None, &[("GET", "/x")]);
    index_schema(&AdmittedSchemaInput {
        api: API,
        authority: "schema",
        snapshot: Snapshot::ComputedTree {
            tree: TREE,
            computed_at: "2026-03-01T10:00:00Z",
            merge_receipt: "mr1",
        },
        entry: Blob {
            path: "swagger.json",
            tree: TREE,
            oid: "schema-blob",
            bytes: &bytes,
            format: Format::Json,
        },
        retrieval_uri: None,
        references: &[],
        config_receipts: &[],
        media_receipts: &[],
    })
    .expect("computed-tree schema fixture")
}

fn computed_patch(receipt: Value) -> Value {
    let mut patch = receipt;
    patch["snapshot_kind"] = json!("computed_tree");
    patch["rev"] = json!(null);
    patch["subject"] = json!(null);
    patch["committed_at"] = json!(null);
    patch["computed_at"] = json!("2026-03-01T10:00:00Z");
    patch["merge_receipt"] = json!("mr1");
    patch
}

#[test]
fn f3_04_a_computed_tree_receipt_must_not_carry_commit_metadata() {
    let schema = computed_index();
    let clean = Receipt {
        rev: None,
        committed_at: None,
        subject: None,
        content_hash: Some(schema.content_hash.clone()),
        info_version: Some("1"),
    };
    let run_it = |r: Receipt| {
        run_with_input(
            &[("i.go", H)],
            &schema,
            &input_with(computed_patch(receipt(&schema, r))),
        )
    };
    assert!(
        binds(&run_it(clean), "op:GET /x"),
        "control: a computed-tree receipt with null commit metadata"
    );
    for (label, edit) in [
        (
            "rev",
            (|r: &mut Receipt| r.rev = Some(REV)) as fn(&mut Receipt),
        ),
        ("committed_at", |r| r.committed_at = Some(WHEN)),
        ("subject", |r| r.subject = Some(SUBJECT)),
    ] {
        let mut r = Receipt {
            rev: None,
            committed_at: None,
            subject: None,
            content_hash: Some(schema.content_hash.clone()),
            info_version: Some("1"),
        };
        edit(&mut r);
        assert!(refused(&run_it(r)), "nested {label} on a computed tree");
    }
}

fn versionless() -> Vec<u8> {
    br#"{"swagger":"2.0","info":{"title":"t"},"paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#
        .to_vec()
}

fn empty_version() -> Vec<u8> {
    br#"{"swagger":"2.0","info":{"title":"t","version":""},"paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#
        .to_vec()
}

#[test]
fn f3_05_an_empty_info_version_is_compared_like_any_other_value() {
    // Against index "1".
    let schema = index(&[("GET", "/x")]);
    assert_eq!(schema.info_version.as_deref(), Some("1"), "fixture premise");
    let mut r = exact(&schema);
    r.info_version = Some("");
    assert!(
        refused(&run_commit(&schema, receipt(&schema, r))),
        "vs \"1\""
    );
    // Against an index that has no info version.
    let none = index_at(&versionless(), TREE, REV);
    assert_eq!(none.info_version, None, "fixture premise");
    let mut r = exact(&none);
    r.info_version = Some("");
    assert!(refused(&run_commit(&none, receipt(&none, r))), "vs none");
    // Controls: an empty declared version equals an index that carries the empty string; an
    // undeclared (null) version is not compared.
    let empty = index_at(&empty_version(), TREE, REV);
    assert_eq!(empty.info_version.as_deref(), Some(""), "fixture premise");
    let mut r = exact(&empty);
    r.info_version = Some("");
    assert!(binds(&run_commit(&empty, receipt(&empty, r)), "op:GET /x"));
    let mut r = exact(&schema);
    r.info_version = None;
    assert!(binds(
        &run_commit(&schema, receipt(&schema, r)),
        "op:GET /x"
    ));
}

#[test]
fn f3_06_an_empty_content_hash_is_compared_like_any_other_value() {
    let schema = index(&[("GET", "/x")]);
    let mut r = exact(&schema);
    r.content_hash = Some(String::new());
    assert!(refused(&run_commit(&schema, receipt(&schema, r))));
    // Control: the real hash binds; an undeclared (null) hash is not compared.
    assert!(binds(
        &run_commit(&schema, receipt(&schema, exact(&schema))),
        "op:GET /x"
    ));
    let mut r = exact(&schema);
    r.content_hash = None;
    assert!(binds(
        &run_commit(&schema, receipt(&schema, r)),
        "op:GET /x"
    ));
}

// ---- Known limit (row 07 consequence): the spurious key withholds a real edge ---------------

#[test]
fn kl_p2_1_09_a_spurious_server_override_key_withholds_an_independently_annotated_edge() {
    // `/z` overrides its servers with `/fixed`; `/x` and `/fixed/x` are root operations. The
    // detached `@Router /x` is keyed under every projection of `/x`, `/fixed/x` included, which
    // `/x` never has; the unrelated, canonically attached `GET /fixed/x` therefore loses
    // uniqueness. CONSTRUCTED, 0 incidence on the public pair (single server, root base path).
    let item = |servers: &str| {
        format!(r#"{{"get":{{{servers}"responses":{{"200":{{"description":"ok"}}}}}}}}"#)
    };
    let schema = |z_servers: &str| {
        format!(
            r#"{{"openapi":"3.0.3","info":{{"title":"t","version":"1"}},"paths":{{
                "/x":{x},"/fixed/x":{x},"/z":{z}}}}}"#,
            x = item(""),
            z = item(z_servers)
        )
        .into_bytes()
    };
    let attached: &[u8] = b"package p\n\n// @Router /fixed/x [get]\nfunc F() {}\n";
    let detached: &[u8] = b"package q\n\n// @Router /x [get]\n\nfunc K() {}\n";
    let overridden = schema(r#""servers":[{"url":"/fixed"}],"#);
    let doc = run(
        &[("p.go", attached), ("q.go", detached)],
        &index_at(&overridden, TREE, REV),
    );
    let got: Vec<_> = edges(&doc)
        .into_iter()
        .map(|(id, grade, _)| (id, grade))
        .collect();
    assert_eq!(
        got,
        [("op:GET /fixed/x".to_string(), Some(EdgeGrade::Candidate))],
        "today: the spurious /fixed/x key downgrades the unrelated edge"
    );
    // Control: without the override on /z the detached key is only /x and the edge is Bound.
    let plain = schema("");
    let doc = run(
        &[("p.go", attached), ("q.go", detached)],
        &index_at(&plain, TREE, REV),
    );
    let got: Vec<_> = edges(&doc)
        .into_iter()
        .map(|(id, grade, _)| (id, grade))
        .collect();
    assert_eq!(
        got,
        [("op:GET /fixed/x".to_string(), Some(EdgeGrade::Bound))]
    );
}
