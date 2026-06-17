//! The Rust AST → scope-graph walk: items → bindings, `use` → bindings/globs,
//! `impl` → (path-reachable, not-bare-visible) associated items, patterns →
//! `Target::Local`s, name-introducing macros → `MacroWildcard`s.
//!
//! Mirrors the `collect_calls_manual_*` DFS idiom in `ast.rs`: a cursor walk
//! dispatching on `node.kind()`. All graph mutation goes through [`Builder`].
//!
//! ## Module layout
//! This file holds the **shared infrastructure** — the walk [`Ctx`], the
//! borrow-friendly [`NodeId`] re-fetch indirection, the top-level entry
//! ([`walk_module_file`]) + per-item dispatch ([`walk_item`]), and the span/text
//! helpers. Item-definition handlers live in [`items`]; the block / statement /
//! expression descent + local-pattern extraction live in [`locals`].

mod items;
mod locals;
mod types;

use tree_sitter::Node;

use crate::ast::ParsedFile;
use crate::name_resolution::types::{FileId, ScopeId, SourceLoc, Span};

use super::builder::Builder;
use super::file_id;

// Item / type / local handlers live in the sibling submodules; brought into
// scope here for the `walk_item` dispatch. They are `pub(in …walk)`, visible
// across the whole `walk` subtree (no further re-export needed).
use items::{walk_extern_crate, walk_function, walk_mod_item, walk_use};
use locals::walk_stmt_or_block;
use types::{
    walk_enum, walk_impl, walk_macro_def, walk_macro_invocation, walk_struct, walk_type_alias,
    walk_type_like, walk_value_item,
};

/// Per-module walk context threaded through the recursion.
#[derive(Clone)]
pub(super) struct Ctx {
    /// The file being walked.
    pub file: FileId,
    /// The directory a child `mod foo;` of THIS module searches (declaring-dir
    /// rule). For `src/foo.rs`/`src/foo/mod.rs` this is `src/foo/`.
    pub dir: String,
    /// The enclosing **module/root** scope — where free items bind and where a
    /// method body is re-parented (so associated items are not bare-visible).
    pub module: ScopeId,
}

/// Walk a whole module file (root or out-of-line `mod`) into `module_scope`.
pub(crate) fn walk_module_file(b: &mut Builder<'_>, path: &str, module_scope: ScopeId, dir: &str) {
    let Some(fid) = file_id(b.files(), path) else {
        return;
    };
    let root = {
        let pf = match b.files().get(path) {
            Some(p) => p,
            None => return,
        };
        pf.tree.root_node()
    };
    let ctx = Ctx {
        file: fid,
        dir: dir.to_string(),
        module: module_scope,
    };
    // `named_children` returns owned `NodeId`s; the `&root` borrow (which borrows
    // `b`) ends here, before the mutable per-item walk below.
    let children = named_children(&root);
    for nid in children {
        walk_item(b, path, nid, module_scope, &ctx);
    }
}

/// Snapshot the named-child `NodeId`s of the node identified by `container_nid`,
/// then walk each as a top-level item into `scope`. The snapshot releases the
/// `&Builder` borrow before the mutable per-item walk.
pub(super) fn walk_items_in(
    b: &mut Builder<'_>,
    path: &str,
    container_nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let children = with_node(b, path, container_nid, |_pf, n| named_children(n));
    for nid in children {
        walk_item(b, path, nid, scope, ctx);
    }
}

/// Dispatch one item node (re-fetched by byte range to dodge borrow conflicts).
pub(super) fn walk_item(b: &mut Builder<'_>, path: &str, nid: NodeId, scope: ScopeId, ctx: &Ctx) {
    match nid.kind.as_str() {
        "mod_item" => walk_mod_item(b, path, &nid, scope, ctx),
        "use_declaration" => walk_use(b, path, &nid, scope, ctx),
        "function_item" => walk_function(b, path, &nid, scope, ctx, /*assoc=*/ false),
        "struct_item" | "union_item" => walk_struct(b, path, &nid, scope, ctx),
        "enum_item" => walk_enum(b, path, &nid, scope, ctx),
        "trait_item" => walk_type_like(b, path, &nid, scope, ctx, /*is_trait=*/ true),
        "impl_item" => walk_impl(b, path, &nid, scope, ctx),
        "const_item" | "static_item" => walk_value_item(b, path, &nid, scope, ctx),
        "type_item" => walk_type_alias(b, path, &nid, scope, ctx),
        "macro_definition" => walk_macro_def(b, path, &nid, scope, ctx),
        "macro_invocation" => walk_macro_invocation(b, path, &nid, scope, ctx),
        "extern_crate_declaration" => walk_extern_crate(b, path, &nid, scope, ctx),
        "expression_statement" | "block" => {
            // Descend into nested expression/blocks for locals + macros + uses.
            walk_stmt_or_block(b, path, &nid, scope, ctx);
        }
        // `let_declaration` only appears in a block — handled in `walk_stmt`.
        _ => {}
    }
}

// ── node-id indirection (borrow-friendly walk) ──────────────────────────────────

/// A re-fetchable handle to a node: its kind + byte range. We snapshot these to
/// avoid holding a `&ParsedFile` borrow across `&mut Builder` calls; each node is
/// re-fetched by `descendant_for_byte_range` when needed.
#[derive(Clone)]
pub(super) struct NodeId {
    pub kind: String,
    pub lo: usize,
    pub hi: usize,
}

