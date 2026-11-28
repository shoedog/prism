//! P2-1 fold 2 rows (PR #1006 round 2): receipt identity beyond the blob (digest, revision,
//! content hash, info version), the once-per-document deployments gap, the grammar pin for
//! `@Router /x[get]`, and the known-limit rows kl_p2_1_01..08. Each known-limit row asserts
//! today's behaviour beside a control (the exact shape that works). All of them are CONSTRUCTED:
//! OBSERVED on the public pair = no (0 entry gaps, root base path, single server), zero incidence.

#[allow(dead_code)]
#[path = "support/producer.rs"]
mod support;

use prism_boundary::model::{
    AnnotationDisposition as D, BindingsDocument, EdgeGrade, ExtractionStatus as S,
};
use serde_json::json;
use support::*;

const H: &[u8] = b"package p\n\n// @Router /x [get]\nfunc H() {}\n";

fn grades(doc: &BindingsDocument) -> Vec<(String, Option<EdgeGrade>)> {
    let mut out: Vec<_> = rows(doc)
        .iter()
        .map(|r| (r.operation_id.clone(), r.grade))
        .collect();
    out.sort();
    out
}

fn bound(id: &str) -> (String, Option<EdgeGrade>) {
    (id.to_string(), Some(EdgeGrade::Bound))
}

fn candidate(id: &str) -> (String, Option<EdgeGrade>) {
    (id.to_string(), Some(EdgeGrade::Candidate))
}

fn withheld(doc: &BindingsDocument) -> bool {
    rows(doc)
        .iter()
        .all(|r| r.evidence.iter().any(|e| e == "competitor_unread"))
}

// ---- Item 1: the declared receipt must agree with the index --------------------------------

fn receipt(
    sha256: &str,
    rev: serde_json::Value,
    hash: serde_json::Value,
    version: &str,
) -> serde_json::Value {
    let info_version: serde_json::Value = if version.is_empty() {
        json!(null)
    } else {
        json!(version)
    };
    json!({"schema": {"source": {"kind": "file", "repo": "pilot-producer", "tree": TREE,
        "rev": rev, "committed_at": null, "path": "swagger.json", "blob": "schema-blob",
        "format": "json", "sha256": sha256}, "content_hash": hash, "info_version": info_version}})
}

#[test]
fn g1_each_declared_receipt_field_that_contradicts_the_index_refuses_admission() {
    let schema = index(&[("GET", "/x")]);
    let (sha, hash) = (schema.byte_sha256.clone(), schema.content_hash.clone());
    assert_eq!(schema.info_version.as_deref(), Some("1"), "fixture premise");
    let wrong = "1".repeat(64);
    let cases = [
        ("sha256", receipt(&wrong, json!(REV), json!(hash), "1")),
        ("rev", receipt(&sha, json!("C2"), json!(hash), "1")),
        ("content_hash", receipt(&sha, json!(REV), json!(wrong), "1")),
        ("info_version", receipt(&sha, json!(REV), json!(hash), "9")),
    ];
    let admitted: Vec<_> = cases
        .iter()
        .filter(|(_, patch)| {
            let doc = run_with_input(&[("i.go", H)], &schema, &input_with(patch.clone()));
            let region = file_region(&doc, "i.go");
            !(edges(&doc).is_empty()
                && (region.status, region.reason.as_deref())
                    == (S::Partial, Some("schema_not_admitted")))
        })
        .map(|(label, _)| *label)
        .collect();
    assert!(admitted.is_empty(), "wrongly admitted: {admitted:?}");
}

#[test]
fn g1_controls_an_exact_receipt_and_undeclared_optional_fields_still_bind() {
    let schema = index(&[("GET", "/x")]);
    let (sha, hash) = (schema.byte_sha256.clone(), schema.content_hash.clone());
    let controls = [
        (
            "every field declared and equal",
            receipt(&sha, json!(REV), json!(hash), "1"),
        ),
        (
            "optional fields undeclared",
            receipt(&sha, json!(null), json!(null), ""),
        ),
    ];
    for (label, patch) in controls {
        let doc = run_with_input(&[("i.go", H)], &schema, &input_with(patch));
        assert_eq!(grades(&doc), [bound("op:GET /x")], "{label}");
        assert_eq!(file_region(&doc, "i.go").status, S::Complete, "{label}");
    }
    // The named-tree fallback has no declared digest and keeps its own contract.
    let doc = run_with_input(
        &[("i.go", H)],
        &schema,
        &input_with(json!({"schema": null})),
    );
    assert_eq!(grades(&doc), [bound("op:GET /x")], "named-tree fallback");
}

