//! JS/TS lexical binding and scope visibility at a call.

use crate::algorithms::taint::assignment_lhs_identifiers;
use crate::algorithms::taint::js_ts::imports_ssrf::{
    js_ts_import_statement_binds_member, js_ts_import_statement_binds_module,
    js_ts_pattern_binds_member, js_ts_require_declarator_binds_member,
    js_ts_require_declarator_binds_module, js_ts_require_member_expression,
};
use crate::algorithms::taint::js_ts::request_aliases::{
    collect_js_ts_lhs_alias_identifiers, js_ts_intersect_byte_scopes, js_ts_intersect_line_scopes,
    js_ts_node_definitely_exits,
};
use crate::algorithms::taint::js_ts::sinks::collect_nodes_of_kind;
use crate::ast::ParsedFile;
use crate::frameworks::js_ts::call_classify::{
    collect_js_ts_assignment_like_nodes, js_ts_assignment_target,
    js_ts_assignment_target_and_value, js_ts_assignment_value, js_ts_require_call_module,
};
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) fn js_ts_identifier_binds_imported_member_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    js_ts_imported_member_binding_visible_at_call(
        parsed,
        call,
        local_name,
        module_name,
        imported_member,
    ) && !js_ts_identifier_has_local_shadow_before_call(
        parsed,
        call,
        local_name,
        module_name,
        Some(imported_member),
    )
}

pub(super) fn js_ts_identifier_binds_module_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    js_ts_module_binding_visible_at_call(parsed, call, local_name, module_name)
        && !js_ts_identifier_has_local_shadow_before_call(
            parsed,
            call,
            local_name,
            module_name,
            None,
        )
}

fn js_ts_identifier_has_local_shadow_before_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: Option<&str>,
) -> bool {
    let call_line = call.start_position().row + 1;
    let Some(func) = parsed.enclosing_function(call_line) else {
        return false;
    };
    if js_ts_function_parameter_binds_name(parsed, &func, local_name) {
        return true;
    }
    if js_ts_function_declaration_shadows_call(parsed, &func, call, local_name) {
        return true;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(func, parsed, &mut assignments);
    for assignment in assignments {
        let is_function_scoped_var = js_ts_binding_is_function_scoped_var(&assignment);
        let binding_starts_after_call = assignment.start_byte() >= call.start_byte();
        if binding_starts_after_call && !is_function_scoped_var {
            continue;
        }
        if !js_ts_binding_scope_reaches_call(parsed, func.id(), call, &assignment) {
            continue;
        }
        let Some(lhs) = js_ts_assignment_target(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == local_name)
        {
            continue;
        }
        if !binding_starts_after_call {
            if let Some(rhs) = js_ts_assignment_value(parsed, &assignment) {
                if js_ts_assignment_imports_allowed_binding(
                    parsed,
                    &lhs,
                    &rhs,
                    local_name,
                    module_name,
                    imported_member,
                ) {
                    continue;
                }
            }
        }
        return true;
    }
    false
}

fn js_ts_binding_scope_reaches_call(
    parsed: &ParsedFile,
    root_func_id: usize,
    call: &Node<'_>,
    binding_node: &Node<'_>,
) -> bool {
    let binding_scope_id = if js_ts_binding_is_function_scoped_var(binding_node) {
        js_ts_nearest_function_scope_id(parsed, binding_node, root_func_id)
    } else {
        js_ts_nearest_scope_block_id(parsed, binding_node, root_func_id)
    };
    let Some(binding_scope_id) = binding_scope_id else {
        return false;
    };
    js_ts_scope_chain_contains(call, root_func_id, binding_scope_id)
}

fn js_ts_binding_is_function_scoped_var(binding_node: &Node<'_>) -> bool {
    if binding_node.kind() != "variable_declarator" {
        return false;
    }
    binding_node
        .parent()
        .is_some_and(|parent| parent.kind() == "variable_declaration")
}

fn js_ts_binding_scope_line_range(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    js_ts_binding_scope_node(parsed, binding_node, root_func_id)
        .map(|scope| (scope.start_position().row + 1, scope.end_position().row + 1))
}

fn js_ts_binding_scope_byte_range(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    js_ts_binding_scope_node(parsed, binding_node, root_func_id)
        .map(|scope| (scope.start_byte(), scope.end_byte()))
}

fn js_ts_binding_scope_node<'a>(
    parsed: &ParsedFile,
    binding_node: &Node<'a>,
    root_func_id: usize,
) -> Option<Node<'a>> {
    let function_scoped = js_ts_binding_is_function_scoped_var(binding_node);
    let mut current = Some(*binding_node);
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return Some(parent);
        }
        if function_scoped {
            if parent.id() != binding_node.id()
                && parsed
                    .language
                    .function_node_types()
                    .contains(&parent.kind())
            {
                return Some(parent);
            }
        } else if js_ts_is_lexical_scope_boundary(parsed, parent.kind()) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

