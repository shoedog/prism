use super::{
    census::{Binding, Import},
    lookup::direct,
    writes::Write,
    Resolver,
};
use crate::targets::inventory::{py_tables, python::walk::ScopeId};
use tree_sitter::Node;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Instance {
    Constructed(usize),
    OwningSelf(ScopeId),
    Field { owner: ScopeId, name: String },
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Slot {
    Name {
        owner: ScopeId,
        name: Option<String>,
    },
    Member {
        object: Option<String>,
        key: Option<String>,
        definite: bool,
    },
    ParameterMember {
        key: Option<String>,
    },
    Instance {
        origin: Instance,
        key: Option<String>,
    },
}

impl Slot {
    pub(super) fn matches_name(&self, owner: ScopeId, name: &str) -> bool {
        matches!(self, Self::Name { owner: written, name: key }
            if *written == owner && key.as_deref().is_none_or(|key| key == name))
    }

    pub(super) fn matches_member(&self, object: &str, key: &str) -> bool {
        matches!(self, Self::Member { object: written, key: member, .. }
            if written.as_deref().is_none_or(|path| py_tables::class_object(path) == py_tables::class_object(object))
            && member.as_deref().is_none_or(|member| member == key))
    }

    pub(super) fn definite(&self) -> bool {
        matches!(
            self,
            Self::Name { name: Some(_), .. } | Self::Member { definite: true, .. }
        )
    }
}

#[derive(Clone, Copy)]
pub(super) enum MemberRead {
    Root,
    Receiver,
}

impl MemberRead {
    fn matches(self, slot: &Slot, object: &str, key: &str) -> bool {
        // Root identity inherits the conservative S2a parameter-write check.
        // Receiver proofs track the identities of their modeled write targets.
        slot.matches_member(object, key)
            || matches!((self, slot), (Self::Root, Slot::ParameterMember { key: written })
                if written.as_deref().is_none_or(|written| written == key))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Read {
    pub scope: ScopeId,
    pub byte: usize,
    pub direct: bool,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn name_rebound(
        &self,
        owner: ScopeId,
        name: &str,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> bool {
        let read = self.read(at, scope);
        self.writes
            .iter()
            .any(|write| write.slot.matches_name(owner, name) && write.reaches(read))
    }

    pub(super) fn builtin_rebound(&self, name: &str, at: Node<'tree>, scope: ScopeId) -> bool {
        self.reads_member_at(
            "builtins",
            name,
            self.read(at, scope),
            MemberRead::Root,
            |_| true,
        )
    }

    pub(super) fn member_rebound(
        &self,
        import: &Import,
        binding: &Binding<'tree>,
        at: Node<'tree>,
        scope: ScopeId,
        rest: &[String],
        member_read: MemberRead,
    ) -> bool {
        let mut parts = import.path.split('.');
        let Some(root) = parts.next() else {
            return false;
        };
        let mut owner = root.to_owned();
        if import.from || import.alias {
            let captured = Read {
                scope: binding.scope,
                byte: binding.position,
                direct: direct(self.walk.scopes[binding.scope].body, binding.node),
            };
            for key in parts {
                if self.reads_member_at(&owner, key, captured, member_read, |_| true) {
                    return true;
                }
                owner.push('.');
                owner.push_str(key);
            }
        } else {
            owner.clone_from(&import.path);
        }

        let read = self.read(at, scope);
        for key in rest {
            if self.reads_member_at(&owner, key, read, member_read, |_| true) {
                return true;
            }
            owner.push('.');
            owner.push_str(key);
        }
        false
    }

    pub(super) fn reads_member(&self, object: &str, key: &str, read: Read) -> bool {
        self.reads_member_at(object, key, read, MemberRead::Receiver, |_| true)
    }

    pub(super) fn reads_member_at(
        &self,
        object: &str,
        key: &str,
        read: Read,
        member_read: MemberRead,
        reaches: impl Fn(&Write) -> bool,
    ) -> bool {
        let owners = py_tables::method_owners(object, key);
        self.writes
            .iter()
            .filter(|write| write.reaches(read) && reaches(write))
            .any(|write| {
                if owners.is_empty() {
                    member_read.matches(&write.slot, object, key)
                        || member_read.matches(&write.slot, object, "__class__")
                } else {
                    owners
                        .iter()
                        .any(|owner| member_read.matches(&write.slot, owner, key))
                        || owners[..owners.len() - 1]
                            .iter()
                            .any(|owner| member_read.matches(&write.slot, owner, "__bases__"))
                }
            })
    }

    pub(super) fn read(&self, at: Node<'tree>, scope: ScopeId) -> Read {
        let suite = self.walk.scopes[scope].body;
        let statement = std::iter::successors(Some(at), |node| node.parent())
            .take_while(|node| *node != suite)
            .last();
        let direct = statement.is_some_and(|stmt| {
            !matches!(
                stmt.kind(),
                "if_statement"
                    | "for_statement"
                    | "while_statement"
                    | "try_statement"
                    | "match_statement"
                    | "with_statement"
            )
        });
        Read {
            scope,
            byte: at.start_byte(),
            direct,
        }
    }
}
