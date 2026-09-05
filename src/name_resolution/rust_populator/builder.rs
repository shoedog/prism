//! The populator's mutable build state + scope/binding/edge minting + the
//! `mod foo;` file-resolution (declaring-directory rule, ambiguity, fail-open).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::ParsedFile;
use crate::name_resolution::binding_lookup::LocalFact;
use crate::name_resolution::graph::{MacroWildcard, ScopeGraph};
use crate::name_resolution::rust_policy::normalize_crate_ident;
use crate::name_resolution::rust_policy::EK_GLOB;
use crate::name_resolution::types::{
    BindTarget, Binding, CfgCond, Edge, FileId, Ident, NamespaceId, RawPath, Scope, ScopeExtent,
    ScopeId, ScopeKind, SourceLoc, Span, Target, Vis,
};

use super::{file_id, RustCrateConfig, PARSE_FAIL_RATE};

/// How a `mod foo;` declaration resolved to (a) file(s).
pub(crate) enum ModFile {
    /// Exactly one file (`foo.rs` XOR `foo/mod.rs`); carries its path + the
    /// directory a *child* `mod` of it should search.
    One { path: String, child_dir: String },
    /// Both `foo.rs` AND `foo/mod.rs` present → an ambiguous binding (rustc
    /// error). Carries both paths.
    Ambiguous(String, String),
    /// No file found → the module is modeled-but-hazardous (Poisoned).
    Missing,
}

/// Mutable build state shared across the crate/module/block walk.
pub(crate) struct Builder<'f> {
    graph: ScopeGraph,
    files: &'f BTreeMap<String, ParsedFile>,
    config: &'f RustCrateConfig,
    only_files: Option<&'f BTreeSet<String>>,
    next_scope: u32,
    next_item: u32,
    /// Files already turned into a module scope (cycle/duplication guard for
    /// pathological `mod` graphs).
    modeled: BTreeSet<String>,
    /// `(enclosing module scope, type name)` → the type's owned `Type` scope, so
    /// a `struct S` def and a later `impl S` in the SAME module share one scope
    /// (the struct binding's `owns` points at it; the impl fills it with
    /// associated items). Associated items live here — path-reachable, NOT
    /// bare-visible (method bodies are re-parented to the module, see `walk`).
    type_scopes: BTreeMap<(ScopeId, String), ScopeId>,
    /// In-source crate name → its crate `Root` scope (for `extern crate name`
    /// and 2018 bare-crate path roots). Populated as each crate root is built.
    crate_roots_by_name: BTreeMap<String, ScopeId>,
    /// Library-root-only index: member directory → its `Root` scope. A bin/test
    /// root is NEVER recorded here (only a library root is a dependency target), so
    /// a lib+bin member does not self-collide. Built as each library root is minted.
    lib_root_by_member_dir: BTreeMap<String, ScopeId>,
    /// Exact paths of successfully modeled roots, including binaries.
    roots_by_path: BTreeMap<String, ScopeId>,
    /// Per consuming-crate library `Root` → (normalized in-source dep name →
    /// depended-on in-repo library `Root`). Built at `finish()` from
    /// `config.member_in_repo_deps` + `lib_root_by_member_dir`; moved onto the graph.
    crate_deps_by_root: BTreeMap<ScopeId, BTreeMap<String, ScopeId>>,
    /// Repo-wide macro-name shadow set (P8 F1 BLOCKER) — computed once from
    /// `files` at construction time (the same whole-program `files` the
    /// extractor's `CallGraph::build_*` entry points independently derive
    /// their own copy from via `rust_macro_args::collect_macro_shadow_set`).
    /// Consumed by `walk/locals.rs`'s wildcard-poison exemption via
    /// [`Builder::macro_shadow`].
    macro_shadow: BTreeSet<String>,
}

impl<'f> Builder<'f> {
    pub(crate) fn new(
        files: &'f BTreeMap<String, ParsedFile>,
        config: &'f RustCrateConfig,
        only_files: Option<&'f BTreeSet<String>>,
    ) -> Self {
        let file_paths = files
            .keys()
            .enumerate()
            .map(|(i, path)| (path.clone(), FileId(i as u32)))
            .collect();
        let mut graph = ScopeGraph::new();
        graph.file_paths = file_paths;
        let macro_shadow = crate::rust_macro_args::collect_macro_shadow_set(files);
        Builder {
            graph,
            macro_shadow,
            files,
            config,
            only_files,
            next_scope: 0,
            next_item: 0,
            modeled: BTreeSet::new(),
            type_scopes: BTreeMap::new(),
            crate_roots_by_name: BTreeMap::new(),
            lib_root_by_member_dir: BTreeMap::new(),
            roots_by_path: BTreeMap::new(),
            crate_deps_by_root: BTreeMap::new(),
        }
    }

