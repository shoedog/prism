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
//!   declarations, and the phantom declarator would otherwise rebind `x` (`nonscalar_decl`);
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
                if !scalar_gate(parsed, node, top, false)
                    && !(super::mute6d::mute("c4")
                        && is_pointer_chain(top)
                        && c4_allowed(parsed, node, top))
                {
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

/// 6d-2 prototype: the c4 lift is allowed for this declaration (allow file, structural shape).
fn c4_allowed(parsed: &ParsedFile, decl: Node<'_>, top: Node<'_>) -> bool {
    let ty = decl.child_by_field_name("type");
    let bare = ty
        .filter(|t| t.kind() == "type_identifier")
        .map(|t| parsed.node_text(&t).to_string());
    let mut structural = ty.is_some_and(|t| {
        matches!(
            t.kind(),
            "struct_specifier" | "class_specifier" | "union_specifier" | "enum_specifier"
        )
    });
    let mut c = decl.walk();
    for k in decl.children(&mut c) {
        if matches!(
            k.kind(),
            "type_qualifier" | "storage_class_specifier" | "attribute_specifier"
        ) {
            structural = true;
        }
    }
    let mut n = Some(top);
    while let Some(x) = n {
        if x.kind() == "pointer_declarator" {
            let mut c = x.walk();
            if x.children(&mut c).any(|k| k.kind() == "type_qualifier") {
                structural = true;
            }
        }
        n = x.child_by_field_name("declarator");
    }
    // templates, qualified bases and the like stay out of the structural class
    let base_ok = ty.is_some_and(|t| {
        matches!(
            t.kind(),
            "type_identifier"
                | "struct_specifier"
                | "class_specifier"
                | "union_specifier"
                | "enum_specifier"
                | "placeholder_type_specifier"
        )
    });
    super::mute6d::allow_c4(bare.as_deref(), structural && base_ok)
}

/// 6d measurement: the declarator is a pointer declarator (through parentheses).
fn is_pointer_chain(top: Node<'_>) -> bool {
    let mut n = top;
    loop {
        match n.kind() {
            "pointer_declarator" => return true,
            "parenthesized_declarator" => match super::prune_roots::operand(n) {
                Some(x) => n = x,
                None => return false,
            },
            _ => return false,
        }
    }
}
