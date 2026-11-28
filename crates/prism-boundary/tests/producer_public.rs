//! Public-pair run for P2-1. Ignored by default: it needs the pinned checkout and schema.
//! The document carries the tree's real schema path, blob id, tree, commit and sha256, and the
//! source bytes are read with `git cat-file`; the external schema copy must equal the blob.
//!
//! PRISM_BOUNDARY_PUBLIC_SWAGGER=<swagger.json> PRISM_BOUNDARY_PUBLIC_CORPUS=<checkout>
//! PRISM_BOUNDARY_PUBLIC_OUT=<RESULT.md> cargo test -p prism-boundary --test producer_public -- --ignored
//! Prints aggregates only and appends them to the output file.

#[allow(dead_code)]
#[path = "support/producer.rs"]
mod support;

use prism_boundary::model::{AnnotationDisposition as D, EdgeGrade, ExtractionStatus};
use prism_boundary::producer::{
    bind_producer, ExpectedFile, ProducerInputs, ProducerOptions, SourceBlob,
};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// (declarations with at least one Bound edge, declarations whose edges are all candidates).
fn declaration_counts(doc: &prism_boundary::model::BindingsDocument) -> (usize, usize) {
    let mut declarations: BTreeMap<_, bool> = BTreeMap::new();
    for row in support::rows(doc) {
        if let Some(handler) = &row.handler {
            *declarations
                .entry((handler.site.file.clone(), handler.site.span))
                .or_default() |= row.grade == Some(EdgeGrade::Bound);
        }
    }
    let bound = declarations.values().filter(|bound| **bound).count();
    (bound, declarations.len() - bound)
}

#[test]
fn the_census_counts_only_bound_declarations_and_labels_candidate_only_ones() {
    let schema = support::index(&[("GET", "/x")]);
    let src: &[u8] = b"package p\n\n// @Router /x [get]\nfunc H() {}\n";
    let other: &[u8] = b"package q\n\n// @Router /x [get]\nfunc K() {}\n";
    // Two handlers annotate one operation: both edges are candidates.
    let doc = support::run(&[("p.go", src), ("q.go", other)], &schema);
    assert!(support::rows(&doc)
        .iter()
        .all(|r| r.grade == Some(EdgeGrade::Candidate)));
    assert_eq!(declaration_counts(&doc), (0, 2));
    // Control: one handler, one Bound declaration.
    let doc = support::run(&[("p.go", src)], &schema);
    assert_eq!(declaration_counts(&doc), (1, 0));
}

fn git_bytes(repo: &str, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    out.stdout
}

fn git(repo: &str, args: &[&str]) -> String {
    String::from_utf8(git_bytes(repo, args)).unwrap()
}

const SCHEMA_PATH: &str = "docs/swagger.json";
const REPO: &str = "apache-answer";

