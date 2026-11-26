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
    Family, Grade, UnprovenCause,
};
use tree_sitter::Node;

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn field_receiver(&self, callee: Node<'tree>, scope: ScopeId) -> Option<Resolution> {
        let (receiver, method) = self.selected_method(callee, scope)?;
        let (owner, field) = self.literal_self_field(receiver, scope)?;
        let instance = Instance::Field {
            owner,
            name: field.clone(),
        };
        let cause = self
            .census
            .field_stores
            .iter()
            .filter_map(|&(scope, target, value)| {
                let (written_owner, written_field) = self.literal_self_field(target, scope)?;
                if written_owner != owner || written_field != field {
                    return None;
                }
                self.constructor(assignment_value(value)?, scope, Some(&method), None)
            })
            .map(|constructor| {
                let class = match constructor {
                    Ok(class) => class,
                    Err(cause) => return cause,
                };
                if self
                    .prove_instance_method(&class, &method, callee, scope)
                    .is_err()
                    || self.instance_rebound(&instance, Some(&method), callee, scope, None)
                {
                    return UnprovenCause::MemberRebound;
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
                if stores == 2 {
                    UnprovenCause::ReceiverNotUnique
                } else {
                    UnprovenCause::ReceiverValueUnproven
                }
            })
            .min()?;
        Some(Resolution {
            family: Family::Http,
            grade: Grade::Exact,
            identity: Err(cause),
            #[cfg(any(test, feature = "inventory-evidence"))]
            route: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver: None,
        })
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
