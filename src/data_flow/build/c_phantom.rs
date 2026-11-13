//! `.c` phantom pointer declarations (#630 slice 6c fold 7). The C grammar reads the expression
//! statement `a * x;` (a discarded product) as the declaration of a local pointer `x` of type
//! `a`, so the lexical resolver binds later uses of the real `x` (a global, a `static` or
//! `extern` name, anything the resolver does not see) to the phantom, an eligible pointer local.
//! A declaration whose type is a bare `type_identifier` that the same file also spells as an
//! object (a parameter, a local, a file-scope variable or an expression operand: any
//! `identifier` node of the file), with only plain or pointer declarators and no qualifier,
//! storage class or initializer, escapes its declarator names by name. `FILE *fp;` keeps
//! pruning while `FILE` is spelled only as a type. An object declared only in a header the
//! file includes is not seen (disclosed, out of model like any external declaration). C++
//! escapes the same names through `cpp_prune_names::nonscalar_decl`.
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

fn collect(node: Node<'_>, parsed: &ParsedFile, out: &mut BTreeSet<String>) {
    if node.kind() == "identifier" {
        out.insert(parsed.node_text(&node).to_owned());
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        collect(c, parsed, out);
    }
}

/// The identifiers of the file that spell an object: every `identifier` node except the ones in
/// the operand of `sizeof`/`alignof`/`offsetof`, where the grammar reads a type name as an
/// expression (`sizeof(kvobj)`). A real object has a declarator, parameter or use elsewhere.
fn object_names(parsed: &ParsedFile) -> BTreeSet<String> {
    fn go(node: Node<'_>, parsed: &ParsedFile, out: &mut BTreeSet<String>) {
        if matches!(
            node.kind(),
            "sizeof_expression" | "alignof_expression" | "offsetof_expression"
        ) {
            return;
        }
        if node.kind() == "identifier" {
            out.insert(parsed.node_text(&node).to_owned());
        }
        let mut cursor = node.walk();
        for c in node.named_children(&mut cursor) {
            go(c, parsed, out);
        }
    }
    let mut set = BTreeSet::new();
    go(parsed.tree.root_node(), parsed, &mut set);
    set
}

/// Escape the declarator names of a phantom pointer declaration (see the module doc).
pub(super) fn escape_phantom(parsed: &ParsedFile, decl: Node<'_>, escaped: &mut BTreeSet<String>) {
    let Some(ty) = decl.child_by_field_name("type") else {
        return;
    };
    if decl.kind() != "declaration" || ty.kind() != "type_identifier" {
        return;
    }
    let mut cursor = decl.walk();
    let mut declarators = Vec::new();
    for c in decl.named_children(&mut cursor) {
        match c.kind() {
            "comment" => {}
            _ if c.id() == ty.id() => {}
            "identifier" | "pointer_declarator" => declarators.push(c),
            _ => return,
        }
    }
    if !declarators.iter().any(|d| d.kind() == "pointer_declarator") {
        return;
    }
    if super::mute6d::log_enabled()
        && declarators.len() == 1
        && declarators[0].kind() == "pointer_declarator"
        && !parsed
            .c_object_names(object_names)
            .contains(parsed.node_text(&ty))
    {
        super::mute6d::log_p1(
            &parsed.path,
            decl.start_position().row + 1,
            parsed.node_text(&ty),
            &super::mute6d::clean(parsed.node_text(&declarators[0])),
        );
    }
    if parsed
        .c_object_names(object_names)
        .contains(parsed.node_text(&ty))
    {
        for d in declarators {
            collect(d, parsed, escaped);
        }
    }
}
