//! P2-1 fold 1 rows (PR #1006 round 1): admission identity, coverage-qualified uniqueness,
//! projected gap keys, the published wire contract and duplicate inventory entries.
//! Each admission conjunct has its own negative and a matching control.

#[allow(dead_code)]
#[path = "support/producer.rs"]
mod support;

use prism_boundary::model::{
    AnnotationDisposition as D, BindingsDocument, DocumentError, EdgeGrade, ExtractionStatus as S,
};
use serde_json::json;
use support::*;

const H: &[u8] = b"package p\n\n// @Router /x [get]\nfunc H() {}\n";
const Y: &[u8] = b"package q\n\n// @Router /y [get]\nfunc K() {}\n";

fn grades(doc: &BindingsDocument) -> Vec<(String, Option<EdgeGrade>)> {
    let mut out: Vec<_> = rows(doc)
        .iter()
        .map(|r| (r.operation_id.clone(), r.grade))
        .collect();
    out.sort();
    out
}

fn not_admitted(doc: &BindingsDocument) -> bool {
    let record = record(doc, "/x");
    let region = file_region(doc, "i.go");
    edges(doc).is_empty()
        && (record.disposition, record.reason.as_deref())
            == (D::NotChecked, Some("schema_not_admitted"))
        && (region.status, region.reason.as_deref()) == (S::Partial, Some("schema_not_admitted"))
}

fn assert_admitted(doc: &BindingsDocument, label: &str) {
    assert_eq!(
        grades(doc),
        [("op:GET /x".to_string(), Some(EdgeGrade::Bound))],
        "{label}: control binds"
    );
    assert_eq!(file_region(doc, "i.go").status, S::Complete, "{label}");
}

// ---- Fix 1: admission is the crate's own schema-source predicate -------------------------

