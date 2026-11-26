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

#[derive(Clone, Debug, PartialEq, Eq)]
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
    Unsupported,
}

impl SnapshotInput {
    pub(crate) fn identity(&self) -> Option<(&str, Option<&str>)> {
        match self {
            Self::Commit { tree, rev, .. } => Some((tree, Some(rev))),
            Self::ComputedTree { tree, .. } => Some((tree, None)),
            Self::Unsupported => None,
        }
    }

    pub(crate) fn as_snapshot(&self) -> Option<Snapshot<'_>> {
        Some(match self {
            Self::Commit {
                tree,
                rev,
                subject,
                committed_at,
            } => Snapshot::Commit {
                tree,
                commit: rev,
                subject: subject.as_deref(),
                committer_date: committed_at.as_deref(),
            },
            Self::ComputedTree {
                tree,
                computed_at,
                merge_receipt,
            } => Snapshot::ComputedTree {
                tree,
                computed_at,
                merge_receipt,
            },
            Self::Unsupported => return None,
        })
    }
}

impl<'de> Deserialize<'de> for SnapshotInput {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "snapshot_kind", rename_all = "snake_case")]
        enum Wire {
            Commit {
                tree: String,
                rev: String,
                subject: Option<String>,
                committed_at: Option<String>,
            },
            ComputedTree {
                tree: String,
                computed_at: String,
                merge_receipt: String,
                rev: Option<String>,
                subject: Option<String>,
                committed_at: Option<String>,
            },
            #[serde(other)]
            Unsupported,
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Commit {
                tree,
                rev,
                subject,
                committed_at,
            } => Self::Commit {
                tree,
                rev,
                subject,
                committed_at,
            },
            Wire::ComputedTree {
                tree,
                computed_at,
                merge_receipt,
                rev: None,
                subject: None,
                committed_at: None,
            } => Self::ComputedTree {
                tree,
                computed_at,
                merge_receipt,
            },
            Wire::ComputedTree { .. } => {
                return Err(serde::de::Error::custom(
                    "computed tree carries commit metadata",
                ))
            }
            Wire::Unsupported => Self::Unsupported,
        })
    }
}

impl Serialize for SnapshotInput {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        match self {
            Self::Commit {
                tree,
                rev,
                subject,
                committed_at,
            } => {
                map.serialize_entry("snapshot_kind", "commit")?;
                map.serialize_entry("tree", tree)?;
                map.serialize_entry("rev", rev)?;
                map.serialize_entry("subject", subject)?;
                map.serialize_entry("committed_at", committed_at)?;
            }
            Self::ComputedTree {
                tree,
                computed_at,
                merge_receipt,
            } => {
                map.serialize_entry("snapshot_kind", "computed_tree")?;
                map.serialize_entry("tree", tree)?;
                map.serialize_entry("rev", &())?;
                map.serialize_entry("subject", &())?;
                map.serialize_entry("committed_at", &())?;
                map.serialize_entry("computed_at", computed_at)?;
                map.serialize_entry("merge_receipt", merge_receipt)?;
            }
            Self::Unsupported => {
                return Err(serde::ser::Error::custom("unsupported snapshot kind"))
            }
        }
        map.end()
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
