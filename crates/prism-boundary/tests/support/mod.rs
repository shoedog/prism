use crate::schema::{read_operations, Inventory, SchemaError};
use prism_boundary::revision::{AdmittedSchemaInput, Blob, Format, ReferenceBlob, Snapshot};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const ROWS: &str = include_str!("../fixtures/ids_matrix/rows.tsv");

pub struct Row<'a> {
    pub id: &'a str,
    pub fixture: &'a str,
    pub operations: Vec<&'a str>,
    pub reason: Option<&'a str>,
}

pub fn rows() -> impl Iterator<Item = Row<'static>> {
    ROWS.lines().skip(1).map(|line| {
        let [id, fixture, ids, _, truth, _] =
            <[&str; 6]>::try_from(line.split('\t').collect::<Vec<_>>()).unwrap();
        Row {
            id,
            fixture,
            operations: ids.split(';').filter(|id| id.starts_with("op:")).collect(),
            reason: truth.strip_prefix("loss:"),
        }
    })
}

pub fn matrix_file(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ids_matrix")
        .join(path)
}

pub fn fixture(row: &Row, filename: Option<&str>) -> Vec<u8> {
    let path = matrix_file(row.fixture);
    fs::read(filename.map_or(path.clone(), |name| path.with_file_name(name))).unwrap()
}

pub fn index(
    bytes: &[u8],
    format: Format,
    api: &str,
    uri: Option<&str>,
    dependencies: &[ReferenceBlob<'_>],
) -> Result<Inventory, SchemaError> {
    read_operations(&AdmittedSchemaInput {
        api,
        authority: "schema",
        snapshot: Snapshot::ComputedTree {
            tree: "test-tree",
            computed_at: "2026-11-25T00:00:00Z",
            merge_receipt: "test-merge",
        },
        entry: Blob {
            path: "schema",
            tree: "test-tree",
            oid: "entry-blob",
            bytes,
            format,
        },
        retrieval_uri: uri,
        references: dependencies,
        config_receipts: &[],
        media_receipts: &[],
    })
}

pub fn json(bytes: &[u8]) -> Inventory {
    index(bytes, Format::Json, "test-api", None, &[]).unwrap()
}

pub fn row_index(row: &Row) -> Inventory {
    let bytes = fixture(row, None);
    let context = matrix_file(row.fixture).with_file_name("context.json");
    let context: Value = fs::read(context)
        .ok()
        .map(|data| serde_json::from_slice(&data).unwrap())
        .unwrap_or(Value::Null);
    let uri = context["retrieval_url"].as_str();
    if row.id == "s1-oas31-external-path-item-server" {
        let parts = fs::read(
            matrix_file(row.fixture)
                .with_file_name("branch")
                .join("parts.json"),
        )
        .unwrap();
        let reference = ReferenceBlob {
            retrieval_uri: "https://moonspindle.invalid/plans/branch/parts.json",
            blob: Blob {
                path: "branch/parts.json",
                tree: "test-tree",
                oid: "path-item-blob",
                bytes: &parts,
                format: Format::Json,
            },
        };
        return index(&bytes, Format::Json, "test-api", uri, &[reference]).unwrap();
    }
    index(&bytes, Format::Json, "test-api", uri, &[]).unwrap()
}
