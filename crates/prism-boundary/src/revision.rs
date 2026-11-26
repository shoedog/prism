//! Inputs already admitted by the caller. Readers use supplied bytes and receipts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Snapshot<'a> {
    Commit {
        tree: &'a str,
        commit: &'a str,
        subject: Option<&'a str>,
        committer_date: Option<&'a str>,
    },
    ComputedTree {
        tree: &'a str,
        computed_at: &'a str,
        merge_receipt: &'a str,
    },
}

impl<'a> Snapshot<'a> {
    pub fn tree(self) -> &'a str {
        match self {
            Self::Commit { tree, .. } | Self::ComputedTree { tree, .. } => tree,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Json,
    Yaml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blob<'a> {
    pub path: &'a str,
    pub tree: &'a str,
    pub oid: &'a str,
    pub bytes: &'a [u8],
    pub format: Format,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceBlob<'a> {
    pub retrieval_uri: &'a str,
    pub blob: Blob<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyReceipt<'a> {
    pub input_id: &'a str,
    pub tree: &'a str,
    pub path: &'a str,
    pub blob: &'a str,
    pub entry: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct AdmittedSchemaInput<'a> {
    pub api: &'a str,
    pub authority: &'a str,
    pub snapshot: Snapshot<'a>,
    pub entry: Blob<'a>,
    pub retrieval_uri: Option<&'a str>,
    pub references: &'a [ReferenceBlob<'a>],
    pub config_receipts: &'a [DependencyReceipt<'a>],
    pub media_receipts: &'a [DependencyReceipt<'a>],
}
