use super::FieldUse;
use crate::targets::inventory::{
    py_receiver::{
        census::{parameter_name, Binding, Kind},
        lookup::{args, direct},
        writes::is_literal_string,
        Resolver,
    },
    python::{
        source::{children, name, ungroup},
        walk::{ScopeId, ScopeKind},
    },
};
use tree_sitter::Node;

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn field_parameter_unique(
        &self,
        binding: &Binding<'tree>,
        key: &str,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> bool {
        let alternatives = self.lookup(scope, key, at);
        matches!(&alternatives[..], [alt]
            if alt.owner == binding.scope
                && matches!(&alt.bindings[..], [found] if found.node == binding.node)
                && !self.name_rebound(alt.owner, key, at, scope))
    }

    pub(super) fn field_definitions(&self, owner: ScopeId, key: &str) -> usize {
        self.class_field_bindings(owner, key)
            .filter(|binding| binding.node.kind() == "function_definition")
            .count()
    }

    pub(super) fn class_field_bindings(
        &self,
        scope: ScopeId,
        key: &str,
    ) -> impl Iterator<Item = &Binding<'tree>> {
        self.census.spaces[scope]
            .bindings
            .get(key)
            .into_iter()
            .flatten()
            .filter(move |binding| {
                binding.scope == scope && !matches!(binding.kind, Kind::Annotation)
            })
    }

    pub(super) fn field_use_shape(&self, site: &FieldUse<'_, 'tree>) -> bool {
        let using = &self.walk.scopes[site.using.scope];
        let mut scope = site.scope;
        while self.walk.scopes[scope].kind == ScopeKind::Comprehension {
            scope = self.walk.scopes[scope].parent;
        }
        site.direct
            && scope == site.using.scope
            && site.using_name != "__init__"
            && undecorated(using.node)
            && first_parameter(using.node) == Some(site.using.node)
            && simple_self(site.using.node)
    }

    pub(super) fn initializer_field_shape(
        &self,
        owner: ScopeId,
        store_scope: ScopeId,
        store: Node<'tree>,
        rhs: Node<'tree>,
        parameter: &Binding<'tree>,
        seed_self: &Binding<'tree>,
    ) -> bool {
        let class = &self.walk.scopes[owner];
        let init = &self.walk.scopes[store_scope];
        let Some(init_name) = init.node.child_by_field_name("name") else {
            return false;
        };
        if !self.field_owner_shape(owner)
            || init.kind != ScopeKind::Function
            || init.parent != owner
            || name(self.parsed.node_text(&init_name), class.class.as_deref()) != "__init__"
            || !undecorated(init.node)
            || !direct(class.body, init.node)
            || is_async(init.node)
            || init.node.has_error()
            || self.census.spaces[store_scope].yields
            || seed_self.scope != store_scope
            || first_parameter(init.node) != Some(seed_self.node)
            || seed_self.node.kind() != "identifier"
            || parameter.scope != store_scope
            || parameter.node == seed_self.node
            || parameter.node.kind() != "typed_parameter"
            || children(parameter.node)
                .next()
                .is_none_or(|target| target.kind() != "identifier")
            || store.child_by_field_name("type").is_some()
            || store.child_by_field_name("right").map(ungroup) != Some(rhs)
        {
            return false;
        }
        self.field_seed_prefix(store_scope, store)
    }

    pub(in crate::targets::inventory::py_receiver) fn field_owner_shape(
        &self,
        scope: ScopeId,
    ) -> bool {
        let class = &self.walk.scopes[scope];
        if !undecorated(class.node) || class.node.child_by_field_name("type_parameters").is_some() {
            return false;
        }
        let mut bases = class
            .node
            .child_by_field_name("superclasses")
            .into_iter()
            .flat_map(children);
        let Some(base) = bases.next().map(ungroup) else {
            return true;
        };
        let key = name(self.parsed.node_text(&base), self.walk.class(class.parent));
        bases.next().is_none()
            && base.kind() == "identifier"
            && matches!(key.as_str(), "object" | "dict")
            && self.field_builtin(class.parent, &key, base)
    }

    fn field_seed_prefix(&self, scope: ScopeId, store: Node<'tree>) -> bool {
        let block = self.walk.scopes[scope].body;
        let Some(statement) = store
            .parent()
            .filter(|stmt| stmt.kind() == "expression_statement" && stmt.parent() == Some(block))
        else {
            return false;
        };
        let mut super_seen = false;
        for (index, prefix) in children(block).enumerate() {
            if prefix == statement {
                return true;
            }
            if prefix.kind() == "pass_statement" {
                continue;
            }
            let Some(expression) = children(prefix)
                .next()
                .filter(|_| prefix.kind() == "expression_statement")
                .map(ungroup)
            else {
                return false;
            };
            if index == 0 && is_literal_string(self.parsed, expression) {
                continue;
            }
            if super_seen || !self.field_super_init(scope, expression) {
                return false;
            }
            super_seen = true;
        }
        false
    }

    fn field_super_init(&self, scope: ScopeId, call: Node<'tree>) -> bool {
        let Some((receiver, method)) = self
            .parsed
            .call_function_node(&call)
            .and_then(|callee| self.selected_method(callee, scope))
        else {
            return false;
        };
        let Some(super_name) = self.parsed.call_function_node(&receiver).map(ungroup) else {
            return false;
        };
        method == "__init__"
            && super_name.kind() == "identifier"
            && name(self.parsed.node_text(&super_name), self.walk.class(scope)) == "super"
            && args(call).next().is_none()
            && args(receiver).next().is_none()
            && self.field_builtin(scope, "super", super_name)
    }

    fn field_builtin(&self, scope: ScopeId, key: &str, at: Node<'tree>) -> bool {
        self.builtin_unshadowed(scope, key, at)
            && !self.builtin_rebound(key, at, scope)
            && !self.name_rebound(0, "__builtins__", at, scope)
    }
}

fn first_parameter(node: Node<'_>) -> Option<Node<'_>> {
    children(node.child_by_field_name("parameters")?).next()
}

fn simple_self(parameter: Node<'_>) -> bool {
    match parameter.kind() {
        "identifier" | "default_parameter" | "typed_default_parameter" => true,
        "typed_parameter" => children(parameter)
            .next()
            .is_some_and(|node| node.kind() == "identifier"),
        _ => false,
    }
}

fn undecorated(node: Node<'_>) -> bool {
    node.parent()
        .is_none_or(|parent| parent.kind() != "decorated_definition")
}

fn is_async(node: Node<'_>) -> bool {
    node.child(0).is_some_and(|first| first.kind() == "async")
}

pub(super) fn literal_self(parsed: &prism::ast::ParsedFile, binding: &Binding<'_>) -> bool {
    matches!(binding.kind, Kind::Parameter)
        && parameter_name(binding.node).is_some_and(|node| parsed.node_text(&node) == "self")
}
