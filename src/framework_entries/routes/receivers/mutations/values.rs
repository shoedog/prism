use super::*;
use crate::framework_entries::routes::receivers::{
    direct,
    imports::{import_items, imported_as, source_path},
    normalized, python_scope, targets, PythonMember, SetupContext,
};
use crate::framework_entries::routes::Reason;

// Mutation indexing has no setup order. Consider earlier reaching bindings
// when a write is guarded or a nested scope reads an outer name.
pub(super) fn possible_values<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    expression: Node<'a>,
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
            values.extend(bindings.possible_imported_views(file, node, &context));
        }
        if node.kind() != "identifier" {
            continue;
        }
        let name = file.node_text(&node);
        reaching_sources(bindings, file, node, |write| {
            if let Some(rhs) = alias_rhs(write) {
                pending.push(ungroup(rhs));
            } else if binding_source(write.kind()) {
                values.push(bindings.bound(PythonMember { file, node: write }, name, &context, 24));
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
            "import_statement" | "import_from_statement" => {
                relevant_import(bindings, file, write, file.node_text(&node))
            }
            "function_definition" | "class_definition" => true,
            _ => alias_rhs(write).is_some_and(|rhs| may_resolve(bindings, file, rhs, depth - 1)),
        }),
        "attribute" => attribute(node).is_some_and(|(root, field)| {
            matches!(
                file.node_text(&field),
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
    let name = file.node_text(&node);
    let local = python_scope(node);
    let mut scope = local;
    loop {
        let mut before = if scope == local {
            node.start_byte()
        } else {
            scope.end_byte()
        };
        let mut had_write = false;
        while let Some(write) = bindings.reaching(file, scope, name, before) {
            had_write = true;
            if visitor(write) {
                return true;
            }
            if scope == local && direct(write, scope) {
                break;
            }
            before = write.start_byte();
        }
        let shadowed = scope.kind() == "function_definition"
            && bindings
                .reaching(file, scope, name, scope.end_byte())
                .is_some()
            && !bindings.has_global_write(file, name);
        if scope.kind() == "module"
            || shadowed
            || parameter(file, scope, name)
            || (scope.kind() == "class_definition" && had_write)
        {
            return false;
        }
        let outer = python_scope(scope);
        if outer == scope {
            return false;
        }
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
        if normalized(imported_as(file, path, alias)) != normalized(name) {
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

fn parameter(file: &ParsedFile, scope: Node<'_>, name: &str) -> bool {
    scope
        .child_by_field_name("parameters")
        .is_some_and(|params| {
            children(params)
                .into_iter()
                .flat_map(targets)
                .any(|id| normalized(file.node_text(&id)) == normalized(name))
        })
}

pub(super) fn unbound_name<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    name: Node<'a>,
) -> bool {
    let name_text = file.node_text(&name);
    let mut scope = python_scope(name);
    loop {
        if bindings
            .reaching(file, scope, name_text, scope.end_byte())
            .is_some()
            || parameter(file, scope, name_text)
        {
            return false;
        }
        let outer = python_scope(scope);
        if outer == scope {
            return true;
        }
        scope = outer;
    }
}
