//! One extraction's exact inventory membership, bound to its immutable file and owner.
use super::{byte_range_eq, CallOwnerDecision, Language, Node, ParsedFile};
use std::cell::OnceCell;

/// Query captures may repeat or include nested owners. Only inventory membership
/// is reused; every capture still runs the original ownership decisions.
/// No state survives this borrow or can be supplied for a different owner.
pub(super) struct CallMembership<'file, 'tree> {
    file: &'file ParsedFile,
    owner: Node<'tree>,
    listed: OnceCell<bool>,
}

impl<'file, 'tree> CallMembership<'file, 'tree> {
    pub(super) fn new(file: &'file ParsedFile, owner: Node<'tree>) -> Self {
        Self {
            file,
            owner,
            listed: OnceCell::new(),
        }
    }

    pub(super) fn is_owned(&self, call: Node<'_>) -> bool {
        matches!(
            self.decision(call),
            CallOwnerDecision::Owned | CallOwnerDecision::LegacyInventory
        )
    }

    fn decision(&self, call: Node<'_>) -> CallOwnerDecision {
        let file = self.file;
        let owner = &self.owner;
        if !matches!(
            file.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || owner.parent().is_none()
        {
            return CallOwnerDecision::LegacyInventory;
        }
        if !file
            .language
            .callable_boundary_node_types()
            .contains(&owner.kind())
            || !*self.listed.get_or_init(|| {
                file.all_functions().iter().any(|candidate| {
                    candidate.kind() == owner.kind() && byte_range_eq(candidate, owner)
                })
            })
        {
            return CallOwnerDecision::Unsupported;
        }
        if call.start_byte() < owner.start_byte() || owner.end_byte() < call.end_byte() {
            return CallOwnerDecision::Outside;
        }

        let mut owner_is_in_class = false;
        let mut ancestor = owner.parent();
        while let Some(node) = ancestor {
            if ParsedFile::is_rvalue_class_node(node.kind()) {
                owner_is_in_class = true;
                break;
            }
            ancestor = node.parent();
        }
        if owner_is_in_class
            || file.rvalue_has_class_ancestor_before_owner(
                call,
                owner.start_byte(),
                owner.end_byte(),
            )
        {
            return CallOwnerDecision::LegacyInventory;
        }

        let mut current = Some(call);
        while let Some(node) = current {
            if file
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
            {
                let is_owner = byte_range_eq(&node, owner);
                let in_execution_region = ["body", "parameters"].into_iter().any(|field| {
                    node.child_by_field_name(field).is_some_and(|region| {
                        region.start_byte() <= call.start_byte()
                            && call.end_byte() <= region.end_byte()
                    })
                });
                if in_execution_region {
                    return if is_owner {
                        CallOwnerDecision::Owned
                    } else {
                        CallOwnerDecision::Nested
                    };
                }

                let in_computed_method_name = node.kind() == "method_definition"
                    && node.child_by_field_name("name").is_some_and(|name| {
                        name.start_byte() <= call.start_byte() && call.end_byte() <= name.end_byte()
                    });
                if in_computed_method_name {
                    if is_owner {
                        return CallOwnerDecision::Outside;
                    }
                } else if is_owner {
                    return CallOwnerDecision::Owned;
                } else {
                    return CallOwnerDecision::Unsupported;
                }
            }
            current = node.parent();
        }
        CallOwnerDecision::Outside
    }
}

impl ParsedFile {
    // The manual visitors retain a fresh membership predicate for every call.
    fn call_capture_owner(&self, owner: &Node<'_>, call: Node<'_>) -> CallOwnerDecision {
        CallMembership::new(self, *owner).decision(call)
    }

    pub(super) fn call_capture_is_owned(&self, owner: &Node<'_>, call: Node<'_>) -> bool {
        matches!(
            self.call_capture_owner(owner, call),
            CallOwnerDecision::Owned | CallOwnerDecision::LegacyInventory
        )
    }
}
