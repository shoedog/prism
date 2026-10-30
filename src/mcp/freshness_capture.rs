//! Bootstrap-only validation against the loader's retained source and input evidence.
use super::*;
use crate::repo_loader::{collect_freshness_source_paths, scope_graph_build_inputs, SkipReason};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub(super) struct Evidence {
    report: Option<FreshnessReport>,
    uncertain: bool,
    observed: Mutex<[u8; 32]>,
}

pub(super) fn validate(repo: &LoadedRepo, tracked: &mut BTreeMap<String, PathStamp>) -> Evidence {
    #[cfg(all(test, unix))]
    super::capture_tests::barrier("before-validation");
    let mut changed = BTreeSet::new();
    let mut uncertain = false;
    // Skipped unreadable entries may have no readable type/size metadata.
    // Their absence from the tolerant inventory does not lose indexed bytes.
    let unreadable: BTreeSet<_> = repo
        .skipped
        .iter()
        .filter(|skip| skip.reason == SkipReason::Unreadable)
        .map(|skip| skip.path.as_str())
        .collect();
    // No pre-load walk: loaded files plus explicit candidate skips supply that
    // boundary. Neither an unreadable sibling nor a parse refusal erases it.
    let expected: BTreeSet<_> = repo
        .files
        .keys()
        .cloned()
        .chain(
            repo.skipped
                .iter()
                .filter(|s| {
                    matches!(
                        s.reason,
                        SkipReason::GoTestdata
                            | SkipReason::ParseFailed
                            | SkipReason::NotUtf8
                            | SkipReason::Unreadable
                    ) && !s.path.ends_with('/')
                        && crate::languages::Language::from_path(&s.path).is_some()
                })
                .map(|s| s.path.clone()),
        )
        .collect();
    for (rel, loaded_hash) in &repo.file_hashes {
        let (valid, unreadable, current_stamp) =
            validate_file(&repo.root, rel, loaded_hash, repo.files[rel].source.len());
        if !valid {
            changed.insert(rel.clone());
        }
        uncertain |= unreadable;
        tracked.insert(rel.clone(), current_stamp);
    }
    for attempt in 0..2 {
        let directories: BTreeMap<_, _> = tracked
            .iter()
            .filter(|(_, s)| s.kind == PathKind::Directory)
            .map(|(p, _)| (p.clone(), PathStamp::read(&repo.root, p)))
            .collect();
        #[cfg(all(test, unix))]
        super::capture_tests::barrier("inputs-between");
        let current = scope_graph_build_inputs(&repo.root, &repo.files);
        let members = collect_freshness_source_paths(&repo.root);
        #[cfg(all(test, unix))]
        super::capture_tests::barrier("directories-validated");
        let stable = directories
            .iter()
            .all(|(p, s)| *s == PathStamp::read(&repo.root, p));
        if let Some(members) = &members {
            changed.extend(
                expected
                    .symmetric_difference(members)
                    .filter(|path| !unreadable.contains(path.as_str()))
                    .cloned(),
            );
        } else {
            changed.insert("source_membership".into());
            uncertain = true;
        }
        if let Some(loaded) = &repo.scope_graph_inputs {
            for key in loaded
                .manifest_hashes
                .keys()
                .chain(current.manifest_hashes.keys())
            {
                if loaded.manifest_hashes.get(key) != current.manifest_hashes.get(key) {
                    changed.insert(key.clone());
                }
            }
            if loaded.manifest_snapshot != current.manifest_snapshot {
                changed.insert("manifest_snapshot".into());
            }
            if loaded.complete != current.complete
                || loaded.js_paths_snapshot.complete != current.js_paths_snapshot.complete
            {
                changed.insert("input_completeness".into());
                uncertain = true;
            }
            for (path, bytes) in &loaded.js_paths_snapshot.configs {
                uncertain |= bytes.is_some()
                    && current
                        .js_paths_snapshot
                        .configs
                        .get(path)
                        .is_none_or(Option::is_none);
            }
        }
        if stable {
            tracked.extend(directories);
            break;
        }
        if attempt == 1 {
            changed.insert("source_membership".into());
        }
    }
    // Stable typed input bytes (including touch-only changes) certify their
    // baseline too. Refusals/absences remain loader-owned replay comparisons.
    for (rel, entry) in repo.manifest_snapshot.entries() {
        if let crate::manifest_snapshot::ManifestSnapshotEntry::Regular { hash, bytes } = entry {
            let (valid, unreadable, current_stamp) =
                validate_file(&repo.root, rel, hash, bytes.len());
            if !valid {
                changed.insert(rel.clone());
            }
            uncertain |= unreadable;
            tracked.insert(rel.clone(), current_stamp);
        }
    }
    let report = FreshnessReport::from_changed_paths(changed);
    Evidence {
        report: report.stale.then_some(report),
        uncertain,
        ..Evidence::default()
    }
}