#[test]
fn f1_each_admission_conjunct_has_its_own_negative_and_a_control() {
    let bytes = swagger(None, &[("GET", "/x")]);
    let ix = |api, authority, tree, commit, path, blob| {
        index_from(&bytes, (api, authority, tree, commit), (path, blob))
    };
    let exact = || ix(API, "schema", TREE, REV, "swagger.json", "schema-blob");
    let own = || input();
    let cases: Vec<(&str, _, _)> = vec![
        (
            "api",
            ix(
                "other-api",
                "schema",
                TREE,
                REV,
                "swagger.json",
                "schema-blob",
            ),
            own(),
        ),
        (
            "tree",
            ix(API, "schema", "T2", REV, "swagger.json", "schema-blob"),
            own(),
        ),
        (
            "commit",
            ix(API, "schema", TREE, "C2", "swagger.json", "schema-blob"),
            own(),
        ),
        (
            "path",
            ix(
                API,
                "schema",
                TREE,
                REV,
                "other/swagger.json",
                "schema-blob",
            ),
            own(),
        ),
        (
            "blob",
            ix(API, "schema", TREE, REV, "swagger.json", "other-blob"),
            own(),
        ),
        (
            "authority",
            ix(API, "route_table", TREE, REV, "swagger.json", "schema-blob"),
            own(),
        ),
        (
            "declared repo differs from the input repo",
            exact(),
            input_with(
                json!({"schema": {"source": {"kind": "file", "repo": "other-repo",
                "tree": TREE, "rev": REV, "committed_at": null, "path": "swagger.json",
                "blob": "schema-blob", "format": "json", "sha256": "0".repeat(64)},
                "content_hash": null, "info_version": null}}),
            ),
        ),
        (
            "declared schema tree",
            exact(),
            input_with(
                json!({"schema": {"source": {"kind": "file", "repo": "pilot-producer",
                "tree": "T9", "rev": REV, "committed_at": null, "path": "swagger.json",
                "blob": "schema-blob", "format": "json", "sha256": "0".repeat(64)},
                "content_hash": null, "info_version": null}}),
            ),
        ),
        (
            "declared schema path",
            exact(),
            input_with(
                json!({"schema": {"source": {"kind": "file", "repo": "pilot-producer",
                "tree": TREE, "rev": REV, "committed_at": null, "path": "docs/swagger.json",
                "blob": "schema-blob", "format": "json", "sha256": "0".repeat(64)},
                "content_hash": null, "info_version": null}}),
            ),
        ),
        (
            "declared schema blob",
            exact(),
            input_with(
                json!({"schema": {"source": {"kind": "file", "repo": "pilot-producer",
                "tree": TREE, "rev": REV, "committed_at": null, "path": "swagger.json",
                "blob": "OTHER-BLOB", "format": "json", "sha256": "0".repeat(64)},
                "content_hash": null, "info_version": null}}),
            ),
        ),
        (
            "declared schema is a route table",
            exact(),
            input_with(json!({"schema": {"source": {"kind": "route_table",
                "repo": "pilot-producer", "tree": TREE, "input_id": "svc"},
                "content_hash": null, "info_version": null}})),
        ),
        (
            "named-tree fallback blob",
            exact(),
            input_with(json!({"schema": null, "origin": {"kind": "named_tree",
                "input_id": "svc", "repo": "pilot-producer", "tree": TREE,
                "path": "swagger.json", "blob": "other-blob", "availability": "available"}})),
        ),
        (
            "named-tree fallback path",
            exact(),
            input_with(json!({"schema": null, "origin": {"kind": "named_tree",
                "input_id": "svc", "repo": "pilot-producer", "tree": TREE,
                "path": "elsewhere.json", "blob": "schema-blob", "availability": "available"}})),
        ),
    ];
    let admitted: Vec<_> = cases
        .iter()
        .filter(|(_, schema, input)| !not_admitted(&run_with_input(&[("i.go", H)], schema, input)))
        .map(|(label, _, _)| *label)
        .collect();
    assert!(admitted.is_empty(), "wrongly admitted: {admitted:?}");
    // Controls: the exactly matching declaration, and the fallback that names the same tree entry.
    assert_admitted(
        &run_with_input(&[("i.go", H)], &exact(), &own()),
        "declared",
    );
    let fallback = input_with(json!({"schema": null}));
    assert_admitted(
        &run_with_input(&[("i.go", H)], &exact(), &fallback),
        "named-tree fallback",
    );
}

// ---- Fix 3: uniqueness needs the competitors to have been read ---------------------------

fn bound_row_withheld(doc: &BindingsDocument) -> bool {
    let rows = rows(doc);
    let record = record(doc, "/x");
    !rows.is_empty()
        && rows.iter().all(|row| {
            row.grade == Some(EdgeGrade::Candidate)
                && row.evidence.iter().any(|e| e == "competitor_unread")
                && row.sites.len() == 1
        })
        && record.disposition == D::Candidate
        && record.operation_ids == ["op:GET /x"]
}