fn js_ts_assignment_target_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    if binding_node.kind() == "variable_declarator" {
        js_ts_binding_scope_line_range(parsed, binding_node, root_func_id)
    } else {
        js_ts_assignment_existing_binding_scope(parsed, binding_node, root_func_id)
    }
}

pub(super) fn js_ts_assignment_target_byte_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    if binding_node.kind() == "variable_declarator" {
        js_ts_binding_scope_byte_range(parsed, binding_node, root_func_id)
    } else {
        js_ts_assignment_existing_binding_byte_scope(parsed, binding_node, root_func_id)
    }
}

pub(super) fn js_ts_assignment_effective_line_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let binding_scope = js_ts_assignment_target_scope(parsed, binding_node, root_func_id);
    let exiting_branch_scope =
        js_ts_enclosing_definitely_exiting_branch(binding_node, root_func_id).map(|branch| {
            (
                branch.start_position().row + 1,
                branch.end_position().row + 1,
            )
        });
    js_ts_intersect_line_scopes(binding_scope, exiting_branch_scope)
}

pub(super) fn js_ts_assignment_effective_byte_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let binding_scope = js_ts_assignment_target_byte_scope(parsed, binding_node, root_func_id);
    let exiting_branch_scope =
        js_ts_enclosing_definitely_exiting_branch(binding_node, root_func_id)
            .map(|branch| (branch.start_byte(), branch.end_byte()));
    js_ts_intersect_byte_scopes(binding_scope, exiting_branch_scope)
}

fn js_ts_enclosing_definitely_exiting_branch<'a>(
    node: &Node<'a>,
    root_func_id: usize,
) -> Option<Node<'a>> {
    let mut child = *node;
    let mut current = child.parent();
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return None;
        }
        if matches!(parent.kind(), "if_statement" | "if_expression") {
            let consequence = parent.child_by_field_name("consequence");
            let alternative = parent.child_by_field_name("alternative");
            if (consequence.is_some_and(|branch| branch.id() == child.id())
                || alternative.is_some_and(|branch| branch.id() == child.id()))
                && js_ts_node_definitely_exits(child)
            {
                return Some(child);
            }
        }
        child = parent;
        current = parent.parent();
    }
    None
}

