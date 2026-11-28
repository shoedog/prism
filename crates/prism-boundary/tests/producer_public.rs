//! Public-pair run for P2-1. Ignored by default: it needs the pinned checkout and schema.
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
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

fn git(repo: &str, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
#[ignore]
fn public_pair_annotation_census() {
    let swagger =
        std::env::var("PRISM_BOUNDARY_PUBLIC_SWAGGER").expect("PRISM_BOUNDARY_PUBLIC_SWAGGER");
    let repo = std::env::var("PRISM_BOUNDARY_PUBLIC_CORPUS").expect("PRISM_BOUNDARY_PUBLIC_CORPUS");
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_string();
    let tree = git(&repo, &["rev-parse", "HEAD^{tree}"]).trim().to_string();
    let schema_bytes = std::fs::read(&swagger).unwrap();
    let schema = support::index_at(&schema_bytes, &tree, &commit);

    let listing = git(&repo, &["ls-files", "-s", "--", "*.go"]);
    let mut owned = Vec::new();
    for line in listing.lines() {
        let (meta, path) = line.split_once('\t').unwrap();
        let oid = meta.split_whitespace().nth(1).unwrap().to_string();
        if path.ends_with("_test.go") {
            continue;
        }
        let bytes = std::fs::read(format!("{repo}/{path}")).unwrap();
        owned.push((path.to_string(), oid, bytes));
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
        "api": support::API, "rev": commit, "tree": tree,
        "origin": {"kind": "named_tree", "input_id": "svc", "repo": "pilot-producer", "tree": tree,
                   "path": "swagger.json", "blob": "schema-blob", "availability": "available"}
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
    let handlers: BTreeSet<_> = support::rows(&doc)
        .iter()
        .filter_map(|r| {
            r.handler
                .as_ref()
                .map(|h| (h.site.file.clone(), h.site.span))
        })
        .collect();
    let bound_ops: BTreeSet<_> = support::rows(&doc)
        .iter()
        .map(|r| r.operation_id.clone())
        .collect();
    let candidate_ops: BTreeSet<_> = support::rows(&doc)
        .iter()
        .filter(|r| r.grade == Some(EdgeGrade::Candidate))
        .map(|r| r.operation_id.clone())
        .collect();
    let schema_ops: BTreeSet<_> = schema.operations.iter().map(|o| o.id.to_string()).collect();
    let unannotated: Vec<_> = schema_ops.difference(&bound_ops).collect();
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
         - Go files examined: {} ({} expected, statuses {statuses:?})\n\
         - schema operations: {}; schema diagnostics: {}\n\
         - annotation records: {} by disposition {by_disposition:?}\n\
         - annotated functions (distinct handler declarations bound): {}\n\
         - operations annotation-bound: {} of {}; candidate operations: {}\n\
         - schema operations with no annotation binding: {} {unannotated:?}\n\
         - annotation_without_operation: {} {without:?}\n\
         - entry-level gaps: {}\n\
         - parse-error files: {} total, {} in router/controller paths {parse_errors:?}\n",
        files.len(),
        expected.len(),
        schema_ops.len(),
        schema.diagnostics.len(),
        records.len(),
        handlers.len(),
        bound_ops.len(),
        schema_ops.len(),
        candidate_ops.len(),
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
