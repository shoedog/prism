use super::{
    census::{Binding, Kind},
    context::{contains, ContextUse, Protocol},
    lookup::{direct, Alternative},
    slots::{Instance, Slot},
    Resolution, Resolver,
};
use crate::targets::inventory::{
    python::{
        source::{children, name, ungroup},
        walk::{ScopeId, ScopeKind},
    },
    Family, Grade, UnprovenCause,
};
use tree_sitter::Node;

struct Construction<'tree> {
    call: Node<'tree>,
    context: Option<(Node<'tree>, Protocol)>,
}

impl<'tree> Construction<'tree> {
    fn from_binding(binding: &Binding<'tree>) -> Option<Self> {
        let (call, context) = match binding.kind {
            Kind::Assigned(Some(value)) => (value, None),
            Kind::Context => {
                let manager = std::iter::successors(binding.node.parent(), Node::parent)
                    .find(|node| node.kind() == "with_statement")?;
                let protocol = Protocol::at(manager);
                (children(binding.node).next()?, Some((manager, protocol)))
            }
            _ => return None,
        };
        let call = assignment_value(call)?;
        (call.kind() == "call").then_some(Self { call, context })
    }

    fn protocol(&self) -> Option<Protocol> {
        self.context.map(|(_, protocol)| protocol)
    }
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn named_receiver(&self, callee: Node<'tree>, scope: ScopeId) -> Option<Resolution> {
        let (receiver, method) = self.selected_method(callee, scope)?;
        if receiver.kind() != "identifier" {
            return None;
        }
        let key = name(self.parsed.node_text(&receiver), self.walk.class(scope));
        let alternatives = self.lookup(scope, &key, receiver);
        let mut refusal = None;
        for alt in &alternatives {
            if !self.named_owner(alt.owner) {
                continue;
            }
            for binding in &alt.bindings {
                let Some(construction) = Construction::from_binding(binding) else {
                    continue;
                };
                let Some(class) = self.constructor(
                    construction.call,
                    binding.scope,
                    Some(&method),
                    construction.protocol(),
                ) else {
                    continue;
                };
                let fresh_suite = self.fresh_suite(alt, binding, callee, scope);
                let unique = alternatives.len() == 1
                    && self.receiver_unique(alt, binding, &key, callee, scope, fresh_suite);
                let result = class.and_then(|class| {
                    if !self.available(binding, receiver, scope) || self.type_only(binding) {
                        return Err(UnprovenCause::BindingUnavailable);
                    }
                    self.prove_instance_method(&class, &method, callee, scope)?;
                    if let Some((manager, _)) = construction.context {
                        let before_exit = binding.scope == scope && contains(manager, callee);
                        let entry = ContextUse {
                            manager,
                            value: construction.call,
                            scope: binding.scope,
                        };
                        if self.context_slots_rebound(&class, entry, !before_exit) {
                            return Err(UnprovenCause::MemberRebound);
                        }
                    }
                    let origin = Instance::Constructed(construction.call.id());
                    let fresh = fresh_suite
                        .filter(|_| self.unescaped_before(binding, &key, receiver, scope));
                    if self.instance_rebound(&origin, Some(&method), callee, scope, fresh)
                        || self.instance_context_rebound(&class, &origin, binding, callee, scope)
                    {
                        return Err(UnprovenCause::MemberRebound);
                    }
                    if !unique {
                        return Err(UnprovenCause::ReceiverNotUnique);
                    }
                    Ok(class)
                });
                match result {
                    Ok(class) => {
                        return Some(Resolution {
                            family: Family::Http,
                            grade: Grade::Exact,
                            identity: Ok(format!("{class}.{method}")),
                            #[cfg(any(test, feature = "inventory-evidence"))]
                            route: Some(super::super::evidence::Route::R2),
                            #[cfg(any(test, feature = "inventory-evidence"))]
                            receiver: Some(super::super::evidence::ReceiverProof::constructed(
                                &class,
                                construction.call,
                            )),
                        })
                    }
                    Err(cause) => {
                        refusal =
                            Some(refusal.map_or(cause, |prior: UnprovenCause| prior.min(cause)))
                    }
                }
            }
        }
        refusal.map(|cause| Resolution {
            family: Family::Http,
            grade: Grade::Exact,
            identity: Err(cause),
            #[cfg(any(test, feature = "inventory-evidence"))]
            route: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver: None,
        })
    }

