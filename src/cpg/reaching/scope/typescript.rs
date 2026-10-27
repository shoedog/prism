use super::super::binding_table::{select_binding_row, DeclarationKind};
use super::super::{transfer, DefSite};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) enum ParameterBinding {
    Owned,
    Legacy,
    FailClosed,
}

pub(super) fn parameter_binding_in_region(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    def: &DefSite,
) -> Option<ParameterBinding> {
    parsed
        .parameter_binding_region(func_node)
        .is_some_and(|parameters| {
            parameters.start_byte() <= def.start_byte && def.start_byte < parameters.end_byte()
        })
        .then(|| {
            if parsed.language == Language::TypeScript {
                parameter_binding(parsed, func_node, def)
            } else {
                ParameterBinding::Owned
            }
        })
}

pub(super) fn parameter_binding(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    def: &DefSite,
) -> ParameterBinding {
    if func_node
        .child_by_field_name("parameter")
        .is_some_and(|parameter| {
            parameter.kind() == "identifier" && parameter.start_byte() == def.start_byte
        })
    {
        return ParameterBinding::Owned;
    }
    let Some(end_byte) = def.start_byte.checked_add(1) else {
        return ParameterBinding::FailClosed;
    };
    let Some(mut node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, end_byte)
    else {
        return ParameterBinding::FailClosed;
    };
    loop {
        if let Some(row) = select_binding_row(parsed, node)
            .filter(|row| row.declaration == Some(DeclarationKind::Parameter))
        {
            if transfer::row_introduced_identifiers(parsed, node, row.fields)
                .into_iter()
                .any(|identifier| identifier.start_byte() == def.start_byte)
            {
                return ParameterBinding::Owned;
            }
        }
        if matches!(node.kind(), "required_parameter" | "optional_parameter") {
            let plain_default = node.child_by_field_name("value").is_some()
                && node
                    .child_by_field_name("pattern")
                    .is_some_and(|pattern| pattern.kind() == "identifier")
                && node.child_by_field_name("name").is_none()
                && node.child_by_field_name("decorator").is_none();
            return if plain_default {
                ParameterBinding::Legacy
            } else {
                ParameterBinding::FailClosed
            };
        }
        if node.id() == func_node.id() {
            return ParameterBinding::FailClosed;
        }
        let Some(parent) = node.parent() else {
            return ParameterBinding::FailClosed;
        };
        node = parent;
    }
}
