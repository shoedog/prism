use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnerAnchor {
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenAnchor {
    pub(crate) needle: &'static str,
    pub(crate) occurrence: usize,
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ExecutionRegion {
    ImmediateBody,
    NestedBody,
    ParameterBinding,
    ParameterDefault,
    ErasedType,
    EagerDefinitionExpression,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpectedToken {
    pub(crate) anchor: TokenAnchor,
    pub(crate) region: ExecutionRegion,
    pub(crate) owner: Option<OwnerAnchor>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuditCase {
    pub(crate) id: &'static str,
    pub(crate) source: String,
    pub(crate) language: Language,
    pub(crate) queried_owner: TokenAnchor,
    pub(crate) expected: Vec<ExpectedToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Consumer {
    QueryNames,
    QueryPaths,
    QuerySpans,
    ManualNames,
    ManualPaths,
    ManualSpans,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ObservedOccurrence {
    pub(crate) consumer: Consumer,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) start_byte: Option<usize>,
    pub(crate) end_byte: Option<usize>,
    pub(crate) owner: Option<OwnerAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnershipMismatch {
    pub(crate) case: &'static str,
    pub(crate) language: String,
    pub(crate) consumer: Consumer,
    pub(crate) token: String,
    pub(crate) expected_region: ExecutionRegion,
    pub(crate) expected_owner: Option<OwnerAnchor>,
    pub(crate) observed_owner: Option<OwnerAnchor>,
    pub(crate) start_byte: Option<usize>,
    pub(crate) end_byte: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GraphObservation {
    pub(crate) file: String,
    pub(crate) function: String,
    pub(crate) function_start_line: usize,
    pub(crate) path: String,
    pub(crate) access: String,
    pub(crate) line: usize,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) confidence_or_refusal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuditError {
    MissingSelector {
        needle: String,
    },
    AmbiguousSelector {
        needle: String,
        matches: usize,
    },
    OccurrenceOutOfRange {
        occurrence: usize,
        matches: usize,
    },
    OutOfRangeOffset {
        offset: usize,
        length: usize,
        source_len: usize,
    },
    InvalidUtf8Boundary {
        offset: usize,
        end: usize,
    },
    WrongText {
        expected: String,
        actual: String,
    },
    WrongOccurrence {
        expected: usize,
        actual: usize,
    },
    WrongLength {
        expected: usize,
        actual: usize,
    },
}

pub(crate) fn resolve_anchor(
    source: &str,
    anchor: &TokenAnchor,
) -> Result<Range<usize>, AuditError> {
    let matches: Vec<_> = source
        .match_indices(anchor.needle)
        .map(|(at, _)| at)
        .collect();
    if matches.is_empty() {
        return Err(AuditError::MissingSelector {
            needle: anchor.needle.to_string(),
        });
    }
    let occurrence = if anchor.occurrence == usize::MAX {
        if matches.len() != 1 {
            return Err(AuditError::AmbiguousSelector {
                needle: anchor.needle.to_string(),
                matches: matches.len(),
            });
        }
        0
    } else {
        anchor.occurrence
    };
    let Some(&actual_start) = matches.get(occurrence) else {
        return Err(AuditError::OccurrenceOutOfRange {
            occurrence,
            matches: matches.len(),
        });
    };
    let Some(end) = anchor.offset.checked_add(anchor.length) else {
        return Err(AuditError::OutOfRangeOffset {
            offset: anchor.offset,
            length: anchor.length,
            source_len: source.len(),
        });
    };
    if end > source.len() {
        return Err(AuditError::OutOfRangeOffset {
            offset: anchor.offset,
            length: anchor.length,
            source_len: source.len(),
        });
    }
    if !source.is_char_boundary(anchor.offset) || !source.is_char_boundary(end) {
        return Err(AuditError::InvalidUtf8Boundary {
            offset: anchor.offset,
            end,
        });
    }
    if anchor.length != anchor.needle.len() {
        return Err(AuditError::WrongLength {
            expected: anchor.needle.len(),
            actual: anchor.length,
        });
    }
    if actual_start != anchor.offset {
        return Err(AuditError::WrongOccurrence {
            expected: anchor.offset,
            actual: actual_start,
        });
    }
    let actual = &source[anchor.offset..end];
    if actual != anchor.needle {
        return Err(AuditError::WrongText {
            expected: anchor.needle.to_string(),
            actual: actual.to_string(),
        });
    }
    Ok(anchor.offset..end)
}

pub(crate) fn source_sha256(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

pub(crate) fn owner_anchor(node: tree_sitter::Node<'_>) -> OwnerAnchor {
    OwnerAnchor {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        kind: node.kind().to_string(),
    }
}

pub(crate) fn nearest_callable_owner(
    parsed: &ParsedFile,
    start_byte: usize,
    end_byte: usize,
) -> Option<OwnerAnchor> {
    parsed
        .all_functions()
        .into_iter()
        .filter(|node| node.start_byte() <= start_byte && end_byte <= node.end_byte())
        .min_by_key(|node| node.end_byte() - node.start_byte())
        .map(owner_anchor)
}

fn name_rows(consumer: Consumer, values: Vec<(String, usize)>) -> Vec<ObservedOccurrence> {
    values
        .into_iter()
        .map(|(path, line)| ObservedOccurrence {
            consumer,
            path,
            line,
            start_byte: None,
            end_byte: None,
            owner: None,
        })
        .collect()
}

fn path_rows(
    consumer: Consumer,
    values: Vec<(crate::access_path::AccessPath, usize)>,
) -> Vec<ObservedOccurrence> {
    name_rows(
        consumer,
        values
            .into_iter()
            .map(|(path, line)| (path.to_string(), line))
            .collect(),
    )
}

fn span_rows(
    parsed: &ParsedFile,
    consumer: Consumer,
    values: Vec<PathSpan>,
) -> Vec<ObservedOccurrence> {
    values
        .into_iter()
        .map(|span| ObservedOccurrence {
            consumer,
            path: span.path.to_string(),
            line: span.line,
            start_byte: Some(span.start_byte),
            end_byte: Some(span.end_byte),
            owner: nearest_callable_owner(parsed, span.start_byte, span.end_byte),
        })
        .collect()
}

pub(crate) fn observe_raw(
    parsed: &ParsedFile,
    owner: tree_sitter::Node<'_>,
    lines: &BTreeSet<usize>,
) -> Vec<ObservedOccurrence> {
    let mut out = name_rows(
        Consumer::QueryNames,
        parsed.rvalue_identifiers_on_lines(&owner, lines),
    );
    out.extend(path_rows(
        Consumer::QueryPaths,
        parsed.rvalue_identifier_paths_on_lines(&owner, lines),
    ));
    out.extend(span_rows(
        parsed,
        Consumer::QuerySpans,
        parsed.rvalue_identifier_spans_on_lines(&owner, lines),
    ));
    out.sort();
    out
}

pub(crate) fn observe_manual(
    parsed: &ParsedFile,
    names: Vec<(String, usize)>,
    paths: Vec<(crate::access_path::AccessPath, usize)>,
    spans: Vec<PathSpan>,
) -> Vec<ObservedOccurrence> {
    let mut out = name_rows(Consumer::ManualNames, names);
    out.extend(path_rows(Consumer::ManualPaths, paths));
    out.extend(span_rows(parsed, Consumer::ManualSpans, spans));
    out.sort();
    out
}

pub(crate) fn canonical_raw_rows(observations: &[ObservedOccurrence]) -> Vec<String> {
    let mut rows: Vec<_> = observations
        .iter()
        .map(|row| {
            format!(
                "{:?}|{}|{}|{}|{}|{}",
                row.consumer,
                row.path,
                row.line,
                row.start_byte
                    .map_or_else(|| "-".to_string(), |v| v.to_string()),
                row.end_byte
                    .map_or_else(|| "-".to_string(), |v| v.to_string()),
                row.owner.as_ref().map_or_else(
                    || "-".to_string(),
                    |o| format!("{}:{}-{}", o.kind, o.start_byte, o.end_byte)
                )
            )
        })
        .collect();
    rows.sort();
    rows
}

pub(crate) fn compare_expected(
    case: &AuditCase,
    observations: &[ObservedOccurrence],
) -> Vec<OwnershipMismatch> {
    let mut mismatches = Vec::new();
    for expected in &case.expected {
        if matches!(
            expected.region,
            ExecutionRegion::ImmediateBody | ExecutionRegion::EagerDefinitionExpression
        ) {
            continue;
        }
        let range = resolve_anchor(&case.source, &expected.anchor)
            .expect("fixture anchors are proved before collector comparison");
        for observed in observations {
            let exact_match =
                observed.start_byte == Some(range.start) && observed.end_byte == Some(range.end);
            let lossy_match = observed.start_byte.is_none()
                && observed.path == expected.anchor.needle
                && case.source.matches(expected.anchor.needle).count() == 1;
            if exact_match || lossy_match {
                mismatches.push(OwnershipMismatch {
                    case: case.id,
                    language: format!("{:?}", case.language),
                    consumer: observed.consumer,
                    token: expected.anchor.needle.to_string(),
                    expected_region: expected.region,
                    expected_owner: expected.owner.clone(),
                    observed_owner: observed.owner.clone(),
                    start_byte: observed.start_byte,
                    end_byte: observed.end_byte,
                });
            }
        }
    }
    mismatches.sort();
    mismatches
}
