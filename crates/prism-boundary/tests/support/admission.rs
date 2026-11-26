use prism_boundary::revision::{AdmittedSchemaInput, Blob, Format, ReferenceBlob, Snapshot};

pub fn admitted<'a>(
    bytes: &'a [u8],
    format: Format,
    api: &'a str,
    uri: Option<&'a str>,
    dependencies: &'a [ReferenceBlob<'a>],
) -> AdmittedSchemaInput<'a> {
    AdmittedSchemaInput {
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
    }
}