impl NodeId {
    pub(super) fn of(n: Node) -> Self {
        NodeId {
            kind: n.kind().to_string(),
            lo: n.start_byte(),
            hi: n.end_byte(),
        }
    }
}

/// The named children of `container` as re-fetchable `NodeId`s.
pub(super) fn named_children(container: &Node) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut cursor = container.walk();
    for c in container.children(&mut cursor) {
        if c.is_named() {
            out.push(NodeId::of(c));
        }
    }
    out
}

/// Re-fetch a node and run `f` over `(ParsedFile, node)`.
///
/// `descendant_for_byte_range` returns the DEEPEST node spanning `[lo,hi)`; when
/// several nodes share that exact range (e.g. `expression_statement` wrapping a
/// sole `for_expression`), we must recover the one whose **kind matches** the
/// snapshot. We ascend through same-range parents to find it (and fall back to
/// the deepest node when no exact kind match exists — recall-safe).
fn refetch<'t>(pf: &'t ParsedFile, nid: &NodeId) -> Option<Node<'t>> {
    let deepest = pf
        .tree
        .root_node()
        .descendant_for_byte_range(nid.lo, nid.hi)?;
    if deepest.kind() == nid.kind {
        return Some(deepest);
    }
    let mut cur = Some(deepest);
    while let Some(n) = cur {
        if n.kind() == nid.kind && n.start_byte() == nid.lo && n.end_byte() == nid.hi {
            return Some(n);
        }
        match n.parent() {
            Some(p) if p.start_byte() == nid.lo && p.end_byte() == nid.hi => cur = Some(p),
            _ => break,
        }
    }
    Some(deepest)
}

pub(super) fn with_node<R>(
    b: &Builder<'_>,
    path: &str,
    nid: &NodeId,
    f: impl FnOnce(&ParsedFile, &Node) -> R,
) -> R {
    let pf = b.files().get(path).expect("file present");
    let node = refetch(pf, nid).expect("node re-fetch");
    f(pf, &node)
}

pub(super) fn node_range(nid: &NodeId) -> (usize, usize) {
    (nid.lo, nid.hi)
}

// ── span helpers ────────────────────────────────────────────────────────────────

pub(super) fn full_file_span(file: FileId) -> Span {
    Span {
        lo: SourceLoc { file, byte: 0 },
        hi: SourceLoc {
            file,
            byte: usize::MAX / 2,
        },
    }
}

/// A span covering the whole extent of `scope` in `file` (used for items, which
/// are visible across their whole enclosing scope).
pub(super) fn full_scope_span(b: &Builder<'_>, scope: ScopeId, file: FileId) -> Span {
    let (lo, hi) = scope_extent(b, scope, file).unwrap_or((0, usize::MAX / 2));
    Span {
        lo: SourceLoc { file, byte: lo },
        hi: SourceLoc { file, byte: hi },
    }
}

/// A span from `from_byte` to the end of `scope` (block-local `use`, after-def).
pub(super) fn vis_extent_from(
    b: &Builder<'_>,
    scope: ScopeId,
    file: FileId,
    from_byte: usize,
) -> Span {
    let end = scope_end_byte(b, scope, file);
    Span {
        lo: SourceLoc {
            file,
            byte: from_byte,
        },
        hi: SourceLoc {
            file,
            byte: end.max(from_byte + 1),
        },
    }
}

pub(super) fn scope_end_byte(b: &Builder<'_>, scope: ScopeId, file: FileId) -> usize {
    scope_extent(b, scope, file)
        .map(|(_, hi)| hi)
        .unwrap_or(usize::MAX / 2)
}

/// The `[lo, hi)` extent of `scope` in `file`, if recorded.
fn scope_extent(b: &Builder<'_>, scope: ScopeId, file: FileId) -> Option<(usize, usize)> {
    let s = b.graph_scope(scope)?;
    for ext in &s.extents {
        if ext.file == file {
            return Some((ext.range.lo.byte, ext.range.hi.byte));
        }
    }
    s.extents
        .first()
        .map(|e| (e.range.lo.byte, e.range.hi.byte))
}

// ── small text helpers ──────────────────────────────────────────────────────────

/// `name`-field text of a type-defining item.
pub(super) fn type_name_text(pf: &ParsedFile, n: &Node) -> String {
    n.child_by_field_name("name")
        .map(|x| pf.node_text(&x).to_string())
        .unwrap_or_default()
}

/// Strip generics from an impl `type` (e.g. `Vec<T>` → `Vec`, `S` → `S`).
pub(super) fn bare_type_name(t: &str) -> String {
    let base = t.split('<').next().unwrap_or(t);
    base.rsplit("::").next().unwrap_or(base).trim().to_string()
}

/// Resolve a `pub(in path)` restrict-path to a `ScopeId`. Phase-1: we do not yet
/// map arbitrary restrict paths to scopes precisely; returning `None` makes the
/// policy fall through (recall-safe — never widens visibility).
pub(super) fn resolve_restrict(
    _b: &Builder<'_>,
    _from: ScopeId,
    _p: &crate::name_resolution::types::RawPath,
) -> Option<ScopeId> {
    None
}
