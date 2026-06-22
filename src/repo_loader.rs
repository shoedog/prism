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
    // Parse on the large-stack pool: ParsedFile::parse walks the AST recursively
    // (error-node counting), which overflows a default ~2 MiB rayon worker on
    // deeply-nested files (e.g. a #if-split 8192-element C initializer). See
    // crate::build_pool.
    crate::build_pool::build_pool()
        .install(|| candidates.into_par_iter().map(parse_candidate).collect())
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
    let complete = has_complete_rust_coverage(root, files);
    ScopeGraphBuildInputs {
        repo_root: root.to_path_buf(),
        all_file_paths: files.keys().cloned().collect(),
        manifest_hashes,
        cfg,
        complete,
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

fn has_complete_rust_coverage(root: &Path, files: &BTreeMap<String, ParsedFile>) -> bool {
    let Some(expected) = collect_supported_source_paths(root) else {
        return false;
    };
    let is_rust = |p: &String| Language::from_path(p) == Some(Language::Rust);
    let expected_rust: BTreeSet<String> = expected.into_iter().filter(is_rust).collect();
    let actual_rust: BTreeSet<String> = files.keys().filter(|k| is_rust(k)).cloned().collect();
    actual_rust == expected_rust
}

fn collect_supported_source_paths(root: &Path) -> Option<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    collect_supported_source_paths_inner(root, root, &mut out).ok()?;
    Some(out)
}

