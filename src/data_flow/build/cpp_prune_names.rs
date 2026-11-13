//! Name introducers the C++ resolver does not see (#630 slice 6c, fold 5), consumed by
//! `cpp_prune::note_node`. Design of record: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md`
//! §5.3.
//!
//! The resolver (`c_prune::resolve`) reads declarations and nothing else, so anything that
//! brings a name into scope or into a type position without being a modelled declaration makes
//! it treat an occurrence as the outer local. Instead of listing misparses, every such form
//! escapes its names by name, owner-wide (trace kinds in parentheses):
//! - the enumerators of any `enum_specifier` of the owner, wherever it sits (`enumerator`);
//! - the declarators of a `declaration` whose base type is not a keyword scalar (or `auto` from
//!   a literal): `ga * x;` and `ga < ga > x;` can be expressions the grammar read as
//!   declarations, and the phantom declarator would otherwise rebind `x` (`nonscalar_decl`).
//!   Exempt (6d-2s, `keyword_proves_declaration`): a declaration whose declarators are all plain
//!   pointer chains (through `init_declarator`) and that a keyword token proves a declaration,
//!   whatever the type name denotes: a direct `const` qualifier, an elaborated type specifier
//!   (`struct`, `class`, `union`, `enum` plus a plain name, no body), `auto` as the type, or a
//!   `const` inside every declarator's pointer chain (`T* const x`). Each spelling is in
//!   `cpp_prune::KEYWORDS`, so an in-repo macro of that name turns the owner off. Attributes,
//!   other qualifiers, storage classes, templates, qualified or `decltype` bases, references,
//!   arrays, function and parenthesized declarators are not exempt; every other escape applies;
//! - every `type_identifier`: `x * ga2;` puts the local `x` in a type position (`type_name`);
//! - template syntax at the source: every name under a `template_argument_list`, and the name
//!   of every `template_type`, `template_function` and `template_method`, since `a < x > y;`,
//!   `a < x > (0)` and `ga<x> *y;` are relational chains read as templates (`template_arg`);
//! - the identifiers of an `offsetof_expression` and of an `alignof`-style form other than the
//!   keyword `alignof`: a user function of that name parses as the keyword form (`keyword_form`).
//!
//! A block-scope `using enum E;` turns the owner off (`cpp_prune::off_reason`), as
//! `using namespace` does. Cost, measured on leveldb: pointer locals of class type
//! (`Iterator* it`) are ineligible.
use super::c_prune::COwner;
use super::cpp_prune::{collect_names, names_in, scalar_gate};
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// Record the name escapes of one node. Called for every node of a C++ owner.
pub(super) fn note(parsed: &ParsedFile, node: Node<'_>, out: &mut COwner) {
    let mut found = Vec::new();
    let kind: &'static str = match node.kind() {
        "enum_specifier" => {
            if let Some(body) = node.child_by_field_name("body") {
                names_in(body, &["enumerator"], &mut found);
                found = found
                    .iter()
                    .filter_map(|e| e.child_by_field_name("name"))
                    .collect();
            }
            "enumerator"
        }
        "type_identifier" => {
            found.push(node);
            "type_name"
        }
        "declaration" if keyword_proves_declaration(parsed, node) => {
            // Exempt (6d-2s): a keyword token makes this a declaration, never an expression.
            "nonscalar_decl"
        }
        "declaration" => {
            let mut cursor = node.walk();
            for d in node.children_by_field_name("declarator", &mut cursor) {
                let top = if d.kind() == "init_declarator" {
                    match d.child_by_field_name("declarator") {
                        Some(t) => t,
                        None => continue,
                    }
                } else {
                    d
                };
                // Pointer declarators count: `ga * x;` is the misparse itself.
                if !scalar_gate(parsed, node, top, false) {
                    names_in(top, &["identifier"], &mut found);
                }
            }
            "nonscalar_decl"
        }
        "template_argument_list" => {
            names_in(
                node,
                &["identifier", "type_identifier", "field_identifier"],
                &mut found,
            );
            "template_arg"
        }
        "template_type" | "template_function" | "template_method" => {
            if let Some(name) = node.child_by_field_name("name") {
                names_in(
                    name,
                    &["identifier", "type_identifier", "field_identifier"],
                    &mut found,
                );
            }
            "template_arg"
        }
        "offsetof_expression" => {
            collect_names(node, &mut found);
            "keyword_form"
        }
        "alignof_expression" if !is_alignof_keyword(parsed, node) => {
            collect_names(node, &mut found);
            "keyword_form"
        }
        _ => return,
    };
    for n in found {
        out.escape(kind, parsed.node_text(&n));
    }
}

