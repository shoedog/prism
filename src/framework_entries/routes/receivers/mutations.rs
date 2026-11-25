use super::{
    attribute, children, direct, python_scope, targets, writes::write_targets, PythonBindings,
    PythonKind, PythonMember, PythonRouter, PythonValue, SetupContext,
};
use crate::ast::ParsedFile;
use crate::framework_entries::routes::python::ungroup;
use std::collections::BTreeSet;
use tree_sitter::Node;

type Allocation<'a> = (&'a str, usize);

#[derive(Default)]
pub(super) struct AttributeWrites<'a> {
    pub(super) modules: BTreeSet<String>,
    routers: BTreeSet<Allocation<'a>>,
    converters: BTreeSet<Allocation<'a>>,
}

fn allocation<'a>(router: &PythonRouter<'a>) -> Allocation<'a> {
    (
        router.allocation.file.path.as_str(),
        router.allocation.node.start_byte(),
    )
}

impl<'a> AttributeWrites<'a> {
    pub(super) fn index(bindings: &PythonBindings<'a>) -> Self {
        let mut writes = Self::default();
        for (&path, &file) in &bindings.files {
            for &node in bindings.nodes(path) {
                for target in write_targets(node) {
                    if let Some((object, field)) = attribute(target) {
                        for value in possible_values(bindings, file, object) {
                            match value {
                                PythonValue::Module(module) => {
                                    writes.modules.insert(module);
                                }
                                PythonValue::Router(router) => {
                                    writes.routers.insert(allocation(&router));
                                }
                                _ => {}
                            }
                        }
                        if file.node_text(&field) == "converters" {
                            writes
                                .converters
                                .extend(url_map_owner(bindings, file, object));
                        }
                    } else if target.kind() == "subscript" {
                        if let Some(registry) = target.child_by_field_name("value") {
                            writes
                                .converters
                                .extend(converter_owner(bindings, file, registry));
                        }
                    }
                }
                if node.kind() == "call" {
                    if let Some((registry, method)) =
                        node.child_by_field_name("function").and_then(attribute)
                    {
                        if matches!(
                            file.node_text(&method),
                            "update"
                                | "clear"
                                | "setdefault"
                                | "pop"
                                | "popitem"
                                | "__setitem__"
                                | "__delitem__"
                                | "__ior__"
                        ) {
                            writes
                                .converters
                                .extend(converter_owner(bindings, file, registry));
                        }
                    }
                }
            }
        }
        writes
    }
}

// Mutation indexing has no setup order. Consider earlier reaching bindings
// when a write is guarded or a nested scope reads an outer name.
fn possible_values<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    node: Node<'a>,
) -> Vec<PythonValue<'a>> {
    let mut values = Vec::new();
    let mut pending = vec![ungroup(node)];
    let mut seen = BTreeSet::new();
    let context = SetupContext::default();
    while let Some(node) = pending.pop() {
        if !seen.insert(node.id()) {
            continue;
        }
        values.push(bindings.value_at(&file.path, node, &context));
        if node.kind() != "identifier" {
            continue;
        }
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
                if write.kind() == "assignment"
                    && write
                        .child_by_field_name("left")
                        .is_some_and(|lhs| lhs.kind() == "identifier")
                {
                    pending.extend(write.child_by_field_name("right").map(ungroup));
                } else if matches!(write.kind(), "import_statement" | "import_from_statement") {
                    values.push(bindings.bound(
                        PythonMember { file, node: write },
                        name,
                        &context,
                        24,
                    ));
                }
                if scope == local && direct(write, scope) {
                    break;
                }
                before = write.start_byte();
            }
            if scope.kind() == "module" {
                break;
            }
            let parameter = scope
                .child_by_field_name("parameters")
                .is_some_and(|params| {
                    children(params)
                        .into_iter()
                        .flat_map(targets)
                        .any(|id| file.node_text(&id) == name)
                });
            let local_shadow = scope.kind() == "function_definition"
                && bindings
                    .reaching(file, scope, name, scope.end_byte())
                    .is_some()
                && !bindings.uncertain_global(file, name);
            if parameter || local_shadow || (scope.kind() == "class_definition" && had_write) {
                break;
            }
            let outer = python_scope(scope);
            if outer == scope {
                break;
            }
            scope = outer;
        }
    }
    values
}

fn converter_owner<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    registry: Node<'a>,
) -> Vec<Allocation<'a>> {
    members_named(bindings, file, registry, "converters")
        .into_iter()
        .flat_map(|(file, map)| url_map_owner(bindings, file, map))
        .collect()
}

fn url_map_owner<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    object: Node<'a>,
) -> Vec<Allocation<'a>> {
    members_named(bindings, file, object, "url_map")
        .into_iter()
        .flat_map(|(file, object)| possible_values(bindings, file, object))
        .filter_map(|value| match value {
            PythonValue::Router(router) => Some(allocation(&router)),
            _ => None,
        })
        .collect()
}

fn members_named<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    object: Node<'a>,
    field: &str,
) -> Vec<(&'a ParsedFile, Node<'a>)> {
    let object = ungroup(object);
    if object.kind() != "identifier"
        && !attribute(object).is_some_and(|(_, name)| file.node_text(&name) == field)
    {
        return vec![];
    }
    possible_values(bindings, file, object)
        .into_iter()
        .filter_map(|value| {
            let PythonValue::Opaque(member) = value else {
                return None;
            };
            let (object, name) = attribute(member.node)?;
            (member.file.node_text(&name) == field).then_some((member.file, object))
        })
        .collect()
}

impl<'a> PythonBindings<'a> {
    pub(super) fn router_mutated(&self, router: &PythonRouter<'a>) -> bool {
        self.mutations
            .as_ref()
            .is_some_and(|writes| writes.routers.contains(&allocation(router)))
    }

    pub(super) fn checked_router(&self, value: PythonValue<'a>) -> PythonValue<'a> {
        match value {
            PythonValue::Router(ref router) if self.router_mutated(router) => {
                PythonValue::Unknown(super::Reason::ReceiverUnproven)
            }
            _ => value,
        }
    }

    pub(crate) fn default_converters(&self, router: &PythonRouter<'a>) -> bool {
        router.kind == PythonKind::FlaskApp
            && self
                .mutations
                .as_ref()
                .is_some_and(|writes| !writes.converters.contains(&allocation(router)))
    }
}
