//! The admission scope conjunct needs a scoped index, which only the crate can construct.

use super::*;
use crate::element::PathToken;
use crate::model::AnnotationDisposition as D;
use crate::revision::{AdmittedSchemaInput, Blob, Format, Snapshot};

const SWAGGER: &[u8] = br#"{"swagger":"2.0","info":{"title":"t","version":"1"},
    "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
const GO: &[u8] = b"package p\n\n// @Router /x [get]\nfunc H() {}\n";

fn index(scope: Scope) -> SchemaIndex {
    let mut index = crate::index_schema(&AdmittedSchemaInput {
        api: "pilot-api",
        authority: "schema",
        snapshot: Snapshot::Commit {
            tree: "T1",
            commit: "C1",
            subject: None,
            committer_date: None,
        },
        entry: Blob {
            path: "swagger.json",
            tree: "T1",
            oid: "schema-blob",
            bytes: SWAGGER,
            format: Format::Json,
        },
        retrieval_uri: None,
        references: &[],
        config_receipts: &[],
        media_receipts: &[],
    })
    .unwrap();
    index.scope = scope;
    index
}

fn input() -> DocumentInput {
    serde_json::from_value(json!({
        "input_id": "svc", "api": "pilot-api", "repo": "pilot-producer", "role": "producer",
        "snapshot_kind": "commit", "rev": "C1", "tree": "T1", "subject": null,
        "committed_at": null, "dirty": null, "tool": {"name": "prism-boundary", "build": "t"},
        "operation_authority": "annotation", "scope": {"kind": "api"},
        "origin": {"kind": "named_tree", "input_id": "svc", "repo": "pilot-producer",
                   "tree": "T1", "path": "swagger.json", "blob": "schema-blob",
                   "availability": "available"},
        "dependencies": [], "config_sources": [], "schema": null
    }))
    .unwrap()
}

fn disposition(schema: &SchemaIndex) -> D {
    let input = input();
    let expected = [ExpectedFile {
        path: "i.go",
        blob: "b",
    }];
    let files = [SourceBlob {
        tree: "T1",
        path: "i.go",
        blob: "b",
        bytes: GO,
    }];
    let doc = bind_producer(
        &ProducerInputs {
            input: &input,
            expected: &expected,
            files: &files,
        },
        schema,
        &ProducerOptions::default(),
    )
    .unwrap();
    doc.annotations.unwrap()[0].disposition
}

#[test]
fn a_scoped_index_is_not_admitted_for_an_api_wide_producer_document() {
    let scoped = Scope::PathPrefix {
        segments: vec![vec![PathToken::Literal("admin".into())]],
    };
    assert_eq!(disposition(&index(scoped)), D::NotChecked);
    assert_eq!(
        disposition(&index(Scope::Api)),
        D::Bound,
        "control: the API-wide index binds"
    );
}
