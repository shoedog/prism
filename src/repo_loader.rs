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

    // Per-workspace-root `[workspace.dependencies]` pre-scan (BLOCKER 2): a repo can
    // hold MULTIPLE workspace roots, so a global name->target map would cross-resolve
    // a same-named workspace dep into the wrong workspace. Key the map by the
    // workspace-root dir (the dir that declared `[workspace]`), and record the set of
    // workspace-root dirs so each member resolves through its OWNING (nearest-ancestor)
    // workspace. Only entries carrying a `path` (the in-repo workspace-dep targets) are
    // recorded; targets are normalized (`..`/`.` collapsed) to the manifest-dir form.
    let mut workspace_dep_paths: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut workspace_root_dirs: BTreeSet<String> = BTreeSet::new();
    for manifest_path in manifest_hashes.keys() {
        let abs = root.join(manifest_path);
        let Ok(text) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        let manifest_dir = manifest_path
            .strip_suffix("Cargo.toml")
            .unwrap_or("")
            .trim_end_matches('/');
        // A manifest that declares `[workspace]` is a workspace root (members and/or
        // `[workspace.dependencies]` hang off it).
        let ws = value.get("workspace");
        if ws.is_some() {
            workspace_root_dirs.insert(manifest_dir.to_string());
        }
        if let Some(ws_deps) = ws
            .and_then(|w| w.get("dependencies"))
            .and_then(|d| d.as_table())
        {
            let entry = workspace_dep_paths
                .entry(manifest_dir.to_string())
                .or_default();
            for (name, spec) in ws_deps {
                if let Some(path) = spec.get("path").and_then(|p| p.as_str()) {
                    // Skip an out-of-repo workspace-dep target (a `..` that escapes the
                    // repo root): `normalize_repo_rel` returns `None` (re-review BLOCKER A).
                    if let Some(target) = normalize_repo_rel(&join_manifest_rel(manifest_dir, path))
                    {
                        entry.insert(name.clone(), target);
                    }
                }
            }
        }
    }
    let mut member_in_repo_deps: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

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

        // Every `[package]` manifest's dir IS a concrete workspace-member dir. Record
        // those (NOT the `[workspace].members` patterns) so the member-dir derivations
        // that prefix-match `workspace_members` (`lib_root_member_dir`,
        // `crate_name_for_root`) work on GLOB workspaces like ruff
        // (`members = ["crates/*"]`), where a raw `crates/*` pattern never prefix-matches
        // a concrete root path (`crates/ruff_db/src/lib.rs`). prism has already walked
        // every member's `Cargo.toml` into `manifest_hashes`, so their dirs ARE the
        // expanded member set; the declared patterns are redundant for this purpose.
        if value.get("package").is_some() && !manifest_dir.is_empty() {
            workspace_members.insert(manifest_dir.to_string());
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
        // Resolve this member's `workspace = true` deps through its OWNING workspace
        // root's `[workspace.dependencies]` (nearest ancestor dir that declared
        // `[workspace]`); an empty map when the member is not under any workspace root.
        let owning_ws = owning_workspace_root(manifest_dir, &workspace_root_dirs);
        let empty_ws_deps = BTreeMap::new();
        let ws_deps_for_member = owning_ws
            .and_then(|ws| workspace_dep_paths.get(ws))
            .unwrap_or(&empty_ws_deps);
        let member_deps = parse_member_in_repo_deps(&value, manifest_dir, ws_deps_for_member);
        if !member_deps.is_empty() {
            member_in_repo_deps.insert(manifest_dir.to_string(), member_deps);
        }
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
    cfg.member_in_repo_deps = member_in_repo_deps;
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

/// Lexically normalize a repo-relative path to the manifest-dir spelling (BLOCKER 1),
/// returning `None` when a `..` escapes the repo root (re-review BLOCKER A).
/// PURE STRING (no filesystem): split on `/`, drop `.` and empty segments, pop the
/// previous segment on `..`. A `..` with nothing left to pop means the path points
/// OUT of the repo (e.g. `../b`, `a/../../b`) → `None`, so the caller skips it (an
/// out-of-repo path dep is not an in-repo member). Produces no trailing slash and no
/// `.`/`..` components, so a `path = "../b"` dep from a non-root member `a/` (`join`
/// → `a/../b`) collapses to `Some("b")`, the SAME key the Builder uses for
/// `lib_root_by_member_dir`. `join_manifest_rel` alone keeps the `..` and would miss
/// the index. (`Some("")` is the repo root itself — a valid in-repo single-crate dir.)
fn normalize_repo_rel(path: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                // Pop the parent; if there is nothing to pop, the `..` escapes the
                // repo root → out-of-repo → decline (do not clamp to an in-repo dir).
                if out.pop().is_none() {
                    return None;
                }
            }
            s => out.push(s),
        }
    }
    Some(out.join("/"))
}