/// The form is spelled with the C++ keyword `alignof` (`_alignof`, `__alignof__` and `_Alignof`
/// are ordinary identifiers a program can declare as functions).
pub(super) fn is_alignof_keyword(parsed: &ParsedFile, node: Node<'_>) -> bool {
    parsed
        .node_text(&node)
        .strip_prefix("alignof")
        .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

fn is_comment(n: Node<'_>) -> bool {
    n.kind() == "comment"
}

/// A declarator that is a chain of `pointer_declarator`s (each holding only the declarator and
/// `const` qualifiers) ending in a plain identifier. Returns `Some(has_const)` for such a chain.
fn plain_pointer_chain(parsed: &ParsedFile, top: Node<'_>) -> Option<bool> {
    let mut d = if top.kind() == "init_declarator" {
        top.child_by_field_name("declarator")?
    } else {
        top
    };
    if d.kind() != "pointer_declarator" {
        return None;
    }
    let mut has_const = false;
    while d.kind() == "pointer_declarator" {
        let inner = d.child_by_field_name("declarator")?;
        let mut cursor = d.walk();
        for c in d.named_children(&mut cursor) {
            if c.id() == inner.id() || is_comment(c) {
                continue;
            }
            if c.kind() == "type_qualifier" && parsed.node_text(&c) == "const" {
                has_const = true;
            } else {
                return None;
            }
        }
        d = inner;
    }
    (d.kind() == "identifier").then_some(has_const)
}

/// The declaration is one whatever the type name denotes, because a keyword token appears
/// where an expression has none (module doc, 6d-2s). Positive list only.
fn keyword_proves_declaration(parsed: &ParsedFile, decl: Node<'_>) -> bool {
    let Some(ty) = decl.child_by_field_name("type") else {
        return false;
    };
    let mut cursor = decl.walk();
    let tops: Vec<_> = decl
        .children_by_field_name("declarator", &mut cursor)
        .collect();
    if tops.is_empty() {
        return false;
    }
    let mut chain_const = true;
    for t in &tops {
        match plain_pointer_chain(parsed, *t) {
            Some(c) => chain_const &= c,
            None => return false,
        }
    }
    // Direct children other than the type and declarators: only `const` qualifiers.
    let mut direct_const = false;
    let mut cursor = decl.walk();
    for c in decl.named_children(&mut cursor) {
        if c.id() == ty.id() || is_comment(c) {
            continue;
        }
        let is_decl = tops.iter().any(|t| t.id() == c.id());
        if is_decl {
            continue;
        }
        if c.kind() == "type_qualifier" && parsed.node_text(&c) == "const" {
            direct_const = true;
        } else {
            return false;
        }
    }
    let type_proof = match ty.kind() {
        "struct_specifier" | "class_specifier" | "union_specifier" | "enum_specifier" => {
            elaborated_plain(ty)
        }
        "placeholder_type_specifier" => parsed.node_text(&ty).trim() == "auto",
        // A plain `T` / `primitive_type`: only the qualifiers can prove it. Qualified,
        // template, `decltype` and dependent bases stay escaped.
        "type_identifier" | "primitive_type" | "sized_type_specifier" => false,
        _ => return false,
    };
    direct_const || type_proof || chain_const
}

/// `struct N`, `class N`, ... : the keyword, a plain `type_identifier` name, no body, no
/// attributes, no base clause.
fn elaborated_plain(ty: Node<'_>) -> bool {
    if ty.child_by_field_name("body").is_some() {
        return false;
    }
    let mut cursor = ty.walk();
    let named: Vec<_> = ty
        .named_children(&mut cursor)
        .filter(|c| !is_comment(*c))
        .collect();
    named.len() == 1
        && named[0].kind() == "type_identifier"
        && ty
            .child_by_field_name("name")
            .is_some_and(|n| n.id() == named[0].id())
}
