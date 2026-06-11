use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::type_db::TypeDatabase;
use anyhow::{Context, Result};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const SEVERE_PARSE_ERROR_RATE: f64 = 0.3;
const BUILTIN_SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "vendor", "dist", "build"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum SkipReason {
    Unsupported,
    Ignored,
    Symlink,
    Hidden,
    TooLarge { bytes: u64 },
    Unreadable,
    NotUtf8,
    ParseFailed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: SkipReason,
}

pub struct LoadedRepo {
    pub root: PathBuf,
    pub files: BTreeMap<String, ParsedFile>,
    pub file_hashes: BTreeMap<String, String>,
    pub skipped: Vec<SkippedFile>,
    pub type_db: Option<TypeDatabase>,
}

enum MergeItem {
    Skip(SkippedFile),
    Candidate { rel: String },
}

struct CandidateData {
    rel: String,
    source: String,
    language: Language,
}

struct ParseOutcome {
    rel: String,
    parsed: Result<ParsedFile>,
    hash: String,
}

pub fn load_repo(root: &Path) -> Result<LoadedRepo> {
    let (items, candidates) = collect_walk_items(root)?;
    let outcomes = parse_candidates_parallel(candidates);
    Ok(merge_walk_items(root, items, outcomes))
}

#[cfg_attr(not(test), allow(dead_code))] // parity twin: used by the in-module loader test
pub(crate) fn load_repo_serial_reference(root: &Path) -> Result<LoadedRepo> {
    let (items, candidates) = collect_walk_items(root)?;
    let outcomes = parse_candidates_serial(candidates);
    Ok(merge_walk_items(root, items, outcomes))
}

fn collect_walk_items(root: &Path) -> Result<(Vec<MergeItem>, Vec<CandidateData>)> {
    let mut items = Vec::new();
    let mut candidates = Vec::new();
    walk(root, root, &mut items, &mut candidates)
        .with_context(|| format!("failed to read repository root {}", root.display()))?;
    Ok((items, candidates))
}

fn parse_candidates_parallel(candidates: Vec<CandidateData>) -> Vec<ParseOutcome> {
    candidates.into_par_iter().map(parse_candidate).collect()
}

#[cfg_attr(not(test), allow(dead_code))] // parity twin: used by the in-module loader test
fn parse_candidates_serial(candidates: Vec<CandidateData>) -> Vec<ParseOutcome> {
    candidates.into_iter().map(parse_candidate).collect()
}