    /// The crate `Root` scope for an in-source crate name, if known.
    pub(crate) fn crate_root_named(&self, name: &str) -> Option<ScopeId> {
        self.crate_roots_by_name.get(name).copied()
    }

    /// The owned `Type` scope for `(module, type_name)`, creating it (parented at
    /// `module`, spanning the type's `[lo,hi)`) on first use. Shared by a
    /// `struct`/`enum`/`trait` def and any same-module `impl`.
    pub(crate) fn type_scope_for(
        &mut self,
        module: ScopeId,
        type_name: &str,
        file: FileId,
        lo: usize,
        hi: usize,
    ) -> ScopeId {
        if let Some(s) = self.type_scopes.get(&(module, type_name.to_string())) {
            return *s;
        }
        let s = self.add_scope(ScopeKind::Type, Some(module), file, lo, hi, None);
        self.type_scopes.insert((module, type_name.to_string()), s);
        s
    }

    pub(crate) fn finish(mut self) -> ScopeGraph {
        // Build the per-consuming-crate dep map from captured per-member deps +
        // the library-root index. For each consuming member M with a LIBRARY root
        // Rc, and each recorded `(in_source_name -> target_dir)`, map the normalized
        // in-source name to the target member's LIBRARY root (skip a target with no
        // library root — a bin-only member is not `use`-nameable). Keys are
        // normalized hyphen→underscore (the §2.2 hook normalizes the query the same).
        for (member_dir, deps) in &self.config.member_in_repo_deps {
            let Some(&consuming_root) = self.lib_root_by_member_dir.get(member_dir) else {
                continue; // consuming member has no library root → not keyed (v1)
            };
            for (in_source_name, target_dir) in deps {
                if let Some(&target_root) = self.lib_root_by_member_dir.get(target_dir) {
                    self.crate_deps_by_root
                        .entry(consuming_root)
                        .or_default()
                        .insert(normalize_crate_ident(in_source_name), target_root);
                }
            }
        }
        for (binary, (name, library)) in &self.config.binary_library_deps {
            if let (Some(&consumer), Some(&target)) = (
                self.roots_by_path.get(binary),
                self.roots_by_path.get(library),
            ) {
                if consumer != target {
                    self.crate_deps_by_root
                        .entry(consumer)
                        .or_default()
                        .insert(name.clone(), target);
                }
            }
        }
        self.graph.crate_deps_by_root = self.crate_deps_by_root;
        self.graph
    }

    // ── id + element minting ─────────────────────────────────────────────────

    pub(crate) fn fresh_scope(&mut self) -> ScopeId {
        let id = ScopeId(self.next_scope);
        self.next_scope += 1;
        id
    }

    pub(crate) fn fresh_item(&mut self) -> crate::name_resolution::types::ItemId {
        let id = crate::name_resolution::types::ItemId(self.next_item);
        self.next_item += 1;
        id
    }

