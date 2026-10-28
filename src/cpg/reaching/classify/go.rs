use super::super::grammar_lint::uncertain_binding_nodes;
use super::super::{Line, ParsedFile};
use crate::data_flow::FlowEdge;
use crate::languages::Language;

pub(super) fn unplaced_parameter(parsed: &ParsedFile, edge: &FlowEdge) -> Option<Line> {
    if parsed.language != Language::Go {
        return None;
    }
    let use_byte = super::edge_use_byte(parsed, edge)?;
    let mut current = Some(
        parsed
            .tree
            .root_node()
            .descendant_for_byte_range(use_byte, use_byte.saturating_add(1))?,
    );
    let function = loop {
        let node = current?;
        if matches!(
            node.kind(),
            "function_declaration" | "method_declaration" | "func_literal"
        ) {
            break node;
        }
        current = node.parent();
    };
    if function.kind() != "func_literal" {
        return None;
    }
    let parameters = parsed.find_parameters_node(&function)?;
    if use_byte < parameters.end_byte() || function.end_byte() <= use_byte {
        return None;
    }
    let construct_line = parameters.start_position().row + 1;
    (edge.from.line < construct_line
        && parameter_declares_name(parsed, parameters, &edge.to.path.base))
    .then_some(construct_line)
}

fn parameter_declares_name(parsed: &ParsedFile, node: tree_sitter::Node<'_>, name: &str) -> bool {
    let mut cursor = node.walk();
    let declares = node
        .named_children(&mut cursor)
        .filter(|child| {
            matches!(
                child.kind(),
                "parameter_declaration" | "variadic_parameter_declaration"
            )
        })
        .any(|parameter| {
            let mut names = parameter.walk();
            let found = parameter
                .children_by_field_name("name", &mut names)
                .any(|identifier| parsed.node_text(&identifier) == name);
            found
        });
    declares
}

pub(super) fn unplaced_type_switch(parsed: &ParsedFile, edge: &FlowEdge) -> Option<Line> {
    if parsed.language != Language::Go {
        return None;
    }
    let use_byte = super::edge_use_byte(parsed, edge)?;
    let use_node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(use_byte, use_byte.saturating_add(1))?;
    uncertain_binding_nodes(parsed, &edge.to.path.base)
        .into_iter()
        .filter(|binder| binder.kind() == "type_switch_statement")
        .filter_map(|binder| {
            let construct_line = binder.start_position().row + 1;
            let mut current = Some(use_node);
            let mut inside_case = false;
            let mut inside_switch = false;
            while let Some(node) = current {
                if node.id() == binder.id() {
                    inside_switch = true;
                    break;
                }
                inside_case |= matches!(node.kind(), "type_case" | "default_case");
                current = node.parent();
            }
            let def_outside_switch = edge.from.start_byte < binder.start_byte()
                || binder.end_byte() <= edge.from.start_byte;
            let mut def_current = Some(parsed.tree.root_node().descendant_for_byte_range(
                edge.from.start_byte,
                edge.from.start_byte.saturating_add(1),
            )?);
            let mut def_case = None;
            let def_case_of_binder = loop {
                let Some(node) = def_current else {
                    break None;
                };
                if matches!(node.kind(), "type_case" | "default_case") && def_case.is_none() {
                    def_case = Some(node);
                }
                if node.kind() == "type_switch_statement" {
                    break (node.id() == binder.id()).then_some(def_case?);
                }
                def_current = node.parent();
            };
            let arm_def_reaches_outside = def_case_of_binder.is_some_and(|case| {
                !(case.start_byte() <= use_byte && use_byte < case.end_byte())
                    && ((inside_switch && inside_case) || binder.end_byte() <= use_byte)
            });
            ((def_outside_switch && inside_switch && inside_case) || arm_def_reaches_outside)
                .then_some(construct_line)
        })
        .min()
}