// ---- Opus N2: the unprojected-deployments gap is one entry per document ----------------------

const DEFAULT_ONLY: &[u8] = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
    "servers":[{"url":"/{region}","variables":{"region":{"default":"north"}}}],
    "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}},
             "/y":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;

fn deployment_entries(doc: &BindingsDocument) -> usize {
    doc.entries
        .iter()
        .filter(|e| e.reason.as_deref() == Some("annotation_deployments_unprojected"))
        .count()
}

#[test]
fn n2_the_unprojected_deployments_gap_is_emitted_once_per_document() {
    let y: &[u8] = b"package q\n\n// @Router /y [get]\nfunc K() {}\n";
    let schema = index_at(DEFAULT_ONLY, TREE, REV);
    let doc = run(&[("p.go", H), ("q.go", y)], &schema);
    assert_eq!(rows(&doc).len(), 2, "two hits");
    assert_eq!(deployment_entries(&doc), 1, "{:?}", doc.entries);
    // Control: one hit, one entry.
    let doc = run(&[("p.go", H)], &schema);
    assert_eq!(deployment_entries(&doc), 1);
}

// ---- Grammar pin (sol 6): `@Router /x[get]` has no separator and is malformed ---------------

#[test]
fn a_router_directive_without_a_blank_before_the_bracket_is_malformed() {
    let src: &[u8] = b"package p\n\n// @Router /x[get]\nfunc H() {}\n";
    let doc = run(&[("i.go", src)], &index(&[("GET", "/x")]));
    let record = record(&doc, "/x[get]");
    assert_eq!(record.disposition, D::Malformed);
    assert_eq!(record.reason.as_deref(), Some("malformed_directive"));
    assert!(edges(&doc).is_empty(), "nothing binds");
    assert_eq!(
        entries(&doc, "i.go"),
        [("annotation_malformed".to_string(), Vec::<String>::new())]
    );
    // Control: the separated form binds.
    let doc = run(&[("i.go", H)], &index(&[("GET", "/x")]));
    assert_eq!(grades(&doc), [bound("op:GET /x")]);
}

// ---- KL1: a keyless gap from a read file withholds uniqueness for every operation ------------

fn kl1(sibling: &[u8], control: &[u8], schema: &prism_boundary::SchemaIndex, id: &str) {
    let doc = run(&[("i.go", sibling)], schema);
    assert_eq!(
        grades(&doc),
        [candidate(id)],
        "today: the visible directive concerns another operation, yet {id} loses uniqueness"
    );
    assert!(withheld(&doc));
    let doc = run(&[("i.go", control)], schema);
    assert_eq!(
        grades(&doc),
        [bound(id)],
        "control: the valid edge alone is unique"
    );
    assert!(rows(&doc).iter().all(|r| r.evidence.is_empty()));
}

#[test]
fn kl_p2_1_01_block_comments_naming_other_operations_withhold_a_valid_edge() {
    let schema = index(&[("GET", "/x"), ("GET", "/y"), ("GET", "/z")]);
    kl1(
        &go("a02"),
        b"package p\n\n// @Router /z [get]\nfunc Line() {}\n",
        &schema,
        "op:GET /z",
    );
}

#[test]
fn kl_p2_1_02_a_split_directive_withholds_an_unrelated_sibling() {
    let schema = index(&[("GET", "/x"), ("GET", "/y")]);
    kl1(
        &go("a03"),
        b"package p\n\n// @Router /y [get]\nfunc Sibling() {}\n",
        &schema,
        "op:GET /y",
    );
}

#[test]
fn kl_p2_1_03_spelling_variants_naming_other_paths_withhold_valid_edges() {
    let schema = index(&[("GET", "/d")]);
    kl1(
        &go("a05"),
        b"package p\n\n\t// @Router /d [get]\nfunc Indented() {}\n",
        &schema,
        "op:GET /d",
    );
}