#[test]
fn f3_an_unread_expected_file_withholds_uniqueness() {
    let schema = index(&[("GET", "/x")]);
    let q: &[u8] = b"package q\n\n// @Router /x [get]\nfunc H() {}\n";
    let p = ("p.go", "blob:p.go", TREE, H);
    let expected = [("p.go", "blob:p.go"), ("q.go", "blob:q.go")];
    let cases: Vec<(&str, Vec<Supplied<'_>>)> = vec![
        ("file_not_supplied", vec![p]),
        (
            "duplicate_file_input",
            vec![
                p,
                ("q.go", "blob:q.go", TREE, q),
                ("q.go", "blob:q.go", TREE, q),
            ],
        ),
        ("tree_mismatch", vec![p, ("q.go", "blob:q.go", "T9", q)]),
        ("blob_mismatch", vec![p, ("q.go", "blob:stale", TREE, q)]),
        (
            "invalid_utf8",
            vec![p, ("q.go", "blob:q.go", TREE, b"package q\n\xff")],
        ),
    ];
    let mut kept = Vec::new();
    for (label, supplied) in cases {
        let doc = run_raw(&expected, &supplied, &schema, &input());
        let region = file_region(&doc, "q.go");
        assert!(
            matches!(region.status, S::Unknown | S::Unreadable),
            "{label}: {:?}",
            region.status
        );
        if !bound_row_withheld(&doc) {
            kept.push(label);
        }
    }
    assert!(
        kept.is_empty(),
        "uniqueness kept despite an unread file: {kept:?}"
    );
    // Control: the competitor is read and complete (and binds elsewhere), so H stays unique.
    let doc = run(&[("p.go", H), ("q.go", Y)], &schema_xy());
    assert_eq!(
        grades(&doc),
        [
            ("op:GET /x".to_string(), Some(EdgeGrade::Bound)),
            ("op:GET /y".to_string(), Some(EdgeGrade::Bound))
        ]
    );
    assert!(rows(&doc).iter().all(|r| r.evidence.is_empty()));
}

fn schema_xy() -> prism_boundary::SchemaIndex {
    index(&[("GET", "/x"), ("GET", "/y")])
}

#[test]
fn f3_a_read_parse_error_file_counts_only_through_its_keyed_gaps() {
    // The text of a partial/parse_error file was read: its directives are all recorded.
    let broken: &[u8] = b"package q\nfunc (\n";
    let doc = run(&[("p.go", H), ("q.go", broken)], &index(&[("GET", "/x")]));
    let region = file_region(&doc, "q.go");
    assert_eq!(
        (region.status, region.reason.as_deref()),
        (S::Partial, Some("parse_error"))
    );
    assert_eq!(
        grades(&doc),
        [("op:GET /x".to_string(), Some(EdgeGrade::Bound))]
    );
    assert!(rows(&doc)[0].evidence.is_empty());
}

#[test]
fn f3_a_gap_withholds_uniqueness_only_when_keyless_or_naming_the_operation() {
    let schema = schema_xy();
    // A keyless gap (a malformed directive) could hide a competitor for any operation.
    let malformed: &[u8] = b"package q\n\n// @router /z [get]\nfunc Z() {}\n";
    let doc = run(&[("p.go", H), ("q.go", malformed)], &schema);
    assert_eq!(
        grades(&doc),
        [("op:GET /x".to_string(), Some(EdgeGrade::Candidate))]
    );
    assert!(rows(&doc)[0]
        .evidence
        .contains(&"competitor_unread".to_string()));
    // A keyed gap that names the operation (a detached @Router /x).
    let detached: &[u8] = b"package q\n\n// @Router /x [get]\n\nfunc Z() {}\n";
    let doc = run(&[("p.go", H), ("q.go", detached)], &schema);
    assert_eq!(
        grades(&doc),
        [("op:GET /x".to_string(), Some(EdgeGrade::Candidate))]
    );
    // Control: a keyed gap for another operation leaves /x unique.
    let other: &[u8] = b"package q\n\n// @Router /y [get]\n\nfunc Z() {}\n";
    let doc = run(&[("p.go", H), ("q.go", other)], &schema);
    assert_eq!(
        grades(&doc),
        [("op:GET /x".to_string(), Some(EdgeGrade::Bound))]
    );
    assert!(rows(&doc)[0].evidence.is_empty());
}

// ---- Fix 4: gap keys are projected operation ids ------------------------------------------

const DETACHED: &[u8] = b"package p\n\n// @Router /x [get]\n\nfunc H() {}\n";