/// The OWNING workspace root for `manifest_dir` (BLOCKER 2): the LONGEST workspace-root
/// dir that is an ancestor of (or equal to) `manifest_dir`. `""` (the repo root) is an
/// ancestor of everything, so a single top-level workspace owns all members. Returns
/// `None` when the member is under no workspace root (a non-workspace single crate).
fn owning_workspace_root<'a>(
    manifest_dir: &str,
    workspace_root_dirs: &'a BTreeSet<String>,
) -> Option<&'a str> {
    workspace_root_dirs
        .iter()
        .filter(|ws| is_dir_ancestor(ws, manifest_dir))
        .map(String::as_str)
        // Longest matching ancestor = the nearest (innermost) owning workspace.
        .max_by_key(|ws| ws.len())
}

/// True iff `ancestor` is a directory-prefix of (or equal to) `dir`, both repo-relative
/// with no trailing slash. `""` (repo root) is an ancestor of everything. Avoids the
/// `"a"` ⊂ `"ab"` false match by requiring a `/` boundary.
fn is_dir_ancestor(ancestor: &str, dir: &str) -> bool {
    if ancestor.is_empty() || ancestor == dir {
        return true;
    }
    dir.strip_prefix(ancestor)
        .map(|rest| rest.starts_with('/'))
        .unwrap_or(false)
}