fn parse_candidate(candidate: CandidateData) -> ParseOutcome {
    let parsed = ParsedFile::parse(&candidate.rel, &candidate.source, candidate.language);

    let mut hasher = Sha256::new();
    hasher.update(candidate.source.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    ParseOutcome {
        rel: candidate.rel,
        parsed,
        hash,
    }
}

fn merge_walk_items(root: &Path, items: Vec<MergeItem>, outcomes: Vec<ParseOutcome>) -> LoadedRepo {
    let mut files = BTreeMap::new();
    let mut file_hashes = BTreeMap::new();
    let mut skipped = Vec::new();
    let mut outcomes = outcomes.into_iter();

    for item in items {
        match item {
            MergeItem::Skip(skip) => skipped.push(skip),
            MergeItem::Candidate { rel } => {
                let outcome = outcomes
                    .next()
                    .expect("candidate parse outcome missing during repository load");
                debug_assert_eq!(outcome.rel, rel);

                match outcome.parsed {
                    Ok(parsed) => {
                        if parsed.error_rate() > SEVERE_PARSE_ERROR_RATE {
                            skipped.push(SkippedFile {
                                path: rel,
                                reason: SkipReason::ParseFailed,
                            });
                            continue;
                        }

                        file_hashes.insert(rel.clone(), outcome.hash);
                        files.insert(rel, parsed);
                    }
                    Err(_) => skipped.push(SkippedFile {
                        path: rel,
                        reason: SkipReason::ParseFailed,
                    }),
                }
            }
        }
    }

    debug_assert!(outcomes.next().is_none());

    LoadedRepo {
        root: root.to_path_buf(),
        files,
        file_hashes,
        skipped,
        type_db: None,
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn rel_dir(root: &Path, path: &Path) -> String {
    let mut path = rel(root, path);
    if !path.ends_with('/') {
        path.push('/');
    }
    path
}

fn walk(
    root: &Path,
    dir: &Path,
    items: &mut Vec<MergeItem>,
    candidates: &mut Vec<CandidateData>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                let mut path = rel_dir(root, dir);
                if path == "/" {
                    path = ".".into();
                }
                items.push(MergeItem::Skip(SkippedFile {
                    path,
                    reason: SkipReason::Unreadable,
                }));
                continue;
            }
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                items.push(MergeItem::Skip(SkippedFile {
                    path: rel(root, &path),
                    reason: SkipReason::Unreadable,
                }));
                continue;
            }
        };

        if BUILTIN_SKIP_DIRS.contains(&name.as_str())
            && (file_type.is_dir() || file_type.is_symlink())
        {
            items.push(MergeItem::Skip(SkippedFile {
                path: rel_dir(root, &path),
                reason: SkipReason::Ignored,
            }));
            continue;
        }

        if file_type.is_symlink() {
            items.push(MergeItem::Skip(SkippedFile {
                path: rel(root, &path),
                reason: SkipReason::Symlink,
            }));
            continue;
        }

        if file_type.is_dir() {
            if name.starts_with('.') {
                items.push(MergeItem::Skip(SkippedFile {
                    path: rel_dir(root, &path),
                    reason: SkipReason::Hidden,
                }));
                continue;
            }
            if walk(root, &path, items, candidates).is_err() {
                items.push(MergeItem::Skip(SkippedFile {
                    path: rel_dir(root, &path),
                    reason: SkipReason::Unreadable,
                }));
            }
            continue;
        }

        let relp = rel(root, &path);
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => {
                items.push(MergeItem::Skip(SkippedFile {
                    path: relp,
                    reason: SkipReason::Unreadable,
                }));
                continue;
            }
        };

        if meta.len() > MAX_FILE_BYTES {
            items.push(MergeItem::Skip(SkippedFile {
                path: relp,
                reason: SkipReason::TooLarge { bytes: meta.len() },
            }));
            continue;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                items.push(MergeItem::Skip(SkippedFile {
                    path: relp,
                    reason: SkipReason::Unreadable,
                }));
                continue;
            }
        };

        let source = match String::from_utf8(bytes) {
            Ok(source) => source,
            Err(_) => {
                items.push(MergeItem::Skip(SkippedFile {
                    path: relp,
                    reason: SkipReason::NotUtf8,
                }));
                continue;
            }
        };

        let language = match Language::from_path(&relp) {
            Some(language) => language,
            None => {
                items.push(MergeItem::Skip(SkippedFile {
                    path: relp,
                    reason: SkipReason::Unsupported,
                }));
                continue;
            }
        };

        items.push(MergeItem::Candidate { rel: relp.clone() });
        candidates.push(CandidateData {
            rel: relp,
            source,
            language,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_loader_is_element_for_element_identical_to_serial_reference() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("sub")).unwrap();
        std::fs::write(p.join("a.py"), "def a():\n    return 1\n").unwrap();
        std::fs::write(p.join("sub/b.rs"), "fn b() {}\n").unwrap();
        std::fs::write(p.join("notes.txt"), "hello").unwrap(); // Unsupported
        std::fs::write(p.join("bad.xyz"), [0xFFu8, 0xFE, 0x00]).unwrap(); // NotUtf8 (read precedes ext check)
        std::fs::write(p.join("big.py"), "x".repeat(3 * 1024 * 1024)).unwrap(); // TooLarge
        let repo = load_repo(p).unwrap();
        let reference = load_repo_serial_reference(p).unwrap();
        assert_eq!(
            repo.files.keys().collect::<Vec<_>>(),
            reference.files.keys().collect::<Vec<_>>()
        );
        assert_eq!(repo.file_hashes, reference.file_hashes);
        let skips = |r: &LoadedRepo| {
            r.skipped
                .iter()
                .map(|s| (s.path.clone(), format!("{:?}", s.reason)))
                .collect::<Vec<_>>()
        };
        assert_eq!(skips(&repo), skips(&reference)); // element-for-element, ORDER included
                                                     // ABSOLUTE classification pin: read->UTF-8 precedes the extension check, so a
                                                     // non-UTF-8 file with an unsupported extension is NotUtf8. Twin-vs-twin parity
                                                     // alone cannot catch a walk-order regression that flips both twins.
        assert!(repo
            .skipped
            .iter()
            .any(|s| s.path == "bad.xyz" && matches!(s.reason, SkipReason::NotUtf8)));
    }
}
