//! The identity function `ident` (DESIGN.md 4.4): the only builder of census paths.
//!
//! A path is a base name plus components, all raw source spellings. Nothing here decodes an escape:
//! an owner holding a `\` in a NAME token is refused before any path is built.

use super::tables::Tables;
use tree_sitter::Node;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Comp {
    /// A static property (`.x`, `.#p`), raw spelling.
    Name(String),
    /// A subscript of any index (`[]`).
    Index,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Path {
    pub base: String,
    pub comps: Vec<Comp>,
}

impl Path {
    pub(crate) fn render(&self) -> String {
        let mut s = self.base.clone();
        for c in &self.comps {
            match c {
                Comp::Name(n) => {
                    s.push('.');
                    s.push_str(n);
                }
                Comp::Index => s.push_str("[]"),
            }
        }
        s
    }

    /// Same base and same component sequence, components compared as raw strings.
    pub(crate) fn eq_path(&self, other: &Path) -> bool {
        self == other
    }

    /// `self`'s components are a prefix of `other`'s (same length allowed) on the same base, where
    /// an index component matches any component on either side.
    pub(crate) fn affects(&self, other: &Path) -> bool {
        self.base == other.base
            && self.comps.len() <= other.comps.len()
            && self
                .comps
                .iter()
                .zip(&other.comps)
                .all(|(a, b)| matches!((a, b), (Comp::Index, _) | (_, Comp::Index)) || a == b)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ident {
    Path(Path),
    /// No owner binding is written (a call, literal or `new` root).
    NoWrite,
    /// A shape the rules do not cover: the owner is refused. Carries the offending kind.
    Refuse(String),
}

fn raw(src: &[u8], n: Node<'_>) -> String {
    String::from_utf8_lossy(&src[n.start_byte()..n.end_byte()]).into_owned()
}

/// Build the identity of a target or chain node. `top` is true for the node in the target slot
/// itself (a refused kind there refuses the owner) and false for a chain root (a refused kind
/// there writes no owner binding).
pub(crate) fn ident(t: &Tables, src: &[u8], n: Node<'_>, top: bool) -> Ident {
    let kind = n.kind();
    let Some(row) = t.targets.get(kind) else {
        return Ident::Refuse(kind.to_string());
    };
    match row.action.as_str() {
        "NAME" => Ident::Path(Path {
            base: raw(src, n),
            comps: Vec::new(),
        }),
        "PATH" => {
            let Some(obj) = n.child_by_field_name("object") else {
                return Ident::Refuse(kind.to_string());
            };
            let comp = if kind == "subscript_expression" {
                Comp::Index
            } else {
                match n.child_by_field_name("property") {
                    Some(p)
                        if matches!(
                            p.kind(),
                            "property_identifier" | "private_property_identifier"
                        ) =>
                    {
                        Comp::Name(raw(src, p))
                    }
                    _ => return Ident::Refuse(kind.to_string()),
                }
            };
            match ident(t, src, obj, false) {
                Ident::Path(mut p) => {
                    p.comps.push(comp);
                    Ident::Path(p)
                }
                other => other,
            }
        }
        "WRAP" => match super::walk::wrap_operand(n) {
            Some(op) => ident(t, src, op, top),
            None => Ident::Refuse(kind.to_string()),
        },
        "NONE" if kind == "this" && !top => Ident::Path(Path {
            base: "this".to_string(),
            comps: Vec::new(),
        }),
        "NONE" => Ident::NoWrite,
        "REFUSE" if kind == "super" && !top => Ident::Path(Path {
            base: "this".to_string(),
            comps: Vec::new(),
        }),
        "REFUSE" if top => Ident::Refuse(kind.to_string()),
        "REFUSE" => Ident::NoWrite,
        _ => Ident::Refuse(kind.to_string()),
    }
}