#[test]
fn f4_a_detached_directive_is_keyed_by_the_projected_id_under_a_base_path() {
    let schema = index_at(&swagger(Some("/api"), &[("GET", "/x")]), TREE, REV);
    let doc = run(&[("i.go", DETACHED)], &schema);
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /api/x".to_string()]
        )]
    );
    // An unattached directive for an operation the schema lacks is keyed by the same projection.
    let doc = run(
        &[("i.go", b"package p\n\n// @Router /y [get]\n\nfunc H() {}\n")],
        &schema,
    );
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /api/y".to_string()]
        )]
    );
    // Control: a root base projects to the authored key.
    let doc = run(&[("i.go", DETACHED)], &index(&[("GET", "/x")]));
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /x".to_string()]
        )]
    );
}

#[test]
fn f4_every_server_projection_is_keyed_and_an_unknown_domain_is_keyless() {
    let servers = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},"servers":[{"url":"/north"},{"url":"/south"}],
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(&[("i.go", DETACHED)], &index_at(servers, TREE, REV));
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /north/x".to_string(), "op:GET /south/x".to_string()]
        )]
    );
    // No operation to learn the base from: the projected ids cannot be enumerated, so the gap
    // blocks everything (keyless).
    let empty = index_at(&swagger(Some("/api"), &[]), TREE, REV);
    let doc = run(&[("i.go", DETACHED)], &empty);
    assert_eq!(
        entries(&doc, "i.go"),
        [("annotation_unattached".to_string(), Vec::<String>::new())]
    );
}

#[test]
fn f4_typed_path_differs_is_keyed_by_the_projected_id_under_a_base() {
    let src: &[u8] = b"package p\n\n// @Router /x/{id}/{id} [get]\nfunc H() {}\n";
    let schema = index_at(&swagger(Some("/api"), &[("GET", "/x/{a}/{b}")]), TREE, REV);
    let doc = run(&[("i.go", src)], &schema);
    let record = record(&doc, "{id}/{id}");
    assert_eq!(
        (record.disposition, record.reason.as_deref()),
        (D::NotChecked, Some("typed_path_differs"))
    );
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_not_checked".to_string(),
            vec!["op:GET /api/x/{}/{}".to_string()]
        )]
    );
}

#[test]
fn f4_incomplete_operations_and_unadmitted_schemas_are_keyless_and_a_base_echo_keeps_its_key() {
    // An unreadable operation: the projected domain is unknown.
    let broken = br#"{"swagger":"2.0","info":{"title":"t","version":"1"},"basePath":"/api",
        "paths":{"/x":{"get":5},"/w":{"post":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let schema = index_at(broken, TREE, REV);
    let doc = run(
        &[("i.go", b"package p\n\n// @Router /x [get]\nfunc H() {}\n")],
        &schema,
    );
    let record = record(&doc, "/x");
    assert_eq!(
        (record.disposition, record.reason.as_deref()),
        (D::NotChecked, Some("schema_operations_incomplete"))
    );
    assert_eq!(
        entries(&doc, "i.go"),
        [("annotation_not_checked".to_string(), Vec::<String>::new())]
    );
    // An unadmitted schema: the file regions are keyless, and so is the entry.
    let doc = run(&[("i.go", H)], &prism_boundary::SchemaIndex::default());
    assert_eq!(
        entries(&doc, "i.go"),
        [("annotation_not_checked".to_string(), Vec::<String>::new())]
    );
    // Control: an annotation that already spells the base names the projected operation itself.
    let schema = index_at(&swagger(Some("/api"), &[("GET", "/x")]), TREE, REV);
    let doc = run(
        &[(
            "i.go",
            b"package p\n\n// @Router /api/x [get]\nfunc H() {}\n",
        )],
        &schema,
    );
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_not_checked".to_string(),
            vec!["op:GET /api/x".to_string()]
        )]
    );
}

// ---- Fix 5: A11 -----------------------------------------------------------------------------

