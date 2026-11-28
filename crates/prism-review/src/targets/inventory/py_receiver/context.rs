use super::{census::Binding, lookup::direct, slots::Instance, Resolver};
use crate::targets::inventory::python::{
    source::{children, ungroup},
    walk::ScopeId,
};
use tree_sitter::Node;

#[derive(Clone, Copy)]
pub(super) enum Protocol {
    Sync,
    Async,
}

impl Protocol {
    pub(super) fn at(manager: Node<'_>) -> Self {
        if manager
            .child(0)
            .is_some_and(|child| child.kind() == "async")
        {
            Self::Async
        } else {
            Self::Sync
        }
    }

    pub(super) fn accepts(self, class: &str) -> bool {
        super::py_tables::context_class(class, matches!(self, Self::Async))
    }

    fn slots(self) -> (&'static str, &'static str) {
        match self {
            Self::Sync => ("__enter__", "__exit__"),
            Self::Async => ("__aenter__", "__aexit__"),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ContextUse<'tree> {
    pub manager: Node<'tree>,
    pub value: Node<'tree>,
    pub scope: ScopeId,
}

impl<'tree> ContextUse<'tree> {
    pub(super) fn from_item(item: Node<'tree>, scope: ScopeId) -> Option<Self> {
        let value = ungroup(item.child_by_field_name("value")?);
        let value = if value.kind() == "as_pattern" {
            ungroup(children(value).next()?)
        } else {
            value
        };
        if !matches!(value.kind(), "identifier" | "attribute") {
            return None;
        }
        let manager = item.parent()?.parent()?;
        (manager.kind() == "with_statement").then_some(Self {
            manager,
            value,
            scope,
        })
    }
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn context_slots_rebound(
        &self,
        class: &str,
        entry: ContextUse<'tree>,
        exit_reaches: bool,
    ) -> bool {
        let (enter, exit) = Protocol::at(entry.manager).slots();
        let capture = self.read(entry.value, entry.scope);
        let direct_module_body = (entry.scope == 0
            && direct(self.walk.scopes[0].body, entry.manager))
        .then(|| entry.manager.child_by_field_name("body"))
        .flatten();
        let rebound = |slot| {
            self.reads_instance_member_at(class, slot, capture, |write| {
                // This suite runs after both callables were captured. A
                // function-local class write can reach a prior activation.
                direct_module_body.is_none_or(|body| !write.follows_in_suite(capture, body))
            })
        };
        rebound(enter) || (exit_reaches && rebound(exit))
    }

    pub(super) fn instance_context_rebound(
        &self,
        class: &str,
        instance: &Instance,
        binding: &Binding<'tree>,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> bool {
        self.census.context_uses.iter().copied().any(|entry| {
            if !Protocol::at(entry.manager).accepts(class)
                || (entry.scope == binding.scope && entry.value.start_byte() < binding.position)
                || !self.names_instance(entry.value, entry.scope, instance)
            {
                return false;
            }
            let same_scope = scope == entry.scope;
            let back_edge = same_scope && can_repeat(entry.manager, at, binding);
            let persistent = binding.scope != entry.scope;
            if same_scope && entry.value.start_byte() > at.start_byte() && !back_edge && !persistent
            {
                return false;
            }
            let before_exit =
                same_scope && contains(entry.manager, at) && !back_edge && !persistent;
            self.context_slots_rebound(class, entry, !before_exit)
        })
    }
}

fn can_repeat(manager: Node<'_>, at: Node<'_>, binding: &Binding<'_>) -> bool {
    std::iter::successors(at.parent(), Node::parent).any(|loop_node| {
        matches!(loop_node.kind(), "for_statement" | "while_statement")
            && contains(loop_node, manager)
            && !loop_node.child_by_field_name("body").is_some_and(|body| {
                direct(body, binding.node)
                    && binding.position <= manager.start_byte()
                    && binding.position <= at.start_byte()
            })
    })
}

pub(super) fn contains(parent: Node<'_>, child: Node<'_>) -> bool {
    parent.start_byte() <= child.start_byte() && child.end_byte() <= parent.end_byte()
}
