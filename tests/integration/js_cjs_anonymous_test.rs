//! #597: a module-scope CommonJS `require` binding called from an anonymous callable. The variant
//! matrix lives in `tests/fixtures/js_cjs_anonymous/matrix.jsonl`, one row per line: the producer
//! (`m.js` unless `producer_file` says otherwise), the consumer (`app.js` unless `consumer_file` says otherwise), the callee spelling and
//! the outcome of the call sites of that callee in the consumer.
//!
//! Row kinds: `admit` rows must resolve to the one Exact `import_member` edge the row names (they
//! fail on the base fec901a4); `refuse` rows pin main's literal outcome and must not carry an Exact
//! `import_member` edge (the barrier: direct top-level `const` or `var`, declared once in the resolved
//! program scope, no write resolving to it, never shadowed, unsupported file environments refuse); `control` rows pin main's outcome, which already holds
//! (named owners, module-level shapes main binds).
use super::js_relative_extension_test::graph;
use prism::call_graph::CallGraph;
use serde::Deserialize;

#[derive(Deserialize)]
struct Row {
    id: String,
    kind: String,
    producer: String,
    consumer: String,
    callee: String,
    expect: Vec<String>,
    consumer_file: Option<String>,
    producer_file: Option<String>,
    extra: Option<Vec<(String, String)>>,
}

const MATRIX: &str = include_str!("../fixtures/js_cjs_anonymous/matrix.jsonl");

fn outcome(cg: &CallGraph, callee: &str, file: &str) -> Vec<String> {
    let mut out = Vec::new();
    for site in cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.callee_name == callee && s.caller.file == file)
    {
        let res = cg.resolve_call_site_full(site);
        match res.drop {
            Some(drop) => out.push(format!("drop {drop:?}")),
            None => out.extend(res.resolved.iter().map(|r| {
                format!(
                    "{}:{}@{}-{} {:?}/{}",
                    r.target.file,
                    r.target.name,
                    r.target.start_line,
                    r.target.end_line,
                    r.confidence,
                    r.kind.as_str()
                )
            })),
        }
    }
    out.sort();
    out
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .map(|l| serde_json::from_str(l).expect("matrix row"))
        .collect()
}

#[test]
fn variant_matrix_holds() {
    let mut failures = Vec::new();
    let all = rows();
    assert!(all.len() >= 300, "matrix shrank to {} rows", all.len());
    for row in &all {
        let file = row.consumer_file.as_deref().unwrap_or("app.js");
        let producer = row.producer_file.as_deref().unwrap_or("m.js");
        let mut files: Vec<(&str, &str)> = vec![
            (producer, row.producer.as_str()),
            (file, row.consumer.as_str()),
        ];
        for (path, src) in row.extra.iter().flatten() {
            files.push((path.as_str(), src.as_str()));
        }
        let cg = graph(&files);
        let actual = outcome(&cg, &row.callee, file);
        if actual != row.expect {
            failures.push(format!(
                "{} [{}]: expected {:?}, got {:?}",
                row.id, row.kind, row.expect, actual
            ));
        }
        let exact_import = actual.iter().any(|a| a.contains("Exact/import_member"));
        let shape_ok = match row.kind.as_str() {
            "admit" => exact_import && actual.len() == 1,
            "refuse" => !exact_import,
            "control" => true,
            other => panic!("row {}: unknown kind {other}", row.id),
        };
        if !shape_ok {
            failures.push(format!(
                "{}: kind {} contradicts {:?}",
                row.id, row.kind, actual
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} rows:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The matrix covers every caller shape the brief names, so a row cannot be dropped silently.
#[test]
fn matrix_covers_the_named_shapes() {
    let all = rows();
    for needle in [
        "arrow-block/",
        "arrow-expr/",
        "function-expr/",
        "async-arrow/",
        "nested-anon/",
        "iife-function/",
        "iife-arrow/",
        "object-method/",
        "class-field-arrow/",
        "module-level/",
        "shadow-param-intermediate/",
        "shadow-inner-var-block/",
        "shadow-for-of-head/",
        "shadow-for-in-head/",
        "shadow-catch-param/",
        "shadow-param-destructured-default/",
        "shadow-switch-case-lexical/",
        "shadow-function-decl/",
        "shadow-with-body/",
        "write-assign-after/",
        "write-member-write/",
        "write-computed-write/",
        "write-object-assign/",
        "write-define-property/",
        "let-arrow-block/",
        "var-arrow-block/",
    ] {
        assert!(all.iter().any(|r| r.id.starts_with(needle)), "{needle}");
    }
}
