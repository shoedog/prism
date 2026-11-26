use super::{context::Protocol, PathUse, Resolution, Resolver};
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
        let proven = self
            .constructor(receiver, scope, Some(&method), None)?
            .and_then(|class| {
                self.prove_instance_method(&class, &method, callee, scope)?;
                Ok(class)
            });
        Some(Resolution {
            family: Family::Http,
            grade: Grade::Exact,
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
        method: Option<&str>,
        protocol: Option<Protocol>,
    ) -> Option<Result<String, UnprovenCause>> {
        if call.kind() != "call" {
            return None;
        }
        let callee = self.parsed.call_function_node(&call)?;
        let (_, class) =
            self.resolve_path(callee, scope, PathUse::Constructor { method, protocol })?;
        let read = self.read(call, scope);
        Some(class.and_then(|class| {
            if ["__new__", "__init__"]
                .iter()
                .any(|slot| self.reads_member(&class, slot, read))
            {
                return Err(UnprovenCause::MemberRebound);
            }
            Ok(class)
        }))
    }

    pub(super) fn prove_instance_method(
        &self,
        class: &str,
        method: &str,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> Result<(), UnprovenCause> {
        let read = self.read(at, scope);
        if self.reads_member(class, "__getattribute__", read)
            || self.reads_member(class, method, read)
        {
            return Err(UnprovenCause::MemberRebound);
        }
        Ok(())
    }
}
