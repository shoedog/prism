//! Bounded, no-follow configuration/occupancy snapshot for the P1 resolver.
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct JsPathsSnapshot {
    pub(crate) configs: BTreeMap<String, Option<Vec<u8>>>,
    pub(crate) entries: BTreeMap<String, u8>, // 0 regular, 1 directory, 2 opaque
    pub(crate) complete: bool,
    opaque_directories: BTreeSet<String>,
    package_hashes: BTreeMap<String, String>,
}
impl JsPathsSnapshot {
    pub(crate) fn capture(root: &Path) -> Self {
        if root.as_os_str().is_empty() {
            return Self::default();
        }
        let mut s = Self {
            complete: true,
            ..Self::default()
        };
        if s.walk(root, root).is_err() {
            s.complete = false;
        }
        s
    }
    fn walk(&mut self, root: &Path, dir: &Path) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if self.entries.len() >= 200_000 {
                return Err(std::io::Error::other("P1 snapshot budget"));
            }
            let p = entry.path();
            let rel = p.strip_prefix(root).map_err(std::io::Error::other)?;
            let Some(rel) = rel.to_str() else {
                return Err(std::io::Error::other("P1 non-UTF8 path"));
            };
            let ft = entry.file_type()?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| std::io::Error::other("P1 path"))?;
            let blocked = ft.is_symlink()
                || name.starts_with('.')
                || ["target", "node_modules", "vendor", "dist", "build"].contains(&name);
            if blocked && (ft.is_dir() || ft.is_symlink()) {
                self.opaque_directories.insert(rel.to_owned());
            }
            if name == "package.json" {
                let digest = if !blocked && ft.is_file() && entry.metadata()?.len() <= 262_144 {
                    std::fs::read(&p)
                        .ok()
                        .map(|b| format!("{:x}", Sha256::digest(b)))
                } else {
                    None
                };
                self.package_hashes
                    .insert(rel.to_owned(), digest.unwrap_or_else(|| "opaque".into()));
            }
            self.entries.insert(
                rel.to_owned(),
                if blocked {
                    2
                } else if ft.is_dir() {
                    1
                } else if ft.is_file() {
                    0
                } else {
                    2
                },
            );
            let config_name = name.to_ascii_lowercase();
            if config_name == "jsconfig.json"
                || config_name == "tsconfig.json"
                || (config_name.starts_with("tsconfig.") && config_name.ends_with(".json"))
            {
                let bytes = if name == config_name
                    && !blocked
                    && ft.is_file()
                    && entry.metadata()?.len() <= 262_144
                {
                    std::fs::read(&p).ok()
                } else {
                    None
                };
                let key = format!("{}{}", &rel[..rel.len() - name.len()], config_name);
                self.configs.insert(key, bytes);
            }
            if ft.is_dir() && !blocked {
                self.walk(root, &p)?;
            }
        }
        Ok(())
    }
    pub(crate) fn topology(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        if self.configs.is_empty() {
            return out;
        }
        for (p, b) in &self.configs {
            out.insert(
                format!("js_paths_config:{p}"),
                b.as_ref()
                    .map(|b| format!("{:x}", Sha256::digest(b)))
                    .unwrap_or_else(|| "opaque".into()),
            );
        }
        // Additions, removals, skipped candidates, symlink and directory replacement all invalidate.
        let occupancy: BTreeMap<_, _> = self
            .entries
            .iter()
            .filter(|(p, kind)| {
                let name = p.rsplit('/').next().unwrap_or(p);
                self.opaque_directories.contains(*p)
                    || (**kind != 1
                        && (name == "package.json"
                            || (!name.starts_with('.') && !name.contains('.'))
                            || [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"]
                                .iter()
                                .any(|e| name.ends_with(e))))
            })
            .collect();
        let bytes = bincode::serialize(&(&occupancy, &self.package_hashes, self.complete))
            .expect("snapshot serializes");
        out.insert(
            "js_paths_occupancy".into(),
            format!("{:x}", Sha256::digest(bytes)),
        );
        out
    }
    pub(crate) fn unblocked(&self, p: &str) -> bool {
        let mut prefix = String::new();
        for part in p.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if self.entries.get(&prefix) == Some(&2) {
                return false;
            }
        }
        true
    }
}
