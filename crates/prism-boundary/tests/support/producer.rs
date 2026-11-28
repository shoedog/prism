//! Shared builders for the P2 producer tests: schema indexes, inputs and Go fixtures.
use prism_boundary::model::{
    AnnotationDisposition as D, AnnotationRecord, Binding, BindingsDocument, DocumentInput,
    EdgeGrade, ExtractionRegion,
};
use prism_boundary::producer::{
    bind_producer, ExpectedFile, ProducerInputs, ProducerOptions, SourceBlob,
};
use prism_boundary::revision::{AdmittedSchemaInput, Blob, Format, Snapshot};
use prism_boundary::{index_schema, SchemaIndex};
use serde_json::json;

pub const API: &str = "pilot-api";
pub const TREE: &str = "T1";
pub const REV: &str = "C1";

pub fn swagger(base: Option<&str>, ops: &[(&str, &str)]) -> Vec<u8> {
    let mut paths = serde_json::Map::new();
    for (method, path) in ops {
        let item = paths.entry(path.to_string()).or_insert_with(|| json!({}));
        item[method.to_ascii_lowercase()] = json!({"responses": {"200": {"description": "ok"}}});
    }
    let mut doc = json!({"swagger": "2.0", "info": {"title": "t", "version": "1"}, "paths": paths});
    if let Some(base) = base {
        doc["basePath"] = json!(base);
    }
    serde_json::to_vec(&doc).unwrap()
}

pub fn index_at(bytes: &[u8], tree: &str, commit: &str) -> SchemaIndex {
    index_schema(&AdmittedSchemaInput {
        api: API,
        authority: "schema",
        snapshot: Snapshot::Commit {
            tree,
            commit,
            subject: None,
            committer_date: None,
        },
        entry: Blob {
            path: "swagger.json",
            tree,
            oid: "schema-blob",
            bytes,
            format: Format::Json,
        },
        retrieval_uri: None,
        references: &[],
        config_receipts: &[],
        media_receipts: &[],
    })
    .expect("supported schema fixture")
}

pub fn index(ops: &[(&str, &str)]) -> SchemaIndex {
    index_at(&swagger(None, ops), TREE, REV)
}

pub fn input() -> DocumentInput {
    input_with(json!({}))
}

pub fn input_with(patch: serde_json::Value) -> DocumentInput {
    let mut value = json!({
        "input_id": "svc", "api": API, "repo": "pilot-producer", "role": "producer",
        "snapshot_kind": "commit", "rev": REV, "tree": TREE, "subject": null, "committed_at": null,
        "dirty": null, "tool": {"name": "prism-boundary", "build": "test"},
        "operation_authority": "annotation", "scope": {"kind": "api"},
        "origin": {"kind": "named_tree", "input_id": "svc", "repo": "pilot-producer", "tree": TREE,
                   "path": "swagger.json", "blob": "schema-blob", "availability": "available"},
        "dependencies": [], "config_sources": [],
        "schema": {"source": {"kind": "file", "repo": "pilot-producer", "tree": TREE, "rev": REV,
                   "committed_at": null, "path": "swagger.json", "blob": "schema-blob",
                   "format": "json", "sha256": "0".repeat(64)}, "content_hash": null, "info_version": null}
    });
    for (key, new) in patch.as_object().into_iter().flatten() {
        value[key] = new.clone();
    }
    serde_json::from_value(value).expect("producer input")
}

