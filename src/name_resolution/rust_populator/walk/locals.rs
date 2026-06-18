//! Block / statement / expression descent + local-pattern extraction.
//!
//! Walks fn/closure bodies, nested blocks, `let`, and the pattern-introducing
//! control-flow forms (`for`/`match`/`if let`/`while let`), minting `Block`/
//! `Callable` scopes and `Target::Local` bindings. Item-statements inside a
//! block delegate back to [`super::items`] via [`super::walk_item`]-style calls.

use tree_sitter::Node;

use crate::ast::ParsedFile;
use crate::name_resolution::binding_lookup::{BindingKind, InitExpr, LocalFact};
use crate::name_resolution::rust_policy::{NS_VALUE, VIS_PUB};
use crate::name_resolution::types::{
    BindTarget, BindingRef, FileId, ScopeId, ScopeKind, SourceLoc, Span, Target,
};

use super::super::builder::Builder;
use super::super::scopes::{pattern_idents, vis};
use super::items::{walk_function, walk_mod_item, walk_use};
use super::types::{
    walk_enum, walk_impl, walk_macro_def, walk_macro_invocation, walk_struct, walk_value_item,
};
use super::{named_children, node_range, scope_end_byte, with_node, Ctx, NodeId};

/// Walk a fn-body `block`: a sequence of statements introducing locals (`let`),
/// nested blocks, `use`, macros, control-flow with patterns.
pub(in crate::name_resolution::rust_populator::walk) fn walk_block_body(
    b: &mut Builder<'_>,
    path: &str,
    body_nid: &NodeId,
    block_scope: ScopeId,
    ctx: &Ctx,
) {
    let stmts = with_node(b, path, body_nid, |_pf, n| named_children(n));
    for st in stmts {
        walk_stmt(b, path, st, block_scope, ctx);
    }
}

/// Dispatch a statement / expression that may introduce locals or sub-scopes.
fn walk_stmt(b: &mut Builder<'_>, path: &str, nid: NodeId, scope: ScopeId, ctx: &Ctx) {
    match nid.kind.as_str() {
        "let_declaration" => walk_let(b, path, &nid, scope, ctx),
        "use_declaration" => walk_use(b, path, &nid, scope, ctx),
        "macro_definition" => walk_macro_def(b, path, &nid, scope, ctx),
        // Item definitions ARE allowed inside a block (item statements).
        "function_item" => walk_function(b, path, &nid, scope, ctx, false),
        "struct_item" | "union_item" => walk_struct(b, path, &nid, scope, ctx),
        "enum_item" => walk_enum(b, path, &nid, scope, ctx),
        "impl_item" => walk_impl(b, path, &nid, scope, ctx),
        "const_item" | "static_item" => walk_value_item(b, path, &nid, scope, ctx),
        "mod_item" => walk_mod_item(b, path, &nid, scope, ctx),
        "expression_statement" | "block" => walk_stmt_or_block(b, path, &nid, scope, ctx),
        _ => {
            // Other expressions may still contain macro invocations / closures /
            // control flow with patterns → descend.
            walk_expr(b, path, &nid, scope, ctx);
        }
    }
}

fn walk_let(b: &mut Builder<'_>, path: &str, nid: &NodeId, scope: ScopeId, ctx: &Ctx) {
    let (names, annotation, init, kind, value_nid) = with_node(b, path, nid, |pf, n| {
        let mut names = Vec::new();
        let mut simple_pattern = false;
        if let Some(p) = n.child_by_field_name("pattern") {
            simple_pattern = p.kind() == "identifier";
            pattern_idents(pf, &p, &mut names);
        }
        let annotation = n
            .child_by_field_name("type")
            .map(|ty| pf.node_text(&ty).trim().to_string())
            .filter(|s| !s.is_empty());
        let value = n.child_by_field_name("value");
        let init = value.and_then(|value| init_expr(pf, &value));
        let kind = if simple_pattern {
            BindingKind::Let
        } else {
            BindingKind::Pattern
        };
        (names, annotation, init, kind, value.map(NodeId::of))
    });
    let scope_end = scope_end_byte(b, scope, ctx.file);
    add_locals(
        b,
        scope,
        ctx.file,
        scope_end,
        &names,
        LocalFact {
            kind,
            annotation,
            init,
        },
    );
    // The initializer expression may contain closures / blocks / macros.
    if let Some(value_nid) = value_nid {
        walk_expr(b, path, &value_nid, scope, ctx);
    }
}

/// Walk an `expression_statement` or `block` node: a nested `{}` becomes a new
/// `Block` scope; otherwise descend into the expression.
pub(in crate::name_resolution::rust_populator::walk) fn walk_stmt_or_block(
    b: &mut Builder<'_>,
    path: &str,
    nid: &NodeId,
    scope: ScopeId,
    ctx: &Ctx,
) {
    if nid.kind == "block" {
        let (lo, hi) = node_range(nid);
        let inner = b.add_scope(ScopeKind::Block, Some(scope), ctx.file, lo, hi, None);
        walk_block_body(b, path, nid, inner, ctx);
    } else {
        walk_expr(b, path, nid, scope, ctx);
    }
}