fn validate_file(
    root: &Path,
    rel: &str,
    loaded_hash: &str,
    loaded_len: usize,
) -> (bool, bool, PathStamp) {
    // One retry avoids calling an equal-content touch a race. A stable current
    // hash is compared to the LOADED hash, never to another current read.
    for attempt in 0..2 {
        let before = PathStamp::read(root, rel);
        #[cfg(all(test, unix))]
        super::capture_tests::barrier("source-stamped");
        if before.len.is_some_and(|len| len != loaded_len as u64) {
            if attempt == 0 {
                continue;
            }
            return (false, false, before);
        }
        // A growing file cannot expand validation beyond the loader's bytes.
        let bytes = fs::File::open(root.join(rel)).and_then(|file| {
            let mut bytes = Vec::new();
            file.take(loaded_len as u64 + 1).read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        #[cfg(all(test, unix))]
        super::capture_tests::barrier("source-read");
        let after = PathStamp::read(root, rel);
        if before != after && attempt == 0 {
            continue;
        }
        return match bytes {
            Ok(bytes) => (
                before == after && format!("{:x}", Sha256::digest(bytes)) == loaded_hash,
                false,
                after,
            ),
            Err(_) => (false, true, after),
        };
    }
    unreachable!("bounded validation returns on the second read")
}

impl FreshnessProbe {
    pub(super) fn check_captured(&self) -> FreshnessReport {
        if !self.capture.uncertain {
            if let Some(report) = &self.capture.report {
                return report.clone();
            }
        }
        let mut changed = BTreeSet::new();
        let mut digest = Sha256::new();
        for (rel, before) in &self.tracked {
            let (after, _metadata) = if before.kind == PathKind::Logical {
                (before.clone(), None)
            } else {
                PathStamp::read_observed(&self.root, rel)
            };
            if self.capture.uncertain {
                digest.update(format!("{rel:?}{after:?}"));
                // Permission-only recovery must re-arm auto refresh. Reuse this
                // stat's access evidence only in suppression, never stamp equality.
                #[cfg(unix)]
                if let Some(metadata) = _metadata {
                    use std::os::unix::fs::MetadataExt;
                    digest.update(format!(
                        "{:?}",
                        (metadata.mode(), metadata.ctime(), metadata.ctime_nsec())
                    ));
                }
            }
            if before != &after {
                changed.insert(rel.clone());
            }
        }
        if let Some(report) = &self.capture.report {
            digest.update(format!("{report:?}"));
        }
        if self.capture.uncertain {
            *self
                .capture
                .observed
                .lock()
                .expect("freshness evidence lock") = digest.finalize().into();
        }
        self.capture
            .report
            .clone()
            .unwrap_or_else(|| FreshnessReport::from_changed_paths(changed))
    }

    pub(crate) fn bootstrap_report(&self) -> Option<&FreshnessReport> {
        self.capture.report.as_ref()
    }
    pub(crate) fn uncertifiable_key(&self) -> Option<[u8; 32]> {
        self.capture.uncertain.then(|| {
            *self
                .capture
                .observed
                .lock()
                .expect("freshness evidence lock")
        })
    }
    pub(crate) fn clear_bootstrap_evidence(&mut self) {
        self.capture = Evidence::default();
    }
}
