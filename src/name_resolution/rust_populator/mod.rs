//! The Rust **populator** (Task 3a, PR-2): walks parsed Rust ASTs into a
//! language-neutral [`ScopeGraph`] the PR-1 engine resolves over.
//!
//! **Pure.** `populate_rust` takes the already-parsed files + a
//! [`RustCrateConfig`] (the parsed-manifest-lite input) + an optional
//! `only_files` filter, and returns a `ScopeGraph`. It performs **no file I/O**
//! and reads **no manifest** — Task 3b parses real `Cargo.toml`s into a
//! `RustCrateConfig`; here we accept one (with a convention-fallback
//! constructor) so the build is unit-testable in isolation. Nothing consumes the
//! graph yet (PR-3); the build wiring/cache is 3b.
//!
//! ## What it builds (spec §4)
//! - **Crate graph:** a `Root` scope per crate root (lib/bin/bench/test/example
//!   + workspace members + `[lib]`/`[[bin]]` overrides; convention fallback).
//! - **Module/block scopes:** `mod foo;` resolved via the **declaring module's
//!   directory** (`src/foo.rs` + `mod bar;` → `src/foo/bar.rs`), inline `mod`,
//!   `#[path]`, `#[cfg]`-conditioned; `foo.rs`+`foo/mod.rs` → an **ambiguous**
//!   pair of bindings; block scopes for `{}` that introduce `use`/items.
//! - **Bindings:** item defs → `Binding`s in `{Type, Value, Macro}` (unit/tuple
//!   struct → Type+Value; enum variant → Value); `use a::b [as c]` →
//!   `Binding(Pending)`; `pub use` → same `vis=Public`; **all** local
//!   value/pattern bindings → `Target::Local`; `use a::*` → a **deferred-glob**
//!   poison edge; name-introducing macros → a `MacroWildcard`.
//! - **Fail-open per region:** a missing `mod` file / a parse-failed *target*
//!   module → **Poisoned** (modeled-but-hazardous: a deferred-glob poison edge);
//!   a parse-failed *containing* file → **Unmodeled** (no scopes). Never panics.
//!
//! ## Associated-items recall-safety (the PR-1 review obligation)
//! An `impl`'s associated items (methods / assoc fns / consts) are reachable by
//! **path** (`T::m`) but must NOT be **bare-visible** in the enclosing scope (a
//! bare `m()` needs `Self::`/`self.`). We achieve this purely-additively, with
//! NO engine change: associated items become bindings in the type's **owned
//! `Type` scope**, and each method **body** scope is **re-parented to the
//! enclosing module** (not the impl/type). A bare rib walk from inside a method
//! therefore ascends `Callable → Module` (module-boundary STOP) and never visits
//! the type scope, so it cannot see a sibling associated item; a `resolve_path`
//! `["T","m"]` anchors *into* the type scope and finds it. This is also faithful
//! Rust: free module fns ARE bare-reachable from inside a method, associated
//! items are not. See [`scopes::ModuleCtx`] re-parenting in the walk.

mod builder;
mod scopes;
mod walk;

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::ParsedFile;
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::types::{FileId, ScopeId};

use builder::Builder;

/// `serde(default)` for `RustCrateConfig::edition_uniform` and
/// `ScopeGraph::edition_uniform`: a legacy cache without the field is treated as
/// uniform (the prior single-edition assumption).
pub fn default_edition_uniform() -> bool {
    true
}

/// The error-rate floor at/above which a file is treated as **failed to parse**
/// (matching the codebase's `check_parse_warnings` "skip" convention). A
/// parse-failed *containing* file is left Unmodeled; a parse-failed *target*
/// module is Poisoned.
pub const PARSE_FAIL_RATE: f64 = 0.30;