fn collect_supported_source_paths_inner(
    root: &Path,
    dir: &Path,
    out: &mut BTreeSet<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type()?;

        if BUILTIN_SKIP_DIRS.contains(&name.as_str())
            && (file_type.is_dir() || file_type.is_symlink())
        {
            continue;
        }
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if !name.starts_with('.') {
                collect_supported_source_paths_inner(root, &path, out)?;
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relp = rel(root, &path);
        if Language::from_path(&relp).is_none() {
            continue;
        }
        if entry.metadata()?.len() <= MAX_FILE_BYTES {
            out.insert(relp);
        }
    }
    Ok(())
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
    let mut editions_seen: BTreeSet<u16> = BTreeSet::new();

    // Collect EVERY discovered `[workspace.package] edition` (not just a last-wins
    // scalar): prism collects all `Cargo.toml` repo-wide into one `manifest_hashes`,
    // so a repo may hold multiple workspace roots on opposite anchoring sides. The
    // full set drives the recall-safe second uniformity term (§2.2); a representative
    // scalar resolves the `{ workspace = true }` value form.
    let mut workspace_editions: BTreeSet<u16> = BTreeSet::new();
    let mut workspace_edition: Option<u16> = None;
    for manifest_path in manifest_hashes.keys() {
        let abs = root.join(manifest_path);
        let Ok(text) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        if let Some(ed) = value
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("edition"))
            .and_then(|e| e.as_str())
            .and_then(parse_edition)
        {
            workspace_editions.insert(ed);
            workspace_edition = Some(ed);
        }
    }

    for manifest_path in manifest_hashes.keys() {
        let abs = root.join(manifest_path);
        let Ok(text) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        parsed_any = true;
        let manifest_dir = manifest_path
            .strip_suffix("Cargo.toml")
            .unwrap_or("")
            .trim_end_matches('/');

        if value.get("package").is_some() {
            // Cargo default: a `[package]` with no `edition` key is edition 2015.
            let pkg_ed = value.get("package").and_then(|p| p.get("edition"));
            let edition = pkg_ed
                .and_then(|e| e.as_str())
                .and_then(parse_edition)
                .or_else(|| {
                    // `edition = { workspace = true }` -> the workspace root edition.
                    if pkg_ed
                        .and_then(|e| e.get("workspace"))
                        .and_then(|w| w.as_bool())
                        .unwrap_or(false)
                    {
                        workspace_edition
                    } else {
                        None
                    }
                })
                .unwrap_or(2015);
            cfg.edition = edition;
            editions_seen.insert(edition);
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
    cfg.edition_uniform =
        anchoring_class_uniform(&editions_seen) && anchoring_class_uniform(&workspace_editions);
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

/// True iff every observed edition is on the same side of the 2015/2018 path-
/// anchoring boundary (`RustPolicy::is_2018_plus`), i.e. all >= 2018 or all < 2018.
/// An empty set is vacuously uniform (matches the prior `len() <= 1` for empty). This
/// is the recall-safety floor for the disproof: 2018/2021/2024 anchor identically, so
/// a same-side workspace is authoritative, but a 2015/2018+ mix is not (keep-all).
fn anchoring_class_uniform(editions: &BTreeSet<u16>) -> bool {
    editions.iter().all(|&e| e >= 2018) || editions.iter().all(|&e| e < 2018)
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
    fn mixed_edition_workspace_is_not_uniform() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2015\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.cfg.edition_uniform,
            "a 2015 + 2021 workspace must record edition_uniform == false"
        );
    }

    #[test]
    fn omitted_plus_explicit_edition_workspace_is_not_uniform() {
        // Faithfulness to Cargo's default: crate `a` OMITS `edition` (⇒ 2015) and
        // crate `b` sets `edition = "2021"`. The resolved edition set is
        // {2015, 2021} ⇒ genuinely mixed ⇒ edition_uniform == false ⇒ the
        // ScopeResolution predicate keeps-all (recall-safe, P1). If omitted were
        // treated as "no edition" the set would be {2021} and this would WRONGLY
        // read uniform, enabling an unsound prune on a mixed-edition repo.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\n", // edition omitted ⇒ Cargo default 2015
        )
        .unwrap();
        std::fs::write(
            p.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.cfg.edition_uniform,
            "omitted (2015) + explicit 2021 must record edition_uniform == false"
        );
    }

    #[test]
    fn single_edition_workspace_is_uniform() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            inputs.cfg.edition_uniform,
            "one manifest is trivially uniform"
        );
    }

    #[test]
    fn workspace_true_inheritance_resolves_to_workspace_edition() {
        // `a` inherits `edition = { workspace = true }`; the workspace root sets 2024.
        // Pre-fix this mis-parses to 2015 (table -> `.as_str()` None -> unwrap_or(2015));
        // post-fix it resolves to 2024, so a single-edition workspace is uniform.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n[workspace.package]\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert_eq!(
            inputs.cfg.edition, 2024,
            "`workspace = true` must inherit the workspace edition 2024, not fall back to 2015"
        );
        assert!(
            inputs.cfg.edition_uniform,
            "one resolved edition is uniform"
        );
    }

    #[test]
    fn pure_2018plus_mixed_workspace_is_uniform() {
        // Two crates on different but same-anchoring-class editions (2021 + 2024).
        // Pre-fix: `editions_seen.len() == 2` -> not uniform. Post-fix: both >= 2018 ->
        // anchoring-class uniform -> the disproof is permitted to run.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            inputs.cfg.edition_uniform,
            "a pure-2018+ workspace ({{2021, 2024}}) is anchoring-class uniform"
        );
    }

    #[test]
    fn multi_workspace_spanning_boundary_is_not_uniform() {
        // prism collects ALL Cargo.toml repo-wide into one manifest set. Two workspace
        // roots on opposite anchoring sides (ws1: 2015, ws2: 2024) must force
        // edition_uniform == false via the `workspace_editions` SET term -- even though a
        // last-wins representative could mis-resolve ws1's inheriting crate to 2024 and
        // make `editions_seen` look all-2018+. Recall-safety (P1).
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("ws1/a/src")).unwrap();
        std::fs::create_dir_all(p.join("ws2/b/src")).unwrap();
        std::fs::write(
            p.join("ws1/Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n[workspace.package]\nedition = \"2015\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws1/a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(p.join("ws1/a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(
            p.join("ws2/Cargo.toml"),
            "[workspace]\nmembers = [\"b\"]\n[workspace.package]\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws2/b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(p.join("ws2/b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.cfg.edition_uniform,
            "workspace editions spanning the 2015/2018 boundary must keep edition_uniform == false"
        );
    }

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

    #[test]
    fn rust_coverage_ignores_non_utf8_python() {
        // A repo whose ONLY skipped file is a non-UTF-8 `.py` (ruff's lint
        // fixtures) must still build a complete scope graph: Rust coverage is
        // total, so `complete == true` and the graph is populated.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("bad.py"), [0xFFu8, 0xFE, 0x00]).unwrap(); // NotUtf8, skipped
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            inputs.complete,
            "a non-UTF-8 .py must not block Rust-scoped completeness"
        );
    }

    #[test]
    fn rust_coverage_false_when_a_rust_file_is_skipped() {
        // If the repo skips one of its OWN `.rs`, Rust coverage is incomplete →
        // `complete == false` (unchanged behavior; the deferred per-crate case).
        // We trigger the skip with a NON-UTF-8 `.rs` — the `String::from_utf8`
        // arm in `walk` (SkipReason::NotUtf8, repo_loader.rs:511) drops `a.rs`
        // before it becomes a parse candidate, so it is absent from `files`. The
        // bytes `[0xFF, 0xFE, 0xFC]` are invalid UTF-8 (no valid sequence begins
        // with 0xFF/0xFE). `a.rs` is still supported-by-path (Language::from_path
        // maps `.rs` → Rust), so `collect_supported_source_paths` counts it in the
        // EXPECTED Rust set while it is missing from the ACTUAL set ⇒ expected !=
        // actual ⇒ complete == false. This avoids an impractical > 2 MiB
        // oversized-file fixture (MAX_FILE_BYTES, repo_loader.rs:12).
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("a.rs"), [0xFFu8, 0xFE, 0xFC]).unwrap(); // NotUtf8, skipped
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.complete,
            "skipping an own .rs (non-UTF-8) must keep completeness false"
        );
    }
}