fn js_ts_assignment_existing_binding_scope(
    parsed: &ParsedFile,
    assignment_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let (lhs, _) = js_ts_assignment_target_and_value(parsed, assignment_node)?;
    let aliases = js_ts_lhs_binding_names(parsed, &lhs);
    if aliases.is_empty() {
        return None;
    }
    let func = parsed.enclosing_function(assignment_node.start_position().row + 1)?;
    if func.id() != root_func_id {
        return None;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(func, parsed, &mut assignments);
    assignments
        .into_iter()
        .filter(|candidate| {
            candidate.kind() == "variable_declarator"
                && candidate.start_byte() < assignment_node.start_byte()
                && js_ts_binding_scope_reaches_call(
                    parsed,
                    root_func_id,
                    assignment_node,
                    candidate,
                )
                && js_ts_assignment_target(parsed, candidate).is_some_and(|candidate_lhs| {
                    js_ts_lhs_binding_names(parsed, &candidate_lhs)
                        .iter()
                        .any(|name| aliases.contains(name))
                })
        })
        .max_by_key(|candidate| candidate.start_byte())
        .and_then(|candidate| js_ts_binding_scope_line_range(parsed, &candidate, root_func_id))
}

fn js_ts_assignment_existing_binding_byte_scope(
    parsed: &ParsedFile,
    assignment_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let (lhs, _) = js_ts_assignment_target_and_value(parsed, assignment_node)?;
    let aliases = js_ts_lhs_binding_names(parsed, &lhs);
    if aliases.is_empty() {
        return None;
    }
    let func = parsed.enclosing_function(assignment_node.start_position().row + 1)?;
    if func.id() != root_func_id {
        return None;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(func, parsed, &mut assignments);
    assignments
        .into_iter()
        .filter(|candidate| {
            candidate.kind() == "variable_declarator"
                && candidate.start_byte() < assignment_node.start_byte()
                && js_ts_binding_scope_reaches_call(
                    parsed,
                    root_func_id,
                    assignment_node,
                    candidate,
                )
                && js_ts_assignment_target(parsed, candidate).is_some_and(|candidate_lhs| {
                    js_ts_lhs_binding_names(parsed, &candidate_lhs)
                        .iter()
                        .any(|name| aliases.contains(name))
                })
        })
        .max_by_key(|candidate| candidate.start_byte())
        .and_then(|candidate| js_ts_binding_scope_byte_range(parsed, &candidate, root_func_id))
}

fn js_ts_lhs_binding_names(parsed: &ParsedFile, lhs: &Node<'_>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_js_ts_lhs_alias_identifiers(parsed, lhs, &mut names);
    names.remove("_");
    names
}

fn js_ts_is_lexical_scope_boundary(parsed: &ParsedFile, kind: &str) -> bool {
    parsed.language.is_scope_block(kind)
        || matches!(
            kind,
            "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "for_await_statement"
                | "switch_statement"
                | "switch_body"
                | "catch_clause"
        )
}

fn js_ts_function_declaration_shadows_call(
    parsed: &ParsedFile,
    func: &Node<'_>,
    call: &Node<'_>,
    local_name: &str,
) -> bool {
    js_ts_scope_has_function_declaration_shadow(parsed, *func, func.id(), call, local_name)
}

fn js_ts_scope_has_function_declaration_shadow(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    call: &Node<'_>,
    local_name: &str,
) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if parsed
            .language
            .function_node_types()
            .contains(&child.kind())
        {
            if matches!(
                child.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && js_ts_binding_scope_reaches_call(parsed, root_func_id, call, &child)
                && parsed
                    .language
                    .function_name(&child)
                    .is_some_and(|name| parsed.node_text(&name) == local_name)
            {
                return true;
            }
            // Function declarations introduce the binding; their bodies are a
            // nested scope and should not contribute shadows to the caller.
            continue;
        }
        if js_ts_scope_has_function_declaration_shadow(
            parsed,
            child,
            root_func_id,
            call,
            local_name,
        ) {
            return true;
        }
    }
    false
}

fn js_ts_nearest_scope_block_id(
    parsed: &ParsedFile,
    node: &Node<'_>,
    root_func_id: usize,
) -> Option<usize> {
    let mut current = Some(*node);
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return Some(root_func_id);
        }
        if js_ts_is_lexical_scope_boundary(parsed, parent.kind()) {
            return Some(parent.id());
        }
        current = parent.parent();
    }
    None
}

fn js_ts_nearest_function_scope_id(
    parsed: &ParsedFile,
    node: &Node<'_>,
    root_func_id: usize,
) -> Option<usize> {
    let mut current = Some(*node);
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return Some(root_func_id);
        }
        if parent.id() != node.id()
            && parsed
                .language
                .function_node_types()
                .contains(&parent.kind())
        {
            return Some(parent.id());
        }
        current = parent.parent();
    }
    None
}

fn js_ts_scope_chain_contains(
    node: &Node<'_>,
    root_func_id: usize,
    target_scope_id: usize,
) -> bool {
    let mut current = Some(*node);
    while let Some(parent) = current {
        if parent.id() == target_scope_id {
            return true;
        }
        if parent.id() == root_func_id {
            return target_scope_id == root_func_id;
        }
        current = parent.parent();
    }
    false
}

