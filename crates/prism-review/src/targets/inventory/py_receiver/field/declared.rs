use super::shape::literal_self;
use crate::targets::inventory::{
    py_receiver::{
        annotation::Declaration,
        census::{Binding, Kind},
        named::assignment_value,
        Resolution, Resolver,
    },
    py_tables,
    python::{
        source::name,
        walk::{ScopeId, ScopeKind},
    },
    Family, Grade, UnprovenCause,
};
use tree_sitter::Node;

pub(super) struct FieldUse<'a, 'tree> {
    pub(super) owner: ScopeId,
    pub(super) scope: ScopeId,
    pub(super) using: &'a Binding<'tree>,
    pub(super) using_name: String,
    pub(super) callee: Node<'tree>,
    #[cfg(any(test, feature = "inventory-evidence"))]
    receiver: Node<'tree>,
    pub(super) self_ref: Node<'tree>,
    pub(super) method: String,
    pub(super) field: String,
    pub(super) direct: bool,
}

struct FieldStore<'a, 'tree> {
    scope: ScopeId,
    node: Node<'tree>,
    self_ref: Node<'tree>,
    self_binding: &'a Binding<'tree>,
    value: Node<'tree>,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn declared_field_receiver(
        &self,
        callee: Node<'tree>,
        scope: ScopeId,
        search_constructors: bool,
    ) -> Option<Resolution> {
        let (receiver, method) = self.selected_method(callee, scope)?;
        let mut prefix = receiver;
        let (self_ref, field) = loop {
            let (base, member) = self.selected_method(prefix, scope)?;
            if base.kind() == "identifier" && self.parsed.node_text(&base) == "self" {
                break (base, member);
            }
            prefix = base;
        };
        let (owner, using) = self.field_self(self_ref, scope)?;
        let method_node = self.walk.scopes[using.scope]
            .node
            .child_by_field_name("name")?;
        let site = FieldUse {
            owner,
            scope,
            using,
            using_name: name(
                self.parsed.node_text(&method_node),
                self.walk.class(using.scope),
            ),
            callee,
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver,
            self_ref,
            method,
            field,
            direct: prefix == receiver,
        };

        let mut result = None;
        for &(store_scope, target, value) in &self.census.field_stores {
            let Some(store) = self.field_store(&site, store_scope, target, value) else {
                continue;
            };
            result = Resolution::prefer(result, || {
                self.field_store_resolution(&site, &store, search_constructors)
            });
        }
        result
    }

    fn field_store(
        &self,
        site: &FieldUse<'_, 'tree>,
        scope: ScopeId,
        target: Node<'tree>,
        value: Node<'tree>,
    ) -> Option<FieldStore<'_, 'tree>> {
        let (self_ref, field) = self.selected_method(target, scope)?;
        if field != site.field {
            return None;
        }
        let (owner, binding) = self.field_self(self_ref, scope)?;
        if owner != site.owner {
            return None;
        }
        // The stored value is the outer assignment's right child, including
        // the inner assignment in a chained store.
        Some(FieldStore {
            scope,
            node: value.parent()?,
            self_ref,
            self_binding: binding,
            value: assignment_value(value)?,
        })
    }

    fn field_store_resolution(
        &self,
        site: &FieldUse<'_, 'tree>,
        store: &FieldStore<'_, 'tree>,
        search_constructors: bool,
    ) -> Option<Resolution> {
        if store.value.kind() != "identifier" {
            if !search_constructors {
                return None;
            }
            let (family, constructor) =
                self.method_constructor(store.value, store.scope, &site.method, None)?;
            let lookup = constructor.and_then(|class| {
                self.prove_instance_method(&class, &site.method, site.callee, site.scope)
            });
            let cause = lookup
                .err()
                .into_iter()
                .chain(self.field_site_veto(site, store.node))
                .min()
                .unwrap_or(UnprovenCause::ReceiverValueUnproven);
            return Some(Resolution::refused(family, Grade::Exact, cause));
        }
        let key = name(
            self.parsed.node_text(&store.value),
            self.walk.class(store.scope),
        );
        self.lookup(store.scope, &key, store.value)
            .iter()
            .flat_map(|alt| &alt.bindings)
            .filter(|binding| matches!(binding.kind, Kind::Parameter))
            .filter_map(|binding| {
                let annotation = binding.node.child_by_field_name("type")?;
                let scope = self.walk.scopes[binding.scope].parent;
                let (family, declaration) =
                    self.field_method_declaration(annotation, scope, &site.method)?;
                Some(self.prove_field_declaration(site, store, binding, &key, family, declaration))
            })
            .fold(None, |previous, candidate| {
                Resolution::prefer(previous, || Some(candidate))
            })
    }

    fn prove_field_declaration(
        &self,
        site: &FieldUse<'_, 'tree>,
        store: &FieldStore<'_, 'tree>,
        binding: &Binding<'tree>,
        key: &str,
        family: Family,
        declaration: Result<Declaration<'tree>, UnprovenCause>,
    ) -> Resolution {
        let unique = self.field_parameter_unique(binding, key, store.value, store.scope)
            && self.field_parameter_unique(store.self_binding, "self", store.self_ref, store.scope);
        let veto = self.field_site_veto(site, store.node);
        if let Some(cause) = declaration
            .as_ref()
            .err()
            .copied()
            .into_iter()
            .chain(veto)
            .chain((!unique).then_some(UnprovenCause::ReceiverNotUnique))
            .min()
        {
            return Resolution::refused(family, Grade::AnnotationTyped, cause);
        }

        let declaration = declaration.expect("annotation failures are handled before shape");
        if declaration.nullable
            || !py_tables::declaration_only_class(&declaration.class)
            || !self.field_use_shape(site)
            || !self.initializer_field_shape(
                site.owner,
                store.scope,
                store.node,
                store.value,
                binding,
                store.self_binding,
            )
            || !self.indexed_field_seed(site.owner, &site.field, store.node)
        {
            return Resolution::refused(
                family,
                Grade::AnnotationTyped,
                UnprovenCause::ReceiverValueUnproven,
            );
        }

        Resolution {
            family,
            grade: Grade::AnnotationTyped,
            identity: Ok(format!("{}.{}", declaration.class, site.method)),
            #[cfg(any(test, feature = "inventory-evidence"))]
            route: Some(crate::targets::inventory::evidence::Route::R3),
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            declaration: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            field: {
                use crate::targets::inventory::evidence::{FieldProof, Span};
                let span = |node| Span::new(self.source, node);
                Some(Box::new(FieldProof {
                    class: declaration.class,
                    owner: span(self.walk.scopes[site.owner].node),
                    initializer: span(self.walk.scopes[store.scope].node),
                    parameter: span(binding.node),
                    annotation: span(declaration.node),
                    store: span(store.node),
                    receiver_use: span(site.receiver),
                }))
            },
        }
    }

    // A class method's parameter supplies the diagnostic tie even when it
    // competes with assignments or belongs to an excluded method shape.
    fn field_self(&self, at: Node<'tree>, scope: ScopeId) -> Option<(ScopeId, &Binding<'tree>)> {
        if at.kind() != "identifier" || self.parsed.node_text(&at) != "self" {
            return None;
        }
        self.lookup(scope, "self", at)
            .iter()
            .flat_map(|alt| &alt.bindings)
            .find_map(|binding| {
                let routine = &self.walk.scopes[binding.scope];
                let owner = routine.parent;
                (routine.kind == ScopeKind::Function
                    && self.walk.scopes[owner].kind == ScopeKind::Class
                    && literal_self(self.parsed, binding))
                .then_some((owner, *binding))
            })
    }
}
