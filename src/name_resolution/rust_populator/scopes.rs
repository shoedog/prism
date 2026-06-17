//! Syntax helpers for the Rust populator: visibility / cfg / `use`-path parsing
//! and pattern → local-name extraction. Pure functions over tree-sitter nodes
//! and a `ParsedFile`; no graph mutation here (that is [`super::walk`]).

use tree_sitter::Node;

use crate::ast::ParsedFile;
use crate::name_resolution::rust_policy::{
    VIS_PRIV, VIS_PUB, VIS_PUB_CRATE, VIS_PUB_IN, VIS_PUB_SUPER,
};
use crate::name_resolution::types::{Anchor, Ident, RawPath, ScopeId, Vis, VisKindId};

/// The first named child of `node` of kind `kind`, if any.
pub(crate) fn child_of_kind<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let res = node.children(&mut cursor).find(|c| c.kind() == kind);
    res
}

/// Text of `node`'s `name`-field identifier, if present.
pub(crate) fn name_text(pf: &ParsedFile, node: &Node) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| pf.node_text(&n).to_string())
}

// ── visibility ────────────────────────────────────────────────────────────────

/// Parse an item's `visibility_modifier` child into a [`Vis`] (+ an optional
/// `pub(in path)` restrict-path text the caller resolves to a `ScopeId`).
///
/// Defaults to private (`VIS_PRIV`) when there is no modifier.
pub(crate) fn parse_vis(pf: &ParsedFile, item: &Node) -> (VisKindId, Option<RawPath>) {
    let Some(vm) = child_of_kind(item, "visibility_modifier") else {
        return (VIS_PRIV, None);
    };
    let text = pf.node_text(&vm);
    // `visibility_modifier` text is one of: "pub", "pub(crate)", "pub(super)",
    // "pub(in <path>)". Inspect the inner keyword child.
    if text == "pub" {
        return (VIS_PUB, None);
    }
    // Look for the discriminating inner token.
    let mut cursor = vm.walk();
    let mut saw_in = false;
    let mut restrict: Option<RawPath> = None;
    for c in vm.children(&mut cursor) {
        match c.kind() {
            "crate" => return (VIS_PUB_CRATE, None),
            "super" => return (VIS_PUB_SUPER, None),
            "in" => saw_in = true,
            "scoped_identifier" | "identifier" if saw_in => {
                restrict = Some(scoped_to_path(pf, &c));
            }
            _ => {}
        }
    }
    if restrict.is_some() {
        (VIS_PUB_IN, restrict)
    } else {
        // `pub(self)` or an unrecognized restricted form → conservative private
        // (recall-safe: never widen visibility beyond what we understand).
        (VIS_PRIV, None)
    }
}

/// Build a [`Vis`] from a kind + optional restrict scope.
pub(crate) fn vis(kind: VisKindId, restrict: Option<ScopeId>) -> Vis {
    Vis {
        kind,
        restrict,
        payload: Default::default(),
    }
}

// ── cfg ─────────────────────────────────────────────────────────────────────

/// Parse the `#[cfg(...)]` attribute(s) attached as PRECEDING `attribute_item`
/// siblings of `item` into a [`CfgCond`] formula. Returns `None` (unconditional)
/// when there is no cfg. Conservative on unknown atoms (kept as `Atom`).
pub(crate) fn parse_cfg_for_item(
    pf: &ParsedFile,
    item: &Node,
) -> Option<crate::name_resolution::types::CfgCond> {
    use crate::name_resolution::types::CfgCond;
    // Walk backward over immediately-preceding `attribute_item` siblings.
    let mut conds: Vec<CfgCond> = Vec::new();
    let mut prev = item.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() != "attribute_item" {
            break;
        }
        if let Some(attr) = child_of_kind(&sib, "attribute") {
            // attribute: identifier (arguments) token_tree
            let head = attr
                .named_child(0)
                .map(|n| pf.node_text(&n).to_string())
                .unwrap_or_default();
            if head == "cfg" {
                if let Some(tt) = attr.child_by_field_name("arguments") {
                    if let Some(c) = parse_cfg_tokens(pf, &tt) {
                        conds.push(c);
                    }
                }
            }
        }
        prev = sib.prev_sibling();
    }
    match conds.len() {
        0 => None,
        1 => conds.pop(),
        _ => Some(CfgCond::And(conds)),
    }
}