#[test]
fn f5_a_hit_on_a_known_projection_records_the_unprojected_deployments() {
    let schema = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "servers":[{"url":"/{region}","variables":{"region":{"default":"north"}}}],
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(&[("i.go", H)], &index_at(schema, TREE, REV));
    assert_eq!(
        grades(&doc),
        [("op:GET /north/x".to_string(), Some(EdgeGrade::Bound))],
        "the known projection still binds, uniquely"
    );
    assert_eq!(
        entries(&doc, "i.go"),
        [(
            "annotation_deployments_unprojected".to_string(),
            Vec::<String>::new()
        )]
    );
    // Control: a finite server set projects every deployment and records no region.
    let finite = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},
        "servers":[{"url":"/{region}","variables":{"region":{"default":"north","enum":["north","south"]}}}],
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(&[("i.go", H)], &index_at(finite, TREE, REV));
    assert_eq!(edges(&doc).len(), 2);
    assert!(doc.entries.is_empty());
}

// ---- Fix 6: a malformed empty method is null on the wire ------------------------------------

#[test]
fn f6_an_empty_method_token_is_null_and_the_document_validates() {
    let src: &[u8] = b"package p\n\n// @Router /x []\nfunc H() {}\n";
    let doc = run(&[("i.go", src)], &index(&[("GET", "/x")]));
    let record = record(&doc, "/x");
    assert_eq!(record.disposition, D::Malformed);
    assert_eq!(record.raw_method, None, "the published schema forbids \"\"");
    assert_eq!(record.raw_path.as_deref(), Some("/x"));
    assert_eq!(record.raw, "// @Router /x []");
    assert_wire_round_trip(&doc);
    assert_schema_valid(&doc);
    // Control: a non-empty invalid token keeps its spelling.
    let src: &[u8] = b"package p\n\n// @Router /x [*]\nfunc H() {}\n";
    let doc = run(&[("i.go", src)], &index(&[("GET", "/x")]));
    assert_eq!(record_of(&doc, "H").raw_method.as_deref(), Some("*"));
    assert_schema_valid(&doc);
}

// ---- Fix 7: duplicate expected entries ------------------------------------------------------

#[test]
fn f7_one_path_with_two_blobs_is_unknown_in_either_order() {
    let schema = index(&[("GET", "/x")]);
    let file: Supplied<'_> = ("x.go", "blob:one", TREE, H);
    let one = run_raw(
        &[("x.go", "blob:one"), ("x.go", "blob:two")],
        &[file],
        &schema,
        &input(),
    );
    let two = run_raw(
        &[("x.go", "blob:two"), ("x.go", "blob:one")],
        &[file],
        &schema,
        &input(),
    );
    for doc in [&one, &two] {
        let region = file_region(doc, "x.go");
        assert_eq!(
            (region.status, region.reason.as_deref()),
            (S::Unknown, Some("duplicate_expected_entry"))
        );
        assert!(
            edges(doc).is_empty(),
            "nothing binds from a contradictory path"
        );
        assert!(records(doc).is_empty());
        assert_eq!(doc.expected_files, ["x.go"]);
    }
    assert_eq!(
        serde_json::to_value(&one).unwrap(),
        serde_json::to_value(&two).unwrap(),
        "independent of input order"
    );
    assert_wire_round_trip(&one);
    // Control: identical duplicates coalesce and the path binds as usual.
    let doc = run_raw(
        &[("x.go", "blob:one"), ("x.go", "blob:one")],
        &[file],
        &schema,
        &input(),
    );
    assert_eq!(edges(&doc).len(), 1);
    assert_eq!(file_region(&doc, "x.go").status, S::Complete);
    assert_eq!(doc.file_status.len(), 1);
}

// ---- Fix 8: D09 -----------------------------------------------------------------------------

#[test]
fn f8_schema_inventory_is_refused_on_an_annotation_document() {
    let schema = index(&[("GET", "/x")]);
    let doc = run(&[("i.go", H)], &schema);
    assert_eq!(
        doc.with_schema_inventory(&schema).err(),
        Some(DocumentError::InvalidDocument),
        "schema inventory and annotation edges are distinct authorities"
    );
}