    /// Mint a scope of `kind` parented at `parent`, spanning `[lo, hi)` in `file`.
    pub(crate) fn add_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        file: FileId,
        lo: usize,
        hi: usize,
        cond: Option<CfgCond>,
    ) -> ScopeId {
        let id = self.fresh_scope();
        let range = span(file, lo, hi);
        self.graph.add_scope(Scope {
            id,
            kind,
            parent,
            extents: vec![ScopeExtent {
                file,
                range,
                cond: cond.clone(),
                occ: None,
            }],
            owner_item: None,
            cond,
        });
        id
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_binding(
        &mut self,
        scope: ScopeId,
        name: Ident,
        ns: NamespaceId,
        target: BindTarget,
        vis: Vis,
        cond: Option<CfgCond>,
        vis_extents: Vec<Span>,
    ) {
        self.graph.add_binding(Binding {
            scope,
            name,
            ns,
            target,
            vis,
            cond,
            vis_extents,
        });
    }

    pub(crate) fn add_local_fact(&mut self, file: FileId, def_byte: usize, fact: LocalFact) {
        self.graph.local_facts.insert((file, def_byte), fact);
    }

    /// Append a `Glob` edge from `scope`. `deferred` ⇒ `to = Pending` (a Phase-1
    /// poison: the engine returns `Poisoned` for any lookup the glob covers).
    pub(crate) fn add_glob_edge(
        &mut self,
        scope: ScopeId,
        target: BindTarget,
        vis: Vis,
        cond: Option<CfgCond>,
        order: u32,
        vis_range: Option<Span>,
    ) {
        self.graph.add_edge(Edge {
            from: scope,
            kind: EK_GLOB,
            to: target,
            vis,
            cond,
            order,
            vis_range,
        });
    }

    /// Mark `scope` as **Poisoned** (modeled-but-hazardous): add a deferred-glob
    /// edge so EVERY `(name, ns)` lookup within `scope` returns `Poisoned`
    /// (`glob_lookup` poisons on a `Pending` glob, name/ns-agnostically). Used
    /// for a missing `mod` file and a parse-failed *target* module.
    pub(crate) fn poison_scope(&mut self, scope: ScopeId, vis: Vis) {
        self.add_glob_edge(
            scope,
            BindTarget::Pending(RawPath(vec![]), Default::default()),
            vis,
            None,
            0,
            None,
        );
    }

    /// Record a name-introducing macro's wildcard over `(scope, ns, [lo,hi))`.
    pub(crate) fn add_macro_wildcard(
        &mut self,
        scope: ScopeId,
        ns: NamespaceId,
        lo: usize,
        hi: usize,
        file: FileId,
    ) {
        self.graph.add_macro_wildcard(MacroWildcard {
            scope,
            ns,
            range: span(file, lo, hi),
        });
    }

    // ── accessors used by the walk ────────────────────────────────────────────

    pub(crate) fn files(&self) -> &'f BTreeMap<String, ParsedFile> {
        self.files
    }

    /// Repo-wide macro-name shadow set (P8 F1 BLOCKER) — see the field doc
    /// on `Builder::macro_shadow`.
    pub(crate) fn macro_shadow(&self) -> &BTreeSet<String> {
        &self.macro_shadow
    }

    pub(crate) fn config(&self) -> &'f RustCrateConfig {
        self.config
    }

    /// Read-only access to a scope record (for extent lookups in the walk).
    pub(crate) fn graph_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.graph.scope(id)
    }

    /// Is this containing file in-scope to model? (`only_files` filter.)
    pub(crate) fn in_scope(&self, path: &str) -> bool {
        self.only_files.map(|s| s.contains(path)).unwrap_or(true)
    }

    /// Has `path` already been turned into a module scope?
    pub(crate) fn already_modeled(&self, path: &str) -> bool {
        self.modeled.contains(path)
    }

    pub(crate) fn mark_modeled(&mut self, path: &str) {
        self.modeled.insert(path.to_string());
    }

    /// Did `path`'s file fail to parse (error rate ≥ floor)?
    pub(crate) fn parse_failed(&self, path: &str) -> bool {
        self.files
            .get(path)
            .map(|pf| pf.error_rate() >= PARSE_FAIL_RATE)
            .unwrap_or(true)
    }

    // ── crate-graph entry ─────────────────────────────────────────────────────

    /// PASS 1: mint the `Root` scope for `root_path` (and register its crate name
    /// for `extern crate`), WITHOUT walking its body. Returns the Root scope, or
    /// `None` for an absent / out-of-`only_files` / parse-failed root (Unmodeled).
    /// Doing all roots first lets `extern crate dep` reach a later crate's Root.
    pub(crate) fn create_root(&mut self, root_path: &str) -> Option<ScopeId> {
        if !self.files.contains_key(root_path) {
            return None;
        }
        if !self.in_scope(root_path) || self.parse_failed(root_path) {
            return None; // Unmodeled containing file
        }
        if self.already_modeled(root_path) {
            return None;
        }
        let fid = file_id(self.files, root_path)?;
        let src_len = self.files[root_path].source.len();
        let root_scope = self.add_scope(ScopeKind::Root, None, fid, 0, src_len.max(1), None);
        self.mark_modeled(root_path);
        self.roots_by_path.insert(root_path.to_string(), root_scope);
        if let Some(name) = crate_name_for_root(root_path, self.config) {
            self.crate_roots_by_name.entry(name).or_insert(root_scope);
        }
        // Library-root-only dependency-target index (P3 / MAJOR): record the member
        // dir → this Root iff `root_path` is a library root (`[lib].path` override or
        // the conventional `.../src/lib.rs`). A bin/test/bench/example root is never a
        // `use`-nameable dependency target, so it is excluded (lib+bin no-self-collide).
        if let Some(member_dir) = lib_root_member_dir(root_path, self.config) {
            self.lib_root_by_member_dir
                .entry(member_dir)
                .or_insert(root_scope);
        }
        Some(root_scope)
    }

    /// PASS 2: walk a root's items into its (already-minted) `Root` scope.
    pub(crate) fn walk_root(&mut self, root_path: &str, root_scope: ScopeId) {
        let dir = parent_dir(root_path);
        super::walk::walk_module_file(self, root_path, root_scope, &dir);
    }

    // ── `mod foo;` file resolution (declaring-directory rule) ──────────────────

    /// Resolve a `mod name;` declared in a module whose directory is `dir`, with
    /// an optional `#[path]` override. Applies the declaring-DIR rule and the
    /// `foo.rs`+`foo/mod.rs` ambiguity.
    pub(crate) fn resolve_mod_file(
        &self,
        dir: &str,
        name: &str,
        path_attr: Option<&str>,
    ) -> ModFile {
        if let Some(rel) = path_attr {
            // `#[path = "rel"]` is relative to the declaring module's directory.
            let p = join(dir, rel);
            if self.files.contains_key(&p) {
                // A `#[path]` file's children search its own parent dir.
                return ModFile::One {
                    child_dir: parent_dir(&p),
                    path: p,
                };
            }
            return ModFile::Missing;
        }
        let flat = join(dir, &format!("{name}.rs")); // dir/foo.rs
        let nested = join(dir, &format!("{name}/mod.rs")); // dir/foo/mod.rs
        let has_flat = self.files.contains_key(&flat);
        let has_nested = self.files.contains_key(&nested);
        match (has_flat, has_nested) {
            (true, true) => ModFile::Ambiguous(flat, nested),
            (true, false) => ModFile::One {
                // children of `dir/foo.rs` search `dir/foo/`
                child_dir: join(dir, &format!("{name}/")),
                path: flat,
            },
            (false, true) => ModFile::One {
                // children of `dir/foo/mod.rs` also search `dir/foo/`
                child_dir: join(dir, &format!("{name}/")),
                path: nested,
            },
            (false, false) => ModFile::Missing,
        }
    }

    /// Build (or poison) the child module scope for an out-of-line `mod name;`
    /// found in `parent_scope`. Returns the new module scope so the caller can
    /// add the `name` Type binding pointing at it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build_out_of_line_module(
        &mut self,
        parent_scope: ScopeId,
        decl_file: FileId,
        name: &str,
        dir: &str,
        path_attr: Option<&str>,
        vis: Vis,
        cond: Option<CfgCond>,
    ) -> Vec<(ScopeId, Option<String>)> {
        // Returns (module_scope, source_path_if_modeled) pairs — 2 entries only
        // for the ambiguous case (each candidate file → its own binding).
        match self.resolve_mod_file(dir, name, path_attr) {
            ModFile::One { path, child_dir } => {
                let s =
                    self.module_scope_for_target(parent_scope, &path, cond.clone(), vis.clone());
                if let Some(scope) = s {
                    vec![(scope, Some(path_then_walk(self, &path, scope, &child_dir)))]
                } else {
                    // target parse-failed / out-of-scope → Poisoned shell
                    let shell = self.empty_module_shell(parent_scope, decl_file, cond);
                    self.poison_scope(shell, vis);
                    vec![(shell, None)]
                }
            }
            ModFile::Ambiguous(a, bf) => {
                // Two distinct module-scope bindings under the same (compatible)
                // cfg ⇒ the engine reports Ambiguous. Build a scope per file.
                let mut out = Vec::new();
                for p in [a, bf] {
                    if let Some(scope) =
                        self.module_scope_for_target(parent_scope, &p, cond.clone(), vis.clone())
                    {
                        let cd = child_dir_for(&p);
                        out.push((scope, Some(path_then_walk(self, &p, scope, &cd))));
                    } else {
                        let shell = self.empty_module_shell(parent_scope, decl_file, cond.clone());
                        self.poison_scope(shell, vis.clone());
                        out.push((shell, None));
                    }
                }
                out
            }
            ModFile::Missing => {
                // Missing file → modeled-but-hazardous: a poisoned shell scope.
                let shell = self.empty_module_shell(parent_scope, decl_file, cond);
                self.poison_scope(shell, vis);
                vec![(shell, None)]
            }
        }
    }

    /// Make the `Module` scope for a target file, unless it is out-of-scope or
    /// parse-failed (→ `None`, caller poisons). Does NOT walk it.
    fn module_scope_for_target(
        &mut self,
        parent: ScopeId,
        path: &str,
        cond: Option<CfgCond>,
        _vis: Vis,
    ) -> Option<ScopeId> {
        if !self.files.contains_key(path) {
            return None;
        }
        if !self.in_scope(path) || self.parse_failed(path) {
            return None; // Unmodeled-as-target → caller makes a poisoned shell
        }
        let fid = file_id(self.files, path)?;
        let len = self.files[path].source.len().max(1);
        Some(self.add_scope(ScopeKind::Module, Some(parent), fid, 0, len, cond))
    }

    /// An empty module shell parented under `parent`. Used to model a
    /// missing/failed/filtered module so a path INTO it resolves to a (poisoned)
    /// scope.
    ///
    /// Its extent is **zero-width** (`[0, 0)` in the declaring file), so
    /// `enclosing_scope` (which needs `byte < hi`) can NEVER select it for a byte
    /// query — it must not shadow a real scope in the declaring file at byte 0.
    /// It exists only as a path target (reached via the parent's `Scope` binding,
    /// not by byte lookup); its deferred-glob poison then poisons any member.
    fn empty_module_shell(
        &mut self,
        parent: ScopeId,
        decl_file: FileId,
        cond: Option<CfgCond>,
    ) -> ScopeId {
        let id = self.fresh_scope();
        let zero = Span {
            lo: SourceLoc {
                file: decl_file,
                byte: 0,
            },
            hi: SourceLoc {
                file: decl_file,
                byte: 0,
            },
        };
        self.graph.add_scope(Scope {
            id,
            kind: ScopeKind::Module,
            parent: Some(parent),
            extents: vec![ScopeExtent {
                file: decl_file,
                range: zero,
                cond: cond.clone(),
                occ: None,
            }],
            owner_item: None,
            cond,
        });
        id
    }
}