/// Parsed-manifest-lite crate configuration (the input Task 3b will populate
/// from real `Cargo.toml`s; 3a accepts it so the populator stays pure).
///
/// All paths are repo-relative and match the keys of the `files` map.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RustCrateConfig {
    /// Crate edition (2015/2018/2021/2024). Drives anchor semantics in the
    /// policy; the populator only records it so the caller can build a matching
    /// `RustPolicy`. Default 2015 (Cargo's default when omitted).
    pub edition: u16,
    /// Every crate-root file path (lib/bin/bench/test/example roots, one Root
    /// scope each). Convention fallback fills this when no manifest is given.
    pub crate_roots: Vec<String>,
    /// Workspace member directory prefixes (each member is its own crate). Used
    /// to route `extern crate`/dep names to an in-workspace Root vs `External`.
    pub workspace_members: Vec<String>,
    /// `bar = { package = "foo" }`: in-source name (`bar`) → package name
    /// (`foo`). Reserved for dep-name routing; recorded for 3b.
    pub dep_renames: BTreeMap<String, String>,
    /// `[lib] path = "..."` override (a non-convention library root).
    pub lib_path: Option<String>,
    /// `[[bin]] path = "..."` overrides (non-convention binary roots).
    pub bin_paths: Vec<String>,
    /// Whether every parsed manifest is in the same anchoring class (2015 vs
    /// 2018+). `false` for a 2015/2018+ mixed workspace; the `ScopeResolution`
    /// disproof predicate keeps-all (disproves nothing) when this is false,
    /// because a wrong-edition anchor could mis-resolve a path and drop a real
    /// edge (P1). Convention fallback (single edition) is `true`.
    #[serde(default = "default_edition_uniform")]
    pub edition_uniform: bool,
    /// Rust: per workspace-member directory → (in-source dependency name → the
    /// target in-repo member directory) for that member's `[dependencies]` PATH
    /// and WORKSPACE deps that resolve to an in-repo crate. External/version-only/
    /// git/registry deps are NOT recorded. The Builder turns this into
    /// `ScopeGraph::crate_deps_by_root` at `finish()` (library roots only). The KEY
    /// is the in-source name (what `use` writes); the VALUE is the target member dir
    /// (resolved by path), so a renamed in-repo dep is handled naturally. Convention
    /// fallback (no manifest) is empty.
    #[serde(default)]
    pub member_in_repo_deps: BTreeMap<String, BTreeMap<String, String>>,
    /// Manifest-backed binary root -> (own library crate name, library root).
    /// Exact per-package paths avoid the legacy flattened `lib_path` index.
    /// This contributes only to the existing 2018+ extern-prelude lane.
    #[serde(default)]
    pub binary_library_deps: BTreeMap<String, (String, String)>,
}

impl Default for RustCrateConfig {
    fn default() -> Self {
        RustCrateConfig {
            edition: 2015,
            crate_roots: Vec::new(),
            workspace_members: Vec::new(),
            dep_renames: BTreeMap::new(),
            lib_path: None,
            bin_paths: Vec::new(),
            edition_uniform: true,
            member_in_repo_deps: BTreeMap::new(),
            binary_library_deps: BTreeMap::new(),
        }
    }
}

impl RustCrateConfig {
    /// Convention-fallback config (no manifest): **edition 2015** and the
    /// conventional crate roots discovered from the file set:
    /// `src/lib.rs`, `src/main.rs`, `src/bin/*.rs`, `src/bin/<n>/main.rs`,
    /// `benches/*.rs`, `tests/*.rs`, `examples/*.rs` (and the same under any
    /// directory, so workspace members are covered when their files are present).
    pub fn from_convention(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut crate_roots: Vec<String> = files
            .keys()
            .filter(|p| is_convention_root(p))
            .cloned()
            .collect();
        crate_roots.sort();
        crate_roots.dedup();
        // Standalone fallback: a SINGLE-file non-Cargo repo (one top-level
        // `main.rs`/`lib.rs`, or one ad-hoc `.rs`) has no conventional root and
        // would otherwise yield a complete-but-empty scope graph. Root that one
        // file. We deliberately do NOT pick a root in a MULTI-file non-Cargo repo:
        // without a manifest we can't distinguish crate roots from `mod`-reached
        // module files, and rooting one would orphan its un-`mod`-wired siblings
        // (regressing cross-file name resolution). Multi-file non-Cargo repos keep
        // the prior whole-file-set fallback. (mod-wiring inference for those is a
        // deferred follow-on.)
        if crate_roots.is_empty() {
            let rust_files: Vec<&String> = files.keys().filter(|p| p.ends_with(".rs")).collect();
            if rust_files.len() == 1 {
                crate_roots.push(rust_files[0].clone());
            }
        }
        RustCrateConfig {
            edition: 2015,
            crate_roots,
            workspace_members: Vec::new(),
            dep_renames: BTreeMap::new(),
            lib_path: None,
            bin_paths: Vec::new(),
            edition_uniform: true,
            member_in_repo_deps: BTreeMap::new(),
            binary_library_deps: BTreeMap::new(),
        }
    }

    /// The full set of crate-root paths (explicit roots + `lib_path` + `bin_paths`),
    /// de-duplicated and sorted for determinism.
    fn all_roots(&self) -> Vec<String> {
        let mut roots: BTreeSet<String> = self.crate_roots.iter().cloned().collect();
        if let Some(l) = &self.lib_path {
            roots.insert(l.clone());
        }
        for b in &self.bin_paths {
            roots.insert(b.clone());
        }
        roots.into_iter().collect()
    }
}

