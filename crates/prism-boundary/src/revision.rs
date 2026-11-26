//! Inputs already admitted by the caller. Readers use supplied bytes and receipts.

mod admission;
mod staleness;
pub use admission::*;
pub use staleness::*;

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "snapshot_kind", rename_all = "snake_case")]
pub enum SnapshotInput {
    Commit {
        rev: String,
        tree: String,
        subject: Option<String>,
        committed_at: Option<String>,
    },
    ComputedTree {
        tree: String,
        computed_at: String,
        merge_receipt: String,
    },
    #[serde(other)]
    Unsupported,
}

impl Serialize for SnapshotInput {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // P1d RED stub. The computed variant must project three explicit nulls.
        serializer.serialize_unit()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Producer,
    Consumer,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    Live,
    HistoricalAsOf,
    VersionComparison,
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
pub struct ProducerMediaEvidence<'a> {
    pub receipt: DependencyReceipt<'a>,
    pub bytes: &'a [u8],
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
    pub media_receipts: &'a [ProducerMediaEvidence<'a>],
}
