//! Type- and macro-definition handlers: `struct`/`enum`/`union`, `trait`,
//! `impl` (+ its associated items), `const`/`static`, `type` aliases, and the
//! `macro_rules!` / `macro_invocation` markers.
//!
//! The recall-safety rule lives here: associated `impl`/`trait` items and enum
//! variants are bound into the **type's owned `Type` scope** (path-reachable via
//! `T::m` / `E::V`), and method bodies are re-parented to the enclosing module
//! (see [`super::items::walk_function`] `assoc=true`) so they are NOT
//! bare-visible.

use crate::name_resolution::rust_policy::{NS_MACRO, NS_TYPE, NS_VALUE, VIS_PRIV};
use crate::name_resolution::types::{BindTarget, ScopeId, Target};

use super::super::builder::Builder;
use super::super::scopes::{name_text, parse_cfg_for_item, parse_vis, vis};
use super::items::walk_function;
use super::{
    bare_type_name, full_scope_span, named_children, node_range, scope_end_byte, type_name_text,
    with_node, Ctx, NodeId,
};

pub(in crate::name_resolution::rust_populator::walk) fn walk_struct(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let (name, vis_kind, cond, lo, hi, is_unit_or_tuple) = with_node(b, path, nid, |pf, n| {
        let body = n.child_by_field_name("body");
        // unit struct = no body; tuple struct = ordered_field_declaration_list.
        let unit_or_tuple = match &body {
            None => true,
            Some(x) => x.kind() == "ordered_field_declaration_list",
        };
        (
            type_name_text(pf, n),
            parse_vis(pf, n).0,
            parse_cfg_for_item(pf, n),
            n.start_byte(),
            n.end_byte(),
            unit_or_tuple,
        )
    });
    if name.is_empty() {
        return;
    }
    let v = vis(vis_kind, None);
    let type_scope = b.type_scope_for(scope, &name, ctx.file, lo, hi);
    let item = b.fresh_item();
    // Type-namespace binding → Item{owns: type_scope} so `S::assoc` paths descend.
    b.add_binding(
        scope,
        name.clone(),
        NS_TYPE,
        BindTarget::Resolved(Target::Item {
            id: item,
            ns: NS_TYPE,
            owns: Some(type_scope),
            callable: false,
        }),
        v.clone(),
        cond.clone(),
        vec![full_scope_span(b, scope, ctx.file)],
    );
    // unit/tuple struct also binds a Value constructor (callable for tuple).
    if is_unit_or_tuple {
        let ctor = b.fresh_item();
        b.add_binding(
            scope,
            name,
            NS_VALUE,
            BindTarget::Resolved(Target::Item {
                id: ctor,
                ns: NS_VALUE,
                owns: None,
                callable: true,
            }),
            v,
            cond,
            vec![full_scope_span(b, scope, ctx.file)],
        );
    }
}

pub(in crate::name_resolution::rust_populator::walk) fn walk_enum(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let (name, vis_kind, cond, lo, hi, variants) = with_node(b, path, nid, |pf, n| {
        let mut variants: Vec<(String, usize)> = Vec::new();
        if let Some(body) = n.child_by_field_name("body") {
            let mut c = body.walk();
            for ch in body.children(&mut c) {
                if ch.kind() == "enum_variant" {
                    if let Some(nm) = ch.child_by_field_name("name") {
                        variants.push((pf.node_text(&nm).to_string(), ch.start_byte()));
                    }
                }
            }
        }
        (
            type_name_text(pf, n),
            parse_vis(pf, n).0,
            parse_cfg_for_item(pf, n),
            n.start_byte(),
            n.end_byte(),
            variants,
        )
    });
    if name.is_empty() {
        return;
    }
    let v = vis(vis_kind, None);
    let type_scope = b.type_scope_for(scope, &name, ctx.file, lo, hi);
    let item = b.fresh_item();
    b.add_binding(
        scope,
        name,
        NS_TYPE,
        BindTarget::Resolved(Target::Item {
            id: item,
            ns: NS_TYPE,
            owns: Some(type_scope),
            callable: false,
        }),
        v.clone(),
        cond.clone(),
        vec![full_scope_span(b, scope, ctx.file)],
    );
    // Enum variants → Value bindings in the enum's Type scope (path-reachable as
    // `E::Variant`, NOT bare-visible — same re-parent rule as assoc items).
    for (vn, _vb) in variants {
        let vi = b.fresh_item();
        b.add_binding(
            type_scope,
            vn,
            NS_VALUE,
            BindTarget::Resolved(Target::Item {
                id: vi,
                ns: NS_VALUE,
                owns: None,
                callable: true,
            }),
            v.clone(),
            cond.clone(),
            vec![full_scope_span(b, type_scope, ctx.file)],
        );
    }
}

