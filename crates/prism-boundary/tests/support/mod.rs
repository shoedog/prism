use crate::schema::{read_operations, Inventory, SchemaError};
use prism_boundary::revision::{Blob, Format, ReferenceBlob};
use serde_json::Value;
use std::fs;
mod admission;
#[allow(dead_code)]
mod matrix;

pub use matrix::{fixture, matrix_file, rows, Row};

pub fn index(
    bytes: &[u8],
    format: Format,
    api: &str,
    uri: Option<&str>,
    dependencies: &[ReferenceBlob<'_>],
) -> Result<Inventory, SchemaError> {
    read_operations(&admission::admitted(bytes, format, api, uri, dependencies))
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
