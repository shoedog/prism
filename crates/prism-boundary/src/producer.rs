//! P2 producer bindings: Go source inputs and swaggo annotation bindings.
//!
//! The extraction core takes receipt-bound inputs and the admitted schema index. It does not
//! read the filesystem or Git. PROVISIONAL: it inherits the admission scope in the crate README.

mod bind;
mod directive;
mod scan;

use crate::model::{BindingsDocument, DocumentInput, ExtractionRegion, ExtractionStatus as Status};
use crate::revision::{Role, Scope};
use crate::SchemaIndex;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// The adapters that can bind source sites to operations and fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerAdapter {
    SwaggoAnnotation,
    GoStructField,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducerOptions {
    pub adapters: Vec<ProducerAdapter>,
}

impl Default for ProducerOptions {
    /// P2-1 implements the annotation adapter only; the struct-field adapter lands in P2-2.
    fn default() -> Self {
        Self {
            adapters: vec![ProducerAdapter::SwaggoAnnotation],
        }
    }
}

/// One file the caller read from the named tree.
#[derive(Clone, Copy, Debug)]
pub struct SourceBlob<'a> {
    pub tree: &'a str,
    pub path: &'a str,
    pub blob: &'a str,
    pub bytes: &'a [u8],
}

/// One file the caller expects to be examined, with the blob id the tree lists for it.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedFile<'a> {
    pub path: &'a str,
    pub blob: &'a str,
}

/// Receipt-bound source inputs. `input` carries the snapshot identity, the repository, the
/// role (producer), the authority (`annotation`) and the scope (API).
#[derive(Clone, Copy, Debug)]
pub struct ProducerInputs<'a> {
    pub input: &'a DocumentInput,
    pub expected: &'a [ExpectedFile<'a>],
    pub files: &'a [SourceBlob<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerError {
    NoAdapters,
    NotImplemented(ProducerAdapter),
    WrongRole,
    WrongAuthority,
    UnsupportedSnapshot,
    UnsupportedScope,
}

impl std::fmt::Display for ProducerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapters => f.write_str("no producer adapter selected"),
            Self::NotImplemented(adapter) => write!(f, "{adapter:?} is not implemented"),
            Self::WrongRole => f.write_str("producer bindings need a producer input"),
            Self::WrongAuthority => {
                f.write_str("annotation bindings need the annotation authority")
            }
            Self::UnsupportedSnapshot => f.write_str("snapshot kind is not supported"),
            Self::UnsupportedScope => f.write_str("only the API scope is supported"),
        }
    }
}

impl std::error::Error for ProducerError {}

pub fn bind_producer(
    inputs: &ProducerInputs<'_>,
    schema: &SchemaIndex,
    opts: &ProducerOptions,
) -> Result<BindingsDocument, ProducerError> {
    if opts.adapters.is_empty() {
        return Err(ProducerError::NoAdapters);
    }
    if opts.adapters.contains(&ProducerAdapter::GoStructField) {
        return Err(ProducerError::NotImplemented(
            ProducerAdapter::GoStructField,
        ));
    }
    let input = inputs.input;
    if input.role != Role::Producer {
        return Err(ProducerError::WrongRole);
    }
    if input.operation_authority != "annotation" {
        return Err(ProducerError::WrongAuthority);
    }
    if !matches!(input.scope, Scope::Api) {
        return Err(ProducerError::UnsupportedScope);
    }
    let (tree, commit) = input
        .snapshot
        .identity()
        .ok_or(ProducerError::UnsupportedSnapshot)?;
    let admitted = schema.source.as_ref().is_some_and(|source| {
        source.api == input.api && source.tree == tree && source.commit.as_deref() == commit
    });

    let expected: BTreeMap<&str, &str> = inputs
        .expected
        .iter()
        .map(|file| (file.path, file.blob))
        .collect();
    let mut supplied: BTreeMap<&str, Vec<&SourceBlob<'_>>> = BTreeMap::new();
    for file in inputs.files {
        supplied.entry(file.path).or_default().push(file);
    }
    let region = |path: &str, status, reason: Option<&str>, blob: Option<&str>| ExtractionRegion {
        path: path.into(),
        scope: input.scope.clone(),
        status,
        reason: reason.map(Into::into),
        possible_keys: Vec::new(),
        evidence: blob.map(|blob| json!({ "blob": blob })),
    };
    let mut file_status = Vec::new();
    let mut scans = Vec::new();
    for (&path, &blob) in &expected {
        let found = supplied.get(path).map(Vec::as_slice).unwrap_or_default();
        let refusal = match found {
            [] => Some((Status::Unknown, "file_not_supplied", None)),
            [_, _, ..] => Some((Status::Unknown, "duplicate_file_input", None)),
            [file] if file.tree != tree => {
                Some((Status::Unknown, "tree_mismatch", Some(file.blob)))
            }
            [file] if file.blob != blob => {
                Some((Status::Unknown, "blob_mismatch", Some(file.blob)))
            }
            [file] if std::str::from_utf8(file.bytes).is_err() => {
                Some((Status::Unreadable, "invalid_utf8", Some(file.blob)))
            }
            [_] => None,
        };
        if let Some((status, reason, evidence)) = refusal {
            file_status.push(region(path, status, Some(reason), evidence));
            continue;
        }
        let file = found[0];
        let source = std::str::from_utf8(file.bytes).expect("checked above");
        match scan::scan(path, source) {
            None => file_status.push(region(
                path,
                Status::Unreadable,
                Some("parse_failed"),
                Some(blob),
            )),
            Some(scan) => {
                let (status, reason) = if scan.has_errors {
                    (Status::Partial, Some("parse_error"))
                } else if !admitted {
                    (Status::Partial, Some("schema_not_admitted"))
                } else {
                    (Status::Complete, None)
                };
                file_status.push(region(path, status, reason, reason.map(|_| blob)));
                scans.push(bind::FileScan {
                    tree: file.tree,
                    path: file.path,
                    blob: file.blob,
                    scan,
                });
            }
        }
    }
    for &path in supplied.keys().filter(|path| !expected.contains_key(*path)) {
        file_status.push(region(
            path,
            Status::Skipped,
            Some("not_in_expected_inventory"),
            None,
        ));
    }
    file_status.sort_by(|a, b| a.path.cmp(&b.path));

    let outcome = bind::bind(&scans, schema, admitted);
    let entries = outcome
        .gaps
        .iter()
        .map(|gap| ExtractionRegion {
            path: scans[gap.file].path.into(),
            scope: input.scope.clone(),
            status: Status::Partial,
            reason: Some(gap.reason.into()),
            possible_keys: gap.keys.clone(),
            evidence: Some(json!({ "blob": scans[gap.file].blob, "span": gap.span })),
        })
        .collect();
    Ok(BindingsDocument {
        schema_version: "0.1".into(),
        input: Some(input.clone()),
        expected_files: expected
            .keys()
            .map(|path| path.to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        file_status,
        entries,
        unaccounted_scopes: Vec::new(),
        dependency_records: Vec::new(),
        operations: Some(outcome.rows),
        calls: None,
        saved_scan: None,
        annotations: Some(outcome.records),
    })
}