/// Generic expression descent: handle the pattern-introducing control-flow forms
/// (`for`/`match`/`if let`/`while let`/closures) + nested blocks + macros, and
/// recurse into all other children.
fn walk_expr(b: &mut Builder<'_>, path: &str, nid: &NodeId, scope: ScopeId, ctx: &Ctx) {
    match nid.kind.as_str() {
        "block" => {
            walk_stmt_or_block(b, path, nid, scope, ctx);
            return;
        }
        "macro_invocation" => {
            walk_macro_invocation(b, path, nid, scope, ctx);
            // fall through to descend (args may contain more)
        }
        "closure_expression" => {
            walk_closure(b, path, nid, scope, ctx);
            return;
        }
        "for_expression" => {
            walk_for(b, path, nid, scope, ctx);
            return;
        }
        "match_expression" => {
            walk_match(b, path, nid, scope, ctx);
            return;
        }
        "if_expression" | "while_expression" => {
            walk_if_while(b, path, nid, scope, ctx);
            return;
        }
        _ => {}
    }
    // Default: recurse into named children within the same scope.
    let children = with_node(b, path, nid, |_pf, n| named_children(n));
    for child in children {
        walk_expr(b, path, &child, scope, ctx);
    }
}

fn walk_closure(b: &mut Builder<'_>, path: &str, nid: &NodeId, scope: ScopeId, ctx: &Ctx) {
    let (names, body_nid, lo, hi) = with_node(b, path, nid, |pf, n| {
        let mut names = Vec::new();
        if let Some(params) = n.child_by_field_name("parameters") {
            let mut c = params.walk();
            for ch in params.children(&mut c) {
                if matches!(ch.kind(), "|" | ",") {
                    continue;
                }
                pattern_idents(pf, &ch, &mut names);
            }
        }
        (
            names,
            n.child_by_field_name("body").map(NodeId::of),
            n.start_byte(),
            n.end_byte(),
        )
    });
    // A closure body is a Callable scope; its args are locals there.
    let body_scope = b.add_scope(ScopeKind::Callable, Some(scope), ctx.file, lo, hi, None);
    add_locals(
        b,
        body_scope,
        ctx.file,
        hi,
        &names,
        LocalFact {
            kind: BindingKind::Param,
            annotation: None,
            init: None,
        },
    );
    if let Some(body_nid) = body_nid {
        walk_expr(b, path, &body_nid, body_scope, ctx);
    }
}

fn walk_for(b: &mut Builder<'_>, path: &str, nid: &NodeId, scope: ScopeId, ctx: &Ctx) {
    let (names, body_nid, value_nid, lo, hi) = with_node(b, path, nid, |pf, n| {
        let mut names = Vec::new();
        if let Some(p) = n.child_by_field_name("pattern") {
            pattern_idents(pf, &p, &mut names);
        }
        (
            names,
            n.child_by_field_name("body").map(NodeId::of),
            n.child_by_field_name("value").map(NodeId::of),
            n.start_byte(),
            n.end_byte(),
        )
    });
    // The loop variable scopes over the loop body block.
    let loop_scope = b.add_scope(ScopeKind::Block, Some(scope), ctx.file, lo, hi, None);
    add_locals(
        b,
        loop_scope,
        ctx.file,
        hi,
        &names,
        LocalFact {
            kind: BindingKind::Pattern,
            annotation: None,
            init: None,
        },
    );
    if let Some(value_nid) = value_nid {
        walk_expr(b, path, &value_nid, scope, ctx); // iterator expr is in outer scope
    }
    if let Some(body_nid) = body_nid {
        walk_block_body(b, path, &body_nid, loop_scope, ctx);
    }
}

fn walk_match(b: &mut Builder<'_>, path: &str, nid: &NodeId, scope: ScopeId, ctx: &Ctx) {
    // Each match arm introduces its pattern's bindings over the arm body.
    let (value_nid, arms) = with_node(b, path, nid, |_pf, n| {
        let value = n.child_by_field_name("value").map(NodeId::of);
        let mut arms = Vec::new();
        if let Some(body) = n.child_by_field_name("body") {
            let mut c = body.walk();
            for ch in body.children(&mut c) {
                if ch.kind() == "match_arm" {
                    arms.push(NodeId::of(ch));
                }
            }
        }
        (value, arms)
    });
    if let Some(value_nid) = value_nid {
        walk_expr(b, path, &value_nid, scope, ctx);
    }
    for arm in arms {
        let (names, arm_value, lo, hi) = with_node(b, path, &arm, |pf, n| {
            let mut names = Vec::new();
            if let Some(p) = n.child_by_field_name("pattern") {
                collect_match_pattern(pf, &p, &mut names);
            }
            (
                names,
                n.child_by_field_name("value").map(NodeId::of),
                n.start_byte(),
                n.end_byte(),
            )
        });
        let arm_scope = b.add_scope(ScopeKind::Block, Some(scope), ctx.file, lo, hi, None);
        add_locals(
            b,
            arm_scope,
            ctx.file,
            hi,
            &names,
            LocalFact {
                kind: BindingKind::Pattern,
                annotation: None,
                init: None,
            },
        );
        if let Some(arm_value) = arm_value {
            walk_expr(b, path, &arm_value, arm_scope, ctx);
        }
    }
}

