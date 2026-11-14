//! One targeted, byte-bounded read of a listed repository source file under a review's captured
//! authority (#602 S4a). The path checks are an honest pre/post path-state guarantee, not an
//! atomic descriptor-based proof against a hostile concurrent rename. The caller owns the staged
//! capacity ledger; nothing here walks, lists, publishes or rolls back.

use super::{MAX_FILE_BYTES, SEVERE_PARSE_ERROR_RATE};
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::slice::FileParseQuality;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// What one `read` call produced, with the bytes actually returned by the stream.
#[non_exhaustive]
pub struct ContextReadAttempt {
    pub attempted_read_bytes: u64,
    pub outcome: ContextReadOutcome,
}

#[non_exhaustive]
pub enum ContextReadOutcome {
    Loaded(ContextFile),
    Skipped(ContextSkip),
    /// The caller's remaining source capacity (below the per-file limit) was exhausted.
    SourceCapacityBound,
}

#[non_exhaustive]
pub struct ContextFile {
    pub path: String,
    pub source: String,
    pub parsed: ParsedFile,
    pub parse_quality: Option<FileParseQuality>,
    pub parse_warnings: Vec<String>,
}

/// A per-file refusal. Messages are data, never a classification key.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextSkip {
    InvalidPath,
    NotListed,
    /// Not JS/JSX/TS/TSX/Python/Go, or a Go test or `testdata` file.
    Unsupported,
    /// An analysed file of the original review; its captured bytes are never replaced.
    OriginalFile,
    /// The root moved, or a component is a link or no longer a directory.
    UnsafePath,
    NotRegular,
    Unreadable {
        kind: std::io::ErrorKind,
        message: String,
    },
    NotUtf8,
    /// More than the per-file limit; the length is what was observed, not the file's size.
    TooLarge {
        observed_at_least: u64,
    },
    ParseFailed {
        message: String,
    },
    SevereParse {
        error_count: usize,
        node_count: usize,
    },
}

/// Captured values a read is judged against, borrowed from the review's capsule.
pub(crate) struct ReadAuthority<'a> {
    pub lexical_root: &'a Path,
    pub canonical_root: &'a Path,
    pub listing: &'a BTreeSet<String>,
    pub original: &'a BTreeSet<String>,
}

fn skipped(attempted: u64, skip: ContextSkip) -> ContextReadAttempt {
    ContextReadAttempt {
        attempted_read_bytes: attempted,
        outcome: ContextReadOutcome::Skipped(skip),
    }
}

fn unreadable(error: &std::io::Error) -> ContextSkip {
    ContextSkip::Unreadable {
        kind: error.kind(),
        message: error.to_string(),
    }
}

pub(crate) fn read(
    authority: &ReadAuthority<'_>,
    path: &str,
    remaining_source_bytes: u64,
) -> ContextReadAttempt {
    read_with(authority, path, remaining_source_bytes, |file| file)
}

/// `read` with the opened file passed through `wrap` before the bounded read. Production wraps
/// nothing (the identity); a test wraps the real file to act when the stream reaches its end.
/// The regular-file check on the opened handle runs before `wrap`.
pub(super) fn read_with<R: Read>(
    authority: &ReadAuthority<'_>,
    path: &str,
    remaining_source_bytes: u64,
    wrap: impl FnOnce(std::fs::File) -> R,
) -> ContextReadAttempt {
    let language = match admit_path(authority, path) {
        Ok(language) => language,
        Err(skip) => return skipped(0, skip),
    };
    let target = match check_path_state(authority, path) {
        Ok(target) => target,
        Err(skip) => return skipped(0, skip),
    };
    let file = match std::fs::File::open(&target) {
        Ok(file) => file,
        Err(error) => return skipped(0, unreadable(&error)),
    };
    // The opened object, not the path's earlier metadata, must be a regular file.
    match file.metadata() {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return skipped(0, ContextSkip::NotRegular),
        Err(error) => return skipped(0, unreadable(&error)),
    }
    read_stream(wrap(file), path, language, remaining_source_bytes, || {
        check_path_state(authority, path).map(drop)
    })
}

/// Spelling, captured-listing, original-file and language admission; no filesystem access.
pub(super) fn admit_path(
    authority: &ReadAuthority<'_>,
    path: &str,
) -> Result<Language, ContextSkip> {
    let normal_components = Path::new(path)
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if path.contains(['\0', '\\'])
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
        || !normal_components
    {
        return Err(ContextSkip::InvalidPath);
    }
    if !authority.listing.contains(path) {
        return Err(ContextSkip::NotListed);
    }
    if authority.original.contains(path) {
        return Err(ContextSkip::OriginalFile);
    }
    let language = Language::from_path(path).filter(|language| match language {
        Language::JavaScript | Language::TypeScript | Language::Tsx | Language::Python => true,
        // The Go rule of `review_context/import_targets/go.rs`.
        Language::Go => {
            path.ends_with(".go")
                && !path.ends_with("_test.go")
                && !path.split('/').any(|part| part == "testdata")
        }
        _ => false,
    });
    language.ok_or(ContextSkip::Unsupported)
}