fn js_ts_function_parameter_binds_name(
    parsed: &ParsedFile,
    func: &Node<'_>,
    local_name: &str,
) -> bool {
    let Some(params) = func.child_by_field_name("parameters") else {
        return false;
    };
    let mut ids = Vec::new();
    collect_nodes_of_kind(params, "identifier", &mut ids);
    ids.iter().any(|id| parsed.node_text(id) == local_name)
}

fn js_ts_assignment_imports_allowed_binding(
    parsed: &ParsedFile,
    lhs: &Node<'_>,
    rhs: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: Option<&str>,
) -> bool {
    match imported_member {
        Some(member) => {
            if js_ts_require_call_module(parsed, rhs).is_some_and(|module| module == module_name) {
                return js_ts_pattern_binds_member(parsed, lhs, member, local_name);
            }
            lhs.kind() == "identifier"
                && parsed.node_text(lhs) == local_name
                && js_ts_require_member_expression(parsed, rhs).is_some_and(
                    |(module, actual_member)| module == module_name && actual_member == member,
                )
        }
        None => {
            lhs.kind() == "identifier"
                && parsed.node_text(lhs) == local_name
                && js_ts_require_call_module(parsed, rhs)
                    .is_some_and(|module| module == module_name)
        }
    }
}

fn js_ts_imported_member_binding_visible_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "import_statement"
            && js_ts_import_statement_binds_member(
                parsed,
                &child,
                local_name,
                module_name,
                imported_member,
            )
        {
            return true;
        }
    }

    let mut declarations = Vec::new();
    collect_js_ts_assignment_like_nodes(root, parsed, &mut declarations);
    declarations.iter().any(|decl| {
        decl.kind() == "variable_declarator"
            && js_ts_require_declarator_binds_member(
                parsed,
                decl,
                local_name,
                module_name,
                imported_member,
            )
            && js_ts_import_declarator_visible_at_call(parsed, call, decl)
    })
}

fn js_ts_import_declarator_visible_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    declarator: &Node<'_>,
) -> bool {
    js_ts_assignment_visible_before_context(parsed, call, declarator)
}

fn js_ts_module_binding_visible_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "import_statement"
            && js_ts_import_statement_binds_module(parsed, &child, local_name, module_name)
        {
            return true;
        }
    }

    let mut declarations = Vec::new();
    collect_js_ts_assignment_like_nodes(root, parsed, &mut declarations);
    declarations.iter().any(|decl| {
        decl.kind() == "variable_declarator"
            && js_ts_require_declarator_binds_module(parsed, decl, local_name, module_name)
            && js_ts_assignment_visible_before_context(parsed, call, decl)
    })
}

pub(super) fn js_ts_assignment_visible_before_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    assignment: &Node<'_>,
) -> bool {
    if assignment.start_byte() < context.start_byte() {
        return js_ts_binding_visible_at_context(parsed, context, assignment);
    }

    // Top-level CommonJS bindings declared after a route callback are still
    // visible when that callback executes after module initialization.
    if js_ts_enclosing_function_id(parsed, assignment).is_none()
        && js_ts_enclosing_function_id(parsed, context).is_some()
    {
        return js_ts_binding_visible_at_context(parsed, context, assignment);
    }

    false
}

fn js_ts_binding_visible_at_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    binding_node: &Node<'_>,
) -> bool {
    let root_id = parsed.tree.root_node().id();
    let context_func_id = js_ts_enclosing_function_id(parsed, context);
    let binding_func_id = js_ts_enclosing_function_id(parsed, binding_node);

    match (binding_func_id, context_func_id) {
        (Some(binding_func_id), Some(context_func_id)) if binding_func_id == context_func_id => {
            js_ts_binding_scope_reaches_call(parsed, context_func_id, context, binding_node)
        }
        (Some(_), _) => false,
        (None, _) => js_ts_binding_scope_reaches_call(parsed, root_id, context, binding_node),
    }
}

pub(super) fn js_ts_enclosing_function_id(parsed: &ParsedFile, node: &Node<'_>) -> Option<usize> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parsed
            .language
            .function_node_types()
            .contains(&parent.kind())
        {
            return Some(parent.id());
        }
        current = parent.parent();
    }
    None
}
