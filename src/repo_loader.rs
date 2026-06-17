use crate::ast::ParsedFile;
use crate::call_graph::ScopeGraphBuildInputs;
use crate::languages::Language;
use crate::name_resolution::rust_populator::RustCrateConfig;
use crate::type_db::TypeDatabase;
use anyhow::{Context, Result};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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
    pub manifest_hashes: BTreeMap<String, String>,
    pub scope_graph_inputs: Option<ScopeGraphBuildInputs>,
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

    let scope_graph_inputs = scope_graph_build_inputs(root, &files);
    let manifest_hashes = scope_graph_inputs.manifest_hashes.clone();

    LoadedRepo {
        root: root.to_path_buf(),
        files,
        file_hashes,
        manifest_hashes,
        scope_graph_inputs: Some(scope_graph_inputs),
        skipped,
        type_db: None,
    }
}

pub fn scope_graph_build_inputs(
    root: &Path,
    files: &BTreeMap<String, ParsedFile>,
) -> ScopeGraphBuildInputs {
    let manifest_hashes = collect_manifest_hashes(root);
    let cfg = parse_rust_crate_config(root, files, &manifest_hashes)
        .unwrap_or_else(|| RustCrateConfig::from_convention(files));
    ScopeGraphBuildInputs {
        repo_root: root.to_path_buf(),
        all_file_paths: files.keys().cloned().collect(),
        manifest_hashes,
        cfg,
        complete: true,
    }
}

fn collect_manifest_hashes(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    collect_manifest_hashes_inner(root, root, &mut out);
    out
}

fn collect_manifest_hashes_inner(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            if BUILTIN_SKIP_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            collect_manifest_hashes_inner(root, &path, out);
            continue;
        }
        if file_type.is_file() && name == "Cargo.toml" {
            if let Ok(bytes) = std::fs::read(&path) {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                out.insert(rel(root, &path), format!("{:x}", hasher.finalize()));
            }
        }
    }
}

fn parse_rust_crate_config(
    root: &Path,
    files: &BTreeMap<String, ParsedFile>,
    manifest_hashes: &BTreeMap<String, String>,
) -> Option<RustCrateConfig> {
    if manifest_hashes.is_empty() {
        return Some(RustCrateConfig::from_convention(files));
    }

    let mut cfg = RustCrateConfig::from_convention(files);
    let mut crate_roots = BTreeSet::new();
    let mut workspace_members = BTreeSet::new();
    let mut bin_paths = BTreeSet::new();
    let mut parsed_any = false;

    for manifest_path in manifest_hashes.keys() {
        let abs = root.join(manifest_path);
        let text = std::fs::read_to_string(&abs).ok()?;
        let value: toml::Value = text.parse().ok()?;
        parsed_any = true;
        let manifest_dir = manifest_path
            .strip_suffix("Cargo.toml")
            .unwrap_or("")
            .trim_end_matches('/');

        if let Some(edition) = value
            .get("package")
            .and_then(|p| p.get("edition"))
            .and_then(|e| e.as_str())
            .and_then(parse_edition)
        {
            cfg.edition = edition;
        }

        if let Some(members) = value
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
        {
            for member in members.iter().filter_map(|m| m.as_str()) {
                workspace_members.insert(join_manifest_rel(manifest_dir, member));
            }
        }

        if let Some(path) = value
            .get("lib")
            .and_then(|l| l.get("path"))
            .and_then(|p| p.as_str())
        {
            let p = join_manifest_rel(manifest_dir, path);
            crate_roots.insert(p.clone());
            cfg.lib_path = Some(p);
        } else {
            let lib = join_manifest_rel(manifest_dir, "src/lib.rs");
            if files.contains_key(&lib) {
                crate_roots.insert(lib);
            }
        }

        let main = join_manifest_rel(manifest_dir, "src/main.rs");
        if files.contains_key(&main) {
            crate_roots.insert(main);
        }

        if let Some(bins) = value.get("bin").and_then(|b| b.as_array()) {
            for bin in bins {
                if let Some(path) = bin.get("path").and_then(|p| p.as_str()) {
                    let p = join_manifest_rel(manifest_dir, path);
                    crate_roots.insert(p.clone());
                    bin_paths.insert(p);
                }
            }
        }

        collect_dep_renames(&value, &mut cfg.dep_renames);
    }

    if !parsed_any {
        return None;
    }
    crate_roots.extend(cfg.crate_roots);
    cfg.crate_roots = crate_roots.into_iter().collect();
    cfg.workspace_members = workspace_members.into_iter().collect();
    cfg.bin_paths = bin_paths.into_iter().collect();
    Some(cfg)
}

fn parse_edition(raw: &str) -> Option<u16> {
    match raw {
        "2015" => Some(2015),
        "2018" => Some(2018),
        "2021" => Some(2021),
        "2024" => Some(2024),
        _ => None,
    }
}

fn join_manifest_rel(manifest_dir: &str, rel_path: &str) -> String {
    let mut p = if manifest_dir.is_empty() {
        PathBuf::from(rel_path)
    } else {
        PathBuf::from(manifest_dir).join(rel_path)
    };
    if p == PathBuf::from(".") {
        p = PathBuf::new();
    }
    p.to_string_lossy().replace('\\', "/")
}

fn collect_dep_renames(value: &toml::Value, out: &mut BTreeMap<String, String>) {
    for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = value.get(table).and_then(|v| v.as_table()) {
            for (alias, spec) in deps {
                if let Some(package) = spec.get("package").and_then(|p| p.as_str()) {
                    out.insert(alias.clone(), package.to_string());
                }
            }
        }
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
