use super::{
    attribute, children, writes::write_targets, PythonBindings, PythonKind, PythonRouter,
    PythonValue, SetupContext,
};
use crate::ast::ParsedFile;
use crate::framework_entries::routes::python::{arguments, literal_string, ungroup};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

mod escapes;
mod metadata;
mod modules;
mod reach;
mod values;
use escapes::ViewArgument;
use metadata::MetadataWrites;
use reach::FileReach;
use values::{possible_values, unbound_name};

type Allocation<'a> = (&'a str, usize);
type Mounts<'a> = BTreeMap<Allocation<'a>, BTreeSet<Allocation<'a>>>;

#[derive(Default)]
pub(super) struct AttributeWrites<'a> {
    pub(super) modules: BTreeSet<String>,
    routers: BTreeSet<Allocation<'a>>,
    converters: BTreeSet<Allocation<'a>>,
    replaced_configs: BTreeSet<Allocation<'a>>,
    router_belts: BTreeSet<&'a str>,
    converter_belts: BTreeSet<&'a str>,
    reach: FileReach<'a>,
    metadata: MetadataWrites<'a>,
    view_arguments: BTreeMap<Allocation<'a>, Vec<ViewArgument<'a>>>,
}

fn allocation<'a>(router: &PythonRouter<'a>) -> Allocation<'a> {
    (
        router.allocation.file.path.as_str(),
        router.allocation.node.start_byte(),
    )
}

impl<'a> AttributeWrites<'a> {
    pub(super) fn index(bindings: &PythonBindings<'a>) -> Self {
        let mut writes = Self {
            reach: FileReach::index(bindings),
            ..Self::default()
        };
        let mut mounts = Mounts::new();
        for (&path, &file) in &bindings.files {
            for &node in bindings.nodes(path) {
                let mut stored = false;
                for target in write_targets(node) {
                    if let Some((object, field)) = attribute(target) {
                        let value = (node.kind() == "assignment"
                            && node.child_by_field_name("left") == Some(target))
                        .then(|| node.child_by_field_name("right"))
                        .flatten();
                        writes.field(bindings, file, node, object, file.node_text(&field), value);
                        stored = true;
                    } else if target.kind() == "subscript" {
                        if let Some(container) = target.child_by_field_name("value") {
                            writes.container_write(bindings, file, container);
                        }
                        stored = true;
                    }
                }
                if stored {
                    if let Some(value) = node.child_by_field_name("right") {
                        writes.escape(bindings, file, value, None);
                    }
                }
                if node.kind() == "call" {
                    writes.call(bindings, file, node, &mut mounts);
                }
            }
        }
        // A mount can pass its blueprint to an opaque implementation when
        // another site invalidates the parent receiver.
        let mut pending: Vec<_> = mounts
            .keys()
            .copied()
            .filter(|&parent| writes.router_tainted(parent))
            .collect();
        while let Some(parent) = pending.pop() {
            if let Some(children) = mounts.remove(&parent) {
                for child in children {
                    writes.routers.insert(child);
                    pending.push(child);
                }
            }
        }
        writes
    }

    fn router_tainted(&self, allocation: Allocation<'a>) -> bool {
        self.routers.contains(&allocation)
            || self
                .router_belts
                .contains(self.reach.component(allocation.0))
    }