// ── free helpers ───────────────────────────────────────────────────────────────

fn span(file: FileId, lo: usize, hi: usize) -> Span {
    Span {
        lo: SourceLoc { file, byte: lo },
        hi: SourceLoc {
            file,
            byte: hi.max(lo + 1),
        },
    }
}

/// The directory (with trailing `/`) of a repo-relative file path; `""` for a
/// top-level file.
pub(crate) fn parent_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path[..=i].to_string(),
        None => String::new(),
    }
}

/// Join a directory (trailing `/` or empty) with a relative tail, normalizing.
pub(crate) fn join(dir: &str, tail: &str) -> String {
    if dir.is_empty() {
        tail.to_string()
    } else if dir.ends_with('/') {
        format!("{dir}{tail}")
    } else {
        format!("{dir}/{tail}")
    }
}

/// For a child-module search: the directory children of `path` search.
/// `dir/foo.rs` and `dir/foo/mod.rs` both → `dir/foo/`.
fn child_dir_for(path: &str) -> String {
    if path.ends_with("/mod.rs") {
        path.trim_end_matches("mod.rs").to_string()
    } else {
        // strip ".rs"
        let base = path.trim_end_matches(".rs");
        format!("{base}/")
    }
}

/// Mark `path` modeled, walk it as a module file into `scope`, and return `path`
/// (so callers can record the modeled source path). Centralizes the
/// mark-then-walk so the ambiguous/one cases share it.
fn path_then_walk(b: &mut Builder<'_>, path: &str, scope: ScopeId, child_dir: &str) -> String {
    if !b.already_modeled(path) {
        b.mark_modeled(path);
        super::walk::walk_module_file(b, path, scope, child_dir);
    }
    path.to_string()
}