/// Is `path` a conventional Rust crate root? (Keys use forward slashes.)
fn is_convention_root(p: &str) -> bool {
    if !p.ends_with(".rs") {
        return false;
    }
    // Anchor on a recognizable directory segment so nested workspace members
    // (e.g. `member/src/lib.rs`) are also discovered.
    let ends_with_seg = |seg: &str| p == seg || p.ends_with(&format!("/{seg}"));
    if ends_with_seg("src/lib.rs") || ends_with_seg("src/main.rs") {
        return true;
    }
    // src/bin/<name>.rs  OR  src/bin/<name>/main.rs
    if let Some(rest) = segment_after(p, "src/bin/") {
        return rest == "main.rs" || (rest.ends_with("/main.rs")) || !rest.contains('/');
    }
    // benches/<name>.rs, tests/<name>.rs, examples/<name>.rs (single level).
    for dir in ["benches/", "tests/", "examples/"] {
        if let Some(rest) = segment_after(p, dir) {
            if !rest.contains('/') && rest.ends_with(".rs") {
                return true;
            }
        }
    }
    false
}

/// The substring after the LAST occurrence of `needle` directory marker, if
/// `path` contains it as a path prefix segment.
fn segment_after<'a>(path: &'a str, needle: &str) -> Option<&'a str> {
    if let Some(idx) = path.rfind(needle) {
        // Ensure `needle` starts at a path boundary (start or after a '/').
        if idx == 0 || path.as_bytes()[idx - 1] == b'/' {
            return Some(&path[idx + needle.len()..]);
        }
    }
    None
}

/// Deterministic `FileId` for `path`: its index in the **sorted key order** of
/// `files`. Returns `None` if `path` is not in the map. The populator uses this
/// exact mapping, so consumers/tests can reproduce it.
pub fn file_id(files: &BTreeMap<String, ParsedFile>, path: &str) -> Option<FileId> {
    files
        .keys()
        .position(|k| k == path)
        .map(|i| FileId(i as u32))
}

/// The path for a `FileId` (the inverse of [`file_id`]).
pub fn path_for_file(files: &BTreeMap<String, ParsedFile>, id: FileId) -> Option<&str> {
    files.keys().nth(id.0 as usize).map(|s| s.as_str())
}