#[test]
#[ignore]
fn public_pair_annotation_census() {
    let swagger =
        std::env::var("PRISM_BOUNDARY_PUBLIC_SWAGGER").expect("PRISM_BOUNDARY_PUBLIC_SWAGGER");
    let repo = std::env::var("PRISM_BOUNDARY_PUBLIC_CORPUS").expect("PRISM_BOUNDARY_PUBLIC_CORPUS");
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_string();
    let tree = git(&repo, &["rev-parse", "HEAD^{tree}"]).trim().to_string();

    // The document is built from the pinned tree's own values. The external schema copy must be
    // the tree's blob byte for byte, or the run refuses.
    let listing = git_bytes(&repo, &["ls-tree", "-r", "-z", &commit]);
    let mut blobs = BTreeMap::new();
    for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8(entry.to_vec()).unwrap();
        let (meta, path) = entry.split_once('\t').unwrap();
        let mut meta = meta.split_whitespace();
        let (_mode, kind, oid) = (meta.next(), meta.next(), meta.next().unwrap());
        if kind == Some("blob") {
            blobs.insert(path.to_string(), oid.to_string());
        }
    }
    let schema_blob = blobs[SCHEMA_PATH].clone();
    let schema_bytes = git_bytes(&repo, &["cat-file", "blob", &schema_blob]);
    assert_eq!(
        std::fs::read(&swagger).unwrap(),
        schema_bytes,
        "the external schema copy is not the pinned tree's {SCHEMA_PATH}"
    );
    let schema_sha = format!("{:x}", sha2::Sha256::digest(&schema_bytes));
    let schema = support::index_from(
        &schema_bytes,
        (support::API, "schema", &tree, &commit),
        (SCHEMA_PATH, &schema_blob),
    );

    // Source bytes are the tree's blobs, read with `git cat-file`, never working-tree bytes.
    let mut owned = Vec::new();
    for (path, oid) in &blobs {
        if !path.ends_with(".go") || path.ends_with("_test.go") {
            continue;
        }
        owned.push((
            path.clone(),
            oid.clone(),
            git_bytes(&repo, &["cat-file", "blob", oid]),
        ));
    }
    let expected: Vec<_> = owned
        .iter()
        .map(|(p, o, _)| ExpectedFile { path: p, blob: o })
        .collect();
    let files: Vec<_> = owned
        .iter()
        .map(|(p, o, b)| SourceBlob {
            tree: &tree,
            path: p,
            blob: o,
            bytes: b,
        })
        .collect();
    let wire = serde_json::json!({
        "api": support::API, "repo": REPO, "rev": commit, "tree": tree,
        "origin": {"kind": "named_tree", "input_id": "svc", "repo": REPO, "tree": tree,
                   "path": SCHEMA_PATH, "blob": schema_blob, "availability": "available"},
        "schema": {"source": {"kind": "file", "repo": REPO, "tree": tree, "rev": commit,
                   "committed_at": null, "path": SCHEMA_PATH, "blob": schema_blob,
                   "format": "json", "sha256": schema_sha},
                   "content_hash": null, "info_version": null}
    });
    let input = support::input_with(wire);
    let doc = bind_producer(
        &ProducerInputs {
            input: &input,
            expected: &expected,
            files: &files,
        },
        &schema,
        &ProducerOptions::default(),
    )
    .unwrap();

    let records = support::records(&doc);
    let mut by_disposition: BTreeMap<String, usize> = BTreeMap::new();
    for record in records {
        *by_disposition
            .entry(format!("{:?}", record.disposition))
            .or_default() += 1;
    }
    let (handlers_bound, handlers_candidate_only) = declaration_counts(&doc);
    // Only a Bound edge counts toward "annotation-bound"; a candidate or any other evidence is
    // reported separately.
    let bound_ops: BTreeSet<_> = support::rows(&doc)
        .iter()
        .filter(|r| r.grade == Some(EdgeGrade::Bound))
        .map(|r| r.operation_id.clone())
        .collect();
    let evidenced_ops: BTreeSet<_> = support::rows(&doc)
        .iter()
        .map(|r| r.operation_id.clone())
        .collect();
    let candidate_ops: BTreeSet<_> = support::rows(&doc)
        .iter()
        .filter(|r| r.grade == Some(EdgeGrade::Candidate))
        .map(|r| r.operation_id.clone())
        .collect();
    let schema_ops: BTreeSet<_> = schema.operations.iter().map(|o| o.id.to_string()).collect();
    let unannotated: Vec<_> = schema_ops.difference(&evidenced_ops).collect();
    let without: Vec<_> = records
        .iter()
        .filter(|r| r.disposition == D::AnnotationWithoutOperation)
        .map(|r| {
            format!(
                "{}:{} {} {}",
                r.site.file,
                r.site.line,
                r.raw_method.clone().unwrap_or_default(),
                r.raw_path.clone().unwrap_or_default()
            )
        })
        .collect();
    let parse_errors: Vec<_> = doc
        .file_status
        .iter()
        .filter(|r| r.reason.as_deref() == Some("parse_error"))
        .map(|r| r.path.clone())
        .collect();
    let in_router = |p: &String| p.contains("/router/") || p.contains("/controller");
    let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
    for region in &doc.file_status {
        let key = format!(
            "{:?}/{}",
            region.status,
            region.reason.clone().unwrap_or_default()
        );
        *statuses.entry(key).or_default() += 1;
    }
    assert!(doc
        .file_status
        .iter()
        .all(|r| r.status != ExtractionStatus::Unreadable));

    let report = format!(
        "\n## ACTUAL (commit {commit}, tree {tree}; non-test Go files only)\n\
         - schema: {SCHEMA_PATH} blob {schema_blob} sha256 {schema_sha}\n\
         - Go files examined: {} ({} expected, statuses {statuses:?})\n\
         - schema operations: {}; schema diagnostics: {}\n\
         - annotation records: {} by disposition {by_disposition:?}\n\
         - annotated functions (distinct handler declarations with a Bound edge): {}; candidate-only declarations: {}\n\
         - operations annotation-bound (grade Bound only): {} of {}; candidate operations: {}; operations with any annotation evidence: {}\n\
         - schema operations with no annotation binding: {} {unannotated:?}\n\
         - annotation_without_operation: {} {without:?}\n\
         - entry-level gaps: {}\n\
         - parse-error files: {} total, {} in router/controller paths {parse_errors:?}\n",
        files.len(),
        expected.len(),
        schema_ops.len(),
        schema.diagnostics.len(),
        records.len(),
        handlers_bound,
        handlers_candidate_only,
        bound_ops.len(),
        schema_ops.len(),
        candidate_ops.len(),
        evidenced_ops.len(),
        unannotated.len(),
        without.len(),
        doc.entries.len(),
        parse_errors.len(),
        parse_errors.iter().filter(|p| in_router(p)).count(),
    );
    println!("{report}");
    if let Ok(out) = std::env::var("PRISM_BOUNDARY_PUBLIC_OUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(out)
            .unwrap();
        file.write_all(report.as_bytes()).unwrap();
    }
}
