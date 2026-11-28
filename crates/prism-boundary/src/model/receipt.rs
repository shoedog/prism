//! The one place that owns every identity field a declared schema source can carry.
//!
//! P1d input validation (against the snapshot) and P2 producer admission (against the snapshot
//! and the schema index) both call `schema_receipt_agrees`, so a field is classified once.
//! The destructuring below names every field of `SchemaOrigin` and `SchemaEvidence` without
//! `..`: adding a field is a compile error until it is placed here.

use super::{SchemaEvidence, SchemaOrigin};
use crate::revision::{instant, SnapshotInput};
use crate::SchemaIndex;

/// Wire names of `SchemaOrigin::File` as classified by `schema_receipt_agrees`, sorted. A unit
/// test serializes the type and fails when its JSON field names differ from this list.
#[cfg(test)]
const FILE_FIELDS: [&str; 10] = [
    "blob",
    "committed_at",
    "format",
    "kind",
    "path",
    "repo",
    "rev",
    "sha256",
    "subject",
    "tree",
];

/// Wire names of `SchemaEvidence`, sorted.
#[cfg(test)]
const EVIDENCE_FIELDS: [&str; 3] = ["content_hash", "info_version", "source"];

/// Whether every identity value `evidence` declares agrees with what the caller can check it
/// against. `snapshot` is the revision the declaring input names; `index` is the schema index the
/// receipt is bound to, when there is one (the validator has none).
///
/// A declared value is present when it is `Some`, whatever its text: an empty string is compared
/// like any other value. A field left null makes no claim and is not compared, except that a
/// commit snapshot's receipt must name the snapshot's revision, and a computed-tree snapshot's
/// receipt must carry no commit metadata at all.
pub(crate) fn schema_receipt_agrees(
    evidence: &SchemaEvidence,
    snapshot: &SnapshotInput,
    index: Option<&SchemaIndex>,
) -> bool {
    let SchemaEvidence {
        source,
        content_hash,
        info_version,
    } = evidence;
    let sha256 = match source {
        SchemaOrigin::File {
            // Location: where the file lives. The binder compares path, blob, repo and tree
            // against the index in `declares_schema_source`; the validator compares repo and
            // tree with the input.
            repo: _,
            tree: _,
            path: _,
            blob: _,
            // Not an identity claim: the wire contract pins it to "json", the index does not
            // carry it (the bytes were indexed as JSON), and a mismatch would change nothing a
            // binding or an impact claim relies on.
            format: _,
            // Identity claims against the snapshot.
            rev,
            committed_at,
            subject,
            // Identity claim against the index.
            sha256,
        } => {
            if !snapshot_metadata_agrees(snapshot, rev, committed_at, subject) {
                return false;
            }
            Some(sha256)
        }
        // These carry no commit metadata or digest; their repo and tree are location, compared
        // by the callers.
        SchemaOrigin::RouteTable {
            repo: _,
            tree: _,
            input_id: _,
        }
        | SchemaOrigin::MissingExpectedSchema {
            repo: _,
            tree: _,
            path: _,
            reason: _,
        } => None,
    };
    let Some(index) = index else {
        return true;
    };
    sha256.is_none_or(|sha256| *sha256 == index.byte_sha256)
        && content_hash
            .as_ref()
            .is_none_or(|hash| *hash == index.content_hash)
        && info_version
            .as_ref()
            .is_none_or(|version| index.info_version.as_ref() == Some(version))
}

fn snapshot_metadata_agrees(
    snapshot: &SnapshotInput,
    rev: &Option<String>,
    committed_at: &Option<String>,
    subject: &Option<String>,
) -> bool {
    match snapshot {
        SnapshotInput::ComputedTree { .. } => {
            rev.is_none() && committed_at.is_none() && subject.is_none()
        }
        SnapshotInput::Commit {
            rev: snapshot_rev,
            committed_at: date,
            subject: title,
            ..
        } => {
            rev.as_deref() == Some(snapshot_rev.as_str())
                && committed_at
                    .as_ref()
                    .is_none_or(|at| instant(at) == date.as_deref().and_then(instant))
                && subject
                    .as_ref()
                    .is_none_or(|text| Some(text) == title.as_ref())
        }
        SnapshotInput::InvalidMetadata | SnapshotInput::Unsupported => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(value: serde_json::Value) -> Vec<String> {
        let mut names: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        names.sort();
        names
    }

    #[test]
    fn the_classified_fields_are_exactly_the_serialized_ones() {
        let file = SchemaOrigin::File {
            repo: "r".into(),
            tree: "t".into(),
            rev: Some("c".into()),
            subject: Some("s".into()),
            committed_at: Some("2026-03-01T10:00:00Z".into()),
            path: "p".into(),
            blob: "b".into(),
            format: "json".into(),
            sha256: "0".repeat(64),
        };
        assert_eq!(
            names(serde_json::to_value(&file).unwrap()),
            FILE_FIELDS.map(String::from),
            "classify the new SchemaOrigin::File field in schema_receipt_agrees"
        );
        let evidence = SchemaEvidence {
            source: file,
            content_hash: Some("h".into()),
            info_version: Some("1".into()),
        };
        assert_eq!(
            names(serde_json::to_value(&evidence).unwrap()),
            EVIDENCE_FIELDS.map(String::from),
            "classify the new SchemaEvidence field in schema_receipt_agrees"
        );
    }
}