    fn field(
        &mut self,
        bindings: &PythonBindings<'a>,
        file: &'a ParsedFile,
        node: Node<'a>,
        object: Node<'a>,
        field: &str,
        value: Option<Node<'a>>,
    ) {
        let owners = possible_values(bindings, file, object);
        self.metadata
            .record(file, node, object, field, value, &owners);
        let resolved = matches!(
            owners.first(),
            Some(PythonValue::Module(_) | PythonValue::Router(_) | PythonValue::Definition(_))
        );
        for owner in owners {
            match owner {
                PythonValue::Module(module) => {
                    self.modules.insert(module);
                }
                PythonValue::Router(router) => {
                    if field == "config" {
                        self.replaced_configs.insert(allocation(&router));
                    }
                    if !setup_field(field) {
                        self.routers.insert(allocation(&router));
                    }
                }
                _ => {}
            }
        }
        if !matches!(field, "converters" | "url_map") && !route_field(field) {
            return;
        }
        let resolved = resolved || is_builtin_object(bindings, file, object);
        let component = self.reach.component(&file.path);
        if field == "converters" {
            let maps = url_map_owner(bindings, file, object);
            if maps.is_empty() && !resolved {
                self.converter_belts.insert(component);
            }
            self.converters.extend(maps);
        } else if !resolved {
            if field == "url_map" {
                self.converter_belts.insert(component);
            } else if route_field(field) {
                self.router_belts.insert(component);
            }
        }
    }

    fn container_write(
        &mut self,
        bindings: &PythonBindings<'a>,
        file: &'a ParsedFile,
        container: Node<'a>,
    ) {
        for (file, map) in members_named(bindings, file, container, "converters") {
            let maps = url_map_owner(bindings, file, map);
            if maps.is_empty() {
                self.converter_belts
                    .insert(self.reach.component(&file.path));
            }
            self.converters.extend(maps);
        }
        for (file, owner) in members_named(bindings, file, container, "view_functions") {
            if !matches!(
                possible_values(bindings, file, owner).first(),
                Some(PythonValue::Router(_) | PythonValue::Module(_) | PythonValue::Definition(_))
            ) && !is_builtin_object(bindings, file, owner)
            {
                self.router_belts.insert(self.reach.component(&file.path));
            }
        }
    }

    fn call(
        &mut self,
        bindings: &PythonBindings<'a>,
        file: &'a ParsedFile,
        call: Node<'a>,
        mounts: &mut Mounts<'a>,
    ) {
        let Some(callee) = call.child_by_field_name("function") else {
            return;
        };
        let args = arguments(call);
        let mut native_owner = None;
        let function = file.node_text(&callee);
        if matches!(function, "setattr" | "delattr") {
            let slots = if function == "setattr" {
                &["obj", "name", "value"][..]
            } else {
                &["obj", "name"][..]
            };
            if let Some((owner, field)) = args.get(file, "obj", 0).written().zip(
                args.get(file, "name", 1)
                    .written()
                    .and_then(|name| literal_string(file, name)),
            ) {
                self.field(bindings, file, call, owner, &field, None);
                if args.accepts(file, slots, false)
                    && !args.unpacked()
                    && unbound_name(bindings, file, callee)
                {
                    native_owner = Some(owner);
                }
            }
        } else if let Some((object, method)) = attribute(callee) {
            let method = file.node_text(&method);
            if method == "register_blueprint"
                && args.accepts(file, &["blueprint"], true)
                && !args.unpacked()
            {
                let context = SetupContext::default();
                if let Some(child) = args.get(file, "blueprint", 0).written() {
                    if let (PythonValue::Router(parent), PythonValue::Router(blueprint)) = (
                        bindings.value_at(&file.path, object, &context),
                        bindings.value_at(&file.path, child, &context),
                    ) {
                        if blueprint.kind == PythonKind::Blueprint {
                            mounts
                                .entry(allocation(&parent))
                                .or_default()
                                .insert(allocation(&blueprint));
                            native_owner = Some(child);
                        }
                    }
                }
            }
            self.converters
                .extend(url_map_owner(bindings, file, object));
            if matches!(
                method,
                "update"
                    | "clear"
                    | "setdefault"
                    | "pop"
                    | "popitem"
                    | "__setitem__"
                    | "__delitem__"
                    | "__ior__"
            ) {
                self.container_write(bindings, file, object);
            }
        }
        for arg in args.nodes() {
            let value = if arg.kind() == "keyword_argument" {
                arg.child_by_field_name("value").unwrap_or(arg)
            } else {
                arg
            };
            if Some(value) != native_owner {
                self.escape(bindings, file, value, Some(call));
            }
        }
    }