/// The captured root still maps to the stored canonical root, and every component of `path`
/// below it is a real directory (the last, a regular file) that canonicalizes under that root.
pub(super) fn check_path_state(
    authority: &ReadAuthority<'_>,
    path: &str,
) -> Result<PathBuf, ContextSkip> {
    match std::fs::canonicalize(authority.lexical_root) {
        Ok(root) if root == authority.canonical_root => {}
        _ => return Err(ContextSkip::UnsafePath),
    }
    let parts: Vec<&str> = path.split('/').collect();
    let mut current = authority.canonical_root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        let file_type = std::fs::symlink_metadata(&current)
            .map_err(|error| unreadable(&error))?
            .file_type();
        if file_type.is_symlink() {
            return Err(ContextSkip::UnsafePath);
        }
        if index + 1 == parts.len() {
            if !file_type.is_file() {
                return Err(ContextSkip::NotRegular);
            }
        } else if !file_type.is_dir() {
            return Err(ContextSkip::UnsafePath);
        }
    }
    match std::fs::canonicalize(&current) {
        Ok(real) if real.starts_with(authority.canonical_root) => Ok(current),
        Ok(_) => Err(ContextSkip::UnsafePath),
        Err(error) => Err(unreadable(&error)),
    }
}

/// At most `limit + 1` bytes: the extra byte is the overflow sentinel. The buffer is kept when
/// the stream fails, so the caller counts what was actually returned.
pub(super) fn read_bounded<R: Read>(reader: R, limit: u64) -> (Vec<u8>, std::io::Result<()>) {
    let mut bytes = Vec::new();
    let result = reader.take(limit + 1).read_to_end(&mut bytes).map(drop);
    (bytes, result)
}

/// Bounded read, overflow table, post-read recheck, strict UTF-8 and parse of one stream.
/// `recheck` is the post-read path-state check, a seam so a test can refuse after the bytes.
pub(super) fn read_stream<R: Read>(
    reader: R,
    path: &str,
    language: Language,
    remaining_source_bytes: u64,
    recheck: impl FnOnce() -> Result<(), ContextSkip>,
) -> ContextReadAttempt {
    let limit = remaining_source_bytes.min(MAX_FILE_BYTES);
    let (bytes, result) = read_bounded(reader, limit);
    let attempted = bytes.len() as u64;
    if let Err(error) = result {
        return skipped(attempted, unreadable(&error));
    }
    if attempted > limit {
        // Equality with the per-file limit is a per-file skip, never a total bound.
        return if remaining_source_bytes >= MAX_FILE_BYTES {
            skipped(
                attempted,
                ContextSkip::TooLarge {
                    observed_at_least: attempted,
                },
            )
        } else {
            ContextReadAttempt {
                attempted_read_bytes: attempted,
                outcome: ContextReadOutcome::SourceCapacityBound,
            }
        };
    }
    if let Err(skip) = recheck() {
        return skipped(attempted, skip);
    }
    let source = match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(_) => return skipped(attempted, ContextSkip::NotUtf8),
    };
    let parsed = crate::build_pool::install(|| ParsedFile::parse(path, &source, language));
    finish(path, source, parsed, attempted)
}

/// Rejects a failed or severe parse, then grades an admitted one like the original review.
pub(super) fn finish(
    path: &str,
    source: String,
    parsed: anyhow::Result<ParsedFile>,
    attempted: u64,
) -> ContextReadAttempt {
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            return skipped(
                attempted,
                ContextSkip::ParseFailed {
                    message: format!("{error:#}"),
                },
            )
        }
    };
    if parsed.error_rate() > SEVERE_PARSE_ERROR_RATE {
        return skipped(
            attempted,
            ContextSkip::SevereParse {
                error_count: parsed.parse_error_count,
                node_count: parsed.parse_node_count,
            },
        );
    }
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let (parse_warnings, mut quality) = crate::ast::parse_quality::check_parse_quality(&files);
    let parsed = files.remove(path).expect("inserted above");
    ContextReadAttempt {
        attempted_read_bytes: attempted,
        outcome: ContextReadOutcome::Loaded(ContextFile {
            path: path.to_string(),
            source,
            parsed,
            parse_quality: quality.remove(path),
            parse_warnings,
        }),
    }
}
