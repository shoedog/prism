use super::{context::Protocol, PathMatch, PathUse, Resolution, Resolver};
use crate::targets::inventory::{
    python::{
        source::{name, ungroup},
        walk::ScopeId,
    },
    Family, Grade, UnprovenCause,
};
use tree_sitter::Node;

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn inline_receiver(
        &self,
        callee: Node<'tree>,
        scope: ScopeId,
    ) -> Option<Resolution> {
        let (receiver, method) = self.selected_method(callee, scope)?;
        let (family, class) = self.method_constructor(receiver, scope, &method, None)?;
        let proven = class.and_then(|class| {
            self.prove_instance_method(&class, &method, callee, scope)?;
            Ok(class)
        });
        Some(Resolution {
            family,
            grade: Grade::Exact,
            #[cfg(any(test, feature = "inventory-evidence"))]
            declaration: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            route: proven.is_ok().then_some(super::super::evidence::Route::R1),
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver: proven
                .as_ref()
                .ok()
                .map(|class| super::super::evidence::ReceiverProof::constructed(class, receiver)),
            identity: proven.map(|class| format!("{class}.{method}")),
        })
    }

    pub(super) fn selected_method(
        &self,
        callee: Node<'tree>,
        scope: ScopeId,
    ) -> Option<(Node<'tree>, String)> {
        let callee = ungroup(callee);
        if callee.kind() != "attribute" {
            return None;
        }
        let receiver = ungroup(callee.child_by_field_name("object")?);
        let method = callee.child_by_field_name("attribute")?;
        Some((
            receiver,
            name(self.parsed.node_text(&method), self.walk.class(scope)),
        ))
    }

    pub(super) fn constructor(
        &self,
        call: Node<'tree>,
        scope: ScopeId,
        protocol: Option<Protocol>,
    ) -> Option<Result<String, UnprovenCause>> {
        self.constructor_match(call, scope, None, protocol)
            .map(|(_, class)| class)
    }

    pub(super) fn method_constructor(
        &self,
        call: Node<'tree>,
        scope: ScopeId,
        method: &str,
        protocol: Option<Protocol>,
    ) -> Option<(Family, Result<String, UnprovenCause>)> {
        let (family, class) = self.constructor_match(call, scope, Some(method), protocol)?;
        Some((family?, class))
    }

    fn constructor_match(
        &self,
        call: Node<'tree>,
        scope: ScopeId,
        method: Option<&str>,
        protocol: Option<Protocol>,
    ) -> Option<(Option<Family>, Result<String, UnprovenCause>)> {
        if call.kind() != "call" {
            return None;
        }
        let callee = self.parsed.call_function_node(&call)?;
        let (kind, class) =
            self.resolve_path(callee, scope, PathUse::Constructor { method, protocol })?;
        let PathMatch::Class(family) = kind else {
            return None;
        };
        let read = self.read(call, scope);
        Some((
            family,
            class.and_then(|class| {
                if self.reads_member(&class, "__new__", read)
                    || self.reads_instance_member_at(&class, "__init__", read, |_| true)
                {
                    return Err(UnprovenCause::MemberRebound);
                }
                Ok(class)
            }),
        ))
    }

    pub(super) fn prove_instance_method(
        &self,
        class: &str,
        method: &str,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> Result<(), UnprovenCause> {
        if self.instance_method_rebound_at(class, method, self.read(at, scope)) {
            return Err(UnprovenCause::MemberRebound);
        }
        Ok(())
    }
}
