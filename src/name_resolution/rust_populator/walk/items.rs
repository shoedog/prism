//! Module / import / callable handlers: `mod` (inline + out-of-line), `use`
//! (named/alias/group/glob), `extern crate`, and `function_item` (free fns +
//! associated-fn bodies). Type/macro definitions live in [`super::types`].

use tree_sitter::Node;

use crate::ast::ParsedFile;
use crate::name_resolution::binding_lookup::{BindingKind, LocalFact};
use crate::name_resolution::rust_policy::{NS_TYPE, NS_VALUE, VIS_PRIV, VIS_PUB};
use crate::name_resolution::types::{BindTarget, ExternRef, FileId, ScopeId, ScopeKind, Target};

use super::super::builder::{join, parent_dir, scope_target, Builder};
use super::super::scopes::{
    child_of_kind, name_text, parse_cfg_for_item, parse_use, parse_vis, pattern_idents, vis,
    UseItem,
};
use super::locals::{add_locals_with_facts, walk_block_body};
use super::{
    full_file_span, full_scope_span, node_range, resolve_restrict, vis_extent_from, with_node, Ctx,
    NodeId,
};

// ── modules ───────────────────────────────────────────────────────────────────

pub(in crate::name_resolution::rust_populator::walk) fn walk_mod_item(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let (name, has_body, body_range, vis_kind, vin, cond, path_attr) =
        with_node(b, path, nid, |pf, n| {
            let name = name_text(pf, n).unwrap_or_default();
            let body = n.child_by_field_name("body");
            let (vk, vin) = parse_vis(pf, n);
            let cond = parse_cfg_for_item(pf, n);
            let path_attr = mod_path_attr(pf, n);
            (
                name,
                body.is_some(),
                body.map(|x| (x.start_byte(), x.end_byte())),
                vk,
                vin,
                cond,
                path_attr,
            )
        });
    if name.is_empty() {
        return;
    }
    let restrict = vin.and_then(|p| resolve_restrict(b, scope, &p));
    let v = vis(vis_kind, restrict);

    if has_body {
        // Inline module: a Module scope spanning the body, parented lexically at
        // `scope`. Its child-search dir is `<ctx.dir>/<name>/` (or `#[path]`).
        let (lo, hi) = body_range.unwrap();
        let inline = b.add_scope(
            ScopeKind::Module,
            Some(scope),
            ctx.file,
            lo,
            hi,
            cond.clone(),
        );
        b.add_binding(
            scope,
            name.clone(),
            NS_TYPE,
            scope_target(inline),
            v,
            cond.clone(),
            vec![vis_extent_from(b, scope, ctx.file, lo)],
        );
        let child_dir = match &path_attr {
            Some(rel) => parent_dir(&join(&ctx.dir, rel)),
            None => join(&ctx.dir, &format!("{name}/")),
        };
        let body_nid = with_node(b, path, nid, |_pf, n| {
            n.child_by_field_name("body").map(NodeId::of)
        });
        if let Some(body_nid) = body_nid {
            let inner_ctx = Ctx {
                file: ctx.file,
                dir: child_dir,
                module: inline,
            };
            super::walk_items_in(b, path, &body_nid, inline, &inner_ctx);
        }
    } else {
        // Out-of-line `mod name;` → resolve to a file via the declaring-dir rule.
        let pairs = b.build_out_of_line_module(
            scope,
            ctx.file,
            &name,
            &ctx.dir,
            path_attr.as_deref(),
            v.clone(),
            cond.clone(),
        );
        for (mod_scope, _src) in pairs {
            b.add_binding(
                scope,
                name.clone(),
                NS_TYPE,
                scope_target(mod_scope),
                v.clone(),
                cond.clone(),
                vec![full_file_span(ctx.file)],
            );
        }
    }
}

/// The `#[path = "..."]` attribute value on a mod, if present.
fn mod_path_attr(pf: &ParsedFile, mod_node: &Node) -> Option<String> {
    let mut prev = mod_node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() != "attribute_item" {
            break;
        }
        if let Some(attr) = child_of_kind(&sib, "attribute") {
            let head = attr.named_child(0).map(|n| pf.node_text(&n)).unwrap_or("");
            if head == "path" {
                if let Some(v) = attr.child_by_field_name("value") {
                    return Some(pf.node_text(&v).trim_matches('"').to_string());
                }
            }
        }
        prev = sib.prev_sibling();
    }
    None
}