#[test]
fn kl_p2_1_04_invalid_path_spellings_withhold_the_literal_encoded_operation() {
    let schema = index(&[("GET", "/x/%7Bid%7D")]);
    kl1(
        &go("a09"),
        b"package p\n\n// @Router /x/%7Bid%7D [get]\nfunc Encoded() {}\n",
        &schema,
        "op:GET /x/%7Bid%7D",
    );
}

// ---- KL2: gap keys with a directive that spells the base, and operation-level servers --------

#[test]
fn kl_p2_1_05_a_detached_directive_that_spells_the_base_is_keyed_under_the_base_twice() {
    let detached: &[u8] = b"package q\n\n// @Router /api/x [get]\n\nfunc K() {}\n";
    let schema = index_at(&swagger(Some("/api"), &[("GET", "/x")]), TREE, REV);
    let doc = run(&[("p.go", H), ("q.go", detached)], &schema);
    assert_eq!(
        entries(&doc, "q.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /api/api/x".to_string()]
        )],
        "today: keyed as if the base were not already spelled"
    );
    assert_eq!(
        grades(&doc),
        [bound("op:GET /api/x")],
        "today: the edge it may compete with keeps its uniqueness"
    );
    // Control: the detached form that spells the authored path names the projected operation.
    let authored: &[u8] = b"package q\n\n// @Router /x [get]\n\nfunc K() {}\n";
    let doc = run(&[("p.go", H), ("q.go", authored)], &schema);
    assert_eq!(grades(&doc), [candidate("op:GET /api/x")]);
}

#[test]
fn kl_p2_1_06_an_attached_directive_spelling_one_server_prefix_blocks_only_that_prefix() {
    let k: &[u8] = b"package q\n\n// @Router /north/x [get]\nfunc K() {}\n";
    let two = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "servers":[{"url":"/north"},{"url":"/south"}],
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(&[("p.go", H), ("q.go", k)], &index_at(two, TREE, REV));
    assert_eq!(
        grades(&doc),
        [candidate("op:GET /north/x"), bound("op:GET /south/x")],
        "today: only /north/x is blocked"
    );
    // Control: one prefix, and the same directive blocks the one projection.
    let one = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "servers":[{"url":"/north"}],
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(&[("p.go", H), ("q.go", k)], &index_at(one, TREE, REV));
    assert_eq!(grades(&doc), [candidate("op:GET /north/x")]);
}

#[test]
fn kl_p2_1_07_an_operation_level_server_override_adds_a_spurious_key() {
    let overridden = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "paths":{"/x":{"get":{"servers":[{"url":"/fixed"}],"responses":{"200":{"description":"ok"}}}},
                 "/y":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let plain = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}},
                 "/y":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let detached: &[u8] = b"package q\n\n// @Router /y [get]\n\nfunc K() {}\n";
    let doc = run(&[("q.go", detached)], &index_at(overridden, TREE, REV));
    assert_eq!(
        entries(&doc, "q.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /fixed/y".to_string(), "op:GET /y".to_string()]
        )],
        "today: /y never sits under /fixed, yet that id is a key"
    );
    // Control: without the override the key is the one operation.
    let doc = run(&[("q.go", detached)], &index_at(plain, TREE, REV));
    assert_eq!(
        entries(&doc, "q.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /y".to_string()]
        )]
    );
}

// ---- KL3: the deployments region is document-wide ---------------------------------------------

#[test]
fn kl_p2_1_08_the_deployments_gap_is_taken_from_any_server_diagnostic() {
    // /x overrides the servers with one finite url; /y inherits the default-only variable.
    let schema = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "servers":[{"url":"/{region}","variables":{"region":{"default":"north"}}}],
        "paths":{"/x":{"get":{"servers":[{"url":"/fixed"}],"responses":{"200":{"description":"ok"}}}},
                 "/y":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(&[("p.go", H)], &index_at(schema, TREE, REV));
    assert_eq!(grades(&doc), [bound("op:GET /fixed/x")]);
    assert_eq!(
        deployment_entries(&doc),
        1,
        "today: /x has complete deployment coverage, yet the region is recorded"
    );
    // Control: no server diagnostic anywhere, no region.
    let doc = run(&[("p.go", H)], &index(&[("GET", "/x"), ("GET", "/y")]));
    assert_eq!(deployment_entries(&doc), 0);
}
