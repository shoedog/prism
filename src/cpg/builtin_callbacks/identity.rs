use super::*;
use crate::ast::{JsBinding, JsBindingCache};

fn inline(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "generator_function"
    )
}

pub(super) fn implementation<'a>(
    parsed: &'a ParsedFile,
    value: Node<'a>,
    trace: &mut Option<diagnostic::Decision<'_>>,
) -> Result<Node<'a>, &'static str> {
    let value = syntax::unwrap(parsed, value).ok_or("non_callable_slot_zero")?;
    if let Some(t) = trace {
        t.callback = Some(span(value));
    }
    if inline(value) {
        return Ok(value);
    }
    if value.kind() != "identifier" {
        return Err("non_callable_slot_zero");
    }

    let mut bindings = JsBindingCache::for_sites();
    let name = parsed.node_text(&value);
    let terminal = match parsed.js_ts_site_binding(value, name, &mut bindings) {
        JsBinding::Callable(terminal) => terminal,
        JsBinding::MayCall => return Err("unproved_live_binding"),
        JsBinding::Import => return Err("import_binding"),
        JsBinding::Alias => return Err("alias_binding"),
        JsBinding::Refused(reason) | JsBinding::Unchecked(reason) => return Err(reason),
    };
    let (scope, declaration) = parsed
        .js_ts_site_declaration(value, name, &mut bindings)
        .ok_or("declaration_identity")?;
    if let Some(t) = trace {
        t.declaration = Some(span(declaration));
    }
    // Direct eval can change a mutable outer declaration through a captured
    // environment. The static classifier proves only indexed writes.
    if parsed.js_ts_direct_eval_anywhere(&scope) {
        return Err("dynamic_binding");
    }
    let function = match declaration.kind() {
        "function_declaration" | "generator_function_declaration" => declaration,
        "variable_declarator" => {
            let list = declaration
                .parent()
                .filter(|list| {
                    list.kind() == "lexical_declaration"
                        && list
                            .child_by_field_name("kind")
                            .is_some_and(|kind| kind.kind() == "const")
                })
                .ok_or("mutable_declaration")?;
            let initializer = declaration
                .child_by_field_name("value")
                .and_then(|node| syntax::unwrap(parsed, node))
                .filter(|node| inline(*node))
                .ok_or("indirect_initializer")?;
            if declaration.end_byte() > value.start_byte() {
                return Err("uninitialized_const");
            }
            if !initialized_in_region(parsed, list, value) {
                return Err("unproved_const_execution");
            }
            initializer
        }
        _ => return Err("unsupported_declaration"),
    };
    if terminal.wrapped
        || parsed.node_line_range(&function) != (terminal.start_line, terminal.end_line)
        || !parsed
            .language
            .function_name(&function)
            .is_some_and(|name| parsed.node_text(&name) == terminal.local)
    {
        return Err("declaration_identity");
    }
    Ok(function)
}

// A preceding declaration in the same ordinary statement list must execute
// before a use in a later sibling. A case label or deferred callable can
// enter a textual suffix without executing the initializer.
fn initialized_in_region(parsed: &ParsedFile, list: Node<'_>, mut value: Node<'_>) -> bool {
    if syntax::execution_owner(parsed, list) != syntax::execution_owner(parsed, value) {
        return false;
    }
    let Some(mut container) = list.parent() else {
        return false;
    };
    if container.kind() == "export_statement" {
        let Some(program) = container.parent() else {
            return false;
        };
        container = program;
    }
    if !matches!(container.kind(), "program" | "statement_block") {
        return false;
    }
    while let Some(parent) = value.parent() {
        if parent == container {
            return value.start_byte() >= list.end_byte();
        }
        value = parent;
    }
    false
}