/// `Target::Scope` convenience.
pub(crate) fn scope_target(id: ScopeId) -> BindTarget {
    BindTarget::Resolved(Target::Scope(id))
}

/// The in-source crate name for a root file, derived from its enclosing
/// workspace member directory (basename) when one matches, else the parent of
/// the conventional `src/` directory. Returns `None` when no name is derivable
/// (a top-level single crate — `extern crate` of it is unusual / irrelevant).
fn crate_name_for_root(root_path: &str, config: &RustCrateConfig) -> Option<String> {
    // Prefer the longest matching workspace member prefix.
    let mut best: Option<&str> = None;
    for m in &config.workspace_members {
        let prefix = if m.ends_with('/') {
            m.clone()
        } else {
            format!("{m}/")
        };
        if root_path.starts_with(&prefix) && best.map(|b| m.len() > b.len()).unwrap_or(true) {
            best = Some(m.as_str());
        }
    }
    if let Some(m) = best {
        return m.rsplit('/').next().map(|s| s.to_string());
    }
    // `<name>/src/lib.rs` → `<name>` (member-style layout without explicit list).
    if let Some(idx) = root_path.find("/src/") {
        let head = &root_path[..idx];
        return head.rsplit('/').next().map(|s| s.to_string());
    }
    None
}