/// Parse a `cfg(...)` token_tree into a formula. Handles `key`, `key = "val"`,
/// `not(..)`, `all(..)`, `any(..)`; anything else → a conservative `Atom`.
fn parse_cfg_tokens(pf: &ParsedFile, tt: &Node) -> Option<crate::name_resolution::types::CfgCond> {
    use crate::name_resolution::types::CfgCond;
    // The token_tree wraps `( ... )`; find the first meaningful inner token.
    let mut cursor = tt.walk();
    let inner: Vec<Node> = tt
        .children(&mut cursor)
        .filter(|c| !matches!(c.kind(), "(" | ")" | ","))
        .collect();
    let first = inner.first()?;
    let head = pf.node_text(first);
    match head {
        "not" | "all" | "any" => {
            // Next token_tree holds the sub-formula list.
            let sub_tt = inner.iter().find(|c| c.kind() == "token_tree")?;
            let mut sc = sub_tt.walk();
            let subs: Vec<CfgCond> = sub_tt
                .children(&mut sc)
                .filter(|c| c.kind() == "token_tree" || c.kind() == "identifier")
                .filter_map(|c| {
                    if c.kind() == "token_tree" {
                        parse_cfg_tokens(pf, &c)
                    } else {
                        Some(CfgCond::Atom(pf.node_text(&c).to_string(), None))
                    }
                })
                .collect();
            match head {
                "not" => subs.into_iter().next().map(|s| CfgCond::Not(Box::new(s))),
                "all" => Some(CfgCond::And(subs)),
                _ => Some(CfgCond::Or(subs)),
            }
        }
        _ => {
            // `key` or `key = "val"`.
            // Look for a `= "val"` following the head identifier.
            let val = inner
                .iter()
                .find(|c| c.kind() == "string_literal")
                .map(|s| {
                    let t = pf.node_text(s);
                    t.trim_matches('"').to_string()
                });
            Some(CfgCond::Atom(head.to_string(), val))
        }
    }
}

// ── `use` path parsing ────────────────────────────────────────────────────────

/// One thing a `use` declaration brings into scope.
pub(crate) enum UseItem {
    /// A named import/re-export: bind `name` to `Pending(path, anchor)`.
    Named {
        name: Ident,
        path: RawPath,
        anchor: Anchor,
    },
    /// A glob (`use a::*`): a deferred-glob poison edge (no member expansion).
    Glob { path: RawPath, anchor: Anchor },
}

/// Flatten a `use_declaration`'s `argument` into the list of [`UseItem`]s it
/// introduces, given the crate `edition` (for the leading-anchor mapping).
pub(crate) fn parse_use(pf: &ParsedFile, decl: &Node, edition: u16) -> Vec<UseItem> {
    let Some(arg) = decl.child_by_field_name("argument") else {
        return vec![];
    };
    let mut out = Vec::new();
    flatten_use_tree(pf, &arg, &[], edition, &mut out);
    out
}

