use super::FieldUse;
use crate::targets::inventory::{
    py_receiver::{
        census::Kind,
        slots::{Instance, Slot},
        Resolver,
    },
    python::{
        source::{children, name, ungroup},
        walk::{ScopeId, ScopeKind},
    },
    UnprovenCause,
};
use tree_sitter::Node;

const LOOKUP_STORAGE: &[&str] = &[
    "__getattribute__",
    "__getattr__",
    "__setattr__",
    "__delattr__",
    "__slots__",
    "__dict__",
    "__class__",
    "__bases__",
];

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn field_site_veto(
        &self,
        site: &FieldUse<'_, 'tree>,
        seed: Node<'tree>,
    ) -> Option<UnprovenCause> {
        let read = self.read(site.callee, site.scope);
        let protocol = LOOKUP_STORAGE.iter().copied();
        let methods = ["__init__", site.using_name.as_str()];
        if self
            .class_field_bindings(site.owner, &site.field)
            .any(|binding| !matches!(binding.kind, Kind::Delete))
            || protocol.clone().any(|key| {
                self.class_field_bindings(site.owner, key)
                    .any(|binding| !matches!(binding.kind, Kind::Delete))
            })
            || methods.iter().any(|key| {
                self.class_field_bindings(site.owner, key)
                    .any(|binding| binding.node.kind() != "function_definition")
            })
        {
            return Some(UnprovenCause::MemberRebound);
        }

        let lookup = protocol.chain(methods).chain([site.method.as_str()]);
        let mut field_write = false;
        for write in &self.writes {
            if !write.reaches(read) || write.at(seed) {
                continue;
            }
            let key = match &write.slot {
                Slot::Member { key, .. }
                | Slot::ParameterMember { key }
                | Slot::Instance { key, .. } => key,
                Slot::Name { .. } => continue,
            };
            let matches = |watched| key.as_deref().is_none_or(|key| key == watched);
            if lookup.clone().any(matches) {
                return Some(UnprovenCause::MemberRebound);
            }
            field_write |= matches(&site.field);
        }

        (field_write
            || self.field_definitions(site.owner, "__init__") > 1
            || self.field_definitions(site.owner, &site.using_name) > 1
            || !self.field_parameter_unique(site.using, "self", site.self_ref, site.scope)
            || self.direct_subclass_binds_field(site.owner, &site.field))
        .then_some(UnprovenCause::ReceiverNotUnique)
    }

    pub(super) fn indexed_field_seed(
        &self,
        owner: ScopeId,
        field: &str,
        seed: Node<'tree>,
    ) -> bool {
        self.writes.iter().any(|write| write.at(seed) && matches!(
            &write.slot, Slot::Instance { origin: Instance::OwningSelf(scope), key: Some(key) }
            if *scope == owner && key == field
        ))
    }

    fn direct_subclass_binds_field(&self, owner: ScopeId, field: &str) -> bool {
        let class = self.walk.scopes[owner].node;
        self.walk
            .scopes
            .iter()
            .enumerate()
            .any(|(scope, candidate)| {
                candidate.kind == ScopeKind::Class
                    && scope != owner
                    && self
                        .class_field_bindings(scope, field)
                        .any(|binding| !matches!(binding.kind, Kind::Delete))
                    && candidate
                        .node
                        .child_by_field_name("superclasses")
                        .into_iter()
                        .flat_map(children)
                        .map(ungroup)
                        .filter_map(|base| {
                            if base.kind() == "subscript" {
                                base.child_by_field_name("value").map(ungroup)
                            } else {
                                Some(base)
                            }
                        })
                        .any(|base| {
                            if base.kind() == "attribute" {
                                return base.child_by_field_name("attribute").is_some_and(|attr| {
                                    let key = name(self.parsed.node_text(&attr), None);
                                    self.walk.class(owner) == Some(key.as_str())
                                });
                            }
                            if base.kind() != "identifier" {
                                return false;
                            }
                            let key = name(
                                self.parsed.node_text(&base),
                                self.walk.class(candidate.parent),
                            );
                            self.lookup(candidate.parent, &key, base)
                                .iter()
                                .flat_map(|alt| &alt.bindings)
                                .any(|binding| {
                                    binding.node == class
                                        && self.available(binding, base, candidate.parent)
                                })
                        })
            })
    }
}
