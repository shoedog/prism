//! The populator's mutable build state + scope/binding/edge minting + the
//! `mod foo;` file-resolution (declaring-directory rule, ambiguity, fail-open).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::ParsedFile;
use crate::name_resolution::graph::{MacroWildcard, ScopeGraph};
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
pub struct Builder<'f> {
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
}

impl<'f> Builder<'f> {
    pub fn new(
        files: &'f BTreeMap<String, ParsedFile>,
        config: &'f RustCrateConfig,
        only_files: Option<&'f BTreeSet<String>>,
    ) -> Self {
        Builder {
            graph: ScopeGraph::new(),
            files,
            config,
            only_files,
            next_scope: 0,
            next_item: 0,
            modeled: BTreeSet::new(),
            type_scopes: BTreeMap::new(),
            crate_roots_by_name: BTreeMap::new(),
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

    pub fn finish(self) -> ScopeGraph {
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

    /// Append a `Glob` edge from `scope`. `deferred` ⇒ `to = Pending` (a Phase-1
    /// poison: the engine returns `Poisoned` for any lookup the glob covers).
    pub(crate) fn add_glob_edge(
        &mut self,
        scope: ScopeId,
        target: BindTarget,
        vis: Vis,
        cond: Option<CfgCond>,
        order: u32,
    ) {
        self.graph.add_edge(Edge {
            from: scope,
            kind: EK_GLOB,
            to: target,
            vis,
            cond,
            order,
            vis_range: None,
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
    pub fn create_root(&mut self, root_path: &str) -> Option<ScopeId> {
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
        if let Some(name) = crate_name_for_root(root_path, self.config) {
            self.crate_roots_by_name.entry(name).or_insert(root_scope);
        }
        Some(root_scope)
    }

    /// PASS 2: walk a root's items into its (already-minted) `Root` scope.
    pub fn walk_root(&mut self, root_path: &str, root_scope: ScopeId) {
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