pub fn go(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/producer/{name}.go",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

pub fn blob_id(path: &str) -> String {
    format!("blob:{path}")
}

/// Bind `files` (path, bytes) against `schema`; each file is expected under `blob:<path>`.
pub fn run(files: &[(&str, &[u8])], schema: &SchemaIndex) -> BindingsDocument {
    let ids: Vec<String> = files.iter().map(|(path, _)| blob_id(path)).collect();
    let expected: Vec<_> = files
        .iter()
        .zip(&ids)
        .map(|((path, _), blob)| ExpectedFile { path, blob })
        .collect();
    let blobs: Vec<_> = files
        .iter()
        .zip(&ids)
        .map(|((path, bytes), blob)| SourceBlob {
            tree: TREE,
            path,
            blob,
            bytes,
        })
        .collect();
    let input = input();
    bind_producer(
        &ProducerInputs {
            input: &input,
            expected: &expected,
            files: &blobs,
        },
        schema,
        &ProducerOptions::default(),
    )
    .expect("annotation adapter runs")
}

pub fn one(name: &str, ops: &[(&str, &str)]) -> BindingsDocument {
    let bytes = go(name);
    let path = format!("{name}.go");
    run(&[(path.as_str(), bytes.as_slice())], &index(ops))
}

pub fn rows(doc: &BindingsDocument) -> &[Binding] {
    doc.operations.as_deref().unwrap_or_default()
}

/// (operation id, grade, handler name), sorted.
pub fn edges(doc: &BindingsDocument) -> Vec<(String, Option<EdgeGrade>, String)> {
    let mut out: Vec<_> = rows(doc)
        .iter()
        .map(|row| {
            let handler = row
                .handler
                .as_ref()
                .map_or(String::new(), |h| h.name.clone());
            (row.operation_id.clone(), row.grade, handler)
        })
        .collect();
    out.sort();
    out
}

pub fn records(doc: &BindingsDocument) -> &[AnnotationRecord] {
    doc.annotations.as_deref().unwrap_or_default()
}

/// The record whose directive text contains `needle`.
pub fn record<'a>(doc: &'a BindingsDocument, needle: &str) -> &'a AnnotationRecord {
    let hits: Vec<_> = records(doc)
        .iter()
        .filter(|r| r.raw.contains(needle))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "one record for {needle:?}: {:?}",
        records(doc)
    );
    hits[0]
}

/// The record attached to the handler `name`.
pub fn record_of<'a>(doc: &'a BindingsDocument, name: &str) -> &'a AnnotationRecord {
    let hits: Vec<_> = records(doc)
        .iter()
        .filter(|r| r.handler.as_ref().is_some_and(|h| h.name == name))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "one record for handler {name}: {:?}",
        records(doc)
    );
    hits[0]
}

pub fn dispositions(doc: &BindingsDocument) -> Vec<D> {
    records(doc).iter().map(|r| r.disposition).collect()
}

pub fn file_region<'a>(doc: &'a BindingsDocument, path: &str) -> &'a ExtractionRegion {
    doc.file_status
        .iter()
        .find(|r| r.path == path)
        .unwrap_or_else(|| panic!("no region for {path}"))
}

/// (reason, possible keys) of every entry region of `path`.
pub fn entries(doc: &BindingsDocument, path: &str) -> Vec<(String, Vec<String>)> {
    doc.entries
        .iter()
        .filter(|r| r.path == path)
        .map(|r| {
            (
                r.reason.clone().unwrap_or_default(),
                r.possible_keys.clone(),
            )
        })
        .collect()
}

/// The source text a site's span covers.
pub fn spanned<'a>(src: &'a [u8], span: Option<[usize; 2]>) -> &'a str {
    let [start, end] = span.expect("span");
    std::str::from_utf8(&src[start..end]).unwrap()
}

pub fn line_of(src: &[u8], span: Option<[usize; 2]>) -> usize {
    let [start, _] = span.expect("span");
    src[..start].iter().filter(|b| **b == b'\n').count() + 1
}

/// The document survives the typed wire round trip and its closed contract.
pub fn assert_wire_round_trip(doc: &BindingsDocument) {
    let bytes = serde_json::to_vec(doc).unwrap();
    let decoded = BindingsDocument::from_json(&bytes).expect("document passes its contract");
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        serde_json::to_value(doc).unwrap()
    );
}