/// trait_item: a Type scope + (signatures are path-reachable assoc items).
pub(in crate::name_resolution::rust_populator::walk) fn walk_type_like(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
    _is_trait: bool,
) {
    let (name, vis_kind, cond, lo, hi, body_nid) = with_node(b, path, nid, |pf, n| {
        (
            type_name_text(pf, n),
            parse_vis(pf, n).0,
            parse_cfg_for_item(pf, n),
            n.start_byte(),
            n.end_byte(),
            n.child_by_field_name("body").map(NodeId::of),
        )
    });
    if name.is_empty() {
        return;
    }
    let v = vis(vis_kind, None);
    let type_scope = b.type_scope_for(scope, &name, ctx.file, lo, hi);
    let item = b.fresh_item();
    b.add_binding(
        scope,
        name,
        NS_TYPE,
        BindTarget::Resolved(Target::Item {
            id: item,
            ns: NS_TYPE,
            owns: Some(type_scope),
            callable: false,
        }),
        v,
        cond,
        vec![full_scope_span(b, scope, ctx.file)],
    );
    if let Some(body_nid) = body_nid {
        walk_assoc_items(b, path, &body_nid, type_scope, ctx);
    }
}

pub(in crate::name_resolution::rust_populator::walk) fn walk_impl(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    // impl <Trait for>? <Type> { ... } — the `type` field is the implementing
    // type; associated items go into ITS type scope.
    let (type_name, body_nid) = with_node(b, path, nid, |pf, n| {
        let ty = n
            .child_by_field_name("type")
            .map(|t| pf.node_text(&t).to_string());
        (
            ty.unwrap_or_default(),
            n.child_by_field_name("body").map(NodeId::of),
        )
    });
    if type_name.is_empty() {
        return;
    }
    let bare = bare_type_name(&type_name);
    // The type scope spans the impl block (so a path member byte falls inside).
    let (lo, hi) = node_range(nid);
    let type_scope = b.type_scope_for(scope, &bare, ctx.file, lo, hi);
    if let Some(body_nid) = body_nid {
        walk_assoc_items(b, path, &body_nid, type_scope, ctx);
    }
}

/// Walk an impl/trait `declaration_list`: associated fns/consts → bindings in
/// the TYPE scope (path-reachable via `T::`), method bodies re-parented to the
/// enclosing MODULE (`ctx.module`) so they are NOT bare-visible.
fn walk_assoc_items(
    b: &mut Builder<'_>,
    path: &str,
    body_nid: &NodeId,
    type_scope: ScopeId,
    ctx: &Ctx,
) {
    let items = with_node(b, path, body_nid, |_pf, n| named_children(n));
    for it in items {
        match it.kind.as_str() {
            "function_item" | "function_signature_item" => {
                let (name, vis_kind, cond) = with_node(b, path, &it, |pf, n| {
                    (
                        name_text(pf, n).unwrap_or_default(),
                        parse_vis(pf, n).0,
                        parse_cfg_for_item(pf, n),
                    )
                });
                if !name.is_empty() {
                    let item = b.fresh_item();
                    b.add_binding(
                        type_scope,
                        name,
                        NS_VALUE,
                        BindTarget::Resolved(Target::Item {
                            id: item,
                            ns: NS_VALUE,
                            owns: None,
                            callable: true,
                        }),
                        vis(vis_kind, None),
                        cond,
                        vec![full_scope_span(b, type_scope, ctx.file)],
                    );
                }
                // ...and build the BODY re-parented to the module (assoc=true).
                walk_function(b, path, &it, type_scope, ctx, /*assoc=*/ true);
            }
            "const_item" | "static_item" => {
                let (name, vis_kind, cond) = with_node(b, path, &it, |pf, n| {
                    (
                        name_text(pf, n).unwrap_or_default(),
                        parse_vis(pf, n).0,
                        parse_cfg_for_item(pf, n),
                    )
                });
                if !name.is_empty() {
                    let item = b.fresh_item();
                    b.add_binding(
                        type_scope,
                        name,
                        NS_VALUE,
                        BindTarget::Resolved(Target::Item {
                            id: item,
                            ns: NS_VALUE,
                            owns: None,
                            callable: false,
                        }),
                        vis(vis_kind, None),
                        cond,
                        vec![full_scope_span(b, type_scope, ctx.file)],
                    );
                }
            }
            _ => {}
        }
    }
}

