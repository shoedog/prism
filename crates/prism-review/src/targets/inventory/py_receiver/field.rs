mod declared;
mod shape;
mod veto;

use declared::FieldUse;

use super::{
    census::{parameter_name, Binding, Kind},
    named::assignment_value,
    slots::{Instance, Slot},
    Resolution, Resolver,
};
use crate::targets::inventory::{
    python::{
        source::{children, name},
        walk::{ScopeId, ScopeKind},
    },
    Grade, UnprovenCause,
};
use tree_sitter::Node;

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn field_receiver(&self, callee: Node<'tree>, scope: ScopeId) -> Option<Resolution> {
        let constructor = self.field_constructor_diagnostic(callee, scope);
        if !self.field_rows {
            return constructor;
        }
        // Preserve the existing constructor diagnostic when it already has a
        // tie. Relaxed discovery also covers shapes that never had one.
        let search_constructors = constructor.is_none();
        Resolution::prefer(constructor, || {
            self.declared_field_receiver(callee, scope, search_constructors)
        })
    }

    fn field_constructor_diagnostic(
        &self,
        callee: Node<'tree>,
        scope: ScopeId,
    ) -> Option<Resolution> {
        let (receiver, method) = self.selected_method(callee, scope)?;
        let (owner, field) = self.literal_self_field(receiver, scope)?;
        let instance = Instance::Field {
            owner,
            name: field.clone(),
        };
        let (cause, family) = self
            .census
            .field_stores
            .iter()
            .filter_map(|&(scope, target, value)| {
                let (written_owner, written_field) = self.literal_self_field(target, scope)?;
                if written_owner != owner || written_field != field {
                    return None;
                }
                self.method_constructor(assignment_value(value)?, scope, &method, None)
            })
            .map(|(family, constructor)| {
                let class = match constructor {
                    Ok(class) => class,
                    Err(cause) => return (cause, family),
                };
                if self
                    .prove_instance_method(&class, &method, callee, scope)
                    .is_err()
                    || self.instance_rebound(&instance, Some(&method), callee, scope, None)
                {
                    return (UnprovenCause::MemberRebound, family);
                }
                let stores = self
                    .writes
                    .iter()
                    .filter(|write| {
                        matches!(&write.slot, Slot::Instance {
                    origin: Instance::OwningSelf(written),
                    key,
                } if *written == owner && key.as_deref().is_none_or(|key| key == field))
                    })
                    .take(2)
                    .count();
                let cause = if stores == 2 {
                    UnprovenCause::ReceiverNotUnique
                } else {
                    UnprovenCause::ReceiverValueUnproven
                };
                (cause, family)
            })
            .min()?;
        Some(Resolution::refused(family, Grade::Exact, cause))
    }

    fn literal_self_field(&self, target: Node<'tree>, scope: ScopeId) -> Option<(ScopeId, String)> {
        let (base, field) = self.selected_method(target, scope)?;
        if base.kind() != "identifier"
            || name(self.parsed.node_text(&base), self.walk.class(scope)) != "self"
        {
            return None;
        }
        let alternatives = self.lookup(scope, "self", base);
        let [alt] = alternatives.as_slice() else {
            return None;
        };
        let [binding] = alt.bindings.as_slice() else {
            return None;
        };
        Some((self.owning_self(binding)?, field))
    }

    pub(super) fn owning_self(&self, binding: &Binding<'tree>) -> Option<ScopeId> {
        let routine = &self.walk.scopes[binding.scope];
        let owner = routine.parent;
        if !matches!(binding.kind, Kind::Parameter)
            || routine.kind != ScopeKind::Function
            || self.walk.scopes[owner].kind != ScopeKind::Class
            || routine
                .node
                .parent()
                .is_some_and(|parent| parent.kind() == "decorated_definition")
        {
            return None;
        }
        let first = children(routine.node.child_by_field_name("parameters")?).next()?;
        if first != binding.node
            || !matches!(
                first.kind(),
                "identifier" | "typed_parameter" | "default_parameter" | "typed_default_parameter"
            )
        {
            return None;
        }
        let parameter = parameter_name(first)?;
        (name(
            self.parsed.node_text(&parameter),
            self.walk.class(binding.scope),
        ) == "self")
            .then_some(owner)
    }
}