fn walk_if_while(b: &mut Builder<'_>, path: &str, nid: &NodeId, scope: ScopeId, ctx: &Ctx) {
    // `if let`/`while let`: the `let_condition`'s pattern bindings scope over the
    // consequence/body block. We attach them to a Block scope covering the whole
    // if/while node (a safe over-approximation of the let-binding extent).
    let (names, blocks, lo, hi) = with_node(b, path, nid, |pf, n| {
        let mut names = Vec::new();
        let mut blocks = Vec::new();
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            match ch.kind() {
                "let_condition" => {
                    if let Some(p) = ch.child_by_field_name("pattern") {
                        pattern_idents(pf, &p, &mut names);
                    }
                }
                "block" => blocks.push(NodeId::of(ch)),
                _ => {}
            }
        }
        (names, blocks, n.start_byte(), n.end_byte())
    });
    let cond_scope = b.add_scope(ScopeKind::Block, Some(scope), ctx.file, lo, hi, None);
    add_locals(
        b,
        cond_scope,
        ctx.file,
        hi,
        &names,
        LocalFact {
            kind: BindingKind::Pattern,
            annotation: None,
            init: None,
        },
    );
    for blk in blocks {
        walk_block_body(b, path, &blk, cond_scope, ctx);
    }
}

/// A `match` arm pattern wraps the actual pattern in `match_pattern`.
fn collect_match_pattern(pf: &ParsedFile, node: &Node, out: &mut Vec<(String, usize)>) {
    if node.kind() == "match_pattern" {
        let mut c = node.walk();
        for ch in node.children(&mut c) {
            if ch.kind() == "if" {
                break; // guard expression — not a binding
            }
            pattern_idents(pf, &ch, out);
        }
    } else {
        pattern_idents(pf, node, out);
    }
}

/// Add a `Target::Local` binding (Value ns) for each `(name, def_byte)`, visible
/// from its def byte to `scope_end`.
///
/// A local's accessibility is its lexical extent, not a Rust `pub`; `VIS_PUB`
/// makes the policy's `visible()` return true (the gate is the `vis_extents`).
pub(in crate::name_resolution::rust_populator::walk) fn add_locals(
    b: &mut Builder<'_>,
    scope: ScopeId,
    file: FileId,
    scope_end: usize,
    names: &[(String, usize)],
    fact: LocalFact,
) {
    let facts = vec![fact; names.len()];
    add_locals_with_facts(b, scope, file, scope_end, names, &facts);
}

pub(in crate::name_resolution::rust_populator::walk) fn add_locals_with_facts(
    b: &mut Builder<'_>,
    scope: ScopeId,
    file: FileId,
    scope_end: usize,
    names: &[(String, usize)],
    facts: &[LocalFact],
) {
    assert_eq!(names.len(), facts.len());
    for (i, ((name, def_byte), fact)) in names.iter().zip(facts).enumerate() {
        b.add_local_fact(file, *def_byte, fact.clone());
        b.add_binding(
            scope,
            name.clone(),
            NS_VALUE,
            BindTarget::Resolved(Target::Local(BindingRef {
                scope,
                ordinal: i as u32,
            })),
            vis(VIS_PUB, None),
            None,
            vec![Span {
                lo: SourceLoc {
                    file,
                    byte: *def_byte,
                },
                hi: SourceLoc {
                    file,
                    byte: scope_end.max(def_byte + 1),
                },
            }],
        );
    }
}

fn init_expr(pf: &ParsedFile, value: &Node) -> Option<InitExpr> {
    match value.kind() {
        "call_expression" => {
            let function = value
                .child_by_field_name("function")
                .or_else(|| value.child_by_field_name("name"))?;
            let function_text = pf.node_text(&function).trim();
            if let Some((_ty, ctor)) = function_text.rsplit_once("::") {
                if matches!(ctor, "new" | "default") {
                    return Some(InitExpr::Ctor(format!("{function_text}()")));
                }
            }
            Some(InitExpr::Call(format!("{function_text}(...)")))
        }
        "struct_expression" => {
            let ty = value
                .child_by_field_name("name")
                .or_else(|| value.child_by_field_name("type"))?;
            Some(InitExpr::Ctor(format!("{}{{}}", pf.node_text(&ty).trim())))
        }
        "field_expression" => Some(InitExpr::Field(pf.node_text(value).trim().to_string())),
        _ => Some(InitExpr::Other),
    }
}