/// `extern crate dep [as bar];` → a Type binding for `bar` (or `dep`) at the
/// crate root, targeting the named in-workspace crate's Root scope when known,
/// else an `External` reference (recall-safe: a known-external, files `[]`).
pub(in crate::name_resolution::rust_populator::walk) fn walk_extern_crate(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let (crate_name, alias) = with_node(b, path, nid, |pf, n| {
        let crate_name = n
            .child_by_field_name("name")
            .map(|x| pf.node_text(&x).to_string())
            .unwrap_or_default();
        let alias = n
            .child_by_field_name("alias")
            .map(|x| pf.node_text(&x).to_string());
        (crate_name, alias)
    });
    if crate_name.is_empty() {
        return;
    }
    let bound = alias.unwrap_or_else(|| crate_name.clone());
    let target = match b.crate_root_named(&crate_name) {
        Some(root) => scope_target(root),
        None => BindTarget::Resolved(Target::External(ExternRef { name: crate_name })),
    };
    b.add_binding(
        scope,
        bound,
        NS_TYPE,
        target,
        vis(VIS_PUB, None),
        None,
        vec![full_scope_span(b, scope, ctx.file)],
    );
}

// ── use declarations ───────────────────────────────────────────────────────────

pub(in crate::name_resolution::rust_populator::walk) fn walk_use(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let edition = b.config().edition;
    let (items, vis_kind, cond, decl_lo) = with_node(b, path, nid, |pf, n| {
        let items = parse_use(pf, n, edition);
        let (vk, _vin) = parse_vis(pf, n);
        let cond = parse_cfg_for_item(pf, n);
        (items, vk, cond, n.start_byte())
    });
    // `use` visibility: `pub use` is a re-export (Public); plain `use` is private.
    let v = vis(vis_kind, None);
    // Block-local `use` is visible from the use site to the END of the enclosing
    // scope; a module/item-level `use` is order-independent and whole-scope.
    let extent = match b.graph_scope(scope).map(|s| &s.kind) {
        Some(ScopeKind::Module | ScopeKind::Root | ScopeKind::Type) => {
            full_scope_span(b, scope, ctx.file)
        }
        Some(ScopeKind::Block | ScopeKind::Callable)
        | Some(ScopeKind::ExternPrelude | ScopeKind::TranslationUnit)
        | None => vis_extent_from(b, scope, ctx.file, decl_lo),
    };
    let glob_vis_range = match b.graph_scope(scope).map(|s| &s.kind) {
        Some(ScopeKind::Block | ScopeKind::Callable) => Some(extent.clone()),
        Some(ScopeKind::Module | ScopeKind::Root | ScopeKind::Type) => None,
        Some(ScopeKind::ExternPrelude | ScopeKind::TranslationUnit) | None => Some(extent.clone()),
    };
    let mut order = 0u32;
    for item in items {
        match item {
            UseItem::Named {
                name,
                path: p,
                anchor,
            } => {
                b.add_binding(
                    scope,
                    name.clone(),
                    NS_VALUE,
                    BindTarget::Pending(p.clone(), anchor),
                    v.clone(),
                    cond.clone(),
                    vec![extent.clone()],
                );
                // A `use` may bring the name in Type ns too (module/type imports,
                // `use a::{self}`); add a Type-ns Pending so those paths resolve.
                b.add_binding(
                    scope,
                    name,
                    NS_TYPE,
                    BindTarget::Pending(p, anchor),
                    v.clone(),
                    cond.clone(),
                    vec![extent.clone()],
                );
            }
            UseItem::Glob { path: p, anchor } => {
                // Deferred-glob poison edge (no member expansion in Phase 1).
                b.add_glob_edge(
                    scope,
                    BindTarget::Pending(p, anchor),
                    v.clone(),
                    cond.clone(),
                    order,
                    glob_vis_range.clone(),
                );
                order += 1;
            }
        }
    }
}

// ── functions / callables ──────────────────────────────────────────────────────