    pub(super) fn construction_origin(
        &self,
        binding: &Binding<'tree>,
        owner: ScopeId,
    ) -> Option<usize> {
        if !self.named_owner(owner) {
            return None;
        }
        let construction = Construction::from_binding(binding)?;
        self.constructor(
            construction.call,
            binding.scope,
            None,
            construction.protocol(),
        )?
        .ok()?;
        Some(construction.call.id())
    }

    fn named_owner(&self, owner: ScopeId) -> bool {
        matches!(
            self.walk.scopes[owner].kind,
            ScopeKind::Module | ScopeKind::Function
        )
    }

    pub(super) fn fresh_suite(
        &self,
        alt: &Alternative<'_, 'tree>,
        binding: &Binding<'tree>,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> Option<Node<'tree>> {
        if alt.owner != scope || binding.scope != scope || binding.position > at.start_byte() {
            return None;
        }
        let suite = enclosing_suite(at)?;
        if direct(suite, binding.node) {
            return Some(suite);
        }
        if matches!(binding.kind, Kind::Context)
            && Construction::from_binding(binding)?
                .context?
                .0
                .child_by_field_name("body")
                == Some(suite)
        {
            return Some(suite);
        }
        None
    }

    pub(super) fn receiver_unique(
        &self,
        alt: &Alternative<'_, 'tree>,
        binding: &Binding<'tree>,
        key: &str,
        at: Node<'tree>,
        scope: ScopeId,
        fresh: Option<Node<'tree>>,
    ) -> bool {
        !self.name_rebound(alt.owner, key, at, scope)
            && alt.bindings.iter().all(|other| {
                std::ptr::eq(*other, binding)
                    || (other.scope == scope
                        && other.position > at.start_byte()
                        && matches!(
                            other.node.kind(),
                            "assignment" | "augmented_assignment" | "delete_statement"
                        )
                        && fresh.is_some_and(|suite| direct(suite, other.node)))
            })
    }

    pub(super) fn unescaped_before(
        &self,
        binding: &Binding<'tree>,
        key: &str,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> bool {
        self.walk.nodes.iter().all(|(node, context)| {
            if node.kind() != "identifier"
                || !context.evaluated
                || node.start_byte() < binding.position
                || node.end_byte() > at.end_byte()
                || name(self.parsed.node_text(node), self.walk.class(context.scope)) != key
                || !self.lookup(context.scope, key, *node).iter().any(|alt| {
                    alt.bindings
                        .iter()
                        .any(|entry| std::ptr::eq(*entry, binding))
                })
            {
                return true;
            }
            context.scope == scope && member_reference(*node)
        })
    }

    pub(super) fn instance_rebound(
        &self,
        origin: &Instance,
        method: Option<&str>,
        at: Node<'tree>,
        scope: ScopeId,
        fresh: Option<Node<'tree>>,
    ) -> bool {
        let read = self.read(at, scope);
        self.writes.iter().any(|write| {
            matches!(&write.slot, Slot::Instance { origin: written, key }
            if written == origin && key.as_deref().is_none_or(|key| {
                Some(key) == method || matches!(key, "__class__" | "__dict__")
            })) && write.reaches(read)
                && fresh.is_none_or(|suite| !write.follows_in_suite(read, suite))
        })
    }
}

pub(super) fn assignment_value(mut node: Node<'_>) -> Option<Node<'_>> {
    while node.kind() == "assignment" {
        node = node.child_by_field_name("right")?;
    }
    Some(ungroup(node))
}

fn enclosing_suite(node: Node<'_>) -> Option<Node<'_>> {
    std::iter::successors(node.parent(), Node::parent)
        .find(|parent| matches!(parent.kind(), "block" | "module"))
}

fn member_reference(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "parenthesized_expression" => node = parent,
            "attribute" => {
                return parent.child_by_field_name("object") == Some(node)
                    || parent.child_by_field_name("attribute") == Some(node)
            }
            "keyword_argument" => return parent.child_by_field_name("name") == Some(node),
            _ => return false,
        }
    }
    false
}
