//! Bounded, no-follow configuration/occupancy snapshot for the P1 resolver.
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct JsPathsSnapshot {
    pub(crate) configs: BTreeMap<String, Option<Vec<u8>>>,
    pub(crate) entries: BTreeMap<String, u8>, // 0 regular, 1 directory, 2 opaque
    pub(crate) complete: bool,
    opaque_directories: BTreeSet<String>,
    package_hashes: BTreeMap<String, String>,
    pub(crate) ambient: BTreeMap<String, Vec<String>>,
    root: PathBuf,
    #[serde(skip)]
    probes: Arc<Mutex<BTreeSet<String>>>,
    #[serde(skip)]
    packages: Arc<Mutex<BTreeMap<String, u8>>>,
}
impl JsPathsSnapshot {
    pub(crate) fn capture(root: &Path) -> Self {
        if root.as_os_str().is_empty() {
            return Self::default();
        }
        let mut s = Self {
            complete: true,
            root: std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf()),
            ..Self::default()
        };
        if let Err(error) = s.walk(root, root) {
            s.complete = false;
            eprintln!(
                "warning: P1 paths disabled: {error} ({} entries)",
                s.entries.len()
            );
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
            if !blocked
                && ft.is_file()
                && [".ts", ".tsx", ".mts", ".cts"]
                    .iter()
                    .any(|e| name.ends_with(e))
            {
                let patterns = if entry.metadata()?.len() > 16 * 1024 * 1024 {
                    vec!["*".into()]
                } else {
                    std::fs::read_to_string(&p)
                        .map(|s| ambient_patterns(&s))
                        .unwrap_or_else(|_| vec!["*".into()])
                };
                if !patterns.is_empty() {
                    self.ambient.insert(rel.to_owned(), patterns);
                }
            }
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
                self.record_config(key, bytes);
            }
            if ft.is_dir() && !blocked {
                self.walk(root, &p)?;
            }
        }
        Ok(())
    }
    fn record_config(&mut self, key: String, bytes: Option<Vec<u8>>) {
        self.configs
            .entry(key)
            .and_modify(|prior| *prior = None)
            .or_insert(bytes);
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
        let dependencies: BTreeMap<_, _> = self
            .probes
            .lock()
            .unwrap()
            .iter()
            .map(|p| (p.clone(), self.entries.get(p).copied()))
            .collect();
        let bytes = bincode::serialize(&(
            &occupancy,
            &dependencies,
            &self.package_hashes,
            &self.ambient,
            &*self.packages.lock().unwrap(),
            self.complete,
        ))
        .expect("snapshot serializes");
        out.insert(
            "js_paths_occupancy".into(),
            format!("{:x}", Sha256::digest(bytes)),
        );
        out
    }
    pub(crate) fn kind(&self, p: &str) -> Option<u8> {
        self.probes.lock().unwrap().insert(p.into());
        self.entries.get(p).copied()
    }
    // These are no-follow occupancy probes, including ancestors above the repo.
    // Loader primes them before freezing topology; graph builds reuse the facts.
    pub(crate) fn package_present(&self, file: &str, spec: &str) -> bool {
        let parts: Vec<_> = spec.split('/').collect();
        let (package, types) =
            if spec.starts_with('@') && parts.get(1).is_some_and(|name| !name.is_empty()) {
                let name = parts[1];
                (
                    format!("{}/{name}", parts[0]),
                    format!("{}__{name}", &parts[0][1..]),
                )
            } else {
                (parts[0].to_owned(), parts[0].to_owned())
            };
        let caller = self.root.join(file);
        for ancestor in caller.parent().into_iter().flat_map(Path::ancestors) {
            for name in [&package, &format!("@types/{types}")] {
                let candidate = ancestor.join("node_modules").join(name);
                let key = candidate.to_string_lossy().into_owned();
                let cached = self.packages.lock().unwrap().get(&key).copied();
                let kind = cached.unwrap_or_else(|| {
                    let mut kind = 3; // absent
                    let mut prefix = ancestor.to_path_buf();
                    for component in candidate.strip_prefix(ancestor).unwrap().components() {
                        prefix.push(component);
                        match std::fs::symlink_metadata(&prefix) {
                            Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
                                kind = 2;
                                break;
                            }
                            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                                kind = 2;
                                break;
                            }
                            _ => {}
                        }
                    }
                    if kind == 3 {
                        kind = match std::fs::symlink_metadata(&candidate) {
                            Ok(_) => 0,
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 3,
                            Err(_) => 2,
                        };
                    }
                    self.packages.lock().unwrap().insert(key, kind);
                    kind
                });
                if kind != 3 {
                    return true;
                }
            }
        }
        false
    }
    pub(crate) fn unblocked(&self, p: &str) -> bool {
        let mut prefix = String::new();
        for part in p.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if self.kind(&prefix) == Some(2) {
                return false;
            }
        }
        true
    }
}

// Over-approximate ambient declarations: comments/strings may only cost yield.
// Escaped or multi-wildcard names are opaque and barrier every alias.
fn ambient_patterns(source: &str) -> Vec<String> {
    fn trivia(mut s: &str) -> &str {
        loop {
            s = s.trim_start();
            if let Some(rest) = s.strip_prefix("/*") {
                let Some((_, tail)) = rest.split_once("*/") else {
                    return "";
                };
                s = tail;
            } else if let Some(rest) = s.strip_prefix("//") {
                s = rest.split_once('\n').map_or("", |(_, tail)| tail);
            } else {
                return s;
            }
        }
    }
    let mut patterns = Vec::new();
    for (start, _) in source.match_indices("declare") {
        if start > 0
            && source[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$')
        {
            continue;
        }
        let rest = &source[start + 7..];
        let rest = trivia(rest);
        let Some(rest) = rest.strip_prefix("module") else {
            continue;
        };
        let rest = trivia(rest);
        let Some(quote @ ('\'' | '"')) = rest.chars().next() else {
            continue;
        };
        let body = &rest[1..];
        let Some(end) = body.find(quote) else {
            patterns.push("*".into());
            continue;
        };
        let name = &body[..end];
        if name.contains('\\') || name.matches('*').count() > 1 {
            patterns.push("*".into());
        } else {
            patterns.push(name.into());
        }
    }
    patterns
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_case_collision_is_order_independent_barrier() {
        for variants in [[Some(b"{}".to_vec()), None], [None, Some(b"{}".to_vec())]] {
            let mut snapshot = JsPathsSnapshot::default();
            for bytes in variants {
                snapshot.record_config("tsconfig.json".into(), bytes);
            }
            assert_eq!(snapshot.configs["tsconfig.json"], None);
        }
    }
}