/// The **smallest** scope whose extent covers `(file, byte)` — the consumer's
/// "from which scope do I resolve this site" primitive (and the test hook).
///
/// "Smallest" = the scope with the narrowest covering extent in that file; ties
/// break toward the **deepest** (largest `ScopeId`, since children are minted
/// after parents). Returns `None` when no scope covers the byte (e.g. an
/// Unmodeled file).
pub fn enclosing_scope(graph: &ScopeGraph, file: FileId, byte: usize) -> Option<ScopeId> {
    let mut best: Option<(ScopeId, usize)> = None; // (id, extent width)
    for (id, scope) in &graph.scopes {
        for ext in &scope.extents {
            if ext.file != file {
                continue;
            }
            let (lo, hi) = (ext.range.lo.byte, ext.range.hi.byte);
            if byte >= lo && byte < hi {
                let width = hi - lo;
                let take = match best {
                    None => true,
                    Some((bid, bw)) => width < bw || (width == bw && *id > bid),
                };
                if take {
                    best = Some((*id, width));
                }
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Build a [`ScopeGraph`] from parsed Rust `files` under `config`.
///
/// `only_files`, when `Some`, restricts which **containing** files are walked
/// into scopes (the changed-file set the build wiring will pass); files outside
/// the set are left Unmodeled. The crate-root *shape* is always computed from
/// `config` so cross-file `mod`/path resolution is stable.
pub fn populate_rust(
    files: &BTreeMap<String, ParsedFile>,
    config: &RustCrateConfig,
    only_files: Option<&BTreeSet<String>>,
) -> ScopeGraph {
    let mut b = Builder::new(files, config, only_files);
    let roots = config.all_roots();
    // Pass 1: mint every crate Root first, so `extern crate dep` (and bare-crate
    // path roots) can reach a crate whose root sorts later.
    let mut to_walk: Vec<(String, ScopeId)> = Vec::new();
    for root in &roots {
        if let Some(scope) = b.create_root(root) {
            to_walk.push((root.clone(), scope));
        }
    }
    // Pass 2: walk each root's items (which recursively pulls in `mod` files).
    for (root, scope) in &to_walk {
        b.walk_root(root, *scope);
    }
    let mut graph = b.finish();
    graph.edition = config.edition;
    graph.edition_uniform = config.edition_uniform;
    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    #[test]
    fn from_convention_edition_is_uniform_by_default() {
        let mut files = BTreeMap::new();
        files.insert(
            "main.rs".to_string(),
            ParsedFile::parse("main.rs", "fn main() {}\n", Language::Rust).unwrap(),
        );
        let cfg = RustCrateConfig::from_convention(&files);
        assert!(
            cfg.edition_uniform,
            "convention fallback is single-edition → uniform"
        );
    }

    #[test]
    fn populate_rust_propagates_edition_uniform_flag() {
        let mut files = BTreeMap::new();
        files.insert(
            "src/lib.rs".to_string(),
            ParsedFile::parse("src/lib.rs", "pub fn a() {}\n", Language::Rust).unwrap(),
        );
        let mut cfg = RustCrateConfig::from_convention(&files);
        cfg.crate_roots = vec!["src/lib.rs".to_string()];
        cfg.edition_uniform = false;
        let graph = populate_rust(&files, &cfg, None);
        assert!(
            !graph.edition_uniform,
            "the flag must ride onto the ScopeGraph"
        );
    }

    #[test]
    fn from_convention_roots_top_level_main_rs() {
        let mut files = BTreeMap::new();
        files.insert(
            "main.rs".to_string(),
            ParsedFile::parse("main.rs", "fn main() {}\n", Language::Rust).unwrap(),
        );
        let cfg = RustCrateConfig::from_convention(&files);
        assert_eq!(
            cfg.crate_roots,
            vec!["main.rs".to_string()],
            "a top-level main.rs must be rooted (standalone fallback), not left empty"
        );
    }

    #[test]
    fn from_convention_roots_single_ad_hoc_rs() {
        let mut files = BTreeMap::new();
        files.insert(
            "scratch.rs".to_string(),
            ParsedFile::parse("scratch.rs", "pub struct A;\n", Language::Rust).unwrap(),
        );
        let cfg = RustCrateConfig::from_convention(&files);
        assert_eq!(cfg.crate_roots, vec!["scratch.rs".to_string()]);
    }

    #[test]
    fn from_convention_does_not_root_multifile_nonconventional() {
        // Two ad-hoc files, no top-level main/lib, no conventional root: module
        // files must be reached via `mod`, not minted as independent roots.
        let mut files = BTreeMap::new();
        for f in ["a.rs", "b.rs"] {
            files.insert(
                f.to_string(),
                ParsedFile::parse(f, "pub struct X;\n", Language::Rust).unwrap(),
            );
        }
        let cfg = RustCrateConfig::from_convention(&files);
        assert!(
            cfg.crate_roots.is_empty(),
            "multi-file non-conventional repo must not blindly root every .rs"
        );
    }

    #[test]
    fn from_convention_does_not_root_multifile_even_with_top_level_main() {
        // A top-level `main.rs` alongside an un-`mod`-wired sibling must NOT be
        // rooted: doing so builds a scope graph from main.rs that orphans the
        // sibling (it isn't reached via `mod`), regressing cross-file resolution
        // that the prior whole-file-set fallback handled. Only single-file repos
        // get the standalone root.
        let mut files = BTreeMap::new();
        files.insert(
            "main.rs".to_string(),
            ParsedFile::parse("main.rs", "fn run() {}\n", Language::Rust).unwrap(),
        );
        files.insert(
            "helpers.rs".to_string(),
            ParsedFile::parse("helpers.rs", "pub fn compute() {}\n", Language::Rust).unwrap(),
        );
        let cfg = RustCrateConfig::from_convention(&files);
        assert!(
            cfg.crate_roots.is_empty(),
            "multi-file non-Cargo repo (even with a top-level main.rs) must not be rooted"
        );
    }

    #[test]
    fn member_in_repo_deps_defaults_empty() {
        let cfg = RustCrateConfig::default();
        assert!(
            cfg.member_in_repo_deps.is_empty(),
            "the per-member dep map defaults empty"
        );
        let mut files = BTreeMap::new();
        files.insert(
            "src/lib.rs".to_string(),
            ParsedFile::parse("src/lib.rs", "pub fn a() {}\n", Language::Rust).unwrap(),
        );
        let conv = RustCrateConfig::from_convention(&files);
        assert!(
            conv.member_in_repo_deps.is_empty(),
            "convention fallback (no manifest) records no member deps"
        );
    }
}