/// Walk a `function_item`. `assoc=false` → a free fn: a Value(callable) binding
/// in `scope`, body Callable parented at `scope`. `assoc=true` → the binding is
/// added by the type walker into the type scope; here we only build the BODY,
/// re-parented to the ENCLOSING MODULE (`ctx.module`) so it is not bare-visible.
pub(in crate::name_resolution::rust_populator::walk) fn walk_function(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
    assoc: bool,
) {
    let (name, params_nid, body_nid, vis_kind, cond) = with_node(b, path, nid, |pf, n| {
        (
            name_text(pf, n).unwrap_or_default(),
            n.child_by_field_name("parameters").map(NodeId::of),
            n.child_by_field_name("body").map(NodeId::of),
            parse_vis(pf, n).0,
            parse_cfg_for_item(pf, n),
        )
    });
    if name.is_empty() {
        return;
    }
    let v = vis(vis_kind, None);
    if !assoc {
        let item = b.fresh_item();
        b.add_binding(
            scope,
            name.clone(),
            NS_VALUE,
            BindTarget::Resolved(Target::Item {
                id: item,
                ns: NS_VALUE,
                owns: None,
                callable: true,
            }),
            v,
            cond.clone(),
            vec![full_scope_span(b, scope, ctx.file)],
        );
    }
    // The body Callable scope. For a free fn its lexical parent IS the module
    // (`scope == ctx.module` at module level). For an associated fn we FORCE the
    // parent to `ctx.module` (NOT the type scope), satisfying the
    // not-bare-visible obligation.
    let body_parent = if assoc { ctx.module } else { scope };
    if let Some(body_nid) = body_nid {
        let (blo, bhi) = node_range(&body_nid);
        let body_scope = b.add_scope(
            ScopeKind::Callable,
            Some(body_parent),
            ctx.file,
            blo,
            bhi,
            cond.clone(),
        );
        if let Some(params_nid) = params_nid {
            bind_params(b, path, &params_nid, body_scope, ctx.file, bhi);
        }
        // Method bodies are re-parented to the module, so carry type-parameter
        // shadow barriers from their syntactic fn/impl/trait ancestors explicitly.
        // Opaque, non-callable Type items block crate fallback without claiming
        // a concrete implementation for a generic receiver.
        let generic_names = with_node(b, path, nid, |pf, n| {
            let mut names = std::collections::BTreeSet::new();
            let mut ancestor = Some(*n);
            while let Some(node) = ancestor {
                if matches!(node.kind(), "mod_item" | "source_file") {
                    break;
                }
                if let Some(params) = node.child_by_field_name("type_parameters") {
                    let mut cursor = params.walk();
                    for param in params
                        .named_children(&mut cursor)
                        .filter(|p| p.kind() == "type_parameter")
                    {
                        if let Some(name) = param.child_by_field_name("name") {
                            names.insert(pf.node_text(&name).to_string());
                        }
                    }
                }
                ancestor = node.parent();
            }
            names
        });
        for name in generic_names {
            let id = b.fresh_item();
            b.add_binding(
                body_scope,
                name,
                NS_TYPE,
                BindTarget::Resolved(Target::Item {
                    id,
                    ns: NS_TYPE,
                    owns: None,
                    callable: false,
                }),
                vis(VIS_PRIV, None),
                None,
                vec![full_scope_span(b, body_scope, ctx.file)],
            );
        }
        let body_ctx = Ctx {
            file: ctx.file,
            dir: ctx.dir.clone(),
            module: ctx.module,
        };
        walk_block_body(b, path, &body_nid, body_scope, &body_ctx);
    }
}

/// Bind every parameter pattern as a `Target::Local` in `body_scope`.
fn bind_params(
    b: &mut Builder<'_>,
    path: &str,
    params_nid: &NodeId,
    body_scope: ScopeId,
    file: FileId,
    body_end: usize,
) {
    let params = with_node(b, path, params_nid, |pf, n| {
        let mut out = Vec::new();
        let mut cursor = n.walk();
        for c in n.children(&mut cursor) {
            if c.kind() == "parameter" {
                let annotation = c
                    .child_by_field_name("type")
                    .map(|ty| pf.node_text(&ty).trim().to_string())
                    .filter(|s| !s.is_empty());
                if let Some(p) = c.child_by_field_name("pattern") {
                    let mut names = Vec::new();
                    pattern_idents(pf, &p, &mut names);
                    let is_simple_param = is_simple_ident_param(&p);
                    for (name, def_byte) in names {
                        let fact = if is_simple_param {
                            LocalFact {
                                kind: BindingKind::Param,
                                annotation: annotation.clone(),
                                init: None,
                            }
                        } else {
                            LocalFact {
                                kind: BindingKind::Pattern,
                                annotation: None,
                                init: None,
                            }
                        };
                        out.push((name, def_byte, fact));
                    }
                }
            }
        }
        out
    });
    let names: Vec<_> = params
        .iter()
        .map(|(name, def_byte, _fact)| (name.clone(), *def_byte))
        .collect();
    let facts: Vec<_> = params
        .into_iter()
        .map(|(_name, _def_byte, fact)| fact)
        .collect();
    add_locals_with_facts(b, body_scope, file, body_end, &names, &facts);
}

fn is_simple_ident_param(pat: &Node<'_>) -> bool {
    match pat.kind() {
        "identifier" => true,
        "mut_pattern" | "ref_pattern" => {
            let mut cursor = pat.walk();
            let mut named = pat.named_children(&mut cursor);
            matches!(named.next(), Some(child) if is_simple_ident_param(&child))
                && named.next().is_none()
        }
        _ => false,
    }
}
