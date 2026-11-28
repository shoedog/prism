use super::*;
use crate::framework_entries::routes::receivers::{
    direct,
    imports::{import_items, imported_as, source_path},
    python_scope,
    writes::effective_name,
    PythonMember, SetupContext,
};
use crate::framework_entries::routes::Reason;

// Mutation indexing has no setup order. Consider earlier reaching bindings
// when a write is guarded or a nested scope reads an outer name.
pub(super) fn possible_values<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    expression: Node<'a>,
) -> Vec<PythonValue<'a>> {
    mutation_candidates(bindings, file, expression, false)
}

pub(super) fn mutation_candidates<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    expression: Node<'a>,
    keep_refused_routers: bool,
) -> Vec<PythonValue<'a>> {
    if !may_resolve(bindings, file, expression, 24) {
        return vec![];
    }
    let mut values = Vec::new();
    let root = ungroup(expression);
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    let context = SetupContext::default();
    while let Some(node) = pending.pop() {
        if !seen.insert(node.id()) {
            continue;
        }
        let exact = if node == root {
            bindings.capture_value(&file.path, node, &context)
        } else {
            bindings.value_at(&file.path, node, &context)
        };
        let refused = matches!(exact, PythonValue::Unknown(Reason::ImportedUnproven));
        values.push(exact);
        if refused {
            values.extend(bindings.possible_imported_values(
                file,
                node,
                &context,
                keep_refused_routers,
            ));
        }
        if node.kind() != "identifier" {
            continue;
        }
        let name = effective_name(file, python_scope(node), file.node_text(&node));
        reaching_sources(bindings, file, node, |write| {
            if let Some(rhs) = alias_rhs(write) {
                pending.push(ungroup(rhs));
            } else if binding_source(write.kind()) {
                values.push(bindings.bound(
                    PythonMember { file, node: write },
                    &name,
                    &context,
                    24,
                ));
            }
            false
        });
    }
    values
}

// Skip lexical roots that cannot resolve to a relevant import, definition, or
// allocation. Callers still check field names on unresolved owners.
fn may_resolve<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    node: Node<'a>,
    depth: usize,
) -> bool {
    if depth == 0 {
        return true;
    }
    let node = ungroup(node);
    match node.kind() {
        "identifier" => reaching_sources(bindings, file, node, |write| match write.kind() {
            "import_statement" | "import_from_statement" => relevant_import(
                bindings,
                file,
                write,
                &effective_name(file, python_scope(node), file.node_text(&node)),
            ),
            "function_definition" | "class_definition" => true,
            _ => alias_rhs(write).is_some_and(|rhs| may_resolve(bindings, file, rhs, depth - 1)),
        }),
        "attribute" => attribute(node).is_some_and(|(root, field)| {
            matches!(
                normalized(file.node_text(&field)).as_ref(),
                "url_map" | "converters" | "view_functions"
            ) || may_resolve(bindings, file, root, depth - 1)
        }),
        "call" => node
            .child_by_field_name("function")
            .is_some_and(|root| may_resolve(bindings, file, root, depth - 1)),
        _ => false,
    }
}

fn reaching_sources<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    node: Node<'a>,
    mut visitor: impl FnMut(Node<'a>) -> bool,
) -> bool {
    let local = python_scope(node);
    let name = effective_name(file, local, file.node_text(&node));
    let writes = &bindings.writes[file.path.as_str()];
    let mut scope = local;
    loop {
        let mut before = if scope == local {
            node.start_byte()
        } else {
            scope.end_byte()
        };
        let directive = writes.directive(scope, &name);
        let mut had_write = false;
        while let Some(write) = writes.named_before(scope, &name, before) {
            had_write = true;
            if visitor(write) {
                return true;
            }
            if scope == local && directive.is_none() && direct(write, scope) {
                // A write can hide the explicit import whose capture needs a refusal.
                return writes
                    .import_before(scope, &name, write.start_byte())
                    .is_some_and(visitor);
            }
            before = write.start_byte();
        }
        if directive.is_none()
            && (if scope.kind() == "class_definition" {
                had_write
            } else {
                writes.binds_local(scope, &name)
            })
        {
            return false;
        }
        let Some(outer) = writes.outer_binding_scope(scope, &name) else {
            return false;
        };
        scope = outer;
    }
}

fn alias_rhs(write: Node<'_>) -> Option<Node<'_>> {
    (write.kind() == "assignment"
        && write
            .child_by_field_name("left")
            .is_some_and(|left| left.kind() == "identifier"))
    .then(|| write.child_by_field_name("right"))
    .flatten()
}

fn binding_source(kind: &str) -> bool {
    matches!(
        kind,
        "import_statement" | "import_from_statement" | "function_definition" | "class_definition"
    )
}

fn relevant_import(
    bindings: &PythonBindings<'_>,
    file: &ParsedFile,
    import: Node<'_>,
    name: &str,
) -> bool {
    let relevant = |module: &str| {
        module == "flask" || module.starts_with("flask.") || bindings.source_module(module)
    };
    let from = import.child_by_field_name("module_name");
    import_items(import).any(|(path, alias)| {
        if effective_name(file, python_scope(import), imported_as(file, path, alias)) != name {
            return false;
        }
        let path = source_path(file, path);
        match from {
            Some(from) => bindings
                .import_module(file, from)
                .is_some_and(|module| relevant(&module) || relevant(&format!("{module}.{path}"))),
            None => relevant(&path) || path.split_once('.').is_some_and(|(root, _)| relevant(root)),
        }
    })
}

pub(super) fn unbound_name<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    name: Node<'a>,
) -> bool {
    let mut scope = python_scope(name);
    let name = effective_name(file, scope, file.node_text(&name));
    let writes = &bindings.writes[file.path.as_str()];
    loop {
        if writes.binds_local(scope, &name)
            || bindings
                .reaching(file, scope, &name, scope.end_byte())
                .is_some()
        {
            return false;
        }
        let Some(outer) = writes.outer_binding_scope(scope, &name) else {
            return true;
        };
        scope = outer;
    }
}