// ── const / static / type alias ─────────────────────────────────────────────────

pub(in crate::name_resolution::rust_populator::walk) fn walk_value_item(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let (name, vis_kind, cond) = with_node(b, path, nid, |pf, n| {
        (
            name_text(pf, n).unwrap_or_default(),
            parse_vis(pf, n).0,
            parse_cfg_for_item(pf, n),
        )
    });
    if name.is_empty() {
        return;
    }
    let item = b.fresh_item();
    b.add_binding(
        scope,
        name,
        NS_VALUE,
        BindTarget::Resolved(Target::Item {
            id: item,
            ns: NS_VALUE,
            owns: None,
            callable: false,
        }),
        vis(vis_kind, None),
        cond,
        vec![full_scope_span(b, scope, ctx.file)],
    );
}

pub(in crate::name_resolution::rust_populator::walk) fn walk_type_alias(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    let (name, vis_kind, cond) = with_node(b, path, nid, |pf, n| {
        (
            type_name_text(pf, n),
            parse_vis(pf, n).0,
            parse_cfg_for_item(pf, n),
        )
    });
    if name.is_empty() {
        return;
    }
    let item = b.fresh_item();
    b.add_binding(
        scope,
        name,
        NS_TYPE,
        BindTarget::Resolved(Target::Item {
            id: item,
            ns: NS_TYPE,
            owns: None,
            callable: false,
        }),
        vis(vis_kind, None),
        cond,
        vec![full_scope_span(b, scope, ctx.file)],
    );
}

// ── macros ────────────────────────────────────────────────────────────────────

pub(in crate::name_resolution::rust_populator::walk) fn walk_macro_def(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    // `macro_rules! name { ... }` introduces a Macro-ns name. (Its textual-scope
    // shadowing is Phase 3; the invocation wildcard handles name-introduction.)
    let name = with_node(b, path, nid, |pf, n| name_text(pf, n).unwrap_or_default());
    if name.is_empty() {
        return;
    }
    let item = b.fresh_item();
    b.add_binding(
        scope,
        name,
        NS_MACRO,
        BindTarget::Resolved(Target::Item {
            id: item,
            ns: NS_MACRO,
            owns: None,
            callable: false,
        }),
        vis(VIS_PRIV, None),
        None,
        vec![full_scope_span(b, scope, ctx.file)],
    );
}

pub(in crate::name_resolution::rust_populator::walk) fn walk_macro_invocation(
    b: &mut Builder<'_>,
    _path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    // A name-introducing item-position macro can emit UNKNOWABLE names → a
    // wildcard poison over the Value + Type + Macro namespaces from the
    // invocation byte to the END of the enclosing scope (§4.3b).
    let inv_lo = node_range(nid).0;
    let scope_end = scope_end_byte(b, scope, ctx.file);
    for ns in [NS_VALUE, NS_TYPE, NS_MACRO] {
        b.add_macro_wildcard(scope, ns, inv_lo, scope_end, ctx.file);
    }
}