/// Recursively flatten a use-tree node, accumulating the path `prefix`.
fn flatten_use_tree(
    pf: &ParsedFile,
    node: &Node,
    prefix: &[String],
    edition: u16,
    out: &mut Vec<UseItem>,
) {
    match node.kind() {
        "identifier" | "crate" | "self" | "super" => {
            // A bare leaf (e.g. `use foo;` or a group element `b`).
            let seg = pf.node_text(node).to_string();
            if seg == "self" {
                // `use a::{self}` / `use a::self` → bind the LAST prefix segment
                // (the module `a` itself).
                if let Some(last) = prefix.last() {
                    emit_named(last.clone(), prefix, edition, out);
                }
            } else {
                let mut segs = prefix.to_vec();
                segs.push(seg.clone());
                emit_named(seg, &segs, edition, out);
            }
        }
        "scoped_identifier" => {
            // path::name — collect the full segment list.
            let segs = scoped_segments(pf, node, prefix);
            if let Some(name) = segs.last().cloned() {
                emit_named(name, &segs, edition, out);
            }
        }
        "use_as_clause" => {
            // <path> as <alias>
            let alias = node
                .child_by_field_name("alias")
                .map(|a| pf.node_text(&a).to_string());
            let path_node = node.child_by_field_name("path");
            if let (Some(alias), Some(pn)) = (alias, path_node) {
                let segs = scoped_segments(pf, &pn, prefix);
                emit_named(alias, &segs, edition, out);
            }
        }
        "scoped_use_list" => {
            // <path>::{ list }
            let base = node
                .child_by_field_name("path")
                .map(|p| scoped_segments(pf, &p, prefix))
                .unwrap_or_else(|| prefix.to_vec());
            if let Some(list) = node.child_by_field_name("list") {
                let mut cursor = list.walk();
                for c in list.children(&mut cursor) {
                    if matches!(c.kind(), "{" | "}" | ",") {
                        continue;
                    }
                    flatten_use_tree(pf, &c, &base, edition, out);
                }
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for c in node.children(&mut cursor) {
                if matches!(c.kind(), "{" | "}" | ",") {
                    continue;
                }
                flatten_use_tree(pf, &c, prefix, edition, out);
            }
        }
        "use_wildcard" => {
            let segs = node
                .child_by_field_name("path")
                .or_else(|| node.named_child(0))
                .map(|path| scoped_segments(pf, &path, prefix))
                .unwrap_or_else(|| prefix.to_vec());
            let (anchor, path_segs) = anchor_of_segments(&segs, edition);
            out.push(UseItem::Glob {
                path: RawPath(path_segs),
                anchor,
            });
        }
        _ => {}
    }
}

/// Emit a `Named` use-item, mapping the leading segment to an [`Anchor`] and
/// stripping anchor keywords (`crate`/`self`/`super`) from the resolved path.
fn emit_named(name: String, segs: &[String], edition: u16, out: &mut Vec<UseItem>) {
    let (anchor, path_segs) = anchor_of_segments(segs, edition);
    out.push(UseItem::Named {
        name,
        path: RawPath(path_segs),
        anchor,
    });
}

/// Map a `use`-path's segment list to (anchor, remaining-segments). `crate::`,
/// `self::`, `super::`×n are stripped into the anchor; a bare leading ident uses
/// the edition-sensitive `UsePath` anchor with all segments kept.
fn anchor_of_segments(segs: &[String], edition: u16) -> (Anchor, Vec<String>) {
    if segs.is_empty() {
        return (Anchor::use_path_2015(), vec![]);
    }
    match segs[0].as_str() {
        "crate" => (Anchor::crate_root(), segs[1..].to_vec()),
        "self" => (Anchor::self_mod(), segs[1..].to_vec()),
        "super" => {
            let mut n = 0u32;
            let mut i = 0;
            while i < segs.len() && segs[i] == "super" {
                n += 1;
                i += 1;
            }
            (Anchor::super_n(n), segs[i..].to_vec())
        }
        _ => {
            // A bare leading ident: edition-sensitive. 2015 `use` paths are
            // crate-root-relative; 2018+ lexical/extern-prelude. The policy's
            // `UsePath` anchor encodes this; pass the full segments.
            let _ = edition;
            (Anchor::use_path_2015(), segs.to_vec())
        }
    }
}

/// Collect the dotted segments of a `scoped_identifier` (recursively), prefixed
/// by `prefix`. e.g. `crate::engine::start` → ["crate","engine","start"].
fn scoped_segments(pf: &ParsedFile, node: &Node, prefix: &[String]) -> Vec<String> {
    let mut segs = prefix.to_vec();
    collect_scoped(pf, node, &mut segs);
    segs
}

fn collect_scoped(pf: &ParsedFile, node: &Node, out: &mut Vec<String>) {
    match node.kind() {
        "scoped_identifier" => {
            if let Some(path) = node.child_by_field_name("path") {
                collect_scoped(pf, &path, out);
            }
            if let Some(name) = node.child_by_field_name("name") {
                out.push(pf.node_text(&name).to_string());
            }
        }
        "identifier" | "crate" | "self" | "super" | "type_identifier" => {
            out.push(pf.node_text(node).to_string());
        }
        _ => {}
    }
}

/// A `scoped_identifier`/`identifier` → a `RawPath` (used for `pub(in path)`).
pub(crate) fn scoped_to_path(pf: &ParsedFile, node: &Node) -> RawPath {
    let mut segs = Vec::new();
    collect_scoped(pf, node, &mut segs);
    RawPath(segs)
}

// ── pattern → local names ─────────────────────────────────────────────────────

/// Collect EVERY binding identifier introduced by a Rust pattern node,
/// including non-first / nested ones: `let (_, f)`, `let Point{y: f, ..}`,
/// `match {(_, f) => }`, closure `|(_, f)|`, etc. Skips `_`, type names, and
/// struct field *names* (only the bound pattern under a field is collected).
pub(crate) fn pattern_idents(pf: &ParsedFile, pat: &Node, out: &mut Vec<(String, usize)>) {
    match pat.kind() {
        "identifier" => {
            let t = pf.node_text(pat);
            if t != "_" {
                out.push((t.to_string(), pat.start_byte()));
            }
        }
        // shorthand `ref`/`mut` bindings wrap an identifier
        "mut_pattern" | "ref_pattern" => {
            let mut c = pat.walk();
            for ch in pat.children(&mut c) {
                pattern_idents(pf, &ch, out);
            }
        }
        "tuple_pattern" | "tuple_struct_pattern" | "slice_pattern" | "or_pattern" => {
            // Children are sub-patterns; for tuple_struct the `(type)` field is
            // the constructor name — skip it, walk the rest.
            let type_field = pat.child_by_field_name("type");
            let mut c = pat.walk();
            for ch in pat.children(&mut c) {
                if Some(ch) == type_field {
                    continue;
                }
                if matches!(
                    ch.kind(),
                    "(" | ")" | "," | "[" | "]" | "_" | "::" | "|" | ".."
                ) {
                    continue;
                }
                pattern_idents(pf, &ch, out);
            }
        }
        "struct_pattern" => {
            // field_pattern: (name) field_identifier : (pattern) <pat>
            //                OR shorthand `field_identifier` (binds that name).
            let mut c = pat.walk();
            for ch in pat.children(&mut c) {
                match ch.kind() {
                    "field_pattern" => {
                        if let Some(p) = ch.child_by_field_name("pattern") {
                            pattern_idents(pf, &p, out);
                        } else {
                            // shorthand `{ x }` binds `x`
                            if let Some(n) = ch.child_by_field_name("name") {
                                let t = pf.node_text(&n);
                                if t != "_" {
                                    out.push((t.to_string(), n.start_byte()));
                                }
                            }
                        }
                    }
                    "remaining_field_pattern" => {}
                    _ => {}
                }
            }
        }
        "ref_mut_pattern" | "reference_pattern" | "captured_pattern" => {
            let mut c = pat.walk();
            for ch in pat.children(&mut c) {
                pattern_idents(pf, &ch, out);
            }
        }
        _ => {
            // Unknown/literal/range/wildcard pattern → no bindings (recall-safe:
            // we never invent a name we cannot see).
        }
    }
}