/// The workspace-member DIRECTORY for a root path **iff it is a library root**
/// (re-review BLOCKER B). A library root is EXACTLY `<member>/src/lib.rs` (the
/// convention) OR the `[lib].path` override (`config.lib_path == Some(root_path)`)
/// for that member; a bin/test/bench/example root returns `None` (never a dependency
/// target — the lib+bin no-self-collide guarantee).
///
/// The member dir is the MANIFEST DIR — the exact spelling `member_in_repo_deps`
/// keys by (Task 2 normalizes both its KEYS (the manifest dir) and the dep-target
/// VALUES (`normalize_repo_rel`) to that form), so the `finish()` lookups
/// `lib_root_by_member_dir.get(member_dir)`/`.get(target_dir)` hit. It is derived
/// from `config.workspace_members` (the LONGEST member prefix of `root_path`),
/// **not** the library file's parent: an explicit `[lib] path = "src/lib.rs"` has
/// root file `<member>/src/lib.rs` whose parent is `<member>/src`, and a nested
/// `[lib] path = "src/inner/lib.rs"` parent is `<member>/src/inner` — neither is the
/// member dir `<member>` that dep-target resolution produces. `workspace_members`
/// carries those member dirs verbatim (`repo_loader.rs:357` =
/// `join_manifest_rel(manifest_dir, member)`, no trailing slash); this mirrors
/// `crate_name_for_root`'s prefix match but returns the FULL member dir, not the
/// basename.
///
/// The library-root gate is EXACT (re-review round-3 MAJOR): for a matched member
/// `m`, accept ONLY `m/src/lib.rs` or the explicit `config.lib_path` override — a
/// bare `ends_with("src/lib.rs")` is too broad and would mis-record a bin/tool path
/// like `m/tools/src/lib.rs` as `m`'s library root, shadowing the real lib root.
/// When no member prefix matches (the single crate at the repo root, member dir `""`,
/// absent from `workspace_members`), accept only the bare `src/lib.rs` or a root
/// `[lib].path`. (Repo-wide, `config.lib_path` holds only the LAST explicit
/// `[lib].path` — a pre-existing `RustCrateConfig` flatness; convention library roots
/// are unaffected because the exact `m/src/lib.rs` gate matches them without it.)
fn lib_root_member_dir(root_path: &str, config: &RustCrateConfig) -> Option<String> {
    // Member dir = the LONGEST workspace-member prefix of root_path (the manifest dir).
    let mut member: Option<&str> = None;
    for m in &config.workspace_members {
        let m = m.trim_end_matches('/');
        let prefix = format!("{m}/");
        if root_path.starts_with(&prefix) && member.map(|b| m.len() > b.len()).unwrap_or(true) {
            member = Some(m);
        }
    }
    match member {
        // A matched member's library root is EXACTLY `<m>/src/lib.rs` or its explicit
        // [lib].path; anything else under `<m>/` (e.g. `<m>/tools/src/lib.rs`) is a
        // bin/test/example root → not a dependency target.
        Some(m) => {
            if config.lib_path.as_deref() == Some(root_path)
                || root_path == format!("{m}/src/lib.rs")
            {
                Some(m.to_string())
            } else {
                None
            }
        }
        // No workspace member matched: the single crate at the repo root (member `""`).
        None => {
            if root_path == "src/lib.rs" || config.lib_path.as_deref() == Some(root_path) {
                Some(String::new())
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{populate_rust, RustCrateConfig};
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use crate::name_resolution::types::{ScopeId, ScopeKind};
    use std::collections::BTreeMap;

    fn rs(src: &str) -> ParsedFile {
        ParsedFile::parse("x.rs", src, Language::Rust).unwrap()
    }

    fn root_for(graph: &crate::name_resolution::graph::ScopeGraph, file_path_idx: u32) -> ScopeId {
        // The Root scope whose extent is in the file at FileId(idx).
        graph
            .scopes
            .iter()
            .find(|(_, s)| {
                matches!(s.kind, ScopeKind::Root)
                    && s.extents.iter().any(|e| e.file.0 == file_path_idx)
            })
            .map(|(id, _)| *id)
            .expect("a Root scope for the file")
    }

    #[test]
    fn crate_deps_by_root_maps_consumer_to_target_lib_root() {
        // Two-crate workspace: a depends on b (recorded by member_in_repo_deps).
        // crate_deps_by_root[a_lib_root]["b_crate"] must be b's lib Root.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b_crate".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // FileIds follow sorted key order: a/src/lib.rs = 0, b/src/lib.rs = 1.
        let a_root = root_for(&graph, 0);
        let b_root = root_for(&graph, 1);
        assert_eq!(
            graph
                .crate_deps_by_root
                .get(&a_root)
                .and_then(|m| m.get("b_crate")),
            Some(&b_root),
            "a's dep map must point b_crate at b's lib Root"
        );
    }

    #[test]
    fn lib_plus_bin_member_does_not_self_collide() {
        // A single member `b` with BOTH src/lib.rs and src/main.rs, depended on by
        // `a`. The recorded dep target must be b's LIBRARY root, never the bin root.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        files.insert("b/src/main.rs".to_string(), rs("fn main() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            crate_roots: vec![
                "a/src/lib.rs".to_string(),
                "b/src/lib.rs".to_string(),
                "b/src/main.rs".to_string(),
            ],
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // a/src/lib.rs=0, b/src/lib.rs=1, b/src/main.rs=2 (sorted key order).
        let a_root = root_for(&graph, 0);
        let b_lib_root = root_for(&graph, 1);
        assert_eq!(
            graph
                .crate_deps_by_root
                .get(&a_root)
                .and_then(|m| m.get("b")),
            Some(&b_lib_root),
            "the dep target must be b's library root (FileId 1), never the bin root (FileId 2)"
        );
    }

    #[test]
    fn explicit_lib_path_target_keys_by_member_dir_not_file_parent() {
        // Re-review BLOCKER B: target `b` has an EXPLICIT `[lib] path = "src/lib.rs"`
        // (cfg.lib_path = "b/src/lib.rs"). The dep-target index MUST key b's lib Root
        // by the MEMBER DIR `b` (from workspace_members), NOT the library file's parent
        // `b/src`. The old `parent_dir`-based helper keyed `b/src` -> crate_deps miss.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string()); // target dir = the member dir `b`
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            // `a` is conventional; `b`'s library root is the explicit [lib].path.
            crate_roots: vec!["a/src/lib.rs".to_string()],
            lib_path: Some("b/src/lib.rs".to_string()),
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // Sorted key order: a/src/lib.rs=0, b/src/lib.rs=1.
        let a_root = root_for(&graph, 0);
        let b_lib_root = root_for(&graph, 1);
        assert_eq!(
            graph
                .crate_deps_by_root
                .get(&a_root)
                .and_then(|m| m.get("b")),
            Some(&b_lib_root),
            "an explicit [lib].path target must key by member dir `b`, not `b/src`"
        );
    }

    #[test]
    fn nested_custom_lib_path_target_keys_by_member_dir() {
        // Re-review BLOCKER B: target `b`'s library root is a NESTED custom path
        // `[lib] path = "src/inner/lib.rs"` (cfg.lib_path = "b/src/inner/lib.rs").
        // The member dir is still `b` (from workspace_members) — never the file parent
        // `b/src/inner`. This is the case a file-parent derivation alone gets wrong,
        // so the workspace_members-prefix derivation is required.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/inner/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            crate_roots: vec!["a/src/lib.rs".to_string()],
            lib_path: Some("b/src/inner/lib.rs".to_string()),
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // Sorted key order: a/src/lib.rs=0, b/src/inner/lib.rs=1.
        let a_root = root_for(&graph, 0);
        let b_lib_root = root_for(&graph, 1);
        assert_eq!(
            graph
                .crate_deps_by_root
                .get(&a_root)
                .and_then(|m| m.get("b")),
            Some(&b_lib_root),
            "a nested custom [lib].path target must still key by member dir `b`"
        );
    }

    #[test]
    fn custom_bin_path_ending_src_lib_rs_does_not_shadow_library_root() {
        // Re-review round-3 MAJOR (round-4 rework): member `b` has the conventional
        // library root `b/src/lib.rs` AND a bin/tool root whose path ALSO ends in
        // `src/lib.rs`. A bare `ends_with("src/lib.rs")` gate would attribute the bin
        // path to member `b` too; the EXACT `b/src/lib.rs` gate keeps it out, so the
        // dep target is always b's LIBRARY root.
        //
        // This test discriminates TWO ways:
        //   1. DIRECT, order-independent helper assertions (the bulletproof
        //      discriminator): the old bare-suffix gate returns `Some("b")` for a
        //      `b/.../src/lib.rs` bin path, the new exact gate returns `None`. These
        //      do NOT depend on FileId/`or_insert` order, so they catch the buggy
        //      helper regardless of insertion ordering.
        //   2. An end-to-end `crate_deps_by_root` check using a decoy bin path that
        //      sorts BEFORE the real lib (`b/bin/src/lib.rs` < `b/src/lib.rs`), so
        //      under the OLD helper the bin root would win the FIRST-wins `or_insert`
        //      in sorted-FileId order and the e2e assertion would fail. (The round-3
        //      decoy `b/tools/src/lib.rs` sorts AFTER `b/src/lib.rs`, so the real lib
        //      won `or_insert` first even under the buggy helper — that decoy did not
        //      discriminate; `b/bin` fixes it.)
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/bin/src/lib.rs".to_string(), rs("pub fn tool() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            // All three are roots; b's conventional lib + a tool bin under b/bin.
            crate_roots: vec![
                "a/src/lib.rs".to_string(),
                "b/bin/src/lib.rs".to_string(),
                "b/src/lib.rs".to_string(),
            ],
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };

        // (1) Direct helper assertions — order-independent, the bulletproof
        // discriminator. `lib_root_member_dir` is a private FREE fn in builder.rs and
        // `mod tests` is in builder.rs, so `super::` reaches it; `cfg` is in scope here.
        assert_eq!(
            super::lib_root_member_dir("b/bin/src/lib.rs", &cfg),
            None,
            "a `b/bin/src/lib.rs` bin/tool path is NOT member b's library root \
             (old bare-suffix gate wrongly returned Some(\"b\"); exact gate returns None)"
        );
        assert_eq!(
            super::lib_root_member_dir("b/src/lib.rs", &cfg),
            Some("b".to_string()),
            "the conventional `b/src/lib.rs` IS member b's library root"
        );

        // (2) End-to-end, made discriminating by the sort-before-bin decoy.
        let graph = populate_rust(&files, &cfg, None);
        // Sorted key order: a/src/lib.rs=0, b/bin/src/lib.rs=1, b/src/lib.rs=2.
        let a_root = root_for(&graph, 0);
        let b_bin_root = root_for(&graph, 1);
        let b_lib_root = root_for(&graph, 2);
        let target = graph
            .crate_deps_by_root
            .get(&a_root)
            .and_then(|m| m.get("b"));
        assert_eq!(
            target,
            Some(&b_lib_root),
            "the dep target must be b's library root (FileId 2), never the bin/tool root"
        );
        assert_ne!(
            target,
            Some(&b_bin_root),
            "a `bin/src/lib.rs` bin root (FileId 1, sorts BEFORE the real lib) must \
             not win or_insert as b's library root"
        );
    }

    #[test]
    fn normalize_crate_ident_hyphen_to_underscore() {
        use crate::name_resolution::rust_policy::normalize_crate_ident;
        assert_eq!(normalize_crate_ident("my-crate"), "my_crate");
        assert_eq!(normalize_crate_ident("plain"), "plain");
    }
}