/// Capture one member manifest's in-repo `[dependencies]` (PATH + WORKSPACE forms)
/// as `in_source_name → target member dir`. External/version-only/git/registry deps
/// are skipped (they resolve to no in-repo path). `manifest_dir` is the member's
/// directory (no trailing slash, "" for the repo root); `workspace_dep_paths` maps a
/// `[workspace.dependencies]` name to its resolved (normalized) target dir, scoped to
/// THIS member's owning workspace (for `workspace = true`).
fn parse_member_in_repo_deps(
    value: &toml::Value,
    manifest_dir: &str,
    workspace_dep_paths: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(deps) = value.get("dependencies").and_then(|d| d.as_table()) else {
        return out;
    };
    for (in_source_name, spec) in deps {
        // A bare `dep = "1.0"` string spec is version-only / external → skip.
        let Some(table) = spec.as_table() else {
            continue;
        };
        if let Some(path) = table.get("path").and_then(|p| p.as_str()) {
            // PATH dep: target dir is the member dir joined with the relative path,
            // normalized (`..`/`.` collapsed) to the manifest-dir spelling (BLOCKER 1).
            // `normalize_repo_rel` returns `None` if a `..` escapes the repo root — an
            // out-of-repo dep, so skip it (do not record) (re-review BLOCKER A).
            if let Some(target) = normalize_repo_rel(&join_manifest_rel(manifest_dir, path)) {
                out.insert(in_source_name.clone(), target);
            }
        } else if table
            .get("workspace")
            .and_then(|w| w.as_bool())
            .unwrap_or(false)
        {
            // WORKSPACE dep: resolve through the owning workspace's
            // `[workspace.dependencies][name].path` (already normalized at pre-scan).
            if let Some(target_dir) = workspace_dep_paths.get(in_source_name) {
                out.insert(in_source_name.clone(), target_dir.clone());
            }
            // No `[workspace.dependencies]` path entry → external → skip.
        }
        // else (version/git/registry table, no path, not workspace) → external → skip.
    }
    out
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
    fn path_dep_records_in_repo_member_dependency() {
        // `a` declares `b_crate = { path = "../b" }`; `b` is an in-repo member.
        // member_in_repo_deps must map a's member dir -> (b_crate -> b's dir).
        // This is ALSO the BLOCKER-1 normalization case: `join("a", "../b")` is
        // `a/../b` lexically, but the recorded target must be the normalized `b`
        // (the same spelling `lib_root_by_member_dir` keys by) -- `normalize_repo_rel`
        // pops the `..`. A non-normalizing `join_manifest_rel` would record `a/../b`
        // and the Builder lookup in Task 3 would miss.
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
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb_crate = { path = \"../b\" }\n",
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
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("a")
                .and_then(|m| m.get("b_crate"))
                .map(String::as_str),
            Some("b"),
            "a path dep on ../b must record (a -> b_crate -> b)"
        );
    }

    #[test]
    fn glob_workspace_members_expand_to_concrete_dirs() {
        // A GLOB workspace (`members = ["crates/*"]`, like ruff). `workspace_members`
        // must hold the CONCRETE member dirs prism parsed (`crates/a`, `crates/b`),
        // NOT the raw `crates/*` pattern — otherwise `lib_root_member_dir`'s prefix
        // match never hits a real root and `crate_deps_by_root` ends up empty (the
        // ruff +0 bug). The per-member dep capture must also work on this layout.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("crates/a/src")).unwrap();
        std::fs::create_dir_all(p.join("crates/b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n[workspace.package]\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("crates/a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb = { path = \"../b\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("crates/b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("crates/a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("crates/b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        // CONCRETE member dirs, never the glob pattern.
        assert!(
            inputs.cfg.workspace_members.contains(&"crates/a".to_string())
                && inputs.cfg.workspace_members.contains(&"crates/b".to_string()),
            "glob members must expand to concrete dirs; got {:?}",
            inputs.cfg.workspace_members
        );
        assert!(
            !inputs.cfg.workspace_members.iter().any(|m| m.contains('*')),
            "no raw glob pattern may remain in workspace_members; got {:?}",
            inputs.cfg.workspace_members
        );
        // And the dep capture works on the glob layout (a -> b via ../b).
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("crates/a")
                .and_then(|m| m.get("b"))
                .map(String::as_str),
            Some("crates/b"),
            "a's path dep on ../b must record (crates/a -> b -> crates/b)"
        );
    }

    #[test]
    fn workspace_dep_records_in_repo_member_dependency() {
        // `a` declares `b_crate = { workspace = true }`; the workspace root has
        // `[workspace.dependencies] b_crate = { path = "b" }`. The ruff-heavy form.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n[workspace.dependencies]\nb_crate = { path = \"b\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb_crate = { workspace = true }\n",
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
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("a")
                .and_then(|m| m.get("b_crate"))
                .map(String::as_str),
            Some("b"),
            "a workspace dep resolving to ../b via [workspace.dependencies] must record (a -> b_crate -> b)"
        );
    }

    #[test]
    fn external_version_dep_is_not_recorded() {
        // `a` depends on an EXTERNAL `serde = "1.0"` (version-only, no path). It
        // must NOT enter member_in_repo_deps (no in-repo target).
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::write(p.join("Cargo.toml"), "[workspace]\nmembers = [\"a\"]\n").unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("a")
                .map(|m| !m.contains_key("serde"))
                .unwrap_or(true),
            "a version-only external dep must not be recorded as an in-repo dep"
        );
    }

    #[test]
    fn normalize_repo_rel_pops_parent_and_drops_dot() {
        // BLOCKER 1: the lexical repo-relative normalizer is pure string (no fs).
        // It pops `..`, drops `.`, and yields no trailing slash so the result matches
        // the manifest-dir form `lib_root_by_member_dir` keys by. In-repo cases return
        // `Some(dir)`.
        assert_eq!(super::normalize_repo_rel("a/../b").as_deref(), Some("b"));
        assert_eq!(
            super::normalize_repo_rel("crates/a/../b").as_deref(),
            Some("crates/b")
        );
        assert_eq!(super::normalize_repo_rel("./b").as_deref(), Some("b"));
        assert_eq!(super::normalize_repo_rel("a/./b/").as_deref(), Some("a/b"));
        assert_eq!(super::normalize_repo_rel("b").as_deref(), Some("b"));
        assert_eq!(super::normalize_repo_rel("").as_deref(), Some(""));
    }

    #[test]
    fn normalize_repo_rel_returns_none_when_escaping_repo_root() {
        // Re-review BLOCKER A: a `..` that pops past the repo root is an out-of-repo
        // path dep, NOT an in-repo member. The normalizer must return `None` (escaped)
        // so the caller skips it — never silently clamp `../b` -> `b` or
        // `a/../../b` -> `b`.
        assert_eq!(super::normalize_repo_rel("../b"), None);
        assert_eq!(super::normalize_repo_rel("a/../../b"), None);
        assert_eq!(super::normalize_repo_rel("../../external/b"), None);
        // A `..` that nets back inside is fine (pops `a`, lands on `b` in-repo).
        assert_eq!(super::normalize_repo_rel("a/../b").as_deref(), Some("b"));
    }

    #[test]
    fn path_dep_escaping_repo_root_is_not_recorded() {
        // Re-review BLOCKER A (capture call site): a single crate AT the repo root
        // (manifest_dir == "") declares an OUT-OF-REPO path dep `ext = { path = "../b" }`.
        // `join_manifest_rel("", "../b")` = `../b`, which `normalize_repo_rel` rejects
        // (a `..` underflows the repo root → None), so `ext` must NOT be recorded — the
        // old clamp-to-`b` behavior would have falsely recorded an in-repo target.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[package]\nname = \"root\"\nedition = \"2021\"\n[dependencies]\next = { path = \"../b\" }\n",
        )
        .unwrap();
        std::fs::write(p.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        // The escaping `../b` dep from the root member must NOT be recorded as
        // in-repo (it would have escaped the repo root).
        assert!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("")
                .map(|m| !m.contains_key("ext"))
                .unwrap_or(true),
            "an out-of-repo `../b` path dep from the root member must not be recorded"
        );
    }

    #[test]
    fn multi_workspace_same_dep_name_resolves_per_owning_workspace() {
        // BLOCKER 2: two SEPARATE workspaces (ws1, ws2), each with a member that
        // declares `dep = { workspace = true }` under the SAME in-source name `shared`,
        // but each workspace's `[workspace.dependencies] shared` points at a DISTINCT
        // in-repo crate (ws1 -> ws1/libx, ws2 -> ws2/liby). Each consuming member must
        // resolve `shared` through ITS OWN owning workspace's map, never the other's.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        for sub in ["ws1/a/src", "ws1/libx/src", "ws2/b/src", "ws2/liby/src"] {
            std::fs::create_dir_all(p.join(sub)).unwrap();
        }
        // ws1: member `a` deps `shared = workspace`; ws root maps shared -> libx.
        std::fs::write(
            p.join("ws1/Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"libx\"]\n[workspace.dependencies]\nshared = { path = \"libx\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws1/a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nshared = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws1/libx/Cargo.toml"),
            "[package]\nname = \"libx\"\nedition = \"2021\"\n",
        )
        .unwrap();
        // ws2: member `b` deps `shared = workspace`; ws root maps shared -> liby.
        std::fs::write(
            p.join("ws2/Cargo.toml"),
            "[workspace]\nmembers = [\"b\", \"liby\"]\n[workspace.dependencies]\nshared = { path = \"liby\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws2/b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n[dependencies]\nshared = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws2/liby/Cargo.toml"),
            "[package]\nname = \"liby\"\nedition = \"2021\"\n",
        )
        .unwrap();
        for f in [
            "ws1/a/src/lib.rs",
            "ws1/libx/src/lib.rs",
            "ws2/b/src/lib.rs",
            "ws2/liby/src/lib.rs",
        ] {
            std::fs::write(p.join(f), "pub fn f() {}\n").unwrap();
        }
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        // ws1/a's `shared` must point at ws1/libx (its OWN workspace), not ws2/liby.
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("ws1/a")
                .and_then(|m| m.get("shared"))
                .map(String::as_str),
            Some("ws1/libx"),
            "ws1/a resolves `shared` through ws1's [workspace.dependencies]"
        );
        // ws2/b's `shared` must point at ws2/liby (its OWN workspace), not ws1/libx.
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("ws2/b")
                .and_then(|m| m.get("shared"))
                .map(String::as_str),
            Some("ws2/liby"),
            "ws2/b resolves `shared` through ws2's [workspace.dependencies]"
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
