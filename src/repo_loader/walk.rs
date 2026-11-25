use super::{
    is_manifest_name, rel, skip_dir, source_files, CandidateData, MergeItem, SkipReason,
    SkippedFile, MAX_FILE_BYTES,
};
use crate::languages::Language;
use crate::manifest_snapshot::ManifestSnapshot;
use anyhow::Result;
use std::path::Path;

pub(super) fn walk(
    root: &Path,
    dir: &Path,
    items: &mut Vec<MergeItem>,
    candidates: &mut Vec<CandidateData>,
    manifest_snapshot: &mut ManifestSnapshot,
    selection: Option<source_files::Selection<'_>>,
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
        let relp = match selection {
            Some(_) => source_files::source_key(root, &path),
            None => Ok(rel(root, &path)),
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                items.push(MergeItem::Skip(SkippedFile {
                    path: relp.unwrap_or_else(|path| path),
                    reason: SkipReason::Unreadable,
                }));
                continue;
            }
        };

        if let Some(selection) = selection {
            if file_type.is_dir() {
                if selection.excluded_directories.contains(&name.as_str()) {
                    continue;
                }
            } else {
                match &relp {
                    Ok(key) if (selection.includes)(key) => {}
                    Err(path) if (selection.includes)(path) => {
                        items.push(MergeItem::Skip(SkippedFile {
                            path: path.clone(),
                            reason: SkipReason::UnrepresentablePath,
                        }));
                        continue;
                    }
                    _ => continue,
                }
            }
        }

        if skip_dir(&name, dir == root) && (file_type.is_dir() || file_type.is_symlink()) {
            items.push(MergeItem::Skip(SkippedFile {
                path: rel_dir(root, &path),
                reason: SkipReason::Ignored,
            }));
            continue;
        }

        if is_manifest_name(&name) {
            let relp = relp.as_ref().unwrap_or_else(|path| path).clone();
            if file_type.is_symlink() {
                manifest_snapshot.insert_symlink_refused(relp.clone());
                items.push(MergeItem::Skip(SkippedFile {
                    path: relp,
                    reason: SkipReason::Symlink,
                }));
                continue;
            }
            if file_type.is_file() {
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        manifest_snapshot.insert_regular(relp.clone(), bytes);
                        items.push(MergeItem::Skip(SkippedFile {
                            path: relp,
                            reason: SkipReason::Unsupported,
                        }));
                    }
                    Err(_) => items.push(MergeItem::Skip(SkippedFile {
                        path: relp,
                        reason: SkipReason::Unreadable,
                    })),
                }
                continue;
            }
        }

        if file_type.is_symlink() {
            items.push(MergeItem::Skip(SkippedFile {
                path: relp.unwrap_or_else(|path| path),
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
            if walk(root, &path, items, candidates, manifest_snapshot, selection).is_err() {
                items.push(MergeItem::Skip(SkippedFile {
                    path: rel_dir(root, &path),
                    reason: SkipReason::Unreadable,
                }));
            }
            continue;
        }

        let relp = relp.unwrap_or_else(|path| path);
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

        if Language::from_path(&relp) == Some(Language::Go)
            && relp.split('/').any(|component| component == "testdata")
        {
            items.push(MergeItem::Skip(SkippedFile {
                path: relp,
                reason: SkipReason::GoTestdata,
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

fn rel_dir(root: &Path, path: &Path) -> String {
    let mut path = rel(root, path);
    if !path.ends_with('/') {
        path.push('/');
    }
    path
}
