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
