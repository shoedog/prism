//! Call-position and receiver helpers shared by the per-language sink rules.
//!
//! A flat sink identifier is only a sink when it is the callee of a call (a bare
//! callee, or the property/field/attribute of a member callee). These helpers
//! find that call from an identifier node and read the receiver chain and the
//! arguments, for JS/TS (`call_expression`, `new_expression`), Python (`call`)
//! and Go (`call_expression` with a `selector_expression` callee).

use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

/// Source text and dialect, so the rules also run from a bare `CallSite`.
#[derive(Clone, Copy)]
pub(super) struct Src<'a> {
    text: &'a str,
    python: bool,
}

impl<'a> Src<'a> {
    pub(super) fn new(text: &'a str, language: Language) -> Self {
        Self {
            text,
            python: language == Language::Python,
        }
    }

    pub(super) fn of(parsed: &'a ParsedFile) -> Self {
        Self::new(&parsed.source, parsed.language)
    }

    pub(super) fn node_text(&self, node: &Node<'_>) -> &'a str {
        node.utf8_text(self.text.as_bytes()).unwrap_or("")
    }
}

/// The call an identifier is the callee of.
pub(super) struct Callee<'a> {
    /// The call node (`call_expression`, `call` or `new_expression`).
    pub call: Node<'a>,
    /// Object/operand of a member callee; `None` for a bare callee.
    pub receiver: Option<Node<'a>>,
    /// `new X(...)`.
    pub is_new: bool,
}

fn is_field<'a>(parent: Node<'a>, field: &str, child: Node<'a>) -> bool {
    parent.child_by_field_name(field) == Some(child)
}

/// Climb out of `(0, eval)`-style wrappers around a bare callee.
fn climb_wrappers(id: Node<'_>) -> Node<'_> {
    let mut node = id;
    while let Some(parent) = node.parent() {
        let climb = match parent.kind() {
            // The grammar has no field names here: the callee is the last element.
            "sequence_expression" => {
                parent.named_child(parent.named_child_count().saturating_sub(1)) == Some(node)
            }
            "parenthesized_expression" => true,
            _ => false,
        };
        if !climb {
            break;
        }
        node = parent;
    }
    node
}

/// Returns the call whose callee `id` names, or `None` when `id` is not in
/// callee position (a variable, a property read, a signature, a type).
pub(super) fn callee_of(id: Node<'_>) -> Option<Callee<'_>> {
    let parent = id.parent()?;
    let (callee, receiver) = match parent.kind() {
        "member_expression" | "attribute" | "selector_expression" => {
            let (name_field, object_field) = match parent.kind() {
                "member_expression" => ("property", "object"),
                "attribute" => ("attribute", "object"),
                _ => ("field", "operand"),
            };
            if !is_field(parent, name_field, id) {
                return None;
            }
            (parent, parent.child_by_field_name(object_field))
        }
        _ => (climb_wrappers(id), None),
    };
    let call = callee.parent()?;
    let is_new = match call.kind() {
        "call_expression" | "call" => {
            if !is_field(call, "function", callee) {
                return None;
            }
            false
        }
        "new_expression" => {
            if !is_field(call, "constructor", callee) {
                return None;
            }
            true
        }
        _ => return None,
    };
    Some(Callee {
        call,
        receiver,
        is_new,
    })
}

/// Named argument nodes of a call, comments excluded.
pub(super) fn args(call: Node<'_>) -> Vec<Node<'_>> {
    let Some(list) = call.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut cursor = list.walk();
    list.named_children(&mut cursor)
        .filter(|n| n.kind() != "comment")
        .collect()
}

/// Text of the last segment of a receiver chain (`s.db` -> `db`, `res` -> `res`).
pub(super) fn chain_tail<'p>(p: Src<'p>, node: Node<'_>) -> Option<&'p str> {
    match node.kind() {
        "identifier" | "property_identifier" | "field_identifier" | "this" => {
            Some(p.node_text(&node))
        }
        "member_expression" => chain_tail(p, node.child_by_field_name("property")?),
        "attribute" => chain_tail(p, node.child_by_field_name("attribute")?),
        "selector_expression" => chain_tail(p, node.child_by_field_name("field")?),
        "parenthesized_expression" | "non_null_expression" | "await_expression" => {
            chain_tail(p, node.named_child(0)?)
        }
        _ => None,
    }
}

/// Content of a string literal that has no interpolation, without its quotes.
pub(super) fn literal_string<'p>(p: Src<'p>, node: Node<'_>) -> Option<&'p str> {
    let text = p.node_text(&node);
    match node.kind() {
        // JS/TS: `string` is always literal; a template is literal without `${}`.
        "string" if !p.python => unquote(text),
        "template_string" => {
            let mut cursor = node.walk();
            let substituted = node
                .named_children(&mut cursor)
                .any(|c| c.kind() == "template_substitution");
            if substituted {
                None
            } else {
                unquote(text)
            }
        }
        // Python: an f-string with `{}` carries `interpolation` children.
        "string" => {
            let mut cursor = node.walk();
            let interpolated = node
                .named_children(&mut cursor)
                .any(|c| c.kind() == "interpolation");
            if interpolated {
                None
            } else {
                unquote(text.trim_start_matches(|c: char| c.is_ascii_alphabetic()))
            }
        }
        // Go.
        "interpreted_string_literal" | "raw_string_literal" => unquote(text),
        _ => None,
    }
}

fn unquote(text: &str) -> Option<&str> {
    for quote in ["\"\"\"", "'''", "\"", "'", "`"] {
        if let Some(inner) = text
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return Some(inner);
        }
    }
    None
}

/// True for `a + b` chains of literals (Go constant format strings).
pub(super) fn is_literal_concat(p: Src<'_>, node: Node<'_>) -> bool {
    if literal_string(p, node).is_some() {
        return true;
    }
    if node.kind() == "binary_expression" {
        let (Some(l), Some(r)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return false;
        };
        return is_literal_concat(p, l) && is_literal_concat(p, r);
    }
    if node.kind() == "parenthesized_expression" {
        return node
            .named_child(0)
            .is_some_and(|inner| is_literal_concat(p, inner));
    }
    false
}

/// Depth-first visit of every node under `node`.
pub(super) fn walk<'a>(node: Node<'a>, visit: &mut impl FnMut(Node<'a>)) {
    visit(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, visit);
    }
}
