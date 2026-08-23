use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) const SYMLINK_REFUSED: &str = "symlink_refused";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ManifestSnapshotEntry {
    Regular { bytes: Vec<u8>, hash: String },
    SymlinkRefused,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestSnapshot {
    entries: BTreeMap<String, ManifestSnapshotEntry>,
}

impl ManifestSnapshot {
    pub(crate) fn insert_regular(&mut self, path: String, bytes: Vec<u8>) {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        self.entries.insert(
            path,
            ManifestSnapshotEntry::Regular {
                bytes,
                hash: format!("{:x}", hasher.finalize()),
            },
        );
    }

    pub(crate) fn insert_symlink_refused(&mut self, path: String) {
        self.entries
            .insert(path, ManifestSnapshotEntry::SymlinkRefused);
    }

    pub(crate) fn get(&self, path: &str) -> Option<&ManifestSnapshotEntry> {
        self.entries.get(path)
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&String, &ManifestSnapshotEntry)> {
        self.entries.iter()
    }

    pub fn topology_hashes(&self) -> BTreeMap<String, String> {
        self.entries
            .iter()
            .map(|(path, entry)| {
                let value = match entry {
                    ManifestSnapshotEntry::Regular { hash, .. } => hash.clone(),
                    ManifestSnapshotEntry::SymlinkRefused => SYMLINK_REFUSED.to_string(),
                };
                (path.clone(), value)
            })
            .collect()
    }
}