    fn escape(
        &mut self,
        bindings: &PythonBindings<'a>,
        file: &'a ParsedFile,
        expression: Node<'a>,
        call: Option<Node<'a>>,
    ) {
        let expression = ungroup(expression);
        match expression.kind() {
            "tuple"
            | "list"
            | "set"
            | "dictionary"
            | "pair"
            | "list_splat"
            | "dictionary_splat"
            | "conditional_expression"
            | "boolean_operator" => {
                for member in children(expression) {
                    self.escape(bindings, file, member, call);
                }
            }
            "identifier" | "attribute" | "call" => {
                let values = possible_values(bindings, file, expression);
                for value in values {
                    match value {
                        PythonValue::Router(router) => {
                            self.routers.insert(allocation(&router));
                        }
                        PythonValue::Definition(member)
                            if member.node.kind() == "function_definition" =>
                        {
                            if let Some(call) = call {
                                self.view_arguments
                                    .entry(metadata::definition(member))
                                    .or_default()
                                    .push(ViewArgument {
                                        file,
                                        call,
                                        value: expression,
                                    });
                            }
                        }
                        PythonValue::Opaque(member) => {
                            let Some((object, field)) = attribute(member.node) else {
                                continue;
                            };
                            match member.file.node_text(&field) {
                                "url_map" => self.converters.extend(router_allocations(
                                    bindings,
                                    member.file,
                                    object,
                                )),
                                "converters" => self.converters.extend(url_map_owner(
                                    bindings,
                                    member.file,
                                    object,
                                )),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_builtin_object<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    object: Node<'a>,
) -> bool {
    let PythonValue::Opaque(member) =
        bindings.value_at(&file.path, object, &SetupContext::default())
    else {
        return false;
    };
    member.node.kind() == "call"
        && member
            .node
            .child_by_field_name("function")
            .is_some_and(|callee| {
                member.file.node_text(&callee) == "object"
                    && unbound_name(bindings, member.file, callee)
            })
}

fn setup_field(field: &str) -> bool {
    matches!(
        field,
        "secret_key"
            | "debug"
            | "testing"
            | "wsgi_app"
            | "config"
            | "json"
            | "json_provider_class"
            | "json_encoder"
            | "json_decoder"
            | "session_interface"
            | "permanent_session_lifetime"
            | "jinja_env"
    )
}

fn route_field(field: &str) -> bool {
    matches!(
        field,
        "route"
            | "get"
            | "post"
            | "put"
            | "delete"
            | "patch"
            | "add_url_rule"
            | "register_blueprint"
            | "endpoint"
            | "record_once"
            | "view_functions"
    )
}

fn url_map_owner<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    object: Node<'a>,
) -> Vec<Allocation<'a>> {
    members_named(bindings, file, object, "url_map")
        .into_iter()
        .flat_map(|(file, object)| router_allocations(bindings, file, object))
        .collect()
}

fn router_allocations<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    object: Node<'a>,
) -> impl Iterator<Item = Allocation<'a>> {
    possible_values(bindings, file, object)
        .into_iter()
        .filter_map(|value| match value {
            PythonValue::Router(router) => Some(allocation(&router)),
            _ => None,
        })
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
            .is_some_and(|writes| writes.router_tainted(allocation(router)))
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
            && self.mutations.as_ref().is_some_and(|writes| {
                let allocation = allocation(router);
                !writes.converters.contains(&allocation)
                    && !writes
                        .converter_belts
                        .contains(writes.reach.component(allocation.0))
            })
    }

    pub(crate) fn config_replaced(&self, identity: &super::RouterId) -> bool {
        self.mutations.as_ref().is_some_and(|writes| {
            writes
                .replaced_configs
                .contains(&(identity.file.as_str(), identity.construction_byte))
        })
    }
}
