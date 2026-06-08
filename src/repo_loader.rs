use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::type_db::TypeDatabase;
use anyhow::{Context, Result};
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

pub fn load_repo(root: &Path) -> Result<LoadedRepo> {
    let mut files = BTreeMap::new();
    let mut file_hashes = BTreeMap::new();
    let mut skipped = Vec::new();
    walk(root, root, &mut files, &mut file_hashes, &mut skipped)
        .with_context(|| format!("failed to read repository root {}", root.display()))?;
    Ok(LoadedRepo {
        root: root.to_path_buf(),
        files,
        file_hashes,
        skipped,
        type_db: None,
    })
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
    files: &mut BTreeMap<String, ParsedFile>,
    hashes: &mut BTreeMap<String, String>,
    skipped: &mut Vec<SkippedFile>,
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
                skipped.push(SkippedFile {
                    path,
                    reason: SkipReason::Unreadable,
                });
                continue;
            }
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                skipped.push(SkippedFile {
                    path: rel(root, &path),
                    reason: SkipReason::Unreadable,
                });
                continue;
            }
        };

        if BUILTIN_SKIP_DIRS.contains(&name.as_str())
            && (file_type.is_dir() || file_type.is_symlink())
        {
            skipped.push(SkippedFile {
                path: rel_dir(root, &path),
                reason: SkipReason::Ignored,
            });
            continue;
        }

        if file_type.is_symlink() {
            skipped.push(SkippedFile {
                path: rel(root, &path),
                reason: SkipReason::Symlink,
            });
            continue;
        }

        if file_type.is_dir() {
            if name.starts_with('.') {
                skipped.push(SkippedFile {
                    path: rel_dir(root, &path),
                    reason: SkipReason::Hidden,
                });
                continue;
            }
            if walk(root, &path, files, hashes, skipped).is_err() {
                skipped.push(SkippedFile {
                    path: rel_dir(root, &path),
                    reason: SkipReason::Unreadable,
                });
            }
            continue;
        }

        let relp = rel(root, &path);
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => {
                skipped.push(SkippedFile {
                    path: relp,
                    reason: SkipReason::Unreadable,
                });
                continue;
            }
        };

        if meta.len() > MAX_FILE_BYTES {
            skipped.push(SkippedFile {
                path: relp,
                reason: SkipReason::TooLarge { bytes: meta.len() },
            });
            continue;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                skipped.push(SkippedFile {
                    path: relp,
                    reason: SkipReason::Unreadable,
                });
                continue;
            }
        };

        let source = match String::from_utf8(bytes) {
            Ok(source) => source,
            Err(_) => {
                skipped.push(SkippedFile {
                    path: relp,
                    reason: SkipReason::NotUtf8,
                });
                continue;
            }
        };

        let language = match Language::from_path(&relp) {
            Some(language) => language,
            None => {
                skipped.push(SkippedFile {
                    path: relp,
                    reason: SkipReason::Unsupported,
                });
                continue;
            }
        };

        match ParsedFile::parse(&relp, &source, language) {
            Ok(parsed) => {
                if parsed.error_rate() > SEVERE_PARSE_ERROR_RATE {
                    skipped.push(SkippedFile {
                        path: relp,
                        reason: SkipReason::ParseFailed,
                    });
                    continue;
                }

                let mut hasher = Sha256::new();
                hasher.update(source.as_bytes());
                hashes.insert(relp.clone(), format!("{:x}", hasher.finalize()));
                files.insert(relp, parsed);
            }
            Err(_) => skipped.push(SkippedFile {
                path: relp,
                reason: SkipReason::ParseFailed,
            }),
        }
    }

    Ok(())
}
